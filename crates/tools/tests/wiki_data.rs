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
    let relations = fs::read(a.with_file_name("relations.json")).unwrap();
    assert_eq!(
        relations,
        fs::read(b.with_file_name("relations.json")).unwrap(),
        "two runs' relations differ"
    );
    check_relations(&assets_dir, &serde_json::from_slice(&relations).unwrap());
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

/// The `at` of every relation, with what it's about, for the citation check.
fn relation_citations(data: &Value) -> Vec<(String, &Value)> {
    let mut out = Vec::new();
    for key in ["spawns_in", "drops"] {
        for (object, list) in data[key].as_object().unwrap() {
            for r in list.as_array().unwrap() {
                out.push((format!("{key} {object}"), &r["at"]));
            }
        }
    }
    for shop in data["shops"].as_array().unwrap() {
        for sale in shop["sells"].as_array().unwrap() {
            out.push((format!("shop {}", shop["slug"]), &sale["at"]));
        }
    }
    for d in data["damage"].as_array().unwrap() {
        out.push((format!("damage {}", d["by"]), &d["at"]));
    }
    out
}

/// The facts of `key` (`spawns_in`, `drops`) about `object` whose citation's file ends with `file`.
fn about<'a>(data: &'a Value, key: &str, object: &str, file: &str) -> Vec<&'a Value> {
    data[key][object]
        .as_array()
        .unwrap_or_else(|| panic!("no {key} for {object}"))
        .iter()
        .filter(|r| r["at"]["file"].as_str().unwrap().ends_with(file))
        .collect()
}

/// The objects a shop sells.
fn sells<'a>(data: &'a Value, shop: &str) -> Vec<&'a str> {
    let shop = data["shops"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["slug"] == shop)
        .unwrap_or_else(|| panic!("no shop {shop}"));
    shop["sells"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["object"].as_str().unwrap())
        .collect()
}

/// The damage facts whose code is `by` and whose citation quotes `source`.
fn damage<'a>(data: &'a Value, by: &str, source: &str) -> &'a Value {
    data["damage"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["by"] == by && d["at"]["source"] == source)
        .unwrap_or_else(|| panic!("no damage {source:?} in {by}"))
}

fn strings(v: &Value) -> Vec<&str> {
    v.as_array()
        .map(|a| a.iter().map(|s| s.as_str().unwrap()).collect())
        .unwrap_or_default()
}

/// relations.json: cited, and right where checked by reading the GML.
fn check_relations(assets: &Path, data: &Value) {
    let citations = relation_citations(data);
    assert!(citations.len() > 500, "{} citations", citations.len());
    for (what, at) in citations {
        let line = gml_line(
            assets,
            at["file"].as_str().unwrap(),
            at["line"].as_u64().unwrap(),
        );
        let first = at["source"].as_str().unwrap().lines().next().unwrap();
        assert!(
            line.contains(first),
            "{what}: {line:?} doesn't have {first:?}"
        );
    }

    // scrEntityGen: in the Mines (`global.levelType == 0`), a floor with air above gets a snake 1 time
    // in 60.
    let [snake] = about(data, "spawns_in", "oSnake", "scrEntityGen.gml")[..] else {
        panic!("one scrEntityGen snake");
    };
    assert_eq!(strings(&snake["areas"]), ["mines"]);
    assert!(snake["only_in"].is_null());
    assert_eq!(
        snake["when"].as_array().unwrap().last().unwrap()["if"],
        "rand(1, 60) == 1"
    );
    // scrRoomGen's snake pit rooms (`roomPath == 8` and `9`) hold the "s" and "S" tiles, which make
    // snakes; no other Mines room string has them.
    let pit = about(data, "spawns_in", "oSnake", "scrRoomGen.gml");
    let tiles: Vec<&str> = pit.iter().map(|s| s["tile"].as_str().unwrap()).collect();
    assert_eq!(tiles, ["s", "S"]);
    for s in &pit {
        assert_eq!(strings(&s["only_in"]), ["snake-pit"]);
    }
    // scrRoomGenYeti's copy of the snake pit is never built (the spec's `unreachable`).
    assert!(about(data, "spawns_in", "oSnake", "scrRoomGenYeti.gml").is_empty());
    // Bats in the Mines and the Jungle; scarabs only on dark levels; zombies only on the graveyard
    // level, where piranhas are dead fish instead.
    let bat_areas: Vec<Vec<&str>> = data["spawns_in"]["oBat"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| strings(&s["areas"]))
        .collect();
    assert_eq!(bat_areas, [["mines"], ["jungle"]]);
    for s in data["spawns_in"]["oScarab"].as_array().unwrap() {
        assert_eq!(strings(&s["only_in"]), ["dark-level"]);
    }
    let [zombie] = &data["spawns_in"]["oZombie"].as_array().unwrap()[..] else {
        panic!("one zombie spawn");
    };
    assert_eq!(strings(&zombie["only_in"]), ["restless-dead"]);
    let [piranha] = &data["spawns_in"]["oPiranha"].as_array().unwrap()[..] else {
        panic!("one piranha spawn");
    };
    assert_eq!(strings(&piranha["not_in"]), ["restless-dead"]);
    // A Mines brick may hold a gem (oBrick's create event, which generation follows).
    let gem = about(data, "spawns_in", "oSapphireBig", "oBrick/create.gml");
    assert_eq!(strings(&gem[0]["areas"]), ["mines"]);
    // Olmec is placed in his room by the room editor.
    assert_eq!(data["placed_in"]["oOlmec"]["rOlmec"], 1);

    // scrShopItemsGen: the Weapon shop's `rand(1,10)==1` shotgun, at oShotgun's `cost = 15000`; the
    // General Store's last `else` has none; the dice house's prize is scrGenerateItem's high-end set
    // (`argument2 == 1`), which has one.
    let weapons = sells(data, "weapon-shop");
    for item in [
        "oShotgun",
        "oPistol",
        "oMachete",
        "oBow",
        "oWebCannon",
        "oBombBox",
        "oBombBag",
    ] {
        assert!(
            weapons.contains(&item),
            "the weapon shop doesn't sell {item}"
        );
    }
    assert_eq!(weapons.len(), 8, "{weapons:?}");
    let general = sells(data, "general-store");
    assert!(!general.contains(&"oShotgun"));
    assert!(general.contains(&"oRopePile"));
    assert!(sells(data, "dice-house").contains(&"oShotgun"));
    assert!(!sells(data, "dice-house").contains(&"oRopePile"));
    assert_eq!(
        strings(&data["sold_in"]["oShotgun"]),
        ["weapon-shop", "dice-house"]
    );
    let shop = &data["shops"].as_array().unwrap()[2];
    let shotgun = shop["sells"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["object"] == "oShotgun")
        .unwrap();
    assert_eq!(shotgun["price"], 15000);
    assert_eq!(shotgun["slug"], "shotgun");
    assert_eq!(sells(data, "ankh-shop"), ["oAnkh"]);

    // The player's step opens a crate it's touching into one of 21 things, a jetpack 1 time in 500; a
    // chest into gems (or, 1 time in 12, a bomb).
    let crate_contents: Vec<&str> = data["drops"]["oCrate"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["object"].as_str().unwrap())
        .collect();
    assert_eq!(crate_contents.len(), 21);
    assert!(crate_contents.contains(&"oJetpack") && !crate_contents.contains(&"oRuby"));
    assert_eq!(data["drops"]["oCrate"][1]["by"], "oPlayer1:step");
    let chest: Vec<&str> = data["drops"]["oChest"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["object"].as_str().unwrap())
        .collect();
    assert!(chest.contains(&"oBomb") && chest.contains(&"oRuby"));
    // A broken jar may let out a spider or a snake.
    let jar = about(data, "drops", "oJar", "oJar/destroy.gml");
    assert!(jar.iter().any(|d| d["object"] == "oSpider"));
    assert!(jar.iter().any(|d| d["object"] == "oSnake"));

    // oEnemy's touch, which the snake runs: `global.plife -= 1` on the player; a stomp, `hp -= 1 * ...`
    // on the enemy; the whip's `hp -= other.damage`.
    let touch = damage(data, "oEnemy:collision_oCharacter", "global.plife -= 1");
    assert_eq!(strings(&touch["victims"]), ["oPlayer1"]);
    assert_eq!(strings(&touch["sources"]), ["oEnemy"]);
    assert_eq!(touch["value"], 1);
    assert!(strings(&touch["runs_as"]).contains(&"oSnake"));
    assert!(!strings(&touch["runs_as"]).contains(&"oCaveman"));
    let whip = damage(data, "oEnemy:collision_oWhip", "hp -= other.damage");
    assert_eq!(strings(&whip["victims"]), ["oEnemy"]);
    assert_eq!(strings(&whip["sources"]), ["oWhip"]);
    // A bomb's explosion: `other.hp -= 30` on any enemy.
    let bomb = damage(data, "oExplosion:collision_oEnemy", "other.hp -= 30");
    assert_eq!(strings(&bomb["victims"]), ["oEnemy"]);
    assert_eq!(bomb["value"], 30);
    // The player's step: lava (`collision_point(x, y+6, oLava, 0, 0)`) takes 99; spikes, through the
    // `colSpikes` it sets from a collision test, 99 too; an arrow 2.
    let player = |source: &str| -> Vec<&Value> {
        data["damage"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|d| d["by"] == "oPlayer1:step" && strings(&d["sources"]) == [source])
            .collect()
    };
    assert_eq!(player("oLava")[0]["value"], 99);
    assert_eq!(player("oSpikes")[0]["value"], 99);
    assert_eq!(player("oArrow")[0]["value"], 2);
    // An enemy on spikes or in lava: `hp = 0`.
    let spiked = data["damage"].as_array().unwrap().iter().any(|d| {
        d["by"] == "oEnemy:step" && d["find"] == "hp = 0" && strings(&d["sources"]) == ["oSpikes"]
    });
    assert!(spiked);
}
