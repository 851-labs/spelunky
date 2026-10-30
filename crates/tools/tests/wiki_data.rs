//! Runs `wiki-data` on a bundle extracted from the real reference zips with the committed specs: the
//! output must be deterministic, its citations must point at the code they quote, and the values must be
//! the ones the GML says (checked by reading it).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

fn repo() -> PathBuf {
    transpile::reference::repo_root()
}

/// Runs it, with the site's snapshots going to `site/` beside `out`.
fn wiki_data(assets: &Path, specs: &Path, out: &Path, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_wiki-data"))
        .arg("--assets")
        .arg(assets)
        .arg("--specs")
        .arg(specs)
        .arg("--out")
        .arg(out)
        .arg("--site")
        .arg(site(out).join("stats.json"))
        .args(extra)
        .output()
        .unwrap()
}

fn site(out: &Path) -> PathBuf {
    out.with_file_name("site")
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
        let run = wiki_data(&assets_dir, &specs, out, &[]);
        assert!(run.status.success(), "{run:?}");
    }
    // The site's snapshots: the same every run, what --check accepts, and the stats' values and lines
    // and the relations a page checks against.
    for file in ["stats.json", "relations.json"] {
        assert_eq!(
            fs::read(site(&a).join(file)).unwrap(),
            fs::read(site(&b).join(file)).unwrap(),
            "two runs' {file} snapshots differ"
        );
    }
    let run = wiki_data(&assets_dir, &specs, &a, &["--check"]);
    assert!(run.status.success(), "{run:?}");
    let site_relations: Value =
        serde_json::from_slice(&fs::read(site(&a).join("relations.json")).unwrap()).unwrap();
    check_site_relations(&site_relations);
    let site: Value =
        serde_json::from_slice(&fs::read(site(&a).join("stats.json")).unwrap()).unwrap();
    let hp = &site["entities"]["snake"]["stats"]["hp"];
    assert_eq!(hp["value"], 1);
    assert_eq!(hp["at"][0], "objects/oSnake/create.gml:10");
    assert_eq!(
        site["objects"]["oSnake"]["init"]["image_speed"]["value"],
        0.4
    );
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
    // oGoldNugget's `value = 500;`, collected as `gem.value + ceil(gem.value / 4) * global.levelType`, which
    // is 4 on level 16, Olmec's Lair.
    assert_eq!(stat(&data, "gold-nugget", "value")["value"], 500);
    assert_eq!(
        rows(stat(&data, "gold-nugget", "value-by-area")),
        [500, 625, 750, 875, 1000]
    );
    // A gem's `alarm[0] = 20;`, which sets `canCollect` (oTreasure's create leaves it false), and the
    // Diamond's ceil(5000 / 4) a level type.
    assert_eq!(stat(&data, "emerald", "collect-delay")["value"], 20);
    assert_eq!(stat(&data, "diamond", "collect-delay")["value"], 20);
    assert_eq!(
        rows(stat(&data, "diamond", "value-by-area")),
        [5000, 6250, 7500, 8750, 10000]
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
    let run = wiki_data(&assets_dir, &bad, &tmp.join("bad.json"), &[]);
    assert!(!run.status.success());
    assert!(String::from_utf8_lossy(&run.stderr).contains("matches nothing"));

    // An area that binds an object's `instance_exists` false fails the run if its room places one or its
    // walk makes one (rOlmec's doors are oEntrances, and a Mines level, in rLevel, has an exit), or if it
    // binds no room to check it against.
    let relations = fs::read_to_string(specs.join("relations.toml")).unwrap();
    for (from, to, error) in [
        (
            "\"instance_exists(oExit)\" = 0",
            "\"instance_exists(oEntrance)\" = 0",
            "rOlmec places oEntrance",
        ),
        (
            "\"global.levelType\" = 0 }",
            "\"global.levelType\" = 0, 'isRoom(\"rLevel\")' = 1, \"instance_exists(oExit)\" = 0 }",
            "generation makes oExit",
        ),
        (
            "\"global.levelType\" = 0 }",
            "\"global.levelType\" = 0, \"instance_exists(oExit)\" = 0 }",
            "no isRoom",
        ),
        (
            "'isRoom(\"rOlmec\")' = 1",
            "'isRoom(\"rOlmek\")' = 1",
            "no room rOlmek",
        ),
        // A `given_by`'s object must be what the code reaching its script tests touching, by name (the
        // sacrifice tests oSacAltarLeft, not its child or its parent, oSolid); not one touching keys drops
        // for already (a crate); and an object at all. Its script must be a script, once, and make a drop.
        (
            "object = \"oSacAltarLeft\"",
            "object = \"oSacAltarRight\"",
            "reaches it without testing touching oSacAltarRight",
        ),
        (
            "object = \"oSacAltarLeft\"",
            "object = \"oSolid\"",
            "reaches it without testing touching oSolid",
        ),
        (
            "object = \"oSacAltarLeft\"",
            "object = \"oCrate\"",
            "oCrate is a drop's key wherever it's touched already",
        ),
        (
            "object = \"oSacAltarLeft\"",
            "object = \"oSacAltar\"",
            "no object oSacAltar",
        ),
        (
            "script = \"scrGetFavorMsg\"",
            "script = \"oEnemy:step\"",
            "is an event, not a script",
        ),
        (
            "script = \"scrGetFavorMsg\"",
            "script = \"scrCreateBlood\"",
            "given_by \"scrCreateBlood\" makes no drop",
        ),
        (
            "script = \"scrGetFavorMsg\"",
            "script = \"scrGetFavourMsg\"",
            "[drops] given_by scrGetFavourMsg:",
        ),
        (
            "given_by = [\n",
            "given_by = [\n  { script = \"scrGetFavorMsg\", object = \"oSacAltarLeft\", why = \"again\" },\n",
            "given_by \"scrGetFavorMsg\" is listed twice",
        ),
    ] {
        let bad = tmp.join("bad-relations");
        let _ = fs::remove_dir_all(&bad);
        fs::create_dir_all(&bad).unwrap();
        assert!(relations.contains(from), "relations.toml has no {from}");
        fs::write(bad.join("relations.toml"), relations.replacen(from, to, 1)).unwrap();
        let run = wiki_data(&assets_dir, &bad, &tmp.join("bad-relations.json"), &[]);
        assert!(!run.status.success());
        let stderr = String::from_utf8_lossy(&run.stderr);
        assert!(stderr.contains(error), "{stderr}");
    }
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
fn sells<'a>(data: &'a Value, slug: &str) -> Vec<&'a str> {
    shop(data, slug)["sells"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["object"].as_str().unwrap())
        .collect()
}

fn shop<'a>(data: &'a Value, slug: &str) -> &'a Value {
    data["shops"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["slug"] == slug)
        .unwrap_or_else(|| panic!("no shop {slug}"))
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
    // scrRoomGen4's main rooms roll `rand(1,12)` in the City of Gold and `rand(1,10)` elsewhere: the
    // two layouts with a Tomb Lord ("?", cases 11 and 12) are the City of Gold's. Its side rooms roll
    // treasure vaults ("t", cases 13 to 15) and never Kali's altar ("x", case 16).
    let [lord] = about(data, "spawns_in", "oTombLord", "scrRoomGen4.gml")[..] else {
        panic!("one scrRoomGen4 Tomb Lord");
    };
    assert_eq!(strings(&lord["areas"]), ["temple"]);
    assert_eq!(strings(&lord["only_in"]), ["city-of-gold"]);
    let [bars] = about(data, "spawns_in", "oGoldBars", "scrRoomGen4.gml")[..] else {
        panic!("one scrRoomGen4 gold bars");
    };
    assert_eq!(strings(&bars["only_in"]), ["city-of-gold"]);
    let [altar] = about(data, "spawns_in", "oSacAltarLeft", "scrRoomGen4.gml")[..] else {
        panic!("one scrRoomGen4 altar");
    };
    assert_eq!(strings(&altar["not_in"]), ["city-of-gold"]);
    // scrRoomGen2's end room is a lake (case 5) only with `global.lake`: its water ("v") is the flooded
    // level's.
    let lake = about(data, "spawns_in", "oWaterSwim", "scrRoomGen2.gml");
    let [lake] = lake.iter().filter(|s| s["tile"] == "v").collect::<Vec<_>>()[..] else {
        panic!("one scrRoomGen2 \"v\" water");
    };
    assert_eq!(strings(&lake["only_in"]), ["flooded-level"]);
    // A Mines brick may hold a gem (oBrick's create event, which generation follows).
    let gem = about(data, "spawns_in", "oSapphireBig", "oBrick/create.gml");
    assert_eq!(strings(&gem[0]["areas"]), ["mines"]);
    // Olmec is placed in his room by the room editor.
    assert_eq!(data["placed_in"]["oOlmec"]["rOlmec"], 1);
    // A thief's or murderer's angry Shopkeeper waits at each exit (scrInitLevel's `with oExit`), and
    // Olmec's Lair has none when it's made (its `instance_exists(oExit) = 0`): rOlmec's doors are two
    // oEntrances, and generation makes no oExit there.
    let [thief] = about(data, "spawns_in", "oShopkeeper", "scrInitLevel.gml")[..] else {
        panic!("one scrInitLevel Shopkeeper");
    };
    assert_eq!(
        strings(&thief["areas"]),
        ["mines", "jungle", "ice-caves", "temple"]
    );
    assert_eq!(data["placed_in"]["oEntrance"]["rOlmec"], 2);
    assert!(data["placed_in"]["oExit"].get("rOlmec").is_none());
    for exit in data["spawns_in"]["oExit"].as_array().unwrap() {
        assert!(!strings(&exit["areas"]).contains(&"olmecs-lair"));
    }

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
    let shotgun = shop(data, "weapon-shop")["sells"]
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
    for d in data["drops"]["oCrate"].as_array().unwrap() {
        assert_eq!(d["by"], "oPlayer1:step");
    }
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
    // A monkey throws out the player's money, rope or bomb; its trip makes him drop what he holds, but
    // putting the item he'd picked up back in his hand (scrHoldItem, the spec's not_via) isn't its drop, nor
    // is the arrow his drawn bow shoots as he falls (scrFireBow).
    let monkey: Vec<&str> = data["drops"]["oMonkey"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["object"].as_str().unwrap())
        .collect();
    assert!(monkey.contains(&"oGoldNugget") && monkey.contains(&"oBomb"));
    assert!(
        !monkey.contains(&"oMachete") && !monkey.contains(&"oArrow"),
        "{monkey:?}"
    );
    assert_eq!(data["not_drops"][0]["script"], "scrHoldItem");
    // A body left on Kali's altar is sacrificed (oEnemy's and oDamsel's step test
    // `collision_point(..., oSacAltarLeft, 0, 0)`), and her gifts (scrGetFavorMsg, the spec's given_by)
    // are the altar's, not the body's.
    assert_eq!(data["given_by"][0]["script"], "scrGetFavorMsg");
    let altar = about(data, "drops", "oSacAltarLeft", "scripts/scrGetFavorMsg.gml");
    for by in ["oEnemy:step", "oDamsel:step"] {
        let gifts: Vec<&str> = (altar.iter())
            .filter(|d| d["by"] == by)
            .map(|d| d["object"].as_str().unwrap())
            .collect();
        assert_eq!(gifts.len(), 11, "{by}: {gifts:?}");
        assert!(gifts.contains(&"oKapala") && gifts.contains(&"oJetpack"));
        assert!(gifts.contains(&"oBombBox") && gifts.contains(&"oGloves"));
    }
    for d in &altar {
        assert_eq!(strings(&d["involves"]), ["oSacAltarLeft"]);
    }
    let elsewhere: Vec<&String> = (data["drops"].as_object().unwrap().iter())
        .filter(|(key, list)| {
            *key != "oSacAltarLeft"
                && (list.as_array().unwrap().iter())
                    .any(|d| d["at"]["file"] == "gml/scripts/scrGetFavorMsg.gml")
        })
        .map(|(key, _)| key)
        .collect();
    assert!(elsewhere.is_empty(), "Kali's gifts keyed by {elsewhere:?}");

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
    // A yeti's whip code hurts it only `if (other.puncture)`: a machete's slash and a mattock's hit set
    // it true, the whip false.
    let yeti_whip = damage(data, "oYeti:collision_oWhip", "hp -= other.damage");
    assert_eq!(strings(&yeti_whip["sources"]), ["oMattockHit", "oSlash"]);
    // Its wind-up code, only `if (other.type == "Machete")`: the machete's wind-up, not the whip's
    // (oWhipPre, its parent) or the mattock's, whose create events set `type` to "Whip" and "Mattock" (and
    // nothing sets it again: scrRoomGen3's `obj.type = "Moai Exit"` is an exit's).
    let yeti_pre = damage(data, "oYeti:collision_oWhipPre", "hp -= other.damage");
    assert_eq!(strings(&yeti_pre["sources"]), ["oMachetePre"]);
    assert_eq!(strings(&yeti_pre["ruled_out"]), ["oMattockPre", "oWhipPre"]);
    // A mantrap's `else` is the rest: the whip's wind-up, less the machete's below it.
    let mantrap_pre = damage(data, "oManTrap:collision_oWhipPre", "hp -= other.damage");
    assert_eq!(
        strings(&mantrap_pre["sources"]),
        ["oMattockPre", "oWhipPre"]
    );
    assert_eq!(strings(&mantrap_pre["ruled_out"]), ["oMachetePre"]);
    // A test that rules nothing out (a stomp's `other.state == 15`, which the player's step changes)
    // names the other object alone.
    let stomp = data["damage"].as_array().unwrap().iter().find(|d| {
        d["by"] == "oCaveman:collision_oCharacter"
            && d["find"] == "hp -= ?"
            && d["when"].to_string().contains("other.state")
    });
    assert_eq!(strings(&stomp.unwrap()["sources"]), ["oCharacter"]);
    // A bomb's explosion: `other.hp -= 30` on any enemy.
    let bomb = damage(data, "oExplosion:collision_oEnemy", "other.hp -= 30");
    assert_eq!(strings(&bomb["victims"]), ["oEnemy"]);
    assert_eq!(bomb["value"], 30);
    // A bullet: `with other { hp -= 4; }`, the `with`'s `other` being the enemy it hit.
    let bullet = damage(data, "oBullet:collision_oEnemy", "hp -= 4");
    assert_eq!(strings(&bullet["victims"]), ["oEnemy"]);
    assert_eq!(strings(&bullet["sources"]), ["oBullet"]);
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

/// The site's snapshot of the relations: what the wiki's build checks the Snake page's against.
fn check_site_relations(site: &Value) {
    let strings = |v: &Value| -> Vec<String> {
        v.as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().unwrap().to_owned())
            .collect()
    };
    assert_eq!(site["places"]["mines"]["kind"], "area");
    assert_eq!(site["places"]["snake-pit"]["kind"], "level");
    assert_eq!(site["shops"]["weapon-shop"], "Weapon Shop");
    let snake = &site["found_in"]["oSnake"];
    assert_eq!(strings(&snake["places"]), ["mines", "snake-pit"]);
    assert_eq!(strings(&snake["rooms"]), ["rTutorial"]);
    assert!(strings(&site["sold_in"]["oShotgun"]).contains(&"weapon-shop".to_owned()));
    // No Shopkeeper waits in Olmec's Lair, which has no exit when it's made.
    assert!(
        !strings(&site["found_in"]["oShopkeeper"]["places"]).contains(&"olmecs-lair".to_owned())
    );
    // scrSetupWalls makes push blocks `with oBlock`, which runs only where stone blocks are made: Olmec's
    // Lair, but not the Ice Caves, whose rooms have no push block tiles either.
    let push_blocks = strings(&site["found_in"]["oPushBlock"]["places"]);
    assert!(push_blocks.contains(&"olmecs-lair".to_owned()));
    assert!(!push_blocks.contains(&"ice-caves".to_owned()));
    // A jar makes Snakes, and a crate holds items (the player's step opens it, but it's the crate's).
    assert!(strings(&site["drops"]["oJar"]["own"]).contains(&"oSnake".to_owned()));
    assert!(strings(&site["drops"]["oCrate"]["own"]).contains(&"oCompass".to_owned()));
    // The Snake runs oEnemy's step, so it inherits what that makes (a Frog, for a Fire Frog in water).
    assert!(strings(&site["drops"]["oSnake"]["inherited"]).contains(&"oFrog".to_owned()));
    // Kali's gifts for a sacrificed body are the altar's (and its other half's, oSacAltarRight, a child
    // of oSacAltarLeft), and no monster's or the Damsel's, by name or inherited.
    let gifts = ["oKapala", "oJetpack", "oBombBox", "oCapePickup", "oCompass"];
    let altar = strings(&site["drops"]["oSacAltarLeft"]["own"]);
    let other_half = strings(&site["drops"]["oSacAltarRight"]["inherited"]);
    for gift in gifts {
        assert!(altar.contains(&gift.to_owned()) && other_half.contains(&gift.to_owned()));
    }
    for body in ["oEnemy", "oSnake", "oCaveman", "oDamsel"] {
        let by = &site["drops"][body];
        let all = [strings(&by["own"]), strings(&by["inherited"])].concat();
        for gift in gifts {
            assert!(!all.contains(&gift.to_owned()), "{body} drops {gift}");
        }
    }
    // What hurts oEnemy hurts the Snake, which runs its step (spikes, lava) and collides as one (a
    // bullet, a whip and the Machete's slash, a kind of whip). The Bat has a step of its own, so oEnemy's
    // spikes and lava don't hurt it.
    let snake = strings(&site["damaged_by"]["oSnake"]["inherited"]);
    for source in [
        "oWhip",
        "oSlash",
        "oSpikes",
        "oLava",
        "oBullet",
        "oExplosion",
    ] {
        assert!(
            snake.contains(&source.to_owned()),
            "oSnake isn't hurt by {source}"
        );
    }
    let bat = strings(&site["damaged_by"]["oBat"]["inherited"]);
    assert!(bat.contains(&"oWhip".to_owned()));
    assert!(!bat.contains(&"oSpikes".to_owned()) && !bat.contains(&"oLava".to_owned()));
    // The Snake's own step kills it inside `oSolid`: that's what hurts it by name, and every solid's
    // test is one for it too, Olmec's among them, but only matches.
    let snake = &site["damaged_by"]["oSnake"];
    assert_eq!(strings(&snake["own"]), ["oSolid"]);
    assert!(strings(&snake["inherited"]).contains(&"oOlmec".to_owned()));
    // The Caveman's whip event names oWhip; the Machete's slash, a kind of whip, only matches.
    let caveman = &site["damaged_by"]["oCaveman"];
    assert!(strings(&caveman["own"]).contains(&"oWhip".to_owned()));
    assert!(!strings(&caveman["own"]).contains(&"oSlash".to_owned()));
    assert!(strings(&caveman["inherited"]).contains(&"oSlash".to_owned()));
    // The Whip's wind-up and the Mattock's don't hurt a Yeti or a Shopkeeper, only the Machete's.
    for victim in ["oYeti", "oShopkeeper"] {
        let by = &site["damaged_by"][victim];
        let all = [strings(&by["own"]), strings(&by["inherited"])].concat();
        assert!(all.contains(&"oMachetePre".to_owned()), "{victim}: {all:?}");
        for source in ["oWhipPre", "oMattockPre"] {
            assert!(
                !all.contains(&source.to_owned()),
                "{victim} is hurt by {source}"
            );
        }
    }
}
