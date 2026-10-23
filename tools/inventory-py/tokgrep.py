#!/usr/bin/env python3
"""tokgrep.py <pkl> <regex-on-identifier> [context]: find identifier tokens (code only, no comments/strings)."""
import pickle, re, sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
G = pickle.load(open(sys.argv[1], "rb"))
pat = re.compile(sys.argv[2])
ctx = len(sys.argv) > 3
TOK = re.compile(r'(?P<c>//[^\n]*|/\*.*?(\*/|\Z))|(?P<s>"[^"]*("|\Z)|\'[^\']*(\'|\Z))|(?P<id>[A-Za-z_][A-Za-z0-9_]*)', re.S)
EVN = {0:"create",1:"destroy",2:"alarm",3:"step",4:"collision",5:"keyboard",6:"mouse",7:"other",8:"draw",9:"keypress",10:"keyrelease",11:"trigger"}
units = [("scripts/"+s["name"], s["code"]) for s in G["scripts"] if s]
for o in G["objects"]:
    if not o: continue
    for e in o["events"]:
        for a in e["actions"]:
            if a["lib"] == 1 and a["id"] == 603:
                units.append(("%s:%s/%d" % (o["name"], EVN[e["type"]], e["sub"]), a["args"][0]))
from collections import Counter
hits = Counter()
for name, code in units:
    code = code.replace("\r\n", "\n")
    lines = code.split("\n")
    for m in TOK.finditer(code):
        if m.group("id") and pat.fullmatch(m.group("id")):
            ln = code.count("\n", 0, m.start())
            hits[name] += 1
            if ctx:
                print("%s:%d: %s" % (name, ln + 1, lines[ln].strip()[:160]))
if not ctx:
    for k, v in hits.most_common():
        print(v, k)
