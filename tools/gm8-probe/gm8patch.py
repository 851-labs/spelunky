#!/usr/bin/env python3
"""Replace scripts in a GameMaker 8.0 executable's embedded game data.

    gm8patch.py IN.exe OUT.exe NAME=FILE.gml [NAME=FILE.gml ...]

Each NAME is an existing script in IN.exe; its code becomes FILE.gml's. Everything else, the runner
included, is copied unchanged. GM8 compiles scripts from their source when the game starts, so the
real runner runs the new code. Written from the format, not from any GameMaker or OpenGMK code.

Only the plain GM 8.0 layout is supported (no UPX, no anti-decompiler), which is what Spelunky.exe is.
That layout, after the runner's own PE image:

    0x144AC0          u32: where to start looking for the game header (then every 10000 bytes)
    header            u32 magic (the loader's CMP at 0xA49C7), u32 version, 8 more bytes
    settings          u32 length, zlib
    DirectX DLL       u32 name length, name, u32 length, data
    encrypted data    u32 n1, u32 n2, n1 dwords of junk, 256-byte swap table, n2 dwords of junk,
                      u32 length, then `length` encrypted bytes holding every asset

Inside the decrypted data, each script is a u32 length and a zlib block of: u32 1 (exists),
pascal string name, u32 800, pascal string source. A pascal string is a u32 length and bytes.
"""

import struct
import sys
import zlib

LOADER_SIG_AT = 0xA49BE
LOADER_SIG = bytes([0x8B, 0x45, 0xF4, 0xE8, 0x2A, 0xBD, 0xFD, 0xFF, 0x3D])
HEADER_PTR_AT = 0x144AC0


def u32(buf, at):
    return struct.unpack_from("<I", buf, at)[0]


def pas(s):
    return struct.pack("<I", len(s)) + s


def locate(exe):
    """Returns (offset of the swap table, offset of the encrypted bytes, their length)."""
    if exe[LOADER_SIG_AT : LOADER_SIG_AT + len(LOADER_SIG)] != LOADER_SIG:
        sys.exit("not a plain GM 8.0 executable (loader signature not found)")
    magic = u32(exe, LOADER_SIG_AT + len(LOADER_SIG))
    at = u32(exe, HEADER_PTR_AT)
    while u32(exe, at) != magic:
        at += 10000
        if at + 4 > len(exe):
            sys.exit("GM 8.0 header not found")
    at += 16
    at += 4 + u32(exe, at)  # settings
    at += 4 + u32(exe, at)  # DLL name
    at += 4 + u32(exe, at)  # DLL data
    n1, n2 = u32(exe, at), u32(exe, at + 4)
    table = at + 8 + n1 * 4
    length_at = table + 256 + n2 * 4
    return table, length_at + 4, u32(exe, length_at)


def decrypt(enc, swap):
    rev = bytearray(256)
    for i, s in enumerate(swap):
        rev[s] = i
    n = len(enc)
    # Each byte was substituted after adding its encrypted predecessor and its position.
    q = bytearray(n)
    q[0] = enc[0]
    for j in range(1, n):
        q[j] = (rev[enc[j]] - enc[j - 1] - j) & 0xFF
    # Then undo the position-dependent swaps, last first.
    for i in range(n - 1, -1, -1):
        b = max(i - swap[i & 0xFF], 0)
        q[i], q[b] = q[b], q[i]
    return q


def encrypt(plain, swap):
    q = bytearray(plain)
    n = len(q)
    for i in range(n):
        b = max(i - swap[i & 0xFF], 0)
        q[i], q[b] = q[b], q[i]
    enc = bytearray(n)
    enc[0] = q[0]
    for j in range(1, n):
        enc[j] = swap[(q[j] + enc[j - 1] + j) & 0xFF]
    return enc


def find_script(data, name):
    """Returns (offset of the block's length, block length) of the script called `name`."""
    want = struct.pack("<I", 1) + pas(name) + struct.pack("<I", 800)
    at = data.find(b"\x78", 4)
    while at != -1:
        if (data[at] * 256 + data[at + 1]) % 31 == 0:
            length = u32(data, at - 4)
            if 0 < length <= len(data) - at:
                try:
                    head = zlib.decompressobj().decompress(bytes(data[at : at + length]), len(want))
                except zlib.error:
                    head = b""
                if head == want:
                    return at - 4, length
        at = data.find(b"\x78", at + 1)
    sys.exit(f"script {name.decode()} not found")


def main():
    if len(sys.argv) < 4:
        sys.exit(__doc__)
    src, dst, pairs = sys.argv[1], sys.argv[2], sys.argv[3:]
    exe = open(src, "rb").read()
    table, start, length = locate(exe)
    swap = exe[table : table + 256]
    data = decrypt(exe[start : start + length], swap)
    for pair in pairs:
        name, path = pair.split("=", 1)
        name = name.encode("cp1252")
        code = open(path, "rb").read().replace(b"\r\n", b"\n").replace(b"\n", b"\r\n")
        at, old = find_script(data, name)
        block = zlib.compress(struct.pack("<I", 1) + pas(name) + struct.pack("<I", 800) + pas(code))
        data[at : at + 4 + old] = struct.pack("<I", len(block)) + block
    out = exe[: start - 4] + struct.pack("<I", len(data)) + encrypt(data, swap) + exe[start + length :]
    open(dst, "wb").write(out)


if __name__ == "__main__":
    main()
