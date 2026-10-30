"""SCRATCH (chambers crumbs lane, 2026-10-30): the screen table -- per arm, world and round: alive at the end, passages
(selrun/tunnels.py `tunnels`), crumb share (nestshape: crumbs / nest space), and the crumb sources (crumbsrc.py: share of
standing crumbs last put down from a load picked up at home), then paired sign tests of each arm against `base`.
An extinct line (no rN dir, or EXTINCT before it) reads alive 0, tunnels 0, crumb share 1 (judge.py's convention).
usage: table.py RUNS ROUND ARM... (first ARM is the baseline)
"""
import glob, math, os, sys
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'selrun'))
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', 'inrun'))
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import tunnels, nestshape, crumbsrc

runs, rnd, arms = sys.argv[1], int(sys.argv[2]), sys.argv[3:]


def alive(d):
    rows = [l.split(',') for l in open(f'{d}/stock.csv')][1:]
    return int(rows[-1][1]) if rows else 0


def one(arm, w):
    d = f'{runs}/{arm}_w{w}/r{rnd}'
    if not os.path.exists(f'{d}/.done'):
        return None if not os.path.exists(f'{runs}/{arm}_w{w}/EXTINCT') else dict(alive=0, tunnels=0, crumb=1.0, pcr=float('nan'), home=float('nan'), ext=1)
    t = tunnels.measure(d) or {}
    n = nestshape.measure(d) or {}
    c = crumbsrc.classify(d, f'{runs}/{arm}_w{w}/r{rnd}.crumb.log')
    j = len(c['rows'])
    home = sum(1 for r in c['rows'] if r['first_home']) / j if j else float('nan')
    pas = sum(1 for r in c['rows'] if r['_place'] == 'passage') / j if j else float('nan')
    return dict(alive=alive(d), tunnels=t.get('tunnels', 0), crumb=n.get('crumb_share', 0.0), crumbs=c['crumbs'], pcr=c['allplace']['passage'],
                space=c['space'], home=home, pas=pas, ext=0)


def sign(pairs):
    w = sum(1 for a, b in pairs if b > a); l = sum(1 for a, b in pairs if b < a); n = w + l
    p = min(1.0, 2 * sum(math.comb(n, k) for k in range(0, min(w, l) + 1)) / 2 ** n) if n else 1.0
    return w, l, p


worlds = sorted({int(p.rsplit('_w', 1)[1]) for p in glob.glob(f'{runs}/{arms[0]}_w*')})
res = {a: {w: one(a, w) for w in worlds} for a in arms}
print(f'round {rnd}; worlds {worlds}')
print('arm   world  alive tunnels crumb_share crumbs/space  from_home  in_passage(joined)  passage_crumbs')
for a in arms:
    for w in worlds:
        r = res[a][w]
        if r is None:
            print(f'{a:5} {w:5}  (not finished)'); continue
        if r['ext']:
            print(f'{a:5} {w:5}  EXTINCT'); continue
        print(f'{a:5} {w:5} {r["alive"]:6} {r["tunnels"]:7} {r["crumb"]:11.3f} {r["crumbs"]:6}/{r["space"]:<6} {r["home"]:9.2f} {r["pas"]:10.2f} {r["pcr"]:14}')
for a in arms[1:]:
    ok = [w for w in worlds if res[a][w] and res[arms[0]][w]]
    for k in ('alive', 'tunnels', 'crumb', 'pcr'):
        pr = [(res[arms[0]][w][k], res[a][w][k]) for w in ok if not (k == 'pcr' and (res[arms[0]][w]['ext'] or res[a][w]['ext']))]
        hi, lo, p = sign(pr)
        med = lambda xs: sorted(xs)[len(xs) // 2] if xs else float('nan')
        print(f'{a} vs {arms[0]}  {k:8} higher {hi} / lower {lo} of n={len(pr)}  sign p {p:.3f}   '
              f'median {med([x for x, _ in pr]):.3g} -> {med([y for _, y in pr]):.3g}')
