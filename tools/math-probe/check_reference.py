"""Judges the probe's disagreements against a correctly rounded reference (mpmath at 200 bits).

Usage: python check_reference.py dump.txt   (needs `pip install mpmath`)

Prints, per function, how often each implementation returned the correctly rounded result.
"""

import math
import struct
import sys
from collections import defaultdict

import mpmath

mpmath.mp.prec = 200

# The probe's implementations, in its dump's column order (`NAMES` in src/lib.rs).
NAMES = ["platform", "libm", "pxfm", "upstream"]

# The smallest normal double, 2^-1022.
MIN_NORMAL = mpmath.ldexp(1, -1022)


def f64(hex_bits):
    return struct.unpack(">d", bytes.fromhex(hex_bits))[0]


def to_double(x):
    """Rounds an mpf to the nearest double, ties to even, with a single rounding.

    `float(mpf)` rounds to 53 bits and then lets the platform round again into the subnormal range, which
    can round twice. Below 2^-1022 the double's last bit is 2^-1074, so round the scaled value to an
    integer instead; that integer (below 2^52) and the scaling are exact.
    """
    if not mpmath.isfinite(x) or x == 0 or abs(x) >= MIN_NORMAL:
        return float(x)
    n = int(mpmath.nint(mpmath.ldexp(x, 1074)))  # nint rounds ties to even
    return math.copysign(math.ldexp(n, -1074), float(x))


def correctly_rounded(function, args):
    x = [mpmath.mpf(a) for a in args]
    exact = {
        "sin": lambda: mpmath.sin(x[0]),
        "cos": lambda: mpmath.cos(x[0]),
        "atan": lambda: mpmath.atan(x[0]),
        "tan": lambda: mpmath.tan(x[0]),
        "atan2": lambda: mpmath.atan2(x[0], x[1]),
    }[function]()
    return to_double(exact)


def main(path):
    skipped = defaultdict(int)
    right = defaultdict(lambda: [0] * len(NAMES))
    total = defaultdict(int)
    for line in open(path):
        function, *hexes = line.split()
        n_args = 2 if function == "atan2" else 1
        args = [f64(h) for h in hexes[:n_args]]
        results = [f64(h) for h in hexes[n_args:]]
        assert len(results) == len(NAMES), line
        reference = correctly_rounded(function, args)
        # mpmath has no signed zero or NaN payloads, so it can't judge those; leave them out.
        if reference == 0 or reference != reference or any(r != r for r in results):
            skipped[function] += 1
            continue
        total[function] += 1
        for i, r in enumerate(results):
            if struct.pack(">d", r) == struct.pack(">d", reference):
                right[function][i] += 1
    for function in sorted(total):
        counts = ", ".join(f"{name} {right[function][i]}" for i, name in enumerate(NAMES))
        print(f"{function}: {total[function]} disagreements; correctly rounded: {counts}; skipped {skipped[function]}")


if __name__ == "__main__":
    main(sys.argv[1])
