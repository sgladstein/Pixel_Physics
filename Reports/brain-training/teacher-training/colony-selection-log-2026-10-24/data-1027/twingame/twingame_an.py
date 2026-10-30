"""twingame-1028: seven non-exam game boxes at 600k, against twinself-1028 exam rows (same engine). Measure as twinself (p < 0.05/7).
Deciding measure (named before launch, colony-selection log 2026-10-28 19:20Z): a box steers if (a) its seed-to-seed
Spearman of lived over the 32 t_fama carries (mean over the 3 seed pairs) >= 0.3 and (b) its 3-seed lived sum ranks
the exam boxes' 9-run lived sum (herb_first_colony, played_bed, crowded_bench x seeds 1-3) at Spearman >= 0.3 with
permutation p < 0.01 (0.05 / 5 boxes). usage: python3 twinself_an.py (from this folder)"""
import csv, random
from collections import defaultdict
def rank(v):
    o = sorted(range(len(v)), key=lambda i: v[i]); r = [0] * len(v); i = 0
    while i < len(o):
        j = i
        while j + 1 < len(o) and v[o[j + 1]] == v[o[i]]: j += 1
        for k in range(i, j + 1): r[o[k]] = (i + j) / 2
        i = j + 1
    return r
def sp(a, b):
    ra, rb = rank(a), rank(b); n = len(a); ma = sum(ra) / n; mb = sum(rb) / n
    num = sum((x - ma) * (y - mb) for x, y in zip(ra, rb))
    den = (sum((x - ma) ** 2 for x in ra) * sum((y - mb) ** 2 for y in rb)) ** .5
    return num / den if den else 0.0
def perm(a, b, n=20000):
    random.seed(1); r = sp(a, b); return sum(sp(a, random.sample(b, len(b))) >= r for _ in range(n)) / n
R = list(csv.DictReader(open('twingame.csv'))) + [r for r in csv.DictReader(open('../twinself/twinself.csv')) if r['side'] == 'X']
V = {(r['colony'], r['box'], int(r['seed'])): float(r['lived']) for r in R}
A = {(r['colony'], r['box'], int(r['seed'])): int(r['alive']) for r in R}
cs = [f'c{i}' for i in range(1, 33)]
twins = ['played_bed_scrambler', 'played_bed_understory', 'played_bed_windfall_reach', 'steady_income', 'the_bank', 'far_larder', 'boom_bust']
exam = ['herb_first_colony', 'played_bed', 'crowded_bench']
def tot(c, bxs): return sum(V[(c, b, s)] for b in bxs for s in (1, 2, 3))
X = [tot(c, exam) for c in cs]
print(f'{"box":18s} self  vs-exam  perm-p  dead/96  median lived   old    std')
for b in twins + exam:
    self_ = sum(sp([V[(c, b, s)] for c in cs], [V[(c, b, t)] for c in cs]) for s, t in ((1, 2), (1, 3), (2, 3))) / 3
    T = [tot(c, [b]) for c in cs]
    others = [x for x in exam if x != b] if b in exam else exam
    Xb = [tot(c, others) for c in cs]
    rho = sp(T, Xb); p = perm(T, Xb)
    dead = sum(A[(c, b, s)] == 0 for c in cs for s in (1, 2, 3))
    lv = sorted(V[(c, b, s)] for c in cs for s in (1, 2, 3))
    ok = 'STEERS' if self_ >= 0.3 and rho >= 0.3 and p < 0.05 / 7 else ''
    print(f'{b:18s} {self_:+.2f}  {rho:+.2f}   {p:.4f}  {dead:3d}     {lv[len(lv)//2]:7.1f}   {tot("old",[b])/3:6.1f} {tot("std",[b])/3:6.1f} {ok}')
print('exam boxes: vs-exam is against the other two exam boxes')
allT = [tot(c, twins) for c in cs]
print(f'all five twins summed vs exam: {sp(allT, X):+.2f} p {perm(allT, X):.4f}')
# after the named measure: the two played_bed variants that passed the exam half, summed (6 runs per colony)
pair = ['played_bed_scrambler', 'played_bed_understory']
P2 = [tot(c, pair) for c in cs]
print(f'scrambler + understory summed vs exam: {sp(P2, X):+.2f} p {perm(P2, X):.4f}')
for k in (1, 2):
    P1 = [sum(V[(c, b, s)] for b in pair for s in range(1, k + 1)) for c in cs]
    print(f'  seeds 1..{k} only: {sp(P1, X):+.2f}')
# kinship control: the variants are played_bed with one thing added, so rank them against the other two exam boxes only
X2 = [tot(c, ['herb_first_colony', 'crowded_bench']) for c in cs]
print(f'scrambler + understory vs herb_first_colony + crowded_bench (played_bed left out): {sp(P2, X2):+.2f} p {perm(P2, X2):.4f}')
PB = [tot(c, ['played_bed']) for c in cs]
print(f'  played_bed itself vs the same two: {sp(PB, X2):+.2f}')
