"""SCRATCH (chambers selection run, 2026-10-29): the judge of batch h3sel, written before launch. Reads the colsel folders
the batch's runners publish and prints the deciding measure, the guards, the every-round log and the seed check.

    git fetch origin batch-results/h3sel && mkdir -p R && git archive FETCH_HEAD | tar -x -C R
    for t in R/*.tgz; do tar -xzf $t -C R/full; done        # (mkdir R/full first) the whole folders, nest maps included
    python3 scratch/nest/selrun/judge.py R/full A=h3eat_std B=h3eat_shape C=std_std [round=N]

DECIDING MEASURE (named before launch; .github/train-jobs/h3sel.batch carries the same text): at the last round, paired
by colony (every arm plays world c*1000+r in round r), B against A, two-sided sign test on
  (1) galleryc -- open passage cells, crumbs and food as wall (scratch/inrun/nestshape.py) -- higher is better, and
  (2) alive at the end of the round -- higher is better (a colony with no run, extinct, is alive 0, galleryc 0),
B KEEPS THE TUNNEL NEST if (1) favours B at p < 0.05 and (2) does not favour A at p < 0.05, AND the crumb-proof check
holds: crumb share (crumbs / nest space) is not higher in B at p < 0.05 -- a win on passages bought with a crumb-packed
nest is not a win. B against C and A against C are reported the same way (does H3 + eat keep its tunnels at all, with
and without the shape case, against the cone founder). Extinct counts beside them. Missing data counts as the worse of
its pair. Everything else printed here is description, not a test.
Seed check (not a test): in A's and B's last-round carries, the share of genomes still holding the eat seed (g[6] > 0,
g[1680] < -30, g[1683] > 30, g[1715] < -19: half the seed's values) and the dig head's drift from H3's (layout 8: the
layout-5 head g[2797:3279] is g[4137:4619]; checked with a probe genome through inrun mutate_to, 2026-10-29).
"""
import csv, glob, math, os, sys
sys.path.insert(0, 'scratch/inrun')
import nestshape

ROOT = sys.argv[1]
ARMS = dict(a.split('=', 1) for a in sys.argv[2:] if a[:1] in 'ABC' and '=' in a)
RND = next((int(a.split('=')[1]) for a in sys.argv[2:] if a.startswith('round=')), None)
H5, H8 = (2797, 3279), (4137, 4619)
SEED = {6: (0.0, 1), 1680: (-30.0, -1), 1683: (30.0, 1), 1715: (-19.2, -1)}  # slot: (half the seed value, sign)


def sign_test(a, b, higher=True):
    """(wins for a, wins for b, two-sided exact p) over paired values; ties dropped."""
    wa = sum(1 for x, y in zip(a, b) if (x > y if higher else x < y))
    wb = sum(1 for x, y in zip(a, b) if (y > x if higher else y < x))
    n, k = wa + wb, min(wa, wb)
    p = min(1.0, 2 * sum(math.comb(n, i) for i in range(k + 1)) / 2 ** n) if n else 1.0
    return wa, wb, p


def rows(path):
    return list(csv.DictReader(open(path))) if os.path.exists(path) else []


def arm_data(out):
    d = f'{ROOT}/{out}'
    cs = rows(f'{d}/colsel.csv')
    ns = rows(f'{d}/nestshape.csv')
    last = max(int(x['round']) for x in cs) if cs else -1
    return d, cs, ns, last


def med(v):
    v = sorted(v)
    return v[len(v) // 2] if v else float('nan')


data = {k: arm_data(v) for k, v in ARMS.items()}
last = RND if RND is not None else min(x[3] for x in data.values())
ncol = max(int(x['colony']) for k in data for x in data[k][1])
print(f'h3sel judge: arms {ARMS}, last round {last}, {ncol} colonies')


def at(k, r):
    """{colony: (alive, galleryc, crumb_share, rooms_open)} at round r; a colony with no run there is (0, 0, 1, 0)."""
    d, cs, ns, _ = data[k]
    al = {int(x['colony']): int(x['alive']) for x in cs if int(x['round']) == r}
    sh = {int(x['colony']): x for x in ns if int(x['round']) == r}
    out = {}
    for c in range(1, ncol + 1):
        s = sh.get(c)
        # alive at the round's end from stock.csv (a row every 1,000 frames) where the folder has it: colsel.csv's
        # alive is nest.csv's last row, written every 25k, so a run shorter than 25k (the smoke) reads 0 there
        st = rows(f'{d}/c{c}/r{r}/stock.csv')
        if st:
            al[c] = int(st[-1]['alive'])
        out[c] = (al.get(c, 0), int(s['galleryc']) if s else 0, float(s['crumb_share']) if s else 1.0,
                  float(s['rooms_open']) if s else 0.0)
    return out


print('\n== every round: median over colonies (extinct = alive 0, galleryc 0); ext = colonies with no ant at the end')
print('arm round  alive  ext  galleryc  crumb_share  rooms_open')
for k in sorted(data):
    for r in range(last + 1):
        v = at(k, r)
        print(f'{k:3s} {r:5d} {med([x[0] for x in v.values()]):6} {sum(1 for x in v.values() if x[0] == 0):4d} '
              f'{med([x[1] for x in v.values()]):9} {med([x[2] for x in v.values()]):12.3f} {med([x[3] for x in v.values()]):11}')

print(f'\n== DECIDING MEASURE, round {last}, paired by colony (wins first arm / second arm, two-sided sign test p)')
fin = {k: at(k, last) for k in data}
verdict = None
for a, b in (('B', 'A'), ('B', 'C'), ('A', 'C')):
    if a not in fin or b not in fin:
        continue
    cs = range(1, ncol + 1)
    g = sign_test([fin[a][c][1] for c in cs], [fin[b][c][1] for c in cs])
    al = sign_test([fin[a][c][0] for c in cs], [fin[b][c][0] for c in cs])
    cr = sign_test([fin[a][c][2] for c in cs], [fin[b][c][2] for c in cs], higher=False)
    ext = (sum(1 for c in cs if fin[a][c][0] == 0), sum(1 for c in cs if fin[b][c][0] == 0))
    print(f'{a} vs {b}: galleryc {g[0]}/{g[1]} p {g[2]:.3g} | alive {al[0]}/{al[1]} p {al[2]:.3g} | '
          f'fewer crumbs {cr[0]}/{cr[1]} p {cr[2]:.3g} | extinct {ext[0]} vs {ext[1]}')
    if (a, b) == ('B', 'A'):
        keeps = g[0] > g[1] and g[2] < 0.05
        alive_ok = not (al[1] > al[0] and al[2] < 0.05)
        crumb_ok = not (cr[1] > cr[0] and cr[2] < 0.05)
        verdict = ('B KEEPS THE TUNNEL NEST' if keeps and alive_ok and crumb_ok else
                   'NOT SHOWN' + ('' if keeps else ': passages not higher at p < 0.05') +
                   ('' if alive_ok else '; alive worse') + ('' if crumb_ok else '; crumb share higher'))
print('VERDICT (B vs A):', verdict)

print('\n== garden (cgshape.csv: selection off, world c*1000+99, the carries of the garden round), medians')
for k in sorted(data):
    g = rows(f'{data[k][0]}/cgshape.csv')
    for r in sorted({int(x['round']) for x in g}):
        v = [x for x in g if int(x['round']) == r]
        print(f'{k} garden r{r}: n {len(v)} galleryc {med([int(x["galleryc"]) for x in v])} '
              f'crumb_share {med([float(x["crumb_share"]) for x in v]):.3f} rooms_open {med([float(x["rooms_open"]) for x in v])}')

print(f'\n== within round {last}: median galleryc at each 25k snapshot (a tunnel that fades to a cone inside a round)')
for k in sorted(data):
    snaps = {}
    for c in range(1, ncol + 1):
        rd = f'{data[k][0]}/c{c}/r{last}'
        for fn in sorted(glob.glob(f'{rd}/nestmap_f*.txt')):
            m = nestshape.measure(rd, fn)
            snaps.setdefault(os.path.basename(fn)[9:16], []).append(m['galleryc'] if m else 0)
    print(f'{k}: ' + ' '.join(f'{f.lstrip("0") or "0"}:{med(v)}' for f, v in sorted(snaps.items())))


def genome(p):
    g = {}
    for l in open(p).read().split('\n')[1:]:
        if l.strip():
            i, w = l.split()
            g[int(i)] = float(w)
    return g


def carry(rd):
    r = rows(f'{rd}/final/index.csv')
    r.sort(key=lambda x: int(x['id']))
    r.sort(key=lambda x: (-float(x['score']), int(x['id'])))
    return [genome(f'{rd}/final/{x["id"]}.raw.txt') for x in r[:30]]


def pool_head(path):
    lines = [l for l in open(path).read().split('\n') if l.strip() and not l.startswith('#')]
    g, started = {}, False
    for l in lines[1:]:
        if l.startswith('member '):
            if started:
                break
            started = True
            continue
        i, w = l.split()
        g[int(i)] = float(w)
    return [g.get(i, 0.0) for i in range(*H5)]


h3 = pool_head('scratch/inrun/genomes/h3eat/h3eat_pool.txt')
std = pool_head('assets/minds/std_pool.txt')
print(f'\n== seed check, round {last} carries (top 30 by ant score); rank 0 = the round\'s best colony')
for k in sorted(data):
    d, cs = data[k][0], data[k][1]
    best = next((int(x['colony']) for x in cs if int(x['round']) == last and x['rank'] == '0'), None)
    tot = held = 0
    drift, nearer = [], 0
    rank0 = None
    for c in range(1, ncol + 1):
        gs = carry(f'{d}/c{c}/r{last}')
        if not gs:
            continue
        h = [sum(1 for s, (half, sg) in SEED.items() if sg * g.get(s, 0.0) > sg * half) == 4 for g in gs]
        tot, held = tot + len(gs), held + sum(h)
        for g in gs:
            hd = [g.get(i, 0.0) for i in range(*H8)]
            d3 = sum(abs(x - y) for x, y in zip(hd, h3))
            ds = sum(abs(x - y) for x, y in zip(hd, std))
            drift.append(d3 / sum(abs(y) for y in h3))
            nearer += d3 < ds
        if c == best:
            rank0 = (c, sum(h), len(gs), [round(med([g.get(s, 0.0) for g in gs]), 2) for s in SEED])
    print(f'{k}: eat seed held by {held}/{tot} carried genomes; dig head nearer H3 than std_pool in {nearer}/{tot}, '
          f'median drift from H3 head {med(drift):.3f} (L1 / |H3 head|); rank-0 colony {rank0} (colony, held, of, medians g6 g1680 g1683 g1715)')
