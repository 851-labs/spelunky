//! Runs `wiki-data` on a bundle extracted from the real reference zips with the committed specs: the
//! output must be deterministic, its citations must point at the code they quote, and the values must be
//! the ones the GML says (checked by reading it).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn wiki_data(assets: &Path, specs: &Path, out: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_wiki-data"))
        .arg("--assets")
        .arg(assets)
        .arg("--specs")
        .arg(specs)
        .arg("--out")
        .arg(out)
        .output()
        .unwrap()
}

fn entity<'a>(data: &'a Value, slug: &str) -> &'a Value {
    data["entities"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["slug"] == slug)
        .unwrap_or_else(|| panic!("no entity {slug}"))
}

fn stat<'a>(data: &'a Value, slug: &str, id: &str) -> &'a Value {
    entity(data, slug)["stats"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == id)
        .unwrap_or_else(|| panic!("{slug} has no stat {id}"))
}

/// A table's values, in row order.
fn rows(stat: &Value) -> Vec<i64> {
    stat["table"]["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["value"].as_i64().unwrap())
        .collect()
}

/// Line `line` (1-based) of a file in the extracted `gml/` tree.
fn gml_line(assets: &Path, file: &str, line: u64) -> String {
    let text = fs::read_to_string(assets.join(file)).unwrap();
    text.lines()
        .nth(line as usize - 1)
        .unwrap_or_else(|| panic!("{file} has no line {line}"))
        .to_owned()
}

#[test]
fn extraction_is_deterministic_cited_and_right() {
    let tmp = Path::new(env!("CARGO_TARGET_TMPDIR")).join("wiki-data-test");
    let _ = fs::remove_dir_all(&tmp);
    let assets_dir = tmp.join("assets");
    let status = Command::new(env!("CARGO_BIN_EXE_extract"))
        .arg("--reference")
        .arg(repo().join("reference"))
        .arg("--out")
        .arg(&assets_dir)
        .status()
        .unwrap();
    assert!(status.success());

    let specs = repo().join("wiki/data");
    let (a, b) = (tmp.join("a/stats.json"), tmp.join("b/stats.json"));
    for out in [&a, &b] {
        let run = wiki_data(&assets_dir, &specs, out);
        assert!(run.status.success(), "{run:?}");
    }
    let bytes = fs::read(&a).unwrap();
    assert_eq!(bytes, fs::read(&b).unwrap(), "two runs differ");
    let data: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(data["room_speed"], 30);
    assert!(data["notice"].as_str().unwrap().contains("Mossmouth"));

    // Every fact's citation is the line of the gml/ tree that starts the code it quotes.
    let mut facts = 0;
    for e in data["entities"].as_array().unwrap() {
        for s in e["stats"].as_array().unwrap() {
            for f in s["facts"].as_array().unwrap() {
                let at = &f["at"];
                let line = gml_line(
                    &assets_dir,
                    at["file"].as_str().unwrap(),
                    at["line"].as_u64().unwrap(),
                );
                let source = at["source"].as_str().unwrap();
                let first = source.lines().next().unwrap();
                assert!(
                    line.contains(first),
                    "{} {}: {line:?} doesn't have {first:?}",
                    e["slug"],
                    s["id"]
                );
                facts += 1;
            }
        }
    }
    assert!(facts > 100, "{facts} facts");
    // And every object's create-event variable is set on its line.
    let snake = &data["objects"]["oSnake"];
    assert_eq!(snake["parent"], "oEnemy");
    assert_eq!(snake["type"], "Snake");
    for (name, init) in snake["init"].as_object().unwrap() {
        let line = gml_line(
            &assets_dir,
            init["file"].as_str().unwrap(),
            init["line"].as_u64().unwrap(),
        );
        assert!(line.contains(name.as_str()), "{name}: {line:?}");
    }
    // Inherited from oEnemy's create event.
    assert_eq!(snake["init"]["stunTime"]["value"], 200);
    assert_eq!(
        snake["init"]["stunTime"]["file"],
        "gml/objects/oEnemy/create.gml"
    );

    // Spot checks, each read off the GML.
    // oSnake's create event: `hp = 1;`; oEnemy's touch, which a snake inherits: `global.plife -= 1;`.
    assert_eq!(stat(&data, "snake", "hp")["value"], 1);
    let damage = stat(&data, "snake", "damage");
    assert_eq!(damage["value"], 1);
    assert_eq!(damage["facts"][0]["at"]["object"], "oEnemy");
    assert_eq!(
        damage["facts"][0]["when"][2],
        serde_json::json!({"if": "other.invincible == 0"})
    );
    // oSnake's step: a 1-in-100 chance a step to pause for `rand(20,50)` steps, walking at 1.
    assert_eq!(stat(&data, "snake", "pause-chance")["value"], 100);
    let pause = stat(&data, "snake", "pause");
    assert_eq!((&pause["min"], &pause["max"]), (&20.into(), &50.into()));
    assert_eq!(pause["facts"][0]["range"]["integer"], true);
    assert_eq!(stat(&data, "snake", "walk-speed")["value"], 1);
    // oBat's step: `dist < 90` to wake, `dist < 160` to chase.
    assert_eq!(stat(&data, "bat", "wake-range")["value"], 90);
    assert_eq!(stat(&data, "bat", "chase-range")["value"], 160);
    // oCaveman: `hp = 3;`, walks at 1.5 and charges at 3.
    assert_eq!(stat(&data, "caveman", "hp")["value"], 3);
    assert_eq!(stat(&data, "caveman", "walk-speed")["value"], 1.5);
    assert_eq!(stat(&data, "caveman", "charge-speed")["value"], 3);
    // oGiantSpiderHang's `hp = 10;`; oGiantSpider's own touch: `global.plife -= 2;`.
    assert_eq!(stat(&data, "giant-spider", "hp")["value"], 10);
    assert_eq!(stat(&data, "giant-spider", "damage")["value"], 2);
    let web = stat(&data, "giant-spider", "web-timer");
    assert_eq!((&web["min"], &web["max"]), (&100.into(), &1000.into()));
    // oShotgun's `cost = 15000;`, 10% more a level past the second (scrShopItemsGen), 6 pellets.
    assert_eq!(stat(&data, "shotgun", "price")["value"], 15000);
    let by_level = rows(stat(&data, "shotgun", "price-by-level"));
    assert_eq!(by_level.len(), 16);
    assert_eq!(&by_level[..4], [15000, 15000, 16500, 18000]);
    assert_eq!(by_level[15], 36000);
    assert_eq!(stat(&data, "shotgun", "pellets")["value"], 6);
    assert_eq!(stat(&data, "shotgun", "cooldown")["value"], 40);
    assert_eq!(stat(&data, "bomb-bag", "bombs")["value"], 3);
    assert_eq!(stat(&data, "ankh", "price")["value"], 50000);
    // oGoldNugget's `value = 500;`, collected as `gem.value + ceil(gem.value / 4) * global.levelType`.
    assert_eq!(stat(&data, "gold-nugget", "value")["value"], 500);
    assert_eq!(
        rows(stat(&data, "gold-nugget", "value-by-area")),
        [500, 625, 750, 875]
    );
    // oGoldIdol's 5000, times `global.levelType + 1`.
    assert_eq!(
        rows(stat(&data, "golden-idol", "value-by-area")),
        [5000, 10000, 15000, 20000]
    );

    // A pattern that matches nothing fails the run, so a spec can't silently go stale.
    let bad = tmp.join("bad");
    fs::create_dir_all(&bad).unwrap();
    fs::write(
        bad.join("x.toml"),
        "[[entity]]\nslug = \"snake\"\nname = \"Snake\"\nkind = \"monster\"\nobject = \"oSnake\"\n\
         [[entity.stat]]\nid = \"hp\"\nlabel = \"Health\"\nfind = \"hitPoints = ?\"\n",
    )
    .unwrap();
    let run = wiki_data(&assets_dir, &bad, &tmp.join("bad.json"));
    assert!(!run.status.success());
    assert!(String::from_utf8_lossy(&run.stderr).contains("matches nothing"));
}
