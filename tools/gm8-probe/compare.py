#!/usr/bin/env python3
"""Prints the probe lines where two runs differ, and a count per probe.

    compare.py [GM8_DIR OPENGMK_DIR]    (default: results/gm8-wine and results/opengmk)

Each directory holds one <probe>.txt per probe: its probe.txt, plus `# stopped: ...` lines if an error
ended it early. A line one run has and the other lacks shows as `-`.
"""

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))


def load(path):
    lines, notes = {}, []
    with open(path, encoding="cp1252") as f:
        for line in f.read().splitlines():
            if line.startswith("#"):
                notes.append(line)
            elif " = " in line:
                name, value = line.split(" = ", 1)
                lines[name] = value
    return lines, notes


def main():
    a_dir, b_dir = sys.argv[1:3] if len(sys.argv) == 3 else (
        os.path.join(HERE, "results", "gm8-wine"),
        os.path.join(HERE, "results", "opengmk"),
    )
    for name in sorted(set(os.listdir(a_dir)) & set(os.listdir(b_dir))):
        a, a_notes = load(os.path.join(a_dir, name))
        b, b_notes = load(os.path.join(b_dir, name))
        keys = list(a) + [k for k in b if k not in a]
        diff = [k for k in keys if a.get(k) != b.get(k)]
        print(f"## {name[:-4]}: {len(keys) - len(diff)} same, {len(diff)} differ")
        for note in a_notes:
            print(f"   GM8     {note}")
        for note in b_notes:
            print(f"   OpenGMK {note}")
        for k in diff:
            print(f"   {k}: GM8 {a.get(k, '-')} | OpenGMK {b.get(k, '-')}")


if __name__ == "__main__":
    main()
