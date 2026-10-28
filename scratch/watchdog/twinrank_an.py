"""SCRATCH (score-gaming watchdog, 2026-10-28): twinrank's verdict -- do the training lab boxes rank carries the way the game's
boxes do? Reads every <ROOT>/**/twr-<carry>-<T|X>-<box>-s<seed>/lab.csv (exam.py's lab rows; the Teacher lane's VM batch
twinrank-1028) and prints, over the 32 colony carries (old/std are the positive control, kept out of the ranks):
  DECIDING MEASURE (named before the run, PLAN.txt): Spearman(twin lived summed over lab_herbs+lab_far_mix and seeds,
  exam lived summed over herb_first_colony+played_bed and seeds). Twins rank like the game iff rho >= 0.3 and one-sided p < .05.
  per pair (lab_herbs~herb_first_colony, lab_far_mix~played_bed), lived and alive; donorlab's own order (alive, lived);
  noise floor: seed-to-seed Spearman inside each box (how well one seed of a box predicts another seed of the same box).
usage: python3 twinrank_an.py ROOT"""
import csv, glob, math, os, re, sys
from collections import defaultdict

ROOT = sys.argv[1]
TW, EX = ['lab_herbs', 'lab_far_mix'], ['herb_first_colony', 'played_bed']
d = defaultdict(dict)  # (carry) -> {(box, seed): (alive, lived)}
for p in glob.glob(f'{ROOT}/**/lab.csv', recursive=True):
    m = re.search(r'twr-(\w+?)-([TX])-(\w+)-s(\d+)$', os.path.basename(os.path.dirname(p)))
    if not m:
        continue
    for r in csv.DictReader(open(p)):
        if r.get('alive', '') == '':
            continue
        d[m.group(1)][(r['box'], int(r['seed']))] = (int(r['alive']), float(r['lived']))


def rank(x):
    o = sorted(range(len(x)), key=lambda i: x[i])
    rk = [0.0] * len(x)
    i = 0
    while i < len(o):
        j = i
        while j + 1 < len(o) and x[o[j + 1]] == x[o[i]]:
            j += 1
        for k in range(i, j + 1):
            rk[o[k]] = (i + j) / 2
        i = j + 1
    return rk


def spear(a, b):
    ra, rb = rank(a), rank(b)
    n = len(a)
    ma, mb = sum(ra) / n, sum(rb) / n
    c = sum((x - ma) * (y - mb) for x, y in zip(ra, rb))
    v = math.sqrt(sum((x - ma) ** 2 for x in ra) * sum((y - mb) ** 2 for y in rb))
    rho = c / v if v else 0.0
    t = rho * math.sqrt((n - 2) / max(1e-12, 1 - rho * rho))
    p1 = 0.5 * math.erfc(t / math.sqrt(2))  # one-sided, normal approx of the t (n ~ 32)
    return rho, p1


cols = sorted((c for c in d if c.startswith('c')), key=lambda c: int(c[1:]))
seeds = sorted({s for c in d for (_, s) in d[c]})
full = [c for c in cols if all((b, s) in d[c] for b in TW + EX for s in seeds)]
print(f'carries {len(cols)} colonies (+{[c for c in d if not c.startswith("c")]}), seeds {seeds}, complete {len(full)}')


def tot(c, boxes, i, ss=None):
    return sum(d[c][(b, s)][i] for b in boxes for s in (ss or seeds))


tw = [tot(c, TW, 1) for c in full]
ex = [tot(c, EX, 1) for c in full]
rho, p = spear(tw, ex)
print(f'\nDECIDING: Spearman(twin lived, exam lived) n={len(full)}: rho {rho:+.2f}, one-sided p {p:.3f} -> '
      + ('twins rank like the game' if rho >= 0.3 and p < 0.05 else 'twins DO NOT rank like the game'))
dl = [tot(c, TW, 0) * 1e6 + tot(c, TW, 1) for c in full]
print(f'donorlab order (alive, then lived) vs exam lived: rho {spear(dl, ex)[0]:+.2f} p {spear(dl, ex)[1]:.3f}')
print(f'twin alive vs exam alive: {spear([tot(c, TW, 0) for c in full], [tot(c, EX, 0) for c in full])[0]:+.2f}')
for t, x in zip(TW, EX):
    for i, nm in ((1, 'lived'), (0, 'alive')):
        r, pp = spear([tot(c, [t], i) for c in full], [tot(c, [x], i) for c in full])
        print(f'  {t:12} ~ {x:18} {nm}: rho {r:+.2f} p {pp:.3f}')
print('\nnoise floor, seed-to-seed inside one box (lived; mean over seed pairs):')
for b in TW + EX:
    rs = [spear([d[c][(b, s)][1] for c in full], [d[c][(b, t)][1] for c in full])[0] for s in seeds for t in seeds if s < t]
    print(f'  {b:18} {sum(rs) / len(rs):+.2f}  ({" ".join(f"{r:+.2f}" for r in rs)})' if rs else f'  {b} one seed')
print(f'box-to-box: lab_herbs~lab_far_mix {spear([tot(c, [TW[0]], 1) for c in full], [tot(c, [TW[1]], 1) for c in full])[0]:+.2f}, '
      f'herb_first_colony~played_bed {spear([tot(c, [EX[0]], 1) for c in full], [tot(c, [EX[1]], 1) for c in full])[0]:+.2f}')
print('\npositive control (lived per box, mean over seeds; colonies: median):')
for b in TW + EX:
    med = sorted(sum(d[c][(b, s)][1] for s in seeds) / len(seeds) for c in full)
    ctl = ' '.join(f'{k} {sum(d[k][(b, s)][1] for s in seeds if (b, s) in d[k]) / max(1, sum((b, s) in d[k] for s in seeds)):.0f}'
                   for k in ('old', 'std') if k in d)
    print(f'  {b:18} {ctl}  colonies median {med[len(med) // 2]:.0f} (range {med[0]:.0f}-{med[-1]:.0f})' if med else b)
print('\nper colony: twin lived, exam lived (sorted by twin)')
for c, a, b in sorted(zip(full, tw, ex), key=lambda z: -z[1]):
    print(f'  {c:4} {a:8.0f} {b:8.0f}')
