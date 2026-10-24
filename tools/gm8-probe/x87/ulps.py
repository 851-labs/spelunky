#!/usr/bin/env python3
"""How far GM8's answers to the probes in probes/x87/ are from the port's, per function, and whether a model of
the x87 explains them.

    uv run --with mpmath python3 -I tools/gm8-probe/x87/ulps.py RESULTS_DIR [OTHER_DIR]

RESULTS_DIR holds deg.txt, rad.txt, dir.txt and funcs.txt (the probes' outputs). For each line it works out the
probe's input (from the line's name, as the probe computed it) and computes, with mpmath:

- what the port computes: correctly rounded primitives, combined in doubles as `gm::math` combines them;
- what the x87 model computes (README.md, "Real x87"): fsin, fcos and fptan reduce their argument by Intel's
  66-bit pi, exactly, and round the exact function of the reduced argument to 64 bits; fpatan and fyl2x round to
  64 bits; the runner keeps those 64-bit results and pi (fldpi) in registers, and every fmul and fdiv rounds to
  53 bits (the precision control GM8 runs with). `exp` and `power` aren't modelled.

It prints, per function, the lines that differ from the port (and by how many ulps) and from the model. With
OTHER_DIR, it also counts the lines where the two runs differ (say real Intel and Rosetta).
"""

import math
import os
import struct
import sys
from collections import defaultdict

import mpmath

mpmath.mp.prec = 300
DEG = math.pi / 180  # f64::to_radians' constant
RAD = 180 / math.pi  # f64::to_degrees'


def rnd(x, bits):
    with mpmath.workprec(bits):
        return +x


def r53(x):
    return rnd(x, 53)


def r64(x):
    return rnd(x, 64)


PI64 = r64(mpmath.pi)  # fldpi
PI_180 = r64(PI64 / 180)  # Delphi's Pi / 180, folded to an extended constant
# The pi fsin, fcos and fptan reduce by: 0xC90FDAA22168C234C / 2^66 (66 significant bits).
PI66 = mpmath.mpf(int("C90FDAA22168C234C", 16)) / mpmath.mpf(2) ** 66


def bits(x):
    return struct.unpack("<q", struct.pack("<d", x))[0]


def real(h):
    return struct.unpack("<d", struct.pack("<Q", int(h, 16)))[0]


def ulps(a, b):
    """Signed distance from b to a in ulps (adjacent doubles are 1 apart, across zero too)."""
    order = lambda x: bits(x) if bits(x) >= 0 else -(bits(x) & (2**63 - 1))
    return order(a) - order(b)


def atan2(y, x):
    if y == 0:  # mpmath has no -0: atan2(+-0, x) is +-0, or +-pi for a negative x (or -0)
        a = mpmath.mpf(0) if math.copysign(1, x) > 0 else +mpmath.pi
        return -a if math.copysign(1, y) < 0 else a
    return mpmath.atan2(y, x)


def cr(f, *args):
    return float(atan2(*args) if f is mpmath.atan2 else f(*(mpmath.mpf(a) for a in args)))


def x87_trig(fn, x):
    """fsin, fcos or fptan of x, before the store to a double: reduction by PI66, rounded to 64 bits."""
    x = mpmath.mpf(x)
    k = int(mpmath.nint(x / (PI66 / 2)))
    r = x - k * (PI66 / 2)
    s, c = mpmath.sin(r), mpmath.cos(r)
    sin, cos = [s, c, -s, -c][k % 4], [c, -s, -c, s][k % 4]
    return r64({"sin": sin, "cos": cos, "tan": sin / cos}[fn])


def x87_degrees(y, x):
    """point_direction and the runner's direction: fpatan, then * 180 / pi, each rounded to 53 bits."""
    if y == 0 and math.copysign(1, x) > 0:
        return y  # +-0, which mpmath can't hold
    d = r53(r53(r64(atan2(y, x)) * 180) / PI64)
    return float(r53(d + 360) if d < 0 else d)


def snap(v):
    # gm::Instance::update_hvspeed and update_speed_direction: within 0.0001 of a whole number snaps to it.
    r = float(round(v))  # half to even, like gm::math::round
    return r if abs(r - v) < 0.0001 else v


def degrees(y, x):
    """gm::math::point_direction: atan2 in degrees, `rem_euclid(360)`."""
    d = cr(mpmath.atan2, y, x) * RAD
    return d + 360 if d < 0 else d


def load(path):
    out = {}
    with open(path, encoding="cp1252") as f:
        for line in f:
            name, _, value = line.rstrip("\n").partition(" = ")
            if value and not value.startswith('"'):
                out[name] = real(value.split()[0])
    return out


def expected(probe, name, lines):
    """(function, the port's answer, the x87 model's or None) for one probe line, or None if it isn't measured."""
    fn, _, arg = name.partition("_")
    if probe == "deg":
        if fn == "rad":
            x = int(arg) / 2
            return "degtorad", x * DEG, float(r53(x * PI_180))
        if fn in ("sin", "cos"):
            x = lines["rad_" + arg]
            return fn + " (degrees)", cr(getattr(mpmath, fn), x), float(r53(x87_trig(fn, x)))
        if fn in ("hsp", "vsp"):
            k = int(arg)
            speed, d = 1 + (k % 8) / 2, k / 2
            f = "cos" if fn == "hsp" else "sin"
            port = cr(getattr(mpmath, f), d * DEG) * speed
            # The runner's own degrees to radians: direction * pi / 180, not degtorad's constant.
            model = r53(speed * x87_trig(f, r53(r53(d * PI64) / 180)))
            if fn == "vsp":
                port, model = -port, -model
            return "motion " + fn, snap(port), snap(float(model))
        if fn in ("dir", "spd"):
            a, b = map(int, arg.split("_"))
            hs, vs = a / 2 - 6, b / 2 - 6
            if fn == "spd":
                return "motion speed", math.sqrt(hs * hs + vs * vs), math.sqrt(hs * hs + vs * vs)
            return "motion direction", snap(degrees(-vs, hs)), snap(x87_degrees(-vs, hs))
    if probe == "rad":
        k = int(arg[1:])
        x = {"c": k / 100, "s": k / 64, "q": k * k / 3}[arg[0]]
        label = {"c": "k/100", "s": "k/64", "q": "k*k/3"}[arg[0]]
        return f"{fn} ({label})", cr(getattr(mpmath, fn), x), float(r53(x87_trig(fn, x)))
    if probe == "dir":
        i, j = map(int, arg.split("_"))
        y = -float(j)  # -0 for j = 0, as in GML
        if fn == "pdir":
            # point_direction(0, 0, 0, 0) is +0 on GM8; (0, 0) to (1, 0) is -0, as atan2(-0, 1).
            return "point_direction (whole)", degrees(y, i), 0.0 if i == j == 0 else x87_degrees(y, i)
        if fn == "atan2":
            return "arctan2 (whole)", cr(mpmath.atan2, y, i), y if j == 0 and i >= 0 else float(r53(r64(atan2(y, i))))
        dx, dy = i * 3 / 8 - 7, j * 5 / 8 - 5
        if fn == "fpdir":
            return "point_direction (fractional)", degrees(-dy, dx), x87_degrees(-dy, dx)
        d = math.sqrt(dx * dx + dy * dy)
        return "point_distance (fractional)", d, d
    if probe == "funcs":
        v = mpmath.mpf(int(arg) / 100)
        f = {
            "arctan": lambda: (cr(mpmath.atan, v), float(r53(r64(mpmath.atan(v))))),
            "sqrt": lambda: (math.sqrt(v), math.sqrt(v)),
            "ln": lambda: (cr(mpmath.log, v), float(r53(r64(mpmath.log(v))))),
            "exp": lambda: (cr(mpmath.exp, v), None),
            "power": lambda: (cr(mpmath.power, v, 1.5), None),
            "pdist": lambda: (math.sqrt(float(v) ** 2 + 1.3 * 1.3),) * 2,
        }[fn]()
        return ({"pdist": "point_distance"}.get(fn, fn),) + f
    return None


def main():
    dirs = sys.argv[1:]
    # function: [lines, {ulps from the port: count}, differ from the model, differ from OTHER_DIR]
    stats = defaultdict(lambda: [0, defaultdict(int), 0, 0])
    for probe in ("deg", "rad", "dir", "funcs"):
        if not os.path.exists(os.path.join(dirs[0], probe + ".txt")):
            continue
        runs = [load(os.path.join(d, probe + ".txt")) for d in dirs]
        for name, got in runs[0].items():
            e = expected(probe, name, runs[0])
            if e is None:
                continue
            s = stats[e[0]]
            s[0] += 1
            d = ulps(got, e[1])
            if d:
                s[1][d] += 1
            s[2] += e[2] is None or bits(e[2]) != bits(got)
            if len(runs) > 1 and bits(runs[1].get(name, math.nan)) != bits(got):
                s[3] += 1
    other = "  differ" if len(dirs) > 1 else ""
    print(f"{'function':30} {'lines':>5} {'port':>5} {'%':>6} {'model':>5}{other}  ulps from the port (count)")
    for fn, (n, off, model, diff) in stats.items():
        k = sum(off.values())
        hist = " ".join(f"{d:+}:{c}" for d, c in sorted(off.items()))
        print(f"{fn:30} {n:5} {k:5} {100 * k / n:6.2f} {model:5}" + (f" {diff:7}" if other else "") + f"  {hist}")


if __name__ == "__main__":
    main()
