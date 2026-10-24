"""Reference vectors for the x87 instructions: exact results (mpmath) rounded to 64 bits.

    python3 vectors.py N SEED > out.txt

Each line: fn x_bits y_bits neg exp sig (hex), fn in sin cos tan atan2 ln exp2m1.
"""
import math, random, struct, sys
import mpmath

mpmath.mp.prec = 400
PI66 = mpmath.mpf(int("C90FDAA22168C234C", 16)) / mpmath.mpf(2) ** 66


def r64(x):
    with mpmath.workprec(64):
        return +x


def f80(v):
    v = r64(v)
    if v == 0:
        return (0, 0, 0)
    neg = 1 if v < 0 else 0
    v = abs(v)
    m, e = mpmath.frexp(v)  # v = m 2^e, m in [0.5, 1)
    sig = int(m * mpmath.mpf(2) ** 64)
    return (neg, int(e) - 1, sig)


def trig(fn, x):
    x = mpmath.mpf(x)
    neg = x < 0
    ax = abs(x)
    k = int(mpmath.nint(ax / (PI66 / 2)))
    r = ax - k * (PI66 / 2)
    s, c = mpmath.sin(r), mpmath.cos(r)
    sin, cos = [s, c, -s, -c][k % 4], [c, -s, -c, s][k % 4]
    v = {"sin": sin, "cos": cos, "tan": sin / cos}[fn]
    if neg and fn != "cos":
        v = -v
    return v


def bits(x):
    return struct.unpack("<Q", struct.pack("<d", x))[0]


def main():
    n, seed = int(sys.argv[1]), int(sys.argv[2])
    rnd = random.Random(seed)
    out = []

    def logu(lo, hi):
        return math.copysign(2 ** rnd.uniform(lo, hi), rnd.choice([-1, 1]))

    for i in range(n):
        kind = i % 6
        if kind < 3:
            fn = ["sin", "cos", "tan"][kind]
            c = rnd.random()
            if c < 0.4:
                x = logu(-30, 4)
            elif c < 0.7:
                x = logu(4, 62.9)
            elif c < 0.85:  # near multiples of pi/2
                x = float(rnd.randint(1, 10**6) * mpmath.pi / 2)
            else:  # game-like: degtorad of a double
                x = rnd.randint(0, 720) / 2 * math.pi / 180
            v = trig(fn, x)
            out.append((fn, bits(x), 0, f80(v)))
        elif kind == 3:
            y, x = logu(-40, 40), logu(-40, 40)
            if rnd.random() < 0.3:
                y, x = float(rnd.randint(-50, 50)), float(rnd.randint(-50, 50))
                if x == 0 and y == 0:
                    continue
            if y == 0:
                continue
            v = mpmath.atan2(y, x)
            out.append(("atan2", bits(y), bits(x), f80(v)))
        elif kind == 4:
            x = abs(logu(-60, 60)) if rnd.random() < 0.7 else 1 + rnd.uniform(-0.01, 0.01)
            out.append(("ln", bits(x), 0, f80(mpmath.log(x))))
        else:
            x = rnd.uniform(-1, 1) if rnd.random() < 0.8 else logu(-40, -1)
            out.append(("exp2m1", bits(x), 0, f80(mpmath.power(2, x) - 1)))
    for fn, x, y, (neg, e, sig) in out:
        print(f"{fn} {x:016x} {y:016x} {neg} {e} {sig:016x}")


main()
