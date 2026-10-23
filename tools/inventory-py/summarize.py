#!/usr/bin/env python3
"""Generate SUMMARY.md tables from parsed data + analysis + a hand-written narrative file.
Usage: python3 -I summarize.py <parsed.pkl> <analysis.json> <config_analysis.json> <narrative.md> <out SUMMARY.md>
The narrative file contains sections; the tables are substituted at {{PLACEHOLDER}} markers.
"""
import collections
import json
import pickle
import re
import sys

G = pickle.load(open(sys.argv[1], "rb"))
A = json.load(open(sys.argv[2]))
AC = json.load(open(sys.argv[3]))
narr = open(sys.argv[4]).read()

O = {o["id"]: o for o in G["objects"] if o}
SPR = {s["id"]: s["name"] for s in G["sprites"] if s}
EVN = {0: "create", 1: "destroy", 2: "alarm", 3: "step", 4: "coll", 5: "kb", 6: "mouse",
       7: "other", 8: "draw", 9: "kpress", 10: "krel", 11: "trigger"}
STEP = {0: "step", 1: "begin_step", 2: "end_step"}
OTHER = {0: "outside", 1: "boundary", 2: "game_start", 3: "game_end", 4: "room_start", 5: "room_end",
         7: "animation_end", 8: "end_of_path", 30: "close_button"}
MOUSE = {4: "left_press", 7: "left_release", 10: "enter", 11: "leave", 60: "wheel_up", 61: "wheel_down"}

FOLDER = {}


def walk(n, path):
    if n["status"] == "secondary":
        FOLDER[(n["kind"], n["index"])] = "/".join(path[1:])
    for c in n.get("children", []):
        walk(c, path + [n["name"]])


for n in G["resource_tree"]:
    walk(n, [])


def ev_short(e):
    t, s = e["type"], e["sub"]
    if t == 2:
        return "alarm%d" % s
    if t == 3:
        return STEP.get(s, "step%d" % s)
    if t == 4:
        return "coll:" + O[s]["name"]
    if t == 7:
        return OTHER.get(s, "user%d" % (s - 10) if 10 <= s < 26 else "other%d" % s)
    if t == 6:
        return "mouse:" + MOUSE.get(s, str(s))
    if t in (5, 9, 10):
        k = s
        kn = {13: "enter", 27: "esc", 8: "bksp", 1: "any", 38: "up", 40: "down"}.get(k)
        if kn is None:
            kn = chr(k) if 48 <= k <= 90 else ("F%d" % (k - 111) if 112 <= k <= 123 else str(k))
        return "%s:%s" % (EVN[t], kn)
    return EVN[t]


def loc(code):
    code = code.replace("\r\n", "\n").replace("\r", "\n")
    nocom = re.sub(r"/\*.*?(\*/|\Z)", lambda m: "\n" * m.group(0).count("\n"), code, flags=re.S)
    nocom = re.sub(r"//[^\n]*", "", nocom)
    return len(code.split("\n")), sum(1 for l in nocom.split("\n") if l.strip())


def name_or_dash(tbl, i):
    return tbl.get(i, "?") if i >= 0 else "-"


# ---------------------------------------------------------------- counts table
defined = {k: sum(1 for x in G[k] if x) for k in ["sprites", "sounds", "backgrounds", "paths", "scripts",
                                                   "fonts", "timelines", "objects", "rooms"]}
slots = {k: len(G[k]) for k in defined}
frames = sum(s["frames"] for s in G["sprites"] if s)
tiles = sum(len(r["tiles"]) for r in G["rooms"] if r)
insts = sum(len(r["instances"]) for r in G["rooms"] if r)
nevents = sum(len(o["events"]) for o in O.values())
ncode = sum(1 for o in O.values() for e in o["events"] for a in e["actions"] if a["lib"] == 1 and a["id"] == 603)
ndnd = sum(1 for o in O.values() for e in o["events"] for a in e["actions"] if not (a["lib"] == 1 and a["id"] == 603))
counts = ["| Resource | Defined | ID slots (incl. deleted) | Notes |", "|---|---:|---:|---|"]
notes = {"sprites": "%d frames total; all pixel data decodable (BGRA, len == w*h*4)" % frames,
         "sounds": "every slot deleted; all audio is external files via supersound.dll",
         "backgrounds": "all pixel data decodable",
         "paths": "", "scripts": "", "fonts": "1 deleted slot; fonts are built at runtime with font_add_sprite",
         "timelines": "2 deleted slots", "objects": "%d events, %d code actions, %d other DnD actions" % (nevents, ncode, ndnd),
         "rooms": "%d placed instances, %d tiles" % (insts, tiles)}
for k in defined:
    counts.append("| %s | %d | %d | %s |" % (k, defined[k], slots[k], notes[k]))
counts.append("| triggers | %d | - | |" % len([t for t in G["triggers"] if t]))
counts.append("| constants | %d | - | |" % len(G["constants"]))
counts.append("| include files | %d | - | |" % len(G["includes"]))
counts.append("| extension packages | %d | - | |" % len(G["extension_packages"]))
u = A["units"]
counts += ["", "GML lines (physical / non-blank / code lines excluding comments and blanks):", "",
           "| Source | Units | Physical | Non-blank | Code |", "|---|---:|---:|---:|---:|"]
tot = [0, 0, 0, 0]
for k, lab in [("script", "Scripts"), ("object", "Object event code actions")]:
    v = u[k]
    counts.append("| %s | %d | %d | %d | %d |" % (lab, v["count"], v["physical_lines"], v["nonblank_lines"], v["code_lines"]))
    for i, f in enumerate(["count", "physical_lines", "nonblank_lines", "code_lines"]):
        tot[i] += v[f]
counts.append("| Room creation code | 0 | 0 | 0 | 0 |")
counts.append("| Instance creation code | 0 | 0 | 0 | 0 |")
counts.append("| **Total** | %d | %d | %d | %d |" % tuple(tot))

# ---------------------------------------------------------------- object table
groups = collections.defaultdict(list)
for o in O.values():
    groups[FOLDER.get(("object", o["id"]), "") or "(top level)"].append(o)
GROUP_ORDER = ["Characters", "Basis", "Enemies", "Items", "Collectibles", "Traps", "Blocks", "Effects",
               "Init", "(top level)", "Other", "Other/Intro", "Other/Title", "Other/End", "Level Editor",
               "supersound"]
objtab = []
for gname in GROUP_ORDER + sorted(set(groups) - set(GROUP_ORDER)):
    if gname not in groups:
        continue
    L = groups[gname]
    total = sum(loc(a["args"][0])[1] for o in L for e in o["events"] for a in e["actions"]
                if a["lib"] == 1 and a["id"] == 603)
    objtab.append("\n#### %s (%d objects, %d code lines)\n" % (gname, len(L), total))
    objtab.append("| Object | Parent | Sprite | Mask | Sol | Pers | Vis | Depth | Events | LOC (phys/code) |")
    objtab.append("|---|---|---|---|:-:|:-:|:-:|---:|---|---:|")
    for o in sorted(L, key=lambda o: o["id"]):
        ph = cl = 0
        for e in o["events"]:
            for a in e["actions"]:
                if a["lib"] == 1 and a["id"] == 603:
                    p, c = loc(a["args"][0])
                    ph += p
                    cl += c
        evs = ", ".join(ev_short(e) for e in sorted(o["events"], key=lambda e: (e["type"], e["sub"]))) or "-"
        objtab.append("| %s | %s | %s | %s | %s | %s | %s | %d | %s | %d/%d |" % (
            o["name"], name_or_dash({k: v["name"] for k, v in O.items()}, o["parent"]),
            name_or_dash(SPR, o["sprite"]), name_or_dash(SPR, o["mask"]),
            "Y" if o["solid"] else "", "Y" if o["persistent"] else "", "" if o["visible"] else "N",
            o["depth"], evs, ph, cl))

# ---------------------------------------------------------------- script table
calls = A["script_calls"]
sg = collections.defaultdict(list)
for s in G["scripts"]:
    if s:
        sg[FOLDER.get(("script", s["id"]), "") or "(top level)"].append(s)
scrtab = ["| Folder | Script | Lines (phys/code) | Call sites |", "|---|---|---:|---:|"]
for gname in sorted(sg):
    for s in sorted(sg[gname], key=lambda s: s["id"]):
        p, c = loc(s["code"])
        scrtab.append("| %s | %s | %d/%d | %d |" % (gname, s["name"], p, c, calls.get(s["name"], 0)))

# ---------------------------------------------------------------- rooms
order = {rid: i for i, rid in enumerate(G["room_order"])}
rmtab = ["| # | Room | Size | Speed | Views on | View 0 (view / port, follow) | Instances | Tiles | BG color |",
         "|---:|---|---|---:|:-:|---|---:|---:|---|"]
for r in sorted((r for r in G["rooms"] if r), key=lambda r: order[r["id"]]):
    v = r["views"][0]
    vs = "%dx%d @%d,%d / %dx%d, %s" % (v["view_w"], v["view_h"], v["view_x"], v["view_y"], v["port_w"],
                                      v["port_h"], O[v["follow_object"]]["name"] if v["follow_object"] >= 0 else "-") \
        if v["visible"] else "-"
    rmtab.append("| %d | %s | %dx%d | %d | %s | %s | %d | %d | #%06X |" % (
        order[r["id"]], r["name"], r["width"], r["height"], r["speed"], "Y" if r["views_enabled"] else "N",
        vs, len(r["instances"]), len(r["tiles"]),
        ((r["background_color_bgr"] & 0xFF) << 16) | (r["background_color_bgr"] & 0xFF00) | (r["background_color_bgr"] >> 16 & 0xFF)))

# ---------------------------------------------------------------- functions
CATS = [
    ("Instances", r"^(instance_|object_get_parent)"),
    ("Collision / movement", r"^(collision_|place_|position_|distance_to_|move_)"),
    ("Drawing", r"^(draw_|screen_|font_add_sprite)"),
    ("Surfaces", r"^surface_"),
    ("Tiles", r"^tile_"),
    ("Rooms / views", r"^(room_|game_end)"),
    ("Strings", r"^(string|ord$|chr$|real$|is_real|is_string)"),
    ("Math / random", r"^(abs|round|ceil|floor|frac|sqr|sqrt|sin|cos|arctan|degtorad|radtodeg|point_|random|make_color)"),
    ("Keyboard / mouse / joystick", r"^(keyboard_|mouse_|joystick_)"),
    ("File I/O", r"^file_"),
    ("Highscore table", r"^highscore_"),
    ("DLL", r"^external_"),
    ("Sound (GM built-in)", r"^sound_"),
    ("Window / dialogs", r"^(window_|show_|get_integer)"),
]
used = A["functions"]
users = A["function_user_counts"]
cat_of = {}
for f in used:
    for c, rx in CATS:
        if re.search(rx, f):
            cat_of[f] = c
            break
    else:
        cat_of[f] = "Other"
fntab = ["| Category | Function | Calls | Code units using it |", "|---|---|---:|---:|"]
for c, _ in CATS + [("Other", "")]:
    for f in sorted((f for f in used if cat_of[f] == c), key=lambda f: -used[f]):
        fntab.append("| %s | `%s` | %d | %d |" % (c, f, used[f], users[f]))
bv = A["builtin_variables"]
bvtab = ["| Variable | References |", "|---|---:|"] + ["| `%s` | %d |" % (k, v) for k, v in bv.items()]
ct = A["constants"]
cttab = ", ".join("`%s` (%d)" % (k, v) for k, v in ct.items())
kw = A["keywords"]
kwtab = ", ".join("`%s` %d" % (k, v) for k, v in kw.items())
dndtab = ["| lib/id | DnD action | Kind | Count | Example locations |", "|---|---|---|---:|---|"]
for k, v in A["dnd_actions"].items():
    m = re.match(r"(\S+) (\S+)\((\w+)\)", k)
    where = A["dnd_where"][k]
    dndtab.append("| %s | %s | %s | %d | %s |" % (m.group(1), m.group(2), m.group(3), v,
                                                  ", ".join(where[:4]) + (" ..." if len(where) > 4 else "")))
cfg_only = sorted(set(AC["functions"]) - set(used))

subs = {
    "COUNTS": "\n".join(counts),
    "OBJECTS": "\n".join(objtab),
    "SCRIPTS": "\n".join(scrtab),
    "ROOMS": "\n".join(rmtab),
    "FUNCS": "\n".join(fntab),
    "NFUNCS": str(len(used)),
    "NCALLS": str(sum(used.values())),
    "BVARS": "\n".join(bvtab),
    "NBVARS": str(len(bv)),
    "CONSTS": cttab,
    "NCONSTS": str(len(ct)),
    "KEYWORDS": kwtab,
    "DND": "\n".join(dndtab),
    "CFG_ONLY": ", ".join("`%s`" % f for f in cfg_only),
    "NOTINREF": ", ".join("`%s`" % f for f in A["functions_not_in_reference_list"]),
    "NEVER_CALLED": ", ".join("`%s`" % f for f in A["scripts_never_called"]),
    "NEVER_ASSIGNED": ", ".join("`%s` (%d)" % (k, v) for k, v in A["identifiers_never_assigned"].items()),
}
out = narr
for k, v in subs.items():
    out = out.replace("{{%s}}" % k, v)
left = re.findall(r"\{\{\w+\}\}", out)
if left:
    raise SystemExit("unfilled placeholders: %s" % left)
open(sys.argv[5], "w").write(out)
print("wrote", sys.argv[5], len(out), "chars")
