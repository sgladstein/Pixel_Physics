"""SCRATCH (teacher-training step 1, not for main): what a genome does in the soil arena, from a copy log.
usage: python3 -I behave.py LOG [LOG...]
Per log: full loads, cuts by class (foot / wall / rim / recut, as pellets.py classes them), and where the decisions
are spent (empty or carrying, surface or below ground) with each state's verb mix. Settle rows are excluded."""
import sys, collections
import numpy as np

VN = ["walk", "turn", "rest", "dig", "pickup", "drop", "eat", "feed", "attack"]
for path in sys.argv[1:]:
    G, C, E, Q = {}, collections.defaultdict(list), {}, []
    for line in open(path):
        p = line.rstrip("\n").split(",")
        tag, k = int(p[0]), p[1]
        if k == "G":
            G[tag] = p
        elif k == "C":
            C[tag].append((int(p[3]), int(p[5]), int(p[6]), int(p[7]), p[8] == "1"))
        elif k == "E":
            E[tag] = int(p[3])
        elif k == "Q" and p[6] == "0":
            o = p[13].split(";")
            Q.append((tag, int(p[4]), int(p[5]), int(o[int(p[7])].split(":")[0]), p[9] == "1"))
    cls = collections.Counter()
    for tag, cs in C.items():
        shaft = int(G[tag][7])
        seen = set()
        for f, x, y, d, g0 in sorted(cs):
            first = g0 and (x, y) not in seen
            seen.add((x, y))
            cls["recut" if not first else "foot" if d >= shaft else "wall" if d > 0 else "rim"] += 1
    def depth(tag, x, y):
        g = G[tag]
        x0 = int(g[3])
        dat = g[8].split(" ")
        lows = [int(dat[c - x0]) for c in range(x - 3, x + 4) if 0 <= c - x0 < len(dat) and dat[c - x0] != "x"]
        return y - max(lows) if lows else 0
    st = collections.defaultdict(collections.Counter)
    for tag, x, y, v, carrying in Q:
        st[("carrying" if carrying else "empty") + (", below" if depth(tag, x, y) > 0 else ", surface")][VN[v]] += 1
    n = len(E)
    print(f"== {path}: {n} worlds, full loads {sum(E.values()) / n:.2f} a world; cuts a world: "
          + ", ".join(f"{k} {cls[k] / n:.1f}" for k in ("foot", "wall", "rim", "recut")))
    tot = sum(sum(c.values()) for c in st.values())
    for s in sorted(st):
        c = st[s]
        m = sum(c.values())
        print(f"   {s:20s} {m / tot:6.1%}  " + " ".join(f"{v} {c[v] / m:.0%}" for v, _ in c.most_common(4)))
