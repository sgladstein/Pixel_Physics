# SCRATCH (chambers crumbs lane, 2026-10-30): arm E2, the ROOM seed -- the larder seed's links (copied from
# claude/chambers-larder scratch/nest/larder/make.py, unchanged below) read with PP_ROOMBESIDE=1 instead of
# PP_FOODBESIDE=1: slot 18 on a crop put-down option is then the room above the spot (options.rs `room_beside`: 0 in a
# passage or shaft, ~1 in a roofed room), not food beside it. So an ant putting food down prefers a spot in a room,
# and (hold=1) a food carrier's Drop job is lowered 16 unless such a spot is in reach: it carries on out of a passage.
# Why: the crumb trace (crumbsrc.py) put 77-89% of the crumbs standing in a nest on loads fed ants picked up at home
# and put down again; the passages fill from that shuffle. The larder's "beside food" grew the mass where it already
# lay (the door shaft); "room above" points away from passages and the shaft wherever the store lies.
# usage: roomseed.py POOLDIR OUTDIR [k=32] [hold=0|1] [w=10] [c=16]     -- run with setenv PP_ROOMBESIDE=1
#
# The larder header, for the slot arithmetic:
# SCRATCH (chambers lane, 2026-10-29): the larder seed -- hand-set brain links so an ant putting crop food down
# prefers a spot beside food already lying there, and (arm `hold`) holds its load at home until such a spot is in
# reach, so the store gathers in one place and the passages stay open. A seed, not a rule: selection may keep or drop
# it, and nothing in the body's act changes. Run on a pool that already carries the eat seed (scratch/nest/eatseed).
#
# What it reads: `PP_FOODBESIDE=1` (options.rs), the share of a crop put-down target's 8 neighbours holding loose food,
# on option slot f::BUILDING (18), 0 on every other option. H3 and std_pool carry no weight on slot 18 in any unit, yet
# the switch alone is NOT a no-op: it reproduces PP_LASTFOOD=off byte for byte (w68 round 0 nest.csv), and that differs
# from the default from the first census (alive 63 vs 58 at 25k) -- the last-food memory reaches decisions some other
# way (a child's mutated link on slot 18 is the likely one; no living founder at the end carries one). So the screen's
# baseline is H3+eat rerun WITH the switch, and the arms differ only by the links below.
#
# What it writes (layout-5 slots; VERBS order walk turn rest dig pickup drop eat feed attack):
#   target   S_FEAT + 12*45 + 18 += -K   unit 12 (output -30.8, its sum on home Drop options near 0, median -0.42)
#            S_FEAT +  2*45 + 18 += +K   unit  2 (output +12.6, median -1.05)
#            Drop options score -0.99 at home today (every one at the floor, so the target is a uniform draw); at K=32
#            one food neighbour lifts the median to ~0.9, a ~300x heavier draw (scratchpad lard/design.py).
#   hold     Drop's job row (1644..1680): best += W, constant += W*0.99 - C, c::SPOIL += C
#            so a pellet carrier's Drop job is unchanged (+W*(best+0.99), best ~-0.98), a food carrier's falls by C
#            unless a put-down beside food is in reach (best ~+0.9 -> +W*1.9). At home today the job sum is ~9.75
#            (0.975); C=16 puts it near -6 (weight 0.02), W=10 restores +19.
# usage: make.py POOLDIR OUTDIR [k=32] [hold=0|1] [w=10] [c=16]
import sys, os, glob, numpy as np
pool, out = sys.argv[1], sys.argv[2]
P = dict(k=32.0, hold=0.0, w=10.0, c=16.0)
for a in sys.argv[3:]:
    k, v = a.split('='); P[k] = float(v)
VERBS, CONTEXT, MEMORY, UNITS, FEATURES = 9, 22, 12, 16, 45
S_CTX = 23; S_IN = CONTEXT + MEMORY + 1; S_FEAT = S_CTX + UNITS * S_IN; S_VERB = S_FEAT + UNITS * FEATURES
S_OUT = S_VERB + UNITS * VERBS; JOB = S_OUT + UNITS + 1; J_IN = CONTEXT + MEMORY + 2
DROP, BESIDE, SPOIL = 5, 18, 2
row = JOB + DROP * J_IN
os.makedirs(out, exist_ok=True)
for p in sorted(glob.glob(f'{pool}/*.raw.txt')):
    L = open(p).read().split('\n'); head = L[0]; n = int(head.split()[1]); g = np.zeros(n)
    for l in L[1:]:
        if l.strip(): i, x = l.split(); g[int(i)] = float(x)
    assert g[S_OUT + 12] < -10 and g[S_OUT + 2] > 5, (p, g[S_OUT + 12], g[S_OUT + 2])
    assert all(g[S_FEAT + u * FEATURES + BESIDE] == 0 for u in range(UNITS)), 'slot 18 already weighted'  # noqa
    g[S_FEAT + 12 * FEATURES + BESIDE] = -P['k']
    g[S_FEAT + 2 * FEATURES + BESIDE] = P['k']
    if P['hold']:
        g[row + CONTEXT + MEMORY] += P['w']
        g[row + CONTEXT + MEMORY + 1] += P['w'] * 0.99 - P['c']
        g[row + SPOIL] += P['c']
    open(f'{out}/{os.path.basename(p)}', 'w').write(head + '\n' + ''.join(f'{i} {x:.8g}\n' for i, x in enumerate(g) if x != 0))
print(f'wrote {len(glob.glob(out + "/*.raw.txt"))} founders to {out}: {P}')
