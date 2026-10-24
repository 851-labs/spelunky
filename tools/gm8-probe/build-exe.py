#!/usr/bin/env python3
"""Builds a probe executable from Spelunky.exe: build-exe.py SPELUNKY.exe OUT.exe PROBE.gml

The probe's code replaces `scrReadStats`, the first script the game runs (oScreen's create event in
rScreenInit). It runs after `probe.txt` is emptied, and is followed by `out("end", "ok")` and a
`game_end()`, so it only has to call `out(name, value)`. Scripts that would touch the sound DLL or the
save files become empty, so the game does nothing else before it ends. `hex` and `out` (probes/lib/)
take the slots of two scripts the probe never needs, and calls to them are renamed to match.
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
            put("scrReadStats", rename(PROLOGUE + f.read() + EPILOGUE))
        for slot in EMPTY:
            put(slot, "")
        subprocess.run([sys.executable, "-I", os.path.join(HERE, "gm8patch.py"), spelunky, out] + pairs, check=True)


if __name__ == "__main__":
    main()
