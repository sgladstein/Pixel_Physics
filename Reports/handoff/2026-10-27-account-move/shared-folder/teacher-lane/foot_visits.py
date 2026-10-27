"""SCRATCH (teacher lane): what happens on a visit to the foot. A visit is a run of consecutive empty-jawed decisions
with a foot cell in reach (teacher label M), per ant per world. Outcome: dug during the visit, or left without digging.
usage: python3 -I foot_visits.py COPIER_DIR LOG [LOG...]"""
import sys
sys.path.insert(0, sys.argv[1])
import copier
import numpy as np
DIG = 3
for path in sys.argv[2:]:
    dec, worlds = copier.parse_log(path)
    runs, cur = [], {}
    for r in dec:
        key = (r["tag"], r["id"])
        m = r["dig"] == "M" and not r["carrying"]
        picked = r["opts"][r["pick"]][0]
        if m:
            v = cur.setdefault(key, {"n": 0, "dug": False, "verbs": []})
            v["n"] += 1
            v["verbs"].append(picked)
            if picked == DIG:
                v["dug"] = True
                runs.append(cur.pop(key))
        elif key in cur:
            runs.append(cur.pop(key))
    runs += cur.values()
    nw = len(worlds)
    dug = [v for v in runs if v["dug"]]
    left = [v for v in runs if not v["dug"]]
    print(f"== {path}: {len(runs)/nw:.1f} foot visits a world; dug on {len(dug)/len(runs):.2f}; left without digging {len(left)/nw:.1f} a world")
    print(f"   decisions before the dig (median / mean) {np.median([v['n'] for v in dug]):.0f} / {np.mean([v['n'] for v in dug]):.1f};"
          f"   length of a visit that left (median / mean) {np.median([v['n'] for v in left]):.0f} / {np.mean([v['n'] for v in left]):.1f}")
    vc = np.bincount([x for v in runs for x in v["verbs"]], minlength=9)
    print("   verbs at the foot: " + " ".join(f"{copier.__dict__.get('VN', ['walk','turn','rest','dig','pickup','drop','eat','feed','attack'])[i]} {vc[i]/vc.sum():.2f}" for i in range(9) if vc[i]))
