# SCRATCH (chambers lane, 2026-10-29): the eat seed -- a hand-set Eat job written into every founder of a pool, so a
# hungry ant holding food eats it instead of putting it back down. A seed, not a rule: selection may keep or drop it.
#
# Why Eat and not Drop: H3's hungry ants put 95% of the food they pick up at home straight back (Drop job ~0.96 at the
# nest, driven by at_nest +13.4), so they starve 2-3 cells from the store. Holding Drop back for the hungry needs
# "hungry AND holding food AND not a pellet", and the pool has no free hidden unit to carry an AND (all 8 are read by
# some job; H0-H2 are the pellet-vs-food units). A linear energy term on Drop also moves pellet drops and, for rich
# ants, food drops in the field. Eat needs no AND: it is offered only while the crop holds food, so one monotone link
# from the energy sense does it. Eat finishes the cell in the crop at once (`eat_lunch_now`); passive digestion keeps
# running either way.
#
# What it writes (layout-5 slots, the same in today's layout; VERBS order walk turn rest dig pickup drop eat feed attack):
#   g[6]                 Eat's switch, -0.95 -> +1 (offered at all)
#   g[1680..1716]        Eat's job row (22 context, 12 memory, best, constant): zeroed (never expressed, drifted), then
#   g[1680]              energy -> -W
#   g[1715]              constant -> +W*thr      so the job is W*(thr - energy), times the genome's sharpness (4)
#   g[2773..2781]        the hidden units' weights into Eat's job: zeroed (drifted)
#   g[1788+46m+30]       each memory unit's "Eat was just chosen" input: zeroed (drifted), so an eat reads as no verb
# energy is e/(1+e) of the start grant: 0.5 a full tank, 1/3 half a tank (the trace's "hungry"). At W=60 thr=0.36 the
# Eat weight (j+1)/2 is 0.97 at energy 0.30, 0.93 at 1/3, 0.5 at 0.36, 0.05 at 0.40 and 0.015 at a full tank -- below
# the old QUIET -10 constant's 0.012 by 0.42, so a fed carrier eats its load no more often than a pre-split champion.
#   nest=K (the arm screened as B): g[1683] at_nest -> +K and the constant -K, so away from home the job is K lower
#   and Eat sits at its floor there. Without it (arm A) a hungry carrier on a long trip eats its load where it stands,
#   loses the load that draws it home, and in 3 of 8 lines the colony drifted to the end walls and died (screen below).
# usage: make.py POOLDIR OUTDIR [w=60] [thr=0.36] [zero=1] [nest=0]
import sys, os, glob, numpy as np
pool, out = sys.argv[1], sys.argv[2]
P = dict(w=60.0, thr=0.36, zero=1.0, nest=0.0)
for a in sys.argv[3:]:
    k, v = a.split('='); P[k] = float(v)
VERBS, CONTEXT, MEMORY, HIDDEN = 9, 22, 12, 8
EAT = 6
JOB, J_IN, MEM, M_IN, HID_OUT = 1464, CONTEXT + MEMORY + 2, 1788, CONTEXT + 2 + VERBS + MEMORY + 1, 2725
row = range(JOB + EAT * J_IN, JOB + (EAT + 1) * J_IN)
hid = range(HID_OUT + EAT * HIDDEN, HID_OUT + (EAT + 1) * HIDDEN)
mem = [MEM + m * M_IN + CONTEXT + 2 + EAT for m in range(MEMORY)]
os.makedirs(out, exist_ok=True)
for p in sorted(glob.glob(f'{pool}/*.raw.txt')):
    L = open(p).read().split('\n'); head = L[0]; n = int(head.split()[1]); g = np.zeros(n)
    for l in L[1:]:
        if l.strip(): i, x = l.split(); g[int(i)] = float(x)
    before = int(np.count_nonzero(g))
    if P['zero']:
        for k in list(row) + list(hid) + mem: g[k] = 0.0
    g[EAT] = 1.0
    g[JOB + EAT * J_IN + 0] = -P['w']
    g[JOB + EAT * J_IN + CONTEXT + MEMORY + 1] = P['w'] * P['thr'] - P['nest']
    if P['nest']:
        g[JOB + EAT * J_IN + 3] = P['nest']   # c::AT_NEST (0/1): eat only at home, `nest` below it away
    open(f'{out}/{os.path.basename(p)}', 'w').write(head + '\n' + ''.join(f'{i} {x:.8g}\n' for i, x in enumerate(g) if x != 0))
    print(os.path.basename(p), 'nonzero', before, '->', int(np.count_nonzero(g)))
sq = lambda x: x / (1 + abs(x))
for e in (0.2, 0.3, 1 / 3, 0.36, 0.4, 0.45, 0.5, 0.6, 0.75):
    j = sq(4 * P['w'] * (P['thr'] - e)); w = (j + 1) / 2
    print(f'energy {e:.3f} (tank {e / (1 - e):.2f}): eat weight {w:.3f}, choose_weighted mass {(0.05 + w) ** 2:.4f}')
