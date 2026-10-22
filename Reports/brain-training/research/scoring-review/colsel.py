"""Colony-level selection prototype (scoring review lane, 2026-10-22). SCRATCH.

usage: colsel.py OUT MODE NCOL ROUNDS FRAMES GENOME [inrun args...]
MODE plain: every colony runs rounds.sh's loop on its own (top 30 by ant score carried into a fresh world).
MODE colony: the same, but after each round the colonies are ranked on their RESULT (rank-sum of: food home within 8
  cols, original-ground cuts within 20 cols in the round's second half, starved deaths (fewer better), final nest score),
  and the bottom half are refounded from the top half's carried ants (best donates to worst). Extinct = bottom.
Round r of colony c runs in world seed c*1000+r, as rounds.sh does; round 0 founds on 30 mutants of GENOME.
"""
import csv, os, subprocess, sys, gzip
from concurrent.futures import ThreadPoolExecutor

OUT, MODE, NCOL, R, F, G = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4]), int(sys.argv[5]), sys.argv[6]
EXTRA = sys.argv[7:]
BIN = os.environ.get('BIN', '/home/user/sr/target/release/examples/inrun')
P = int(os.environ.get('P', '4'))
SEL = os.environ.get('SEL', 'score')
C = [f'frames={F}', 'dist=25', 'spread=10', 'grants=3', 'window=4000', f'sel={SEL}'] + EXTRA


def run(c, r, founders):
    out = f'{OUT}/c{c}/r{r}'
    os.makedirs(out, exist_ok=True)
    seed = c if r == 0 else c * 1000 + r
    args = [BIN] + founders + [f'seed={seed}'] + C + [f'out={out}']
    with open(f'{out}.log', 'w') as lf:
        subprocess.run(args, stdout=lf, stderr=subprocess.STDOUT, check=False)
    return out


def carried(rdir):
    """rounds.sh's carry: top 30 by ant score, topped up to 30 requests by repeating."""
    idx = f'{rdir}/final/index.csv'
    if not os.path.exists(idx):
        return None
    r = list(csv.DictReader(open(idx)))
    r.sort(key=lambda x: int(x['id']))
    r.sort(key=lambda x: (-float(x['score']), int(x['id'])))
    r = r[:30]
    n = len(r)
    if n == 0:
        return None
    return ','.join(f"{rdir}/final/{x['id']}.raw.txt@{x['family']}.{x['id']}:{30 // n + (1 if i < 30 % n else 0)}" for i, x in enumerate(r))


def result(rdir):
    """(home8, late cuts, starved, nest score, alive) of one colony-round."""
    ev = f'{rdir}/events.csv'
    home = late = starved = 0
    op = gzip.open if not os.path.exists(ev) and os.path.exists(ev + '.gz') else open
    if not os.path.exists(ev) and os.path.exists(ev + '.gz'):
        ev += '.gz'
    with op(ev, 'rt') as f:
        next(f)
        for line in f:
            p = line.split(',')
            k = p[2]
            if k == 'deliver' and abs(int(p[3]) - 128) <= 8:
                home += 1
            elif k == 'dig' and abs(int(p[3]) - 128) <= 20 and int(p[0]) > F // 2:
                late += 1
            elif k == 'death' and p[5] == '1':
                starved += 1
    ns, alive = 0.0, 0
    nf = f'{rdir}/nest.csv'
    if os.path.exists(nf):
        rows = list(csv.DictReader(open(nf)))
        if rows:
            ns, alive = float(rows[-1]['nest_score']), int(rows[-1]['alive'])
    return home, late, starved, ns, alive


def ranks(vals, higher_better=True):
    order = sorted(range(len(vals)), key=lambda i: vals[i], reverse=higher_better)
    rk = [0.0] * len(vals)
    i = 0
    while i < len(order):  # average ranks over ties
        j = i
        while j + 1 < len(order) and vals[order[j + 1]] == vals[order[i]]:
            j += 1
        for t in range(i, j + 1):
            rk[order[t]] = (i + j) / 2
        i = j + 1
    return rk


log = open(f'{OUT}/colsel.csv', 'a')
if os.path.getsize(f'{OUT}/colsel.csv') == 0:
    log.write('round,colony,home8,late_cuts,starved,nest_score,alive,ranksum,refounded_from\n')
founders = {c: [f'founders={G}:1', 'mutants=30:0.03:1.0:7'] for c in range(1, NCOL + 1)}
for r in range(R):
    live = [c for c in founders if founders[c] is not None]
    with ThreadPoolExecutor(P) as ex:
        dirs = dict(zip(live, ex.map(lambda c: run(c, r, founders[c]), live)))
    res, carry = {}, {}
    for c in range(1, NCOL + 1):
        if c in dirs:
            res[c] = result(dirs[c])
            carry[c] = carried(dirs[c])
        else:
            res[c], carry[c] = None, None
    cols = list(range(1, NCOL + 1))
    # An extinct colony (no carry) ranks last on everything.
    ok = [c for c in cols if carry[c] is not None]
    rs = {c: 1e9 for c in cols}
    if ok:
        h = ranks([res[c][0] for c in ok]); lc = ranks([res[c][1] for c in ok])
        sv = ranks([res[c][2] for c in ok], higher_better=False); ns = ranks([res[c][3] for c in ok])
        for i, c in enumerate(ok):
            rs[c] = h[i] + lc[i] + sv[i] + ns[i]
    order = sorted(cols, key=lambda c: (rs[c], c))  # best first
    src = {c: '' for c in cols}
    nxt = {c: carry[c] for c in cols}
    if MODE == 'colony' and ok:
        half = NCOL // 2
        top = [c for c in order[:half] if carry[c] is not None]
        bottom = order[half:][::-1]  # worst first
        for i, b in enumerate(bottom):
            if top:
                d = top[i % len(top)]
                nxt[b] = carry[d]
                src[b] = str(d)
    for c in cols:
        v = res[c] or (0, 0, 0, 0.0, 0)
        log.write(f'{r},{c},{v[0]},{v[1]},{v[2]},{v[3]:.2f},{v[4]},{rs[c] if rs[c] < 1e9 else ""},{src[c]}\n')
    log.flush()
    for c in cols:
        founders[c] = None if nxt[c] is None else [f'founders={nxt[c]}']
    # keep the disk small: maps and per-window ant lists are not needed
    for c in live:
        d = dirs[c]
        for fn in os.listdir(d):
            if fn.startswith('map_f') or fn.startswith('ants_f'):
                os.remove(f'{d}/{fn}')
        if r < R - 1 and os.path.exists(f'{d}/events.csv'):
            subprocess.run(['gzip', '-f', f'{d}/events.csv'])
    print(f'round {r} done: ' + ' '.join(f'c{c}:{rs[c] if rs[c] < 1e9 else "X"}{"<-" + src[c] if src[c] else ""}' for c in order), flush=True)
print('DONE', flush=True)
