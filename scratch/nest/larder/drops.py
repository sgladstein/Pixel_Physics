"""SCRATCH (chambers lane, larder seed): where crop food was put down, from smoke.sh's OUT.drops.gz (M rows whose pick is
Drop with food in the crop and no pellet; run with PP_FOODBESIDE=1 so the picked option's slot 18 is the share of its
8 neighbours holding loose food). Prints, at home and away: put-downs, the share beside food, and the neighbour-count
histogram; and at home how deep (rows under the datum, from the pick's f::DEPTH) the put-downs were.
usage: drops.py OUT.drops.gz [...]"""
import sys, gzip, collections
for fn in sys.argv[1:]:
    H = {0: collections.Counter(), 1: collections.Counter()}; deep = collections.Counter()
    for line in gzip.open(fn, 'rt'):
        p = line.rstrip().split(',')
        ctx = p[10].split(); feats = p[14].split()
        home = int(float(ctx[3])); k = round(float(feats[18]) * 8)
        H[home][k] += 1
        if home:
            d = float(feats[26]); rows = 16 * d / (1 - abs(d)) if abs(d) < 1 else 99
            deep['above 0' if rows < 0 else '0-10' if rows < 10 else '10-20' if rows < 20 else '20+'] += 1
    for home, c in ((1, H[1]), (0, H[0])):
        n = sum(c.values())
        if not n: continue
        beside = n - c[0]
        print(f"{fn.split('/')[-1]:22s} {'home' if home else 'away'}: put-downs {n:6d} beside food {beside:6d} ({100 * beside / n:4.1f}%) "
              f"| by food neighbours {dict(sorted(c.items()))}")
    print(f"{'':22s} home put-downs by depth: {dict(deep)}")
