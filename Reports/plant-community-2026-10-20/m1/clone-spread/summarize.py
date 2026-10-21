#!/usr/bin/env python3
"""Size spread of one lone plant per arm: alive count, CV, and the per-seed
shoot sizes, from `plant_m0 mode=plastic axis=shade` SEED lines.

    python3 summarize.py base5000001
"""
import glob, re, statistics as st, sys
for f in sorted(glob.glob(f"{sys.argv[1]}/*.txt")):
    v = {}
    for line in open(f):
        m = re.match(r"\s+SEED bright, off s=(\d+) shoot (\S+)", line)
        if m:
            v[int(m[1])] = None if m[2] == "dead" else int(m[2])
    alive = [x for x in v.values() if x is not None]
    cv = f"{st.pstdev(alive) / st.mean(alive):.2f}" if len(alive) > 1 else "-"
    print(f"{f:44} alive {len(alive)}/{len(v)} CV {cv}  by seed {[v[k] for k in sorted(v)]}")
