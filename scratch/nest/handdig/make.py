# A hand-written dig head (8 units over 45 option features + 12 dig features), written into every std_pool founder.
# Dig features: 0 out, 1 down, 2 tdepth, 3 light@t, 4 dark@t, 5 open nbrs/8, 6 door dist/16, 7 ant depth/8,
# 8 ant light, 9 open share 7x7 round the ant, 10 level cut, 11 bottom sense.
# usage: make.py POOLDIR OUTDIR [key=val ...]   keys: lvl dn out flr far stop thr c door
# POOLDIR holds the std_pool founders (*.raw.txt, layout 5: the dig head is g[2797:3279], the lay gate rides last).
# The arm the nest lane ran (H3, 2026-10-29): thr=0.2 out=3 door=8.
import sys, os, glob, numpy as np
pool, out = sys.argv[1], sys.argv[2]; P = dict(lvl=6, dn=3, out=1, flr=2, far=4, stop=12, thr=0.45, c=0.0, door=0)
for a in sys.argv[3:]: k, v = a.split('='); P[k] = float(v)
H, NF, ND = 8, 45, 12; N = NF + ND
W = np.zeros((H, N)); b = np.zeros(H); a = np.zeros(H); e = np.zeros(H)
f = lambda k: NF + k
W[0, f(10)] = 3; b[0] = -2; a[0] = P['lvl']                        # level cut: along the floor of the room
W[1, f(1)] = 3; a[1] = P['dn']                                      # down beats up (no ceiling raising)
W[2, f(0)] = 2; a[2] = P['out']                                     # away from the door: passages push outward
W[3, f(11)] = 4; a[3] = P['flr']                                    # at the floor, not the passage's upper row
W[4, f(1)] = 2; W[4, f(6)] = 2; b[4] = -4; a[4] = P['far']          # far from the door, turn down: a new level
W[5, f(9)] = 8; b[5] = -8 * P['thr']; e[5] = -P['stop']             # when: in a wide room, dig less; tight, dig more
W[6, f(6)] = -4; b[6] = 2; e[6] = -P['door']                       # when: near the door (within ~8 cols), dig less
v = np.concatenate([W.ravel(), b, a, [P['c']], e, [0.0]])
os.makedirs(out, exist_ok=True)
for p in sorted(glob.glob(f'{pool}/*.raw.txt')):
    L = open(p).read().split('\n'); n = int(L[0].split()[1]); g = np.zeros(n)
    for l in L[1:]:
        if l.strip(): i, x = l.split(); g[int(i)] = float(x)
    g[2797:2797 + len(v)] = v
    open(f'{out}/{os.path.basename(p)}', 'w').write(f'genome {n} layout=5\n' + ''.join(f'{i} {x:.8g}\n' for i, x in enumerate(g) if x != 0))
ss = lambda x: x / (1 + abs(x))
for name, d in (('level', dict(l=1, dn=0)), ('down', dict(l=0, dn=1)), ('up', dict(l=0, dn=-1))):
    r = P['c'] + P['lvl'] * ss(3 * d['l'] - 2) + P['dn'] * ss(3 * d['dn'])
    print(f'{name}: rank {r:.2f} mult {(ss(r) + 1) / 2:.2f}')
for o in (0.2, 0.3, 0.45, 0.6, 0.8): print(f'open share {o}: when {-P["stop"] * ss(8 * o - 8 * P["thr"]):.1f}')
