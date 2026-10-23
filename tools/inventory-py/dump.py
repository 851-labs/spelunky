#!/usr/bin/env python3
"""Dump a parsed GMK (pickle from gmk_parse.py) into an inventory tree.
Usage: python3 -I dump.py <parsed.pkl> <outdir>
"""
import json
import os
import pickle
import re
import shutil
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from gmk_parse import EVENT_NAMES, STEP_SUB, OTHER_SUB, MOUSE_SUB, VK  # noqa: E402

G = pickle.load(open(sys.argv[1], "rb"))
OUT = sys.argv[2]
if os.path.exists(OUT):
    shutil.rmtree(OUT)
os.makedirs(OUT)


def names(lst):
    return {x["id"]: x["name"] for x in lst if x}


SPR = names(G["sprites"])
BG = names(G["backgrounds"])
OBJ = names(G["objects"])
RM = names(G["rooms"])
SCR = names(G["scripts"])
SND = names(G["sounds"])
FNT = names(G["fonts"])
TL = names(G["timelines"])
PTH = names(G["paths"])
ARG_RES = {"sprite": SPR, "sound": SND, "background": BG, "path": PTH, "script": SCR,
           "object": OBJ, "room": RM, "font": FNT, "timeline": TL}


def ref(table, i):
    if i < 0:
        return None
    return table.get(i, "<missing id %d>" % i)


def safe(name):
    return re.sub(r"[^A-Za-z0-9_.-]", "_", name)


def keyname(k):
    if k in VK:
        return VK[k]
    if 48 <= k <= 57 or 65 <= k <= 90:
        return chr(k)
    if 112 <= k <= 123:
        return "F%d" % (k - 111)
    if 96 <= k <= 105:
        return "numpad%d" % (k - 96)
    return "key%d" % k


def event_name(t, sub):
    if t == 0:
        return "create"
    if t == 1:
        return "destroy"
    if t == 2:
        return "alarm_%d" % sub
    if t == 3:
        return STEP_SUB.get(sub, "step_%d" % sub)
    if t == 4:
        return "collision_%s" % OBJ.get(sub, "obj%d" % sub)
    if t == 5:
        return "keyboard_%s" % keyname(sub)
    if t == 6:
        return "mouse_%s" % MOUSE_SUB.get(sub, str(sub))
    if t == 7:
        return "other_%s" % OTHER_SUB.get(sub, str(sub))
    if t == 8:
        return "draw"
    if t == 9:
        return "keypress_%s" % keyname(sub)
    if t == 10:
        return "keyrelease_%s" % keyname(sub)
    if t == 11:
        return "trigger_%d" % sub
    return "event%d_%d" % (t, sub)


def applies(a):
    v = a["applies_to"]
    return {-1: "self", -2: "other"}.get(v, OBJ.get(v, v))


def action_json(a):
    d = dict(a)
    d["applies_to"] = applies(a)
    resolved = []
    for kind, val in zip(a["arg_kinds"], a["args"]):
        if kind in ARG_RES:
            try:
                resolved.append(ARG_RES[kind].get(int(val), val))
            except ValueError:
                resolved.append(val)
        else:
            resolved.append(val)
    d["args_resolved"] = resolved
    return d


def is_code(a):
    return a["lib"] == 1 and a["id"] == 603


def jdump(path, obj):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        json.dump(obj, f, indent=1, ensure_ascii=False)


def wtext(path, text):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write(text.replace("\r\n", "\n").replace("\r", "\n"))


# --- resource tree / folder paths -------------------------------------------------
FOLDER = {}


def walk(n, path):
    ch = n.get("children", [])
    if n["status"] == "secondary":
        FOLDER[(n["kind"], n["index"])] = "/".join(path[1:])
    for c in ch:
        walk(c, path + [n["name"]])


for n in G["resource_tree"]:
    walk(n, [])
jdump(os.path.join(OUT, "resource_tree.json"), G["resource_tree"])

# --- scripts ------------------------------------------------------------------------
scripts_index = []
for s in G["scripts"]:
    if not s:
        continue
    wtext(os.path.join(OUT, "scripts", safe(s["name"]) + ".gml"), s["code"])
    scripts_index.append({"id": s["id"], "name": s["name"], "folder": FOLDER.get(("script", s["id"]), ""),
                          "lines": len(s["code"].replace("\r\n", "\n").split("\n"))})
jdump(os.path.join(OUT, "scripts.json"), scripts_index)

# --- objects ------------------------------------------------------------------------
objects_index = []
for o in G["objects"]:
    if not o:
        continue
    od = os.path.join(OUT, "objects", safe(o["name"]))
    os.makedirs(od, exist_ok=True)
    evlist = []
    for e in o["events"]:
        en = event_name(e["type"], e["sub"])
        parts = []
        ncode = 0
        ndnd = 0
        for k, a in enumerate(e["actions"]):
            if is_code(a):
                ncode += 1
                if len(e["actions"]) > 1 or a["applies_to"] != -1:
                    parts.append("// ==== action %d: execute code (lib 1 id 603) applies_to=%s ====\n"
                                 % (k, applies(a)))
                parts.append(a["args"][0])
            else:
                ndnd += 1
                aj = action_json(a)
                parts.append("// ==== action %d: DnD lib=%d id=%d kind=%s fn=%s applies_to=%s "
                             "relative=%s not=%s args=%s ====\n"
                             % (k, a["lib"], a["id"], a["kind"], a["function"] or "-", aj["applies_to"],
                                int(a["relative"]), int(a["not"]),
                                json.dumps(aj["args_resolved"], ensure_ascii=False)))
        txt = ""
        for p in parts:
            if txt and not txt.endswith("\n"):
                txt += "\n"
            txt += p
        wtext(os.path.join(od, en + ".gml"), txt)
        evlist.append({"event": en, "type": e["type"], "type_name": EVENT_NAMES[e["type"]],
                       "sub": e["sub"], "code_actions": ncode, "dnd_actions": ndnd,
                       "actions": [action_json(a) if not is_code(a) else
                                   {"lib": 1, "id": 603, "kind": "code", "applies_to": applies(a),
                                    "code_file": en + ".gml",
                                    "code_lines": len(a["args"][0].replace("\r\n", "\n").split("\n"))}
                                   for a in e["actions"]]})
    meta = {"id": o["id"], "name": o["name"], "folder": FOLDER.get(("object", o["id"]), ""),
            "sprite": ref(SPR, o["sprite"]), "mask": ref(SPR, o["mask"]),
            "parent": ref(OBJ, o["parent"]), "solid": o["solid"], "visible": o["visible"],
            "persistent": o["persistent"], "depth": o["depth"], "events": evlist}
    jdump(os.path.join(od, "object.json"), meta)
    idx = {k: v for k, v in meta.items() if k != "events"}
    idx["events"] = [e["event"] for e in evlist]
    objects_index.append(idx)
jdump(os.path.join(OUT, "objects.json"), objects_index)

# --- rooms --------------------------------------------------------------------------
order = {rid: i for i, rid in enumerate(G["room_order"])}
rooms_index = []
for r in G["rooms"]:
    if not r:
        continue
    tiles_by_bg = {}
    for t in r["tiles"]:
        k = ref(BG, t["background"])
        tiles_by_bg[k] = tiles_by_bg.get(k, 0) + 1
    d = {"id": r["id"], "name": r["name"], "order_index": order.get(r["id"]),
         "folder": FOLDER.get(("room", r["id"]), ""), "caption": r["caption"],
         "width": r["width"], "height": r["height"], "speed": r["speed"],
         "persistent": r["persistent"], "background_color_bgr": "0x%06X" % r["background_color_bgr"],
         "draw_background_color": r["draw_background_color"], "snap": [r["snap_x"], r["snap_y"]],
         "isometric": r["isometric"],
         "creation_code": r["creation_code"],
         "backgrounds": [dict(b, background=ref(BG, b["background"])) for b in r["backgrounds"]
                         if b["background"] >= 0 or b["visible"]],
         "views_enabled": r["views_enabled"],
         "views": [dict(v, follow_object=ref(OBJ, v["follow_object"])) for v in r["views"]
                   if v["visible"]],
         "tiles_count": len(r["tiles"]), "tiles_by_background": tiles_by_bg,
         "instances_count": len(r["instances"]),
         "instances": [{"object": ref(OBJ, i["object"]), "x": i["x"], "y": i["y"], "id": i["id"],
                        "creation_code": i["creation_code"]} for i in r["instances"]],
         "tiles": [dict(t, background=ref(BG, t["background"])) for t in r["tiles"]]}
    jdump(os.path.join(OUT, "rooms", safe(r["name"]) + ".json"), d)
    if r["creation_code"].strip():
        wtext(os.path.join(OUT, "rooms", safe(r["name"]) + ".creation.gml"), r["creation_code"])
    rooms_index.append({k: d[k] for k in ["id", "name", "order_index", "width", "height", "speed",
                                          "persistent", "views_enabled", "instances_count",
                                          "tiles_count"]}
                       | {"has_creation_code": bool(r["creation_code"].strip()),
                          "instances_with_creation_code": sum(1 for i in r["instances"]
                                                              if i["creation_code"].strip())})
jdump(os.path.join(OUT, "rooms.json"), sorted(rooms_index, key=lambda x: x["order_index"]))

# --- other resources -----------------------------------------------------------------
for key in ["sprites", "backgrounds", "sounds", "fonts", "paths"]:
    lst = []
    for x in G[key]:
        if not x:
            continue
        y = dict(x)
        y["folder"] = FOLDER.get((key[:-1], x["id"]), "")
        lst.append(y)
    jdump(os.path.join(OUT, key + ".json"), lst)
tls = []
for t in G["timelines"]:
    if not t:
        continue
    tls.append({"id": t["id"], "name": t["name"],
                "moments": [{"step": m["step"], "actions": [action_json(a) for a in m["actions"]]}
                            for m in t["moments"]]})
jdump(os.path.join(OUT, "timelines.json"), tls)

jdump(os.path.join(OUT, "game_settings.json"), {
    "format_version": G["format_version"], "game_id": G["game_id"], "guid": G["guid"],
    "settings": G["settings"], "last_instance_id": G["last_instance_id"],
    "last_tile_id": G["last_tile_id"], "room_order": [RM.get(i, i) for i in G["room_order"]],
    "library_creation_code": G["library_creation_code"],
    "game_information": {k: v for k, v in G["game_information"].items() if k != "text_rtf"}})
wtext(os.path.join(OUT, "game_information.rtf"), G["game_information"]["text_rtf"])
jdump(os.path.join(OUT, "constants.json"), G["constants"])
jdump(os.path.join(OUT, "triggers.json"), [t for t in G["triggers"] if t])
jdump(os.path.join(OUT, "includes_and_extensions.json"),
      {"include_files": G["includes"], "extension_packages": G["extension_packages"]})
jdump(os.path.join(OUT, "parse_report.json"), {
    "file_bytes": G["_file_bytes"], "consumed_bytes": G["_consumed_bytes"], "warnings": G["_warnings"],
    "section_offsets": G["_offsets"],
    "slots": {k: len(G[k]) for k in ["sounds", "sprites", "backgrounds", "paths", "scripts", "fonts",
                                     "timelines", "objects", "rooms"]},
    "defined": {k: sum(1 for x in G[k] if x) for k in ["sounds", "sprites", "backgrounds", "paths",
                                                        "scripts", "fonts", "timelines", "objects",
                                                        "rooms"]}})
print("dumped to", OUT)
