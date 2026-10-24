"""Every original-ground cut in an in-run colony: how deep, and whether it was cut from an open pit (the cell above it
open to the sky in the last map before the cut) or under a roof. Plus where pellets went."""
import sys, csv, glob, re, collections
import numpy as np
def maps(run):
    out = {}
    for p in sorted(glob.glob(f"{run}/map_f*.txt")):
        f = int(re.search(r"map_f(\d+)", p).group(1))
        with open(p) as fh:
            x0, y0, w, h = map(int, fh.readline().split())
            rows = [ln.rstrip("\n") for ln in fh][:h]
        m = np.array([[c in ".af" for c in (r + " " * w)[:w]] for r in rows])
        sky = np.zeros_like(m)
        for x in range(w):
            y = 0
            while y < h and m[y, x]:
                sky[y, x] = True; y += 1
        out[f] = (x0, y0, sky)
    return out
for run in sys.argv[1:]:
    M = maps(run); frames = sorted(M)
    surf = [int(l) for l in open(f"{run}/surface.txt")]
    depth = collections.Counter(); skyc = collections.Counter(); n = 0
    kinds = collections.Counter()
    for r in csv.DictReader(open(f"{run}/events.csv")):
        kinds[r["kind"]] += 1
        if r["kind"] != "dig": continue
        f, x, y, v = int(r["frame"]), int(r["x"]), int(r["y"]), int(r["v"])
        if abs(x - 128) > 20: continue
        n += 1
        d = "0 (top row)" if v == 0 else "1-2" if v <= 2 else "3-7" if v <= 7 else "8+"
        depth[d] += 1
        prev = [g for g in frames if g <= f]
        if not prev: skyc["(no map yet)"] += 1; continue
        x0, y0, sky = M[prev[-1]]
        yy, xx = y - 1 - y0, x - x0
        open_above = 0 <= yy < sky.shape[0] and 0 <= xx < sky.shape[1] and sky[yy, xx]
        skyc[("from open pit" if v > 0 else "surface") if open_above else ("under a roof" if v > 0 else "top row, roofed?")] += 1
    print(run, "cuts within 20 cols:", n)
    print("   depth:", dict(depth))
    print("   cut from:", dict(skyc))
    print("   events:", {k: kinds[k] for k in ("dig", "recut", "out", "in", "deliver")})
