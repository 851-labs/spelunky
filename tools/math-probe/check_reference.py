"""Judges the probe's results against a correctly rounded reference (mpmath at 300 bits).

Usage: python check_reference.py dump.txt   (needs `pip install mpmath`)

The dump holds the probe's disagreements, and every `hypot` evaluation with `PROBE_DUMP_ALL_HYPOT=1`. Prints,
per function and input domain, how often each implementation returned the correctly rounded result.

300 bits is plenty to decide the rounding. For hypot, either the arguments' exponents are more than 28 apart
(then the result is within 2^-57 of the larger argument, a double, so far from a half-way point), or the
exact sum of squares fits in about 170 bits, so the exact result is either exactly half-way between two
doubles (mpmath then gets it exactly) or at least about 2^-170 away from it.
"""

import math
import struct
import sys
from collections import defaultdict

import mpmath

mpmath.mp.prec = 300

# The probe's implementations, in its dump's column order (`NAMES` in src/lib.rs).
NAMES = ["platform", "libm", "pxfm", "upstream"]

# The smallest normal double, 2^-1022.
MIN_NORMAL = mpmath.ldexp(1, -1022)

ARITY = {"atan2": 2, "hypot": 2}


def f64(hex_bits):
    return struct.unpack(">d", bytes.fromhex(hex_bits))[0]


def to_double(x):
    """Rounds an mpf to the nearest double, ties to even, with a single rounding (inf past the largest).

    `float(mpf)` rounds to 53 bits and then lets the platform round again into the subnormal range, which
    can round twice. Below 2^-1022 the double's last bit is 2^-1074, so round the scaled value to an
    integer instead; that integer (below 2^52) and the scaling are exact.
    """
    if not mpmath.isfinite(x) or x == 0 or abs(x) >= MIN_NORMAL:
        return float(x)
    n = int(mpmath.nint(mpmath.ldexp(x, 1074)))  # nint rounds ties to even
    return math.copysign(math.ldexp(n, -1074), float(x))


def correctly_rounded(function, args):
    if function == "hypot" and any(math.isinf(a) for a in args):
        return math.inf  # IEEE: an infinite argument wins, even over NaN
    x = [mpmath.mpf(a) for a in args]
    exact = {
        "sin": lambda: mpmath.sin(x[0]),
        "cos": lambda: mpmath.cos(x[0]),
        "atan": lambda: mpmath.atan(x[0]),
        "tan": lambda: mpmath.tan(x[0]),
        "atan2": lambda: mpmath.atan2(x[0], x[1]),
        "hypot": lambda: mpmath.hypot(x[0], x[1]),
    }[function]()
    return to_double(exact)


def main(path):
    skipped = defaultdict(int)
    right = defaultdict(lambda: [0] * len(NAMES))
    total = defaultdict(int)
    for line in open(path):
        function, domain, hexes = line.rstrip("\n").split("\t")
        hexes = hexes.split()
        n_args = ARITY.get(function, 1)
        args = [f64(h) for h in hexes[:n_args]]
        results = [f64(h) for h in hexes[n_args:]]
        assert len(results) == len(NAMES), line
        key = (function, domain)
        reference = correctly_rounded(function, args)
        # mpmath has no signed zero or NaN payloads, so it can't judge those; leave them out.
        if reference == 0 or reference != reference or any(r != r for r in results):
            skipped[key] += 1
            continue
        total[key] += 1
        for i, r in enumerate(results):
            if struct.pack(">d", r) == struct.pack(">d", reference):
                right[key][i] += 1
    for key in sorted(total.keys() | skipped.keys()):
        function, domain = key
        counts = ", ".join(f"{name} {right[key][i]}" for i, name in enumerate(NAMES))
        what = "evaluations" if function == "hypot" else "disagreements"
        print(f"{function} ({domain}): {total[key]} {what}; correctly rounded: {counts}; skipped {skipped[key]}")


if __name__ == "__main__":
    main(sys.argv[1])
