"""Read-only reader for the server world's region files (Anvil), and `replay`, which copies
a box of it onto the staging site so a scenario can use the exact terrain of a play run.
Block states (stairs facing, doors) are dropped: only the block ids are copied.
"""
import os, struct, zlib
R = os.environ.get("WORLD_REGION", os.path.expanduser(
    "~/minecraft/vanilla-26.3/world/dimensions/minecraft/overworld/region"))

def nbt(b):
    pos = 0
    def rd(fmt):
        nonlocal pos
        v = struct.unpack_from(">" + fmt, b, pos); pos += struct.calcsize(fmt); return v[0]
    def name():
        nonlocal pos
        n = rd("H"); s = b[pos:pos+n].decode(); pos += n; return s
    def payload(t):
        nonlocal pos
        if t == 1: return rd("b")
        if t == 2: return rd("h")
        if t == 3: return rd("i")
        if t == 4: return rd("q")
        if t == 5: return rd("f")
        if t == 6: return rd("d")
        if t == 7:
            n = rd("i"); v = b[pos:pos+n]; pos += n; return v
        if t == 8: return name()
        if t == 9:
            it = rd("b"); n = rd("i"); return [payload(it) for _ in range(n)]
        if t == 10:
            d = {}
            while True:
                tt = rd("b")
                if tt == 0: return d
                k = name(); d[k] = payload(tt)
        if t == 11:
            n = rd("i"); v = struct.unpack_from(">%di" % n, b, pos); pos += 4*n; return list(v)
        if t == 12:
            n = rd("i"); v = struct.unpack_from(">%dq" % n, b, pos); pos += 8*n; return list(v)
    t = rd("b"); name(); return payload(t)

def chunk(cx, cz):
    rx, rz = cx >> 5, cz >> 5
    f = open(f"{R}/r.{rx}.{rz}.mca", "rb")
    f.seek(4 * ((cx & 31) + (cz & 31) * 32)); e = struct.unpack(">I", f.read(4))[0]
    off, n = (e >> 8) * 4096, e & 255
    if not off: return None
    f.seek(off); ln = struct.unpack(">I", f.read(4))[0]; ct = f.read(1)[0]
    data = f.read(ln - 1)
    return nbt(zlib.decompress(data) if ct == 2 else data)

_cache = {}
def sections(cx, cz):
    if (cx, cz) not in _cache:
        c = chunk(cx, cz); out = {}
        if c:
            for s in c.get("sections", []):
                bs = s.get("block_states")
                if bs: out[s["Y"]] = bs
        _cache[(cx, cz)] = out
    return _cache[(cx, cz)]

def nm(e):
    v = e if isinstance(e, str) else (e.get("Name") or e.get("") or next(iter(e.values())))
    return v.replace("minecraft:", "")

def block(x, y, z):
    secs = sections(x >> 4, z >> 4); bs = secs.get(y >> 4)
    if bs is None: return "air"
    pal = bs["palette"]
    if len(pal) == 1: return nm(pal[0])
    bits = max(4, (len(pal) - 1).bit_length())
    per = 64 // bits
    idx = ((y & 15) << 8) | ((z & 15) << 4) | (x & 15)
    word = bs["data"][idx // per] & ((1 << 64) - 1)
    return nm(pal[(word >> ((idx % per) * bits)) & ((1 << bits) - 1)])


AIR = ("air", "cave_air", "void_air")


def runs(x0, x1, y0, y1, z0, z1):
    """Yields (y, z, xa, xb, block) for runs of one non-air block along x."""
    for y in range(y0, y1 + 1):
        for z in range(z0, z1 + 1):
            xa, cur = x0, None
            for x in range(x0, x1 + 2):
                b = block(x, y, z) if x <= x1 else None
                if b != cur:
                    if cur is not None and cur not in AIR:
                        yield y, z, xa, x - 1, cur
                    xa, cur = x, b


def replay(rc, box, to):
    """Copies the world box (x0, x1, y0, y1, z0, z1) so that its min corner lands on `to`
    (x, y, z), as fill commands over RCON. Returns the number of commands sent."""
    x0, x1, y0, y1, z0, z1 = box
    n, batch = 0, []
    for y, z, xa, xb, b in runs(*box):
        c = (f"fill {to[0] + xa - x0} {to[1] + y - y0} {to[2] + z - z0} "
             f"{to[0] + xb - x0} {to[1] + y - y0} {to[2] + z - z0} {b}")
        batch.append(c)
        if len(batch) >= 200:
            rc(*batch); n += len(batch); batch = []
    if batch:
        rc(*batch); n += len(batch)
    return n
