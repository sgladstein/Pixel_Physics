"""twinenv-1028: box or door rule? Pairs each colony-seed against twinself's rows (same build, same founders):
herb_first_colony with PP_MIND_DOOR_OPEN=drop vs without (box held, rule changed); lab_herbs without the lab env vs
with it (box held, rule changed); and lab_herbs vs herb_first_colony with the rule held. usage: python3 twinenv_an.py"""
import csv, statistics as st
from math import comb
T = {(r['colony'], r['box'], int(r['seed'])): r for r in csv.DictReader(open('../twinself/twinself.csv'))}
E = {(r['colony'], r['arm'], int(r['seed'])): r for r in csv.DictReader(open('twinenv.csv'))}
cs = [f'c{i}' for i in range(1, 33)]
arms = {'hfc plain': lambda c, s: T[(c, 'herb_first_colony', s)], 'hfc drop': lambda c, s: E[(c, 'hfcdrop', s)],
        'herbs plain': lambda c, s: E[(c, 'herbsplain', s)], 'herbs drop+bedw': lambda c, s: T[(c, 'lab_herbs', s)]}
def sign(a, b):
    up = sum(x > y for x, y in zip(a, b)); dn = sum(x < y for x, y in zip(a, b)); n = up + dn; k = min(up, dn)
    return f'{up}/{dn} p={1.0 if n == 0 else min(1.0, 2 * sum(comb(n, i) for i in range(k + 1)) / 2 ** n):.2g}'
V = {}
for a, f in arms.items():
    V[a] = [float(f(c, s)['lived']) for c in cs for s in (1, 2, 3)]
    dead = sum(int(f(c, s)['alive']) == 0 for c in cs for s in (1, 2, 3))
    ctl = {k: st.mean(float(f(k, s)['lived']) for s in (1, 2, 3)) for k in ('old', 'std')}
    print(f'{a:16s} dead {dead:2d}/96  median lived {st.median(V[a]):6.1f}  hand-built {ctl["old"]:6.1f}  std {ctl["std"]:6.1f}')
print('drop vs plain, herb_first_colony:', sign(V['hfc drop'], V['hfc plain']))
print('drop+bedw vs plain, lab_herbs:   ', sign(V['herbs drop+bedw'], V['herbs plain']))
print('lab_herbs vs hfc, both plain:    ', sign(V['herbs plain'], V['hfc plain']))
print('lab_herbs vs hfc, both drop:     ', sign(V['herbs drop+bedw'], V['hfc drop']))
