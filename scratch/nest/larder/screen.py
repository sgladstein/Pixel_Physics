"""SCRATCH (chambers lane, larder seed): the screen's table -- per world, round 3 (or ROUND), each arm's deciding
measures (larder.py), the paired sign against BASE, and whether each round's top-30 founders still carry the seed.
usage: screen.py BASEDIR BASEARM ARM [ARM ...]      (reads BASEDIR/ARM_wW/rR; WORLDS, ROUND env)"""
import sys, os, glob, csv, math
sys.path.insert(0, 'scratch/nest/larder')
from larder import measure
base, b0, arms = sys.argv[1], sys.argv[2], sys.argv[3:]
W = [int(w) for w in os.environ.get('WORLDS', '68 73 61 64').split()]
R = int(os.environ.get('ROUND', 3))
KEYS = ['passage', 'strewn', 'inpass', 'crumbs', 'open', 'alive', 'under', 'starved', 'space']
BETTER = dict(passage=1, strewn=-1, inpass=-1, crumbs=-1, open=1, alive=1, under=-1, starved=-1, space=1)
SEED = {691: 32, 1141: -32, 1678: 10}


def get(a, w):
    d = f'{base}/{a}_w{w}/r{R}'
    if not os.path.exists(f'{d}/nest.csv'):
        return None
    return measure(d)


def sign_p(k, n):  # two-sided sign test
    if n == 0: return 1.0
    t = sum(math.comb(n, i) for i in range(0, min(k, n - k) + 1)) / 2 ** n
    return min(1.0, 2 * t)


M = {(a, w): get(a, w) for a in [b0] + arms for w in W}
for a in [b0] + arms:
    print(f'== {a} round {R}')
    for w in W:
        m = M[(a, w)]
        if m is None: print(f'  w{w}: extinct / missing'); continue
        ps = 'n/a' if m['passage'] is None else f"{100 * m['passage']:.0f}%"
        print(f"  w{w}: passage {ps:>4s} strewn {m['strewn']:4d} inpass {m['inpass']:4d} crumbs {m['crumbs']:4d} pile {m['pile']:4d} "
              f"open {m['open']:4d} alive {m['alive']:3d} under {m['under']:3d} starved {m['starved']:4d} space {m['space']:4d}")
for a in arms:
    print(f'== {a} vs {b0}, better per world (sign test, two-sided)')
    for k in KEYS:
        up = n = 0
        for w in W:
            x, y = M[(a, w)], M[(b0, w)]
            if k == 'alive':  # a line extinct before round R counts as 0 alive
                x, y = x or dict(alive=0), y or dict(alive=0)
            if x is None or y is None: continue
            if k == 'passage' and (x[k] is None or y[k] is None): continue
            if x[k] == y[k]: continue
            n += 1; up += (x[k] - y[k]) * BETTER[k] > 0
        print(f'  {k:8s} better {up}/{n}  p {sign_p(up, n):.3f}')
    for r in range(4):
        tot = keep = 0
        for w in W:
            f = f'{base}/{a}_w{w}/r{r}/final/index.csv'
            if not os.path.exists(f): continue
            rows = sorted(csv.DictReader(open(f)), key=lambda x: -float(x['score']))[:30]
            for x in rows:
                g = {}
                for l in open(f"{base}/{a}_w{w}/r{r}/final/{x['id']}.raw.txt").read().split('\n')[1:]:
                    if l.strip(): i, v = l.split(); g[int(i)] = float(v)
                tot += 1
                keep += all(abs(g.get(i, 0) - v) / abs(v) < 0.25 for i, v in SEED.items() if (a != 'lt' or i != 1678))
        print(f'  round {r}: top-30 founders carrying the seed (each link within 25%): {keep}/{tot}')
