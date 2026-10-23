#!/usr/bin/env python3
"""Tokenize all GML in a parsed GMK and collect runtime-subset statistics.
Usage: python3 -I analyze.py <parsed.pkl> <keywords.properties> <gmlfunctions.txt> <outdir>
"""
import collections
import json
import os
import pickle
import re
import sys

G = pickle.load(open(sys.argv[1], "rb"))
kw_text = open(sys.argv[2]).read().replace("\\\n", " ")
KW = {}
for line in kw_text.splitlines():
    if "=" in line:
        k, v = line.split("=", 1)
        KW[k] = [re.sub(r"[\*\[\]0-9]*$", "", w).rstrip("[") for w in v.split()]
        KW[k] = [re.sub(r"\[\d+\]\*?$", "", w).rstrip("*") for w in v.split()]
KNOWN_FUNCS = set()
for line in open(sys.argv[3]).read().splitlines():
    if re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", line.strip()):
        KNOWN_FUNCS.add(line.strip())
OUT = sys.argv[4]
os.makedirs(OUT, exist_ok=True)

KEYWORDS = set(KW["CONSTRUCTS"]) | set(KW["OPERATORS"]) | {"true", "false"}
BUILTIN_VARS = set(KW["VARIABLES"]) | {
    "argument", "image_single", "transition_steps", "transition_time", "cursor_sprite",
    "debug_mode", "instance_count", "instance_id", "room_caption", "keyboard_lastchar",
    "argument_relative", "view_current", "background_color"}
BUILTIN_VARS -= {"phy_active"}  # irrelevant, harmless
CONSTANTS = set(KW["CONSTANTS"]) | {"all", "noone", "self", "other", "global", "pi", "true",
                                    "false", "local"}
SCRIPTS = {s["name"] for s in G["scripts"] if s}
RES = {}
for key in ["sprites", "backgrounds", "sounds", "fonts", "paths", "timelines", "objects", "rooms"]:
    for x in G[key]:
        if x:
            RES[x["name"]] = key[:-1]
OBJN = {o["id"]: o["name"] for o in G["objects"] if o}

TOKEN_RE = re.compile(r"""
    (?P<ws>\s+)
  | (?P<lc>//[^\n]*)
  | (?P<bc>/\*.*?(\*/|\Z))
  | (?P<str>"[^"]*("|\Z)|'[^']*('|\Z))
  | (?P<num>\$[0-9A-Fa-f]+|\d+\.?\d*|\.\d+)
  | (?P<id>[A-Za-z_][A-Za-z0-9_]*)
  | (?P<op><<=|>>=|\+=|-=|\*=|/=|\|=|&=|\^=|==|!=|<=|>=|<>|&&|\|\||\^\^|<<|>>|\+\+|--|:=|.)
""", re.S | re.X)


def tokenize(code):
    toks = []
    for m in TOKEN_RE.finditer(code):
        k = m.lastgroup
        if k in ("ws", "lc", "bc"):
            continue
        toks.append((k, m.group(k)))
    return toks


# ---- collect code units ----------------------------------------------------------------
units = []  # (kind, owner, event, code)
for s in G["scripts"]:
    if s:
        units.append(("script", s["name"], "", s["code"]))
EVN = {0: "create", 1: "destroy", 2: "alarm", 3: "step", 4: "collision", 5: "keyboard",
       6: "mouse", 7: "other", 8: "draw", 9: "keypress", 10: "keyrelease", 11: "trigger"}
dnd = collections.Counter()
dnd_where = collections.defaultdict(list)
dnd_exprs = []
dnd_script_calls = collections.Counter()
SCRIPT_BY_ID = {x["id"]: x["name"] for x in G["scripts"] if x}
for o in G["objects"]:
    if not o:
        continue
    for e in o["events"]:
        for a in e["actions"]:
            if a["lib"] == 1 and a["id"] == 603:
                units.append(("object", o["name"], "%s/%d" % (EVN[e["type"]], e["sub"]), a["args"][0]))
            else:
                key = "%d/%d %s(%s)" % (a["lib"], a["id"], a["function"] or "-", a["kind"])
                dnd[key] += 1
                dnd_where[key].append("%s:%s/%d" % (o["name"], EVN[e["type"]], e["sub"]))
                if a["function"] == "action_execute_script":
                    dnd_script_calls[SCRIPT_BY_ID.get(int(a["args"][0]), a["args"][0])] += 1
                for kind, val in zip(a["arg_kinds"], a["args"]):
                    if kind in ("expression", "both"):
                        dnd_exprs.append(val)
for r in G["rooms"]:
    if not r:
        continue
    if r["creation_code"].strip():
        units.append(("room", r["name"], "creation", r["creation_code"]))
    for i in r["instances"]:
        if i["creation_code"].strip():
            units.append(("instance", r["name"], OBJN.get(i["object"], "?"), i["creation_code"]))


def line_stats(code):
    code = code.replace("\r\n", "\n").replace("\r", "\n")
    lines = code.split("\n")
    # strip trailing empty lines from physical count for a fair number
    phys = len(lines)
    nonblank = sum(1 for l in lines if l.strip())
    # code lines: drop comments
    nocom = re.sub(r"/\*.*?(\*/|\Z)", lambda m: "\n" * m.group(0).count("\n"), code, flags=re.S)
    nocom = re.sub(r"//[^\n]*", "", nocom)
    sloc = sum(1 for l in nocom.split("\n") if l.strip())
    return phys, nonblank, sloc


lines = collections.defaultdict(lambda: [0, 0, 0, 0])
for kind, owner, ev, code in units:
    p, nb, sl = line_stats(code)
    L = lines[kind]
    L[0] += 1
    L[1] += p
    L[2] += nb
    L[3] += sl

funcs = collections.Counter()
func_users = collections.defaultdict(set)
script_calls = collections.Counter(dnd_script_calls)
bvars = collections.Counter()
consts = collections.Counter()
resrefs = collections.Counter()
kwc = collections.Counter()
assigned = set()
all_ids = collections.Counter()
id_users = collections.defaultdict(set)
string_literals = []
func_str_args = collections.defaultdict(list)

ASSIGN_OPS = {"=", "+=", "-=", "*=", "/=", "|=", "&=", "^=", ":=", "++", "--"}

for kind, owner, ev, code in units + [("dnd", "dnd", "", c) for c in dnd_exprs]:
    t = tokenize(code)
    where = owner if kind != "object" else owner
    for i, (k, v) in enumerate(t):
        if k == "str":
            string_literals.append((owner, v))
            continue
        if k != "id":
            continue
        nxt = t[i + 1][1] if i + 1 < len(t) else ""
        prev = t[i - 1][1] if i > 0 else ""
        if v in KEYWORDS or v in ("begin", "end", "then"):
            kwc[v] += 1
            if v == "var" or v == "globalvar":
                j = i + 1
                while j < len(t) and t[j][0] == "id":
                    assigned.add(t[j][1])
                    if j + 1 < len(t) and t[j + 1][1] == ",":
                        j += 2
                    else:
                        break
            continue
        if nxt == "(" and prev != ".":
            if v in SCRIPTS:
                script_calls[v] += 1
            else:
                funcs[v] += 1
                func_users[v].add(owner)
                # capture the first string literal argument
                if i + 2 < len(t) and t[i + 2][0] == "str":
                    func_str_args[v].append(t[i + 2][1])
            continue
        if v in SCRIPTS:
            script_calls[v + " (as value)"] += 1
            continue
        if v in BUILTIN_VARS:
            bvars[v] += 1
            continue
        if v in CONSTANTS:
            consts[v] += 1
            continue
        if v in RES:
            resrefs[RES[v]] += 1
            continue
        all_ids[v] += 1
        id_users[v].add(owner)
        # assignment detection
        j = i + 1
        if j < len(t) and t[j][1] == "[":
            depth = 0
            while j < len(t):
                if t[j][1] == "[":
                    depth += 1
                elif t[j][1] == "]":
                    depth -= 1
                    if depth == 0:
                        j += 1
                        break
                j += 1
        if j < len(t) and t[j][1] in ASSIGN_OPS:
            assigned.add(v)
        if prev in ("++", "--"):
            assigned.add(v)
        # `for (i = ...` / `with` / `repeat` covered by '='

# DnD set-variable actions assign variables
for o in G["objects"]:
    if not o:
        continue
    for e in o["events"]:
        for a in e["actions"]:
            if a["kind"] == "variable":
                assigned.add(re.sub(r"^.*\.", "", a["args"][0]).split("[")[0])

never_assigned = {k: v for k, v in all_ids.items() if k not in assigned and not k.startswith("argument")}
unknown_funcs = {k: v for k, v in funcs.items() if k not in KNOWN_FUNCS}

res = {
    "units": {k: {"count": v[0], "physical_lines": v[1], "nonblank_lines": v[2], "code_lines": v[3]}
              for k, v in lines.items()},
    "functions": dict(funcs.most_common()),
    "function_user_counts": {k: len(func_users[k]) for k in funcs},
    "functions_not_in_reference_list": unknown_funcs,
    "script_calls": dict(script_calls.most_common()),
    "scripts_never_called": sorted(s for s in SCRIPTS if s not in script_calls),
    "builtin_variables": dict(bvars.most_common()),
    "constants": dict(consts.most_common()),
    "resource_refs_by_type": dict(resrefs),
    "keywords": dict(kwc.most_common()),
    "dnd_actions": dict(dnd.most_common()),
    "dnd_script_calls": dict(dnd_script_calls),
    "dnd_where": {k: v for k, v in dnd_where.items()},
    "game_identifiers_distinct": len(all_ids),
    "identifiers_never_assigned": dict(sorted(never_assigned.items(), key=lambda x: -x[1])),
    "func_string_args": {k: sorted(set(v)) for k, v in func_str_args.items()
                         if k.startswith(("file_", "ini_", "external_", "sound_", "SS_", "execute_",
                                          "variable_", "registry_", "background_add", "sprite_add",
                                          "show_", "directory_", "font_add"))},
}
json.dump(res, open(os.path.join(OUT, "analysis.json"), "w"), indent=1)

# per-unit index for grep-style questions
idx = []
for kind, owner, ev, code in units:
    p, nb, sl = line_stats(code)
    idx.append({"kind": kind, "owner": owner, "event": ev, "lines": p, "sloc": sl})
json.dump(idx, open(os.path.join(OUT, "code_units.json"), "w"), indent=1)

print(json.dumps(res["units"], indent=1))
print("distinct functions", len(funcs), "calls", sum(funcs.values()))
print("distinct builtin vars", len(bvars), "refs", sum(bvars.values()))
print("distinct constants", len(consts))
print("unknown funcs", unknown_funcs)
print("never assigned (top 80)", list(never_assigned.items())[:80])
