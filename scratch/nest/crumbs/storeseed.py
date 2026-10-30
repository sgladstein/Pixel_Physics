"""SCRATCH (chambers crumbs lane, 2026-10-30, step 2): the STORE seed -- hand-set links so a FED ant at home leaves the
nest store where it lies, while a hungry ant still picks it up (to eat: the eat seed) and a forager in the field is
untouched. Reads PP_STORESENSE=1 (options.rs `store_sense`: on a PickUp option, 0 unless the food lies in the hole,
then its depth below the surface / 6, clamped to 1). A seed, not a rule: selection may keep or drop it. Run on a pool
that already carries the eat seed.

What it writes (layout-5 slots; VERBS walk turn rest dig pickup drop eat feed attack):
  scorer   S_FEAT + 12*45 + 18 += +K, S_FEAT + 2*45 + 18 += -K: units 12 (output -30.8) and 2 (+12.6), the larder
           seed's pair with the sign turned, so a pick-up of store food scores at the floor. Probe (world 61, 40k
           frames, mind log, probe.py): underground at home, best PickUp score median +0.976 -> -0.996; field -0.973 ->
           -0.981. So WHICH food a home pick-up takes moves off the store -- but the job barely reads `best` (home
           PickUp job -0.880 -> -0.866) and fed home pick-ups did not fall (3.2% -> 4.3% of decisions), so:
  job      PickUp's job row (JOB + 4*J_IN): at_nest += -KJ, energy += -W, constant += W, i.e. the job moves by
             W*(1 - energy) - KJ*at_nest
           Away from home it only rises (W*(1 - energy) >= 0; the field job is already +0.94). At home it is
           W*(1 - e) - KJ: KJ=42, W=60 leaves a hungry ant (energy sense 0.3) level and pushes a fed one (0.9) 36 down,
           to the verb draw's floor. `best` is NOT used: H3's field pick-ups score at the floor too (-0.97), so best
           cannot tell field from store, and the probe ruled the larder-style `best` gate out.
usage: storeseed.py POOLDIR OUTDIR [k=32] [w=60] [kj=42] [jobonly=0] [unitsonly=0]
"""
import sys, os, glob, numpy as np
pool, out = sys.argv[1], sys.argv[2]
P = dict(k=32.0, w=60.0, kj=42.0, jobonly=0.0, unitsonly=0.0)
for a in sys.argv[3:]:
    k, v = a.split('='); P[k] = float(v)
VERBS, CONTEXT, MEMORY, UNITS, FEATURES = 9, 22, 12, 16, 45
S_CTX = 23; S_IN = CONTEXT + MEMORY + 1; S_FEAT = S_CTX + UNITS * S_IN; S_VERB = S_FEAT + UNITS * FEATURES
S_OUT = S_VERB + UNITS * VERBS; JOB = S_OUT + UNITS + 1; J_IN = CONTEXT + MEMORY + 2
PICK, STORE, ENERGY, AT_NEST = 4, 18, 0, 3
row = JOB + PICK * J_IN
assert JOB == 1464
os.makedirs(out, exist_ok=True)
for p in sorted(glob.glob(f'{pool}/*.raw.txt')):
    L = open(p).read().split('\n'); head = L[0]; n = int(head.split()[1]); g = np.zeros(n)
    for l in L[1:]:
        if l.strip(): i, x = l.split(); g[int(i)] = float(x)
    assert g[S_OUT + 12] < -10 and g[S_OUT + 2] > 5, (p, g[S_OUT + 12], g[S_OUT + 2])
    assert all(g[S_FEAT + u * FEATURES + STORE] == 0 for u in range(UNITS)), 'slot 18 already weighted'
    if not P['jobonly']:
        g[S_FEAT + 12 * FEATURES + STORE] = P['k']
        g[S_FEAT + 2 * FEATURES + STORE] = -P['k']
    if not P['unitsonly']:
        g[row + AT_NEST] += -P['kj']
        g[row + ENERGY] += -P['w']
        g[row + CONTEXT + MEMORY + 1] += P['w']
    open(f'{out}/{os.path.basename(p)}', 'w').write(head + '\n' + ''.join(f'{i} {x:.8g}\n' for i, x in enumerate(g) if x != 0))
print(f'wrote {len(glob.glob(out + "/*.raw.txt"))} founders to {out}: {P}')
