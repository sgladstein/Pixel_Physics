"""SCRATCH (teacher lane): where the brain's Digs go. Per label (M = a foot cell in reach, N = none): how many Dig
picks a world, and on M, whether the picked target was in the teacher's foot set, and the target's row relative
to the head.  usage: python3 -I dig_targets.py LOG [LOG...]"""
import sys, collections
for path in sys.argv[1:]:
    c = collections.Counter(); rel = collections.Counter(); worlds = set()
    for l in open(path):
        p = l.rstrip("\n").split(",")
        if p[1] != "Q" or p[6] == "1":
            continue
        worlds.add(p[0])
        lab, digset = p[10].split("|")[:2]
        opts = p[13].split(";") if p[13] else []
        pick = int(p[7]); q = opts[pick].split(":", 4)
        if q[0] != "3" or lab not in "MN":
            continue
        ds = set() if digset == "-" else {int(i) for i in digset.split(";")}
        foot_opts = ds  # the arg-max set; any foot cell would be wider, so count the target's row too
        c[lab] += 1
        if lab == "M":
            c["M_in_set"] += pick in ds
            rel[int(q[2]) - int(p[5])] += 1
    n = len(worlds)
    print(f"== {path}: Dig picks a world: foot in reach {c['M']/n:.1f} (in the teacher's set {c['M_in_set']/max(c['M'],1):.2f}), walls only {c['N']/n:.1f}")
    print("   on foot-in-reach digs, target row minus head row: " + " ".join(f"{k:+d}:{v/max(c['M'],1):.2f}" for k, v in sorted(rel.items())))
