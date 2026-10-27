"""SCRATCH: fresh-colony comparison. usage: python3 -I freshcmp.py label=dir/prefix ... (prefix: dir/arm, runs dir/arm{s}_{t})"""
import sys, glob, os, re, collections, numpy as np
NEST = 128
def tot(path):
    alive = {}
    for p in glob.glob(f"{path}/ants_f*.txt"):
        alive[int(re.search(r"ants_f(\d+)", p).group(1))] = [l.split() for l in open(p)]
    fr = sorted(alive); surf = [int(s) for s in open(path + "/surface.txt").read().split()]
    c = collections.Counter(); deep = 0; starved = 0
    for l in open(path + "/events.csv").read().split("\n")[1:]:
        p = l.split(",")
        if len(p) < 6: continue
        c[p[2]] += 1
        if p[2] == "deliver" and int(p[4]) - surf[int(p[3])] >= 3: deep += 1
        if p[2] == "death" and p[5] in ("1", "1.0"): starved += 1
    at = [abs(int(r[1]) - NEST) for f in fr for r in alive[f]]
    return dict(surv=len(alive[fr[-1]]), starved=starved, deliver=c["deliver"], deep=deep, homeeat=c["homeeat"], out=c["out"], dig=c["dig"],
                pile=np.mean([a >= 15 for a in at]) if at else 0, home=np.mean([a <= 8 for a in at]) if at else 0)
for spec in sys.argv[1:]:
    lab, pre = spec.split("=")
    runs = sorted(glob.glob(pre + "*_1[1-4]"))
    pops = collections.defaultdict(list)
    for r in runs:
        if os.path.exists(r + "/events.csv"):
            pops[re.sub(r"_1[1-4]$", "", r)].append(tot(r))
    allr = [x for v in pops.values() for x in v]
    per = [np.mean([x["surv"] for x in v]) for v in pops.values()]
    print(f"{lab:10s} pops {len(pops)} runs {len(allr):2d} " + " ".join(f"{k} {np.mean([x[k] for x in allr]):.2f}" if k in ("pile", "home") else f"{k} {np.mean([x[k] for x in allr]):.0f}" for k in allr[0]) + f"  worst pop {min(per):.1f}")
