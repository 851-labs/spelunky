#!/usr/bin/env python3
"""Builds a probe executable from Spelunky.exe: build-exe.py SPELUNKY.exe OUT.exe PROBE.gml

The probe's code replaces `scrReadStats`, the first script the game runs (oScreen's create event in
rScreenInit). It runs after `probe.txt` is emptied, and is followed by `out("end", "ok")` and a
`game_end()`, so it only has to call `out(name, value)`. Scripts that would touch the sound DLL or the
save files become empty, so the game does nothing else before it ends. `hex` and `out` (probes/lib/)
take the slots of two scripts the probe never needs, and calls to them are renamed to match.

A probe that needs the runner to step (friction, gravity) splits at a `// step:` line. The code before it
runs as above but isn't followed by `game_end()`: it returns, oScreen's create event goes on, and the game
moves to the next room. So `scrInit` is kept: the next room's objects need the globals it sets, and it only
reads the config files (its sound calls go to the emptied SuperSound scripts). The code after the line
replaces `checkBombPressed`, which oScreen calls at the start of its begin step, so it runs once a step,
before that step's motion, with `global.probe_step` counting the calls from 1. It ends the probe itself,
with `out("end", "ok")` and `game_end()`.
"""

import os
import re
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
LIB = {"hex": "scrGetFavorMsg", "out": "scrGetName"}
EMPTY = ["scrInit", "scrWriteStats", "stopAllMusic"] + [
    "SS_" + n
    for n in (
        "Init Unload LoadSound FreeSound PlaySound LoopSound StopSound PauseSound ResumeSound IsSoundPlaying "
        "IsSoundLooping IsSoundPaused IsHandleValid SetSoundVol GetSoundVol SetSoundPan GetSoundPan "
        "SetSoundFreq GetSoundFreq GetSoundPosition SetSoundPosition GetSoundLength GetSoundBytesPerSecond"
    ).split()
]
PROLOGUE = 'file_text_close(file_text_open_write("probe.txt"));\n'
# game_end() ends the game at once, running oScreen's room end and game end events before the rest of
# its create event has set `screen` and the other surfaces they use.
EPILOGUE = '\nout("end", "ok");\nscreen = surface_create(2, 2);\npSurf = -1;\ndarkSurf = -1;\ngame_end();\n'


STEP = "// step:"
STEP_EPILOGUE = "\nglobal.probe_step = 0;\n"
# oPlayer1 calls checkBombPressed too; only oScreen's call is the probe's.
STEP_HOOK = "if (object_index != oScreen) return false;\nglobal.probe_step += 1;\n{0}\nreturn false;\n"


def rename(code):
    return re.sub(r"\b(" + "|".join(LIB) + r")\s*\(", lambda m: LIB[m.group(1)] + "(", code)


def main():
    spelunky, out, probe = sys.argv[1:4]
    with tempfile.TemporaryDirectory() as tmp:
        pairs = []

        def put(slot, code):
            path = os.path.join(tmp, slot + ".gml")
            with open(path, "w", encoding="cp1252") as f:
                f.write(code)
            pairs.append(f"{slot}={path}")

        for name, slot in LIB.items():
            with open(os.path.join(HERE, "probes", "lib", name + ".gml"), encoding="cp1252") as f:
                put(slot, rename(f.read()))
        with open(probe, encoding="cp1252") as f:
            code = f.read()
        if STEP in code:
            create, step = code.split(STEP, 1)
            put("scrReadStats", rename(PROLOGUE + create + STEP_EPILOGUE))
            put("checkBombPressed", rename(STEP_HOOK.replace("{0}", step)))
        else:
            put("scrReadStats", rename(PROLOGUE + code + EPILOGUE))
        for slot in EMPTY:
            if not (slot == "scrInit" and STEP in code):
                put(slot, "")
        subprocess.run([sys.executable, "-I", os.path.join(HERE, "gm8patch.py"), spelunky, out] + pairs, check=True)


if __name__ == "__main__":
    main()
