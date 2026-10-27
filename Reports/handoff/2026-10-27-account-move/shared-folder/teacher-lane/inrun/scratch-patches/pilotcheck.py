"""SCRATCH: the in-run selection pilot, selection vs random parents. usage: python3 -I pilotcheck.py pilot 8"""
import sys, os, glob, re
import numpy as np
D, N = sys.argv[1], int(sys.argv[2])
QS = [(0, 50000), (50000, 100000), (100000, 150000), (150000, 200001)]
def run(path):
    alive = {}
    for p in glob.glob(f"{path}/ants_f*.txt"):
        alive[int(re.search(r"ants_f(\d+)", p).group(1))] = sum(1 for _ in open(p))
    fr = sorted(alive)
    step = (fr[1] - fr[0]) / 1000 if len(fr) > 1 else 2
    end = fr[-1]
    QS = [(end * q // 4, end * (q + 1) // 4 + (1 if q == 3 else 0)) for q in range(4)]
    def antk(a, b):  # ant-thousand-frames in [a,b)
        return sum(alive[f] * step for f in fr if a < f <= b)
    ev = {}
    for ln in open(f"{path}/events.csv").read().split("\n")[1:]:
        if ln:
            f, i, k, *_ = ln.split(",")
            ev.setdefault(k, []).append(int(f))
    deaths = [ln.split(",") for ln in open(f"{path}/events.csv").read().split("\n")[1:] if ",death," in ln]
    starved = sum(1 for d in deaths if d[5] in ("1", "1.0"))
    out = dict(final=alive[fr[-1]], starved=starved, QS=QS)
    for k in ["deliver", "dig", "out"]:
        out[k] = [sum(1 for f in ev.get(k, []) if a <= f < b) / max(antk(a, b), 1) for a, b in QS]
    if os.path.exists(f"{path}/final/index.csv"):
        rows = [l.split(",") for l in open(f"{path}/births.csv").read().split("\n")[1:] if l]
        out["births"] = sum(r[5] == "birth" for r in rows); out["culls"] = sum(r[5] == "cull" for r in rows)
        out["maxgen"] = max([int(r[4]) for r in rows if r[5] == "birth"] or [0])
        fam = [l.split(",")[1] for l in open(f"{path}/final/index.csv").read().split("\n")[1:] if l]
        out["families"] = len(set(fam))
    return out
for arm in (sys.argv[3].split(",") if len(sys.argv) > 3 else ["score", "random"]):
    rs = [run(f"{D}/{arm}_{s}") for s in range(1, N + 1) if os.path.exists(f"{D}/{arm}_{s}/final")]
    print(f"== {arm}: {len(rs)} runs")
    for k in ["final", "starved", "births", "culls", "maxgen", "families"]:
        print(f"  {k:9s}", [r.get(k) for r in rs])
    for k in ["deliver", "dig", "out"]:
        m = np.array([r[k] for r in rs])
        print(f"  {k:8s} per 1k ant-frames by quarter: " + "  ".join(f"{m[:, q].mean():.3f}" for q in range(4)) + f"   (last/first {m[:, 3].mean() / max(m[:, 0].mean(), 1e-9):.2f})")
