"""SCRATCH: what refills the dug pit. For each pair of maps: below-datum cells that turn from open to food (f) or
to soil (#), set against spoil pellets let go below the datum ('in') and re-cuts in that interval."""
import sys, glob, re, numpy as np
def maps(run):
    out = []
    for p in sorted(glob.glob(f"{run}/map_f*.txt")):
        fr = int(re.search(r"map_f(\d+)", p).group(1))
        L = open(p).read().split("\n"); x0, y0, w, h = map(int, L[0].split())
        out.append((fr, np.array([list(r) for r in L[1:1 + h]])))
    return x0, y0, out
tot = dict(open_peak=0, open_end=0, to_food=0, to_soil=0, dropped_in=0, rim_out=0, recut=0, dug=0)
for run in sys.argv[1:]:
    x0, y0, M = maps(run)
    surf = np.array([int(s) for s in open(f"{run}/surface.txt").read().split()])
    H, W = M[0][1].shape
    below = np.array([[y0 + yy >= surf[x0 + xx] for xx in range(W)] for yy in range(H)])
    ev = [l.split(",") for l in open(f"{run}/events.csv").read().split("\n")[1:] if l]
    vol = [((m == ".") | (m == "a"))[below].sum() for _, m in M]
    pk = int(np.argmax(vol))
    r = dict(open_peak=vol[pk] - vol[0], open_end=vol[-1] - vol[0], to_food=0, to_soil=0)
    for (f0, a), (f1, b) in zip(M[pk:], M[pk + 1:]):
        was = ((a == ".") | (a == "a")) & below
        r["to_food"] += (was & (b == "f")).sum(); r["to_soil"] += (was & (b == "#")).sum()
    fpk = M[pk][0]
    r["dropped_in"] = sum(1 for e in ev if e[2] == "in" and int(e[0]) > fpk)
    r["rim_out"] = sum(1 for e in ev if e[2] == "out" and int(e[0]) > fpk and abs(int(e[3]) - 128) <= 30)
    r["recut"] = sum(1 for e in ev if e[2] == "recut" and int(e[0]) > fpk)
    r["dug"] = sum(1 for e in ev if e[2] == "dig" and int(e[0]) > fpk)
    print(run, f"peak at {fpk}", r)
    for k in tot: tot[k] += r[k]
print("TOTAL", tot)
