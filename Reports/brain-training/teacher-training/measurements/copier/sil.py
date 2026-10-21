"""SCRATCH (teacher-training, the self-imitation label check, not for main): can the brain's OWN outcomes supply the
teacher's labels?  usage: python3 -I sil.py LOG [T_DOWN=200] [T_HOME=150]

Self-imitation copies the brain's own decisions that turned out well. For each of the teacher's three rules, this labels
the brain's decisions by what followed (hindsight, per decision -- measurement 1 showed a whole run's score is mostly
luck) and asks how well that outcome label picks out the decisions the teacher would have made:

  dig   the brain dug: kept if that pellet became a full load.  Teacher: the cut was a foot cell.
  down  empty, below ground, no foot cell in reach, walking, a deeper step offered: kept if the ant's next cut is a
        foot cut within T_DOWN frames.  Teacher: the brain took a deepest step.
  home  empty, on the surface, 3+ columns from the mouth, walking: kept if the ant is within 3 columns of the mouth
        within T_HOME frames.  Teacher: the brain took the step nearest the mouth.

Printed per rule: how many decisions, the teacher's agreement rate in the kept set and in the dropped set (a useful
filter keeps far more teacher-agreeing decisions than it drops), and what self-imitation CANNOT label: decisions the
brain never took (the teacher's "dig here" when the brain walked on), which outcome-copying has no example of."""
import sys
import numpy as np

sys.argv = [sys.argv[0], sys.argv[1], "4", "none"] + sys.argv[2:]
LOG = sys.argv[1]
T_DOWN = int(sys.argv[4]) if len(sys.argv) > 4 else 200
T_HOME = int(sys.argv[5]) if len(sys.argv) > 5 else 150
src = open(__file__.replace("sil.py", "probe.py")).read()
exec(src.split("# ---------------------------------------------------------------- build the four data sets")[0])

# put-down events (probe's parser skips them)
# put-down events, per world in log order (mirrored worlds share a seed, so a seed is not a key)
D, cur = [], []
for line in open(LOG):
    if line[0] == "D":
        p = line.split(",")
        cur.append((int(p[2]), int(p[3]), int(p[4]), int(p[5]), p[6].strip() == "1"))
    elif line[0] == "E":
        D.append(cur)
        cur = []

stats = {k: {"n": 0, "kept": 0, "kept_agree": 0, "drop_agree": 0, "untaken": 0} for k in ("dig", "down", "home")}
for wi, w in enumerate(worlds(LOG)):
    ground0, depth, cuts, founding, mouth = facts(w)
    S = w["shaft"]
    # the scorer's jaws rule, as pellets.py: each cut fills the jaws, the next put-down empties them
    puts = sorted(D[wi])
    # world rows for a seed come in order; split this world's puts by frame range (single ant, threads=1)
    seen, fate = set(), {}
    held = None
    allev = sorted([(c[0], 0, c) for c in cuts] + [(p[0], 1, p) for p in puts if True], key=lambda e: (e[0], e[1]))
    for f, kind, e in allev:
        if kind == 0:
            _, x, y = e
            first = ground0(x, y) and (x, y) not in seen
            seen.add((x, y))
            d = depth(x, y)
            cls = "recut" if not first else ("foot" if d >= S else "wall" if d > 0 else "rim")
            held = dict(f=f, x=x, y=y, d=d, cls=cls, full=False)
            fate[(f, x, y)] = held
        elif held is not None:
            _, _, px, py, away = e
            lift = min(held["y"] - py, held["d"])
            held["full"] = away and lift >= S and held["cls"] == "foot"
            held = None
    cut_seq = sorted((r["f"], r["cls"]) for r in fate.values())
    decs = w["dec"]
    near_mouth = [(d["f"], abs(d["x"] - mouth) < 3) for d in decs]
    for i, d in enumerate(decs):
        opts = d.get("opts", [])
        carrying = d["ctx"][C["spoil"]] > 0.5
        digs = [o for o in opts if o[0] == DIG]
        walks = [o for o in opts if o[0] == WALK]
        cut_before = {(x, y) for f, x, y in cuts if f < d["f"]}
        foot = lambda o: ground0(o[1], o[2]) and (o[1], o[2]) not in cut_before and depth(o[1], o[2]) >= S
        here = depth(d["x"], d["y"])
        if not carrying and d["verb"] == DIG:
            r = next((v for (f, x, y), v in fate.items() if (x, y) == d["t"] and d["f"] <= f <= d["f"] + 2), None)
            if r is None:
                continue
            s = stats["dig"]
            s["n"] += 1
            agree = r["cls"] == "foot"
            if r["full"]:
                s["kept"] += 1
                s["kept_agree"] += agree
            else:
                s["drop_agree"] += agree
        if not carrying and d["verb"] != DIG and any(foot(o) for o in digs):
            stats["dig"]["untaken"] += 1
        if not carrying and d["verb"] == WALK and len(walks) >= 2 and here > 0 and not any(foot(o) for o in digs):
            dz = [depth(o[1], o[2]) for o in walks]
            if max(dz) > here:
                s = stats["down"]
                s["n"] += 1
                agree = depth(*d["t"]) == max(dz)
                nxt = next(((f, c) for f, c in cut_seq if f > d["f"]), None)
                good = nxt is not None and nxt[1] == "foot" and nxt[0] - d["f"] <= T_DOWN
                if good:
                    s["kept"] += 1
                    s["kept_agree"] += agree
                else:
                    s["drop_agree"] += agree
        if not carrying and d["verb"] == WALK and len(walks) >= 2 and here <= 0 and abs(d["x"] - mouth) >= 3:
            s = stats["home"]
            s["n"] += 1
            dist = [abs(o[1] - mouth) for o in walks]
            agree = abs(d["t"][0] - mouth) == min(dist)
            good = any(d["f"] < f <= d["f"] + T_HOME and nm for f, nm in near_mouth[i + 1:i + 60])
            if good:
                s["kept"] += 1
                s["kept_agree"] += agree
            else:
                s["drop_agree"] += agree

print(f"T_DOWN {T_DOWN} frames, T_HOME {T_HOME} frames")
for k, s in stats.items():
    kept, drop = s["kept"], s["n"] - s["kept"]
    ka = s["kept_agree"] / kept if kept else float("nan")
    da = s["drop_agree"] / drop if drop else float("nan")
    base = (s["kept_agree"] + s["drop_agree"]) / s["n"] if s["n"] else float("nan")
    print(f"{k:5s} decisions {s['n']:6d}  kept by outcome {kept:6d} ({kept / max(s['n'], 1):.2f})  "
          f"teacher agrees: kept {ka:.3f}  dropped {da:.3f}  all {base:.3f}"
          + (f"   | teacher's 'dig here' the brain never took: {s['untaken']}" if k == "dig" else ""))
