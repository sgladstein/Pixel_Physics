"""SCRATCH (chambersense lane, 2026-10-30): hand-set weights on the room sense (`PP_DIGROOM=1`, dig slot 8 = room
distance / 8: 0 inside a space a 5x5 block fits, 3 at 24+ cells of walk) written on top of every H3+eat founder.

    python3 scratch/nest/chambersense/make.py POOL.txt OUTDIR [key=val ...]
    python3 scratch/nest/chambersense/make.py POOL.txt OUTDIR flat=0 g=0              # H3+eat itself, expanded
POOL.txt: scratch/inrun/genomes/h3eat/h3eat_pool.txt. OUTDIR gets p00..p29.raw.txt. Keys (0 turns a piece off):
  stop  the wide-room stop (H3 unit 5) lets the space get wider the farther the ant is from a room:
        z5 = 8*(open - 0.2) - stop*R  -> it stops digging at open share 0.2 + stop*R/8 (R = room distance / 8)
  flat  far from a room, the level-cut and push-out preferences (H3 units 0, 2) fade: bias -flat*R on each
  unit 7 (spare in H3), the room unit: z7 = g*(R - rt) - dn*down + nb*(nbrs - 0.375), saturated off near open space
        and turning on past rt*8 cells of walk from it.
        e7 = K, with c2 = K*cz (cz = -squash(z7) at R = 0, so near open space the "when" sum is H3's): far from any
        room or the open air, take the dig job more -- so a long passage's end keeps being cut after it widens.
        a7 = a: in the same band, prefer cuts upward (dn) and into open-sided cells (nb): widen, not advance.
Layout 5: head g[2797:3279] = W (8 x 57) b (8) a (8) c e (8) c2; dig feature k is column 45 + k.
"""
import sys, os

pool, out = sys.argv[1], sys.argv[2]
P = dict(stop=0.0, flat=2.0, g=4.0, rt=1.5, dn=2.0, nb=0.0, a=3.0, K=6.0)
for a in sys.argv[3:]:
    k, v = a.split('=')
    P[k] = float(v)
# expand the pool (colsel.expand_pool's reading)
lines = [l for l in open(pool).read().split('\n') if l.strip() and not l.startswith('#')]
head = lines[0].split()
assert head[0] == 'pool' and head[2] == 'genome', head
members = []
for l in lines[1:]:
    if l.startswith('member '):
        members.append(dict(members[0]) if members else {})
        continue
    i, w = l.split()
    members[-1][int(i)] = float(w)
H0, N, NF = 2797, 57, 45
W = lambda u, k: H0 + u * N + NF + k
B = lambda u: H0 + 456 + u
A = lambda u: H0 + 464 + u
E = lambda u: H0 + 473 + u
R = 8  # the room sense's slot
os.makedirs(out, exist_ok=True)
for k, m in enumerate(members):
    g = dict(m)
    # H3 sanity: slot 8 unread, unit 7 empty, unit 5 the wide-room stop
    assert all(g.get(W(u, R), 0) == 0 for u in range(8)), 'slot 8 already read'
    assert all(g.get(H0 + 7 * N + j, 0) == 0 for j in range(N)) and g.get(A(7), 0) == 0, 'unit 7 in use'
    assert g.get(W(5, 9)) == 8 and g.get(E(5)) == -12, (g.get(W(5, 9)), g.get(E(5)))
    if P['stop']:
        g[W(5, R)] = -P['stop']
    if P['flat']:
        g[W(0, R)] = -P['flat']
        g[W(2, R)] = -P['flat']
    if P['g'] and (P['a'] or P['K']):
        g[W(7, R)] = P['g']
        g[W(7, 1)] = -P['dn']
        g[W(7, 5)] = P['nb']
        g[B(7)] = -P['g'] * P['rt'] - 0.375 * P['nb']
        g[A(7)] = P['a']
        g[E(7)] = P['K']
        z0 = -P['g'] * P['rt']
        g[H0 + 481] = P['K'] * (-z0 / (1 + abs(z0)))  # c2
    with open(f'{out}/p{k:02}.raw.txt', 'w') as f:
        f.write(f'genome {head[3]} {head[4]}\n')
        f.writelines(f'{i} {w:.8g}\n' for i, w in sorted(g.items()) if w != 0.0)
print(f'{out}: {len(members)} founders, {P}')
