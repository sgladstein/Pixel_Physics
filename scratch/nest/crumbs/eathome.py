"""SCRATCH (chambers crumbs lane, 2026-10-30): arm E1, "eat what you hold at home, fed or not" -- a variant of the eat
seed (scratch/nest/eatseed/make.py, arm B nest=60) on a pool that already carries it. A seed, not a rule.

Why: the crumb trace (crumbsrc.py) put 77-89% of the crumbs standing in the nest at the end of a round on loads a FED
ant (over half a tank; the colony's mean is ~10 tanks) picked up AT HOME and put down again, 58% of those put-downs
within 5 cells of the pick-up -- the store is shuffled by fed ants, and the shuffle spreads it into the passages. The
eat seed only eats below half a tank. Here Eat's job at home is high whatever the energy: its constant -38.4
(W*thr - nest, thr 0.36) becomes W*thr2 - nest, thr2 = 1.0 by default, so the job is 60*(1 - energy) at home (energy
sense < 1 always: 2.4 at a 10-tank ant, x sharpness 4 -> weight 0.95) and 60*(1 - energy) - 60 away (at its floor).
Expected cost: a forager arriving home eats some of what it carries (Eat and Drop both near 1 at home).

usage: eathome.py POOLDIR OUTDIR [thr=1.0]
"""
import glob, os, sys
src, out = sys.argv[1], sys.argv[2]
thr = float(sys.argv[3].split('=')[1]) if len(sys.argv) > 3 else 1.0
W, NEST, SLOT = 60.0, 60.0, 1715
os.makedirs(out, exist_ok=True)
for p in sorted(glob.glob(f'{src}/*.raw.txt')):
    L = open(p).read().split('\n')
    hit = 0
    for i, l in enumerate(L):
        if l.startswith(f'{SLOT} '):
            assert abs(float(l.split()[1]) + 38.4) < 1e-4, (p, l)
            v = W * thr - NEST
            L[i] = f'{SLOT} {v:.8g}' if v != 0 else ''
            hit += 1
    assert hit == 1, p
    open(f'{out}/{os.path.basename(p)}', 'w').write('\n'.join(l for l in L if l) + '\n')
print(f'{len(glob.glob(out + "/*.raw.txt"))} founders -> {out}, Eat constant {W * thr - NEST:g}')
