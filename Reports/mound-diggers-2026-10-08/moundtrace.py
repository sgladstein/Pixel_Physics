#!/usr/bin/env python3
"""Trace every ant that cut a MOUND cell in a `deeptrace dig=1` run.

    python3 moundtrace.py RUNDIR [OUTPREFIX]

MOUND cut: y <= ground_y and |x - nest_x| <= 40 and mat != provisions.
Cells within 12 columns of food_x are counted but flagged `nearfood`
(the harness zone() calls them "food").  NEST cut: y > ground_y (control).

Reads cuts.csv, cells.csv.gz, ledger.csv, events.txt, nest_f*.txt and
streams digrows.csv.gz once.  Writes OUTPREFIX_diggers.csv (one row per
mound digger) and OUTPREFIX_cuts.csv (one row per mound/nest cut joined to
its decision row), prints summary tables.
"""
import csv, gzip, glob, os, re, sys
from collections import Counter, defaultdict

RUN = sys.argv[1].rstrip('/')
OUT = sys.argv[2] if len(sys.argv) > 2 else os.path.join(os.path.dirname(os.path.abspath(__file__)), os.path.basename(RUN))
MOUND_REACH, NEARFOOD = 40, 12
RUN_GAP_F, RUN_GAP_D = 600, 2   # a "tunnel run": next cut by same ant within 600 frames and 2 cells

m = re.search(r'FOUNDED nest_x=(\d+) food_x=(\d+) ground_y=(\d+)', open(f'{RUN}/events.txt').read())
NX, FX, GY = map(int, m.groups())


def klass(x, y, mat):
    if y > GY:
        return 'nest'
    if abs(x - NX) <= MOUND_REACH and mat != 'provisions':
        return 'mound'
    return 'other'


def pct(a, b):
    return f'{100.0 * a / b:.1f}%' if b else '-'


def q(xs, p):
    xs = sorted(xs)
    return xs[min(len(xs) - 1, int(p * len(xs)))] if xs else float('nan')


def cnt_str(c, n=4, tot=None):
    tot = tot or sum(c.values())
    return ' '.join(f'{k}:{pct(v, tot)}' for k, v in c.most_common(n))


# ---------------------------------------------------------------- cuts
cuts = []
for r in csv.DictReader(open(f'{RUN}/cuts.csv')):
    x, y = int(r['x']), int(r['y'])
    k = klass(x, y, r['mat'])
    if k == 'other':
        continue
    r['k'] = k
    r['f'], r['i'], r['x'], r['y'] = int(r['frame']), int(r['id']), x, y
    r['nearfood'] = int(k == 'mound' and abs(x - FX) <= NEARFOOD)
    cuts.append(r)
cut_at = {(c['f'], c['i']): c for c in cuts}
by_ant = defaultdict(list)
for c in cuts:
    by_ant[c['i']].append(c)
mound = [c for c in cuts if c['k'] == 'mound']
nest = [c for c in cuts if c['k'] == 'nest']
print(f'# {RUN}\ngeometry nest_x={NX} food_x={FX} ground_y={GY}')
print(f'mound cuts {len(mound)} (nearfood {sum(c["nearfood"] for c in mound)}), nest cuts {len(nest)}')
print('mound mats:', cnt_str(Counter(c['mat'] for c in mound), 6))

# ---------------------------------------------------------------- ledger
ledger = {int(r['id']): r for r in csv.DictReader(open(f'{RUN}/ledger.csv'))}

# ---------------------------------------------------------------- cells: refill + drops
cell_ev = defaultdict(list)    # (x,y) -> [(f, from, to, cause, id)]
drops = defaultdict(list)      # id -> [(f, x, y)]
with gzip.open(f'{RUN}/cells.csv.gz', 'rt') as fh:
    rd = csv.reader(fh)
    next(rd)
    for f, x, y, fr, to, cause, i in rd:
        f, x, y = int(f), int(x), int(y)
        if y <= GY + 60 and abs(x - NX) <= 80:
            cell_ev[(x, y)].append((f, fr, to, cause, i))
        if cause in ('drop', 'lift') and fr == 'empty' and i:
            drops[int(i)].append((f, x, y))
END_F = max(c['f'] for c in cuts)
for c in cuts:
    ev = [e for e in cell_ev.get((c['x'], c['y']), []) if e[0] > c['f']]
    refill = next((e for e in ev if e[1] == 'empty'), None)
    c['refill_f'] = refill[0] - c['f'] if refill else ''
    c['refill_by'] = ('' if not refill else
                      'self' if refill[3] in ('drop', 'lift') and refill[4] == str(c['i']) else
                      'other_ant' if refill[3] in ('drop', 'lift') else 'fell_or_slid')
    last = ev[-1] if ev else None
    c['open_at_end'] = int(last is None or last[2] == 'empty')
    nd = next((d for d in drops.get(c['i'], []) if d[0] > c['f']), None)
    nxt_cut = next((o for o in by_ant[c['i']] if o['f'] > c['f']), None)
    if nd and (nxt_cut is None or nd[0] <= nxt_cut['f']):
        c['drop_f'], c['drop_x'], c['drop_y'] = nd[0] - c['f'], nd[1], nd[2]
        c['drop_k', 'same_cell_back', 'drop_spoil', 'drop_sflags', 'drop_pull'] = 'nest' if nd[2] > GY else ('mound' if abs(nd[1] - NX) <= MOUND_REACH else 'surface')
    else:
        c['drop_f'] = c['drop_x'] = c['drop_y'] = ''
        c['drop_k', 'same_cell_back', 'drop_spoil', 'drop_sflags', 'drop_pull'] = 'none_before_next_cut'

# tunnel runs: consecutive cuts by the same ant, close in time and space, same class
for i, cs in by_ant.items():
    cs.sort(key=lambda c: c['f'])
    run = 0
    for j, c in enumerate(cs):
        p = cs[j - 1] if j else None
        cont = (p is not None and p['k'] == c['k'] and c['f'] - p['f'] <= RUN_GAP_F
                and max(abs(c['x'] - p['x']), abs(c['y'] - p['y'])) <= RUN_GAP_D)
        run = run + 1 if cont else 1
        c['run_pos'] = run
        c['cont_prev'] = int(cont)

# ---------------------------------------------------------------- digrows stream
for c in cuts:
    c['same_cell_back'] = int(c['drop_x'] != '' and (c['drop_x'], c['drop_y']) == (c['x'], c['y']) and c['drop_f'] <= 20)
drop_key = {(c['f'] + c['drop_f'], c['i']): c for c in cuts if c['drop_f'] != ''}
MOUND_IDS = {c['i'] for c in mound}
funnel = {k: Counter() for k in ('mound', 'nest', 'mound_nearfood')}
headz = defaultdict(Counter)          # head zone -> verdict
headz_p = defaultdict(lambda: [0, 0.0])
inp_cond = defaultdict(lambda: [0, 0.0, 0])   # (zone, food_adj, at_nest) -> n, sum dig_p, cuts
ant_verd = defaultdict(Counter)       # mound diggers: verdict over rows with head in mound
ant_rows = Counter()
carry = {}                            # id -> class of last cut while still holding
carry_pull = {k: Counter() for k in ('mound', 'nest')}
pred_ok = pred_n = 0
curv_bin = defaultdict(lambda: [0, 0.0, 0])   # (zone, curvature bin) -> n, sum dig_p, cuts
mz_rows = {k: Counter() for k in ('worker', 'hold', 'pull', 'ebin')}
gate_y = defaultdict(Counter)                 # verdict on a mound target -> target row
term_sum = {k: Counter() for k in ('mound', 'nest')}
with gzip.open(f'{RUN}/digrows.csv.gz', 'rt') as fh:
    rd = csv.reader(fh)
    H = {h: j for j, h in enumerate(next(rd))}
    F, I, Z, HOLD, DIG, DP, DX, DY, DM = (H[k] for k in ('frame', 'id', 'zone', 'hold', 'dig', 'dig_p', 'dig_x', 'dig_y', 'dig_mat'))
    AN, CR, CU, FA, MG, EN, PW, SW, G8, AH, WK = (H[k] for k in ('at_nest', 'crowding', 'curvature', 'food_adj', 'moisture_grad', 'energy', 'pull_why', 'spoil_why', 'ground8', 'ahead', 'worker'))
    RX, RY, HX, HY = H['ret_x'], H['ret_y'], H['hx'], H['hy']
    for a in rd:
        i = int(a[I])
        z = a[Z]
        v = a[DIG]
        hold = a[HOLD]
        # carrying after a cut: what pulls it
        if i in carry:
            if hold == '2':
                carry_pull[carry[i]][a[PW]] += 1
            else:
                del carry[i]
        zz = 'mound' if z in ('mound_in', 'mound_top') else z
        headz[zz][v] += 1
        if a[DP]:
            hp = headz_p[zz]
            hp[0] += 1
            hp[1] += float(a[DP])
        if zz in ('mound', 'nest') and a[DP] and a[CU]:
            cb = max(-1.0, min(0.6, round(float(a[CU]) * 5) / 5))
            cbn = curv_bin[(zz, cb)]
            cbn[0] += 1
            cbn[1] += float(a[DP])
            cbn[2] += v == 'cut'
        if zz == 'mound':
            mz_rows['worker'][a[WK]] += 1
            mz_rows['hold'][hold] += 1
            mz_rows['pull'][a[PW]] += 1
            mz_rows['ebin'][('e>=1' if a[EN] and float(a[EN]) >= .999 else 'e<0.5' if a[EN] and float(a[EN]) < .5 else '0.5<=e<1')] += 1
        if zz in ('mound', 'nest') and a[DP]:
            ic = inp_cond[(zz, a[FA], a[AN])]
            ic[0] += 1
            ic[1] += float(a[DP])
            ic[2] += v == 'cut'
            # reconstruct the urge from ant.ron's wires (ReLU gate units assumed)
            cu, fa, mg, an, cr = float(a[CU] or 0), float(a[FA] or 0), float(a[MG] or 0), float(a[AN] or 0), float(a[CR] or 0)
            gate = 2.5 * (max(0.0, -30 + 30 * an + 6 * cr) - max(0.0, -30 + 30 * an - 6 * cr))
            pred = min(1.0, max(0.0, -0.3 - cu + 0.8 * fa - 0.55 * mg + gate))
            pred_n += 1
            pred_ok += abs(pred - float(a[DP])) < 0.02
        if a[DX]:
            tx, ty = int(a[DX]), int(a[DY])
            k = klass(tx, ty, a[DM])
            if k == 'mound':
                funnel['mound'][v] += 1
                gate_y[v][ty] += 1
                if abs(tx - FX) <= NEARFOOD:
                    funnel['mound_nearfood'][v] += 1
            elif k == 'nest':
                funnel['nest'][v] += 1
        if i in MOUND_IDS:
            ant_rows[i] += 1
            if zz == 'mound':
                ant_verd[i][v] += 1
        dc = drop_key.get((int(a[F]), i))
        if dc is not None:
            dc.update(drop_spoil=a[SW], drop_sflags=a[H['spoil_flags']], drop_pull=a[PW], drop_hz=z)
        if v == 'cut':
            c = cut_at.get((int(a[F]), i))
            if c is not None:
                c.update(energy=a[EN], hold=hold, pull=a[PW], spoil=a[SW], head_zone=z, ground8=a[G8], ahead=a[AH],
                         curv=a[CU], food_adj=a[FA], moist=a[MG], ret=int(bool(a[RX])),
                         ret_d=(max(abs(int(a[RX]) - c['x']), abs(int(a[RY]) - c['y'])) if a[RX] else ''))
                carry[i] = c['k']
                cu, fa, mg, an, cr = float(a[CU] or 0), float(a[FA] or 0), float(a[MG] or 0), float(a[AN] or 0), float(a[CR] or 0)
                ts = term_sum[c['k']]
                ts['n'] += 1
                ts['curv(-1)'] += -cu
                ts['food_adj(+0.8)'] += 0.8 * fa
                ts['moist(-0.55)'] += -0.55 * mg
                ts['gate(at_nest x crowding)'] += 2.5 * (max(0.0, -30 + 30 * an + 6 * cr) - max(0.0, -30 + 30 * an - 6 * cr))

# drop decision: spoil_why at the drop frame needs a second look-up; skip (pull while carrying is above)

# ---------------------------------------------------------------- tables
def share(cs, key, val):
    return pct(sum(str(c.get(key, '')) == val for c in cs), len(cs))


def mean(cs, key):
    xs = [float(c[key]) for c in cs if c.get(key, '') != '']
    return sum(xs) / len(xs) if xs else float('nan')


print('\n## 1. who cut the mound')
mc = Counter(c['i'] for c in mound)
nc = Counter(c['i'] for c in nest)
tot = len(mound)
top = mc.most_common()
cum, n50, n90 = 0, None, None
for j, (_, v) in enumerate(top, 1):
    cum += v
    if n50 is None and cum >= tot / 2:
        n50 = j
    if n90 is None and cum >= 0.9 * tot:
        n90 = j
print(f'distinct mound diggers {len(mc)} (of {len(ledger)} ants in ledger, {len(nc)} nest diggers, {len(set(mc) & set(nc))} cut both)')
print(f'top-10 share {pct(sum(v for _, v in top[:10]), tot)}; ants for 50% {n50}, for 90% {n90}')
rows = []
for i, v in top:
    cs = sorted(by_ant[i], key=lambda c: c['f'])
    ms = [c for c in cs if c['k'] == 'mound']
    L = ledger.get(i, {})
    died = L.get('died') == '1'
    end = int(L['end']) if L.get('end') else None
    en = [float(c['energy']) for c in ms if c.get('energy', '') != '']
    rows.append(dict(
        id=i, worker=Counter(c['worker'] for c in ms).most_common(1)[0][0], mound=len(ms), nest=nc.get(i, 0),
        age_first=ms[0]['age'], age_last=ms[-1]['age'], f_first=ms[0]['f'], f_last=ms[-1]['f'],
        e_med=f'{q(en, .5):.2f}' if en else '', e_full=pct(sum(e >= 0.999 for e in en), len(en)),
        hold=cnt_str(Counter(c.get('hold', '?') for c in ms), 3), pull=cnt_str(Counter(c.get('pull', '?') for c in ms), 2),
        food_adj=pct(sum(c.get('food_adj') == '1' for c in ms), len(ms)), nearfood=pct(sum(c['nearfood'] for c in ms), len(ms)),
        roofed=pct(sum(c['roofed'] == '1' for c in ms), len(ms)), cont=pct(sum(c['cont_prev'] for c in ms), len(ms)),
        mat=cnt_str(Counter(c['mat'] for c in ms), 2),
        verdict_in_mound=cnt_str(ant_verd[i], 4), born=L.get('born', ''),
        death=(f"{L.get('cause')} @{end} (+{end - ms[-1]['f']} after last mound cut)" if died else f'alive @{end}'),
        nest_dec=L.get('nest', ''), mound_dec=L.get('mound', ''), surf_dec=L.get('surface', ''), heap_dec=L.get('heap', ''),
    ))
with open(f'{OUT}_diggers.csv', 'w', newline='') as fh:
    w = csv.DictWriter(fh, fieldnames=list(rows[0]))
    w.writeheader()
    w.writerows(rows)
cols = ['id', 'worker', 'mound', 'nest', 'age_first', 'age_last', 'e_med', 'e_full', 'hold', 'pull', 'food_adj', 'nearfood', 'roofed', 'cont', 'mat', 'death']
print('| ' + ' | '.join(cols) + ' |')
print('|' + '---|' * len(cols))
for r in rows[:15]:
    print('| ' + ' | '.join(str(r[k]) for k in cols) + ' |')
print('verdicts over top-15 diggers\' decisions with head in the mound:')
for r in rows[:15]:
    print(f"  {r['id']}: {r['verdict_in_mound']}   ledger decisions nest/mound/surface/heap {r['nest_dec']}/{r['mound_dec']}/{r['surf_dec']}/{r['heap_dec']}")
dead_soon = sum(1 for r in rows if 'after last' in r['death'] and int(re.search(r'\+(\d+)', r['death']).group(1)) <= 5000)
print(f'diggers dead within 5000 frames of their last mound cut: {dead_soon}/{len(rows)}; causes ' +
      str(Counter(r['death'].split(' ')[0] for r in rows if 'after last' in r['death'])))

print('\n## 2. inputs at the cut: mound vs nest (control)')
def block(cs, name):
    return dict(name=name, n=len(cs), worker=share(cs, 'worker', '1'), dig_p=f"{mean(cs, 'dig_p'):.3f}",
                at_nest=share(cs, 'at_nest', '1'), crowd=f"{mean(cs, 'crowding'):.3f}", food_adj=share(cs, 'food_adj', '1'),
                curv=f"{mean(cs, 'curv'):.3f}", moist=f"{mean(cs, 'moist'):.3f}", energy=f"{mean(cs, 'energy'):.3f}",
                e_full=pct(sum(float(c.get('energy') or 0) >= .999 for c in cs), len(cs)),
                roofed=share(cs, 'roofed', '1'), home=share(cs, 'home', '1'), open8=f"{mean(cs, 'open8'):.2f}",
                ground24=f"{mean(cs, 'ground24'):.1f}", ants3=f"{mean(cs, 'ants3'):.2f}", joins=share(cs, 'joins', '1'),
                hold_crop=share(cs, 'hold', '1'), ret=share(cs, 'ret', '1'),
                flags=cnt_str(Counter(c['flags'] for c in cs), 4), headzone=cnt_str(Counter(c.get('head_zone', '?') for c in cs), 3),
                pull=cnt_str(Counter(c.get('pull', '?') for c in cs), 4))
blocks = [block(mound, 'mound'), block([c for c in mound if not c['nearfood']], 'mound_not_nearfood'),
          block([c for c in mound if c['nearfood']], 'mound_nearfood'), block(nest, 'nest')]
for k in blocks[0]:
    print(f'{k:>10} ' + ' | '.join(str(b[k]) for b in blocks))
print(f'urge reconstruction (ant.ron wires, ReLU gate) within 0.02 of logged dig_p: {pct(pred_ok, pred_n)} of {pred_n} rows')
for k in ('mound', 'nest'):
    ts = term_sum[k]
    n = ts.pop('n', 0)
    print(f'  mean urge terms at {k} cuts (n={n}): ' + ', '.join(f'{t} {v / max(n, 1):+.3f}' for t, v in ts.items()) + ', bias -0.300')
print('dig_p and cut rate by head zone, food_adj, at_nest (all decisions with a dig_p):')
for key in sorted(inp_cond):
    n, s, cn = inp_cond[key]
    print(f'  {key}: n={n} mean dig_p {s / n:.3f} cut/row {cn / n:.4f}')
print('dig funnel, by where the target cell is (rows whose roll won and had a target):')
for k, cn in funnel.items():
    t = sum(cn.values())
    print(f'  {k}: n={t} ' + ' '.join(f'{v}:{pct(cn[v], t)}' for v in ('cut', 'cue', 'roof', 'face', 'no_ground')))
print('verdicts by head zone (all decisions):')
for zz in ('mound', 'nest', 'surface', 'food'):
    cn = headz[zz]
    t = sum(cn.values())
    hp = headz_p[zz]
    print(f'  {zz}: n={t} mean dig_p {hp[1] / max(hp[0], 1):.3f} ' + cnt_str(cn, 6))

print('\n## 3. what the hole is')
for name, cs in (('mound', mound), ('nest', nest)):
    runs = [c['run_pos'] for c in cs]
    ends = [c for j, c in enumerate(cs)]
    rf = [c for c in cs if c['refill_f'] != '']
    print(f'{name}: n={len(cs)} roofed {share(cs, "roofed", "1")}, head in mound_in {share(cs, "head_zone", "mound_in")}, '
          f'continues own previous cut (<= {RUN_GAP_F} f, <= {RUN_GAP_D} cells) {pct(sum(c["cont_prev"] for c in cs), len(cs))}, '
          f'run position >=5 {pct(sum(r >= 5 for r in runs), len(runs))}, open8 mean {mean(cs, "open8"):.2f}')
    print(f'   refilled later {pct(len(rf), len(cs))} (median after {q([c["refill_f"] for c in rf], .5)} f; by ' +
          cnt_str(Counter(c['refill_by'] for c in rf), 3) + f'); open at end {share(cs, "open_at_end", "1")}; '
          f'refilled within 2000 f {pct(sum(c["refill_f"] <= 2000 for c in rf), len(cs))}')

print('\n## 4. where the pellet goes')
for name, cs in (('mound', mound), ('nest', nest)):
    dk = Counter(c['drop_k', 'same_cell_back', 'drop_spoil', 'drop_sflags', 'drop_pull'] for c in cs)
    dd = [max(abs(c['drop_x'] - c['x']), abs(c['drop_y'] - c['y'])) for c in cs if c['drop_x'] != '']
    print(f'{name}: next drop ' + cnt_str(dk, 4) + f'; median distance {q(dd, .5)} cells, p90 {q(dd, .9)}; '
          f'median frames to drop {q([c["drop_f"] for c in cs if c["drop_f"] != ""], .5)}')
    print(f'   spoil_why at cut row ' + cnt_str(Counter(c.get('spoil', '?') for c in cs), 4))
    print(f'   pull while carrying the pellet ' + cnt_str(carry_pull[name], 5))
md = [c for c in mound if c['drop_k', 'same_cell_back', 'drop_spoil', 'drop_sflags', 'drop_pull'] == 'mound']
print('mound-cut pellets dropped back on the mound: rows ' + str(sorted(Counter(c['drop_y'] for c in md).items())[:30]))

print('\n## 5. over time (per 20k frames)')
print('| frames | mound cuts | of which nearfood | nest cuts | mound share | mound diggers |')
print('|---|---|---|---|---|---|')
for b in range(0, END_F + 1, 20000):
    ms = [c for c in mound if b <= c['f'] < b + 20000]
    ns = [c for c in nest if b <= c['f'] < b + 20000]
    print(f'| {b}-{b + 20000} | {len(ms)} | {sum(c["nearfood"] for c in ms)} | {len(ns)} | {pct(len(ms), len(ms) + len(ns))} | {len({c["i"] for c in ms})} |')

print('\n## standing state: the holes on screen, from nest_f maps (mound box, y < ground_y)')
print('covered = open cell (o, ., or an ant a) with ground #/=/s somewhere above it in the box; o = open and once cut')
print('| frame | ground cells | covered open cells | of which o (once cut) | of which . (never cut) | ants a in mound |')
print('|---|---|---|---|---|---|')
for p in sorted(glob.glob(f'{RUN}/nest_f*.txt'))[3::4]:
    L = open(p).read().split('\n')
    x0, y0, wd, ht = map(int, L[0].split())
    g = an = cov = cov_o = cov_d = 0
    for x in range(NX - MOUND_REACH, NX + MOUND_REACH + 1):
        roof = False
        for y in range(y0, GY):
            ch = L[1 + y - y0][x - x0]
            if ch in '#=s':
                g += 1
                roof = True
            elif ch in 'o.a':
                an += ch == 'a'
                if roof:
                    cov += 1
                    cov_o += ch == 'o'
                    cov_d += ch == '.'
    print(f'| {p[-10:-4]} | {g} | {cov} | {cov_o} | {cov_d} | {an} |')

print('\n## extra: dig_p and cut rate by curvature at the head (5x5 disc), mound vs nest')
print('| zone | curv bin | rows | mean dig_p | cuts per 1000 rows |')
print('|---|---|---|---|---|')
for key in sorted(curv_bin):
    n, sp, cn = curv_bin[key]
    if n >= 500:
        print(f'| {key[0]} | {key[1]:+.1f} | {n} | {sp / n:.3f} | {1000 * cn / n:.2f} |')
print('who stands in the mound zone (all decisions there):')
for k, cn in mz_rows.items():
    print(f'  {k}: ' + cnt_str(cn, 5))
print('mound-target verdict by target row (roof/cue/cut):')
for v in ('roof', 'cue', 'cut'):
    print(f'  {v}: ' + str(sorted(gate_y[v].items())))
# reconstruction at mound cuts (gate is ~0 there)
ok = n = 0
for c in mound:
    if c.get('curv', '') == '':
        continue
    an, cr = float(c['at_nest']), float(c['crowding'])
    gate = 2.5 * (max(0.0, -30 + 30 * an + 6 * cr) - max(0.0, -30 + 30 * an - 6 * cr))
    pred = min(1.0, max(0.0, -0.3 - float(c['curv']) + 0.8 * float(c['food_adj']) - 0.55 * float(c['moist']) + gate))
    n += 1
    ok += abs(pred - float(c['dig_p'])) < 0.02
print(f'urge reconstruction within 0.02 at mound cuts: {pct(ok, n)} of {n}')

# unclamped energy at the cut, from colony.csv (sample at or before the cut)
esamp = defaultdict(list)
for r in csv.DictReader(open(f'{RUN}/colony.csv')):
    esamp[int(r['id'])].append((int(r['frame']), float(r['energy_j'])))
import bisect
for c in cuts:
    s_ = esamp.get(c['i'], [])
    j = bisect.bisect_right([t for t, _ in s_], c['f']) - 1
    c['energy_j'] = s_[j][1] if j >= 0 else ''
for name, cs in (('mound', mound), ('nest', nest)):
    ej = [c['energy_j'] for c in cs if c['energy_j'] != '']
    print(f'energy_j at {name} cuts (colony.csv sample <=1000 f before): n={len(ej)} p10 {q(ej, .1):.0f} median {q(ej, .5):.0f} p90 {q(ej, .9):.0f}; '
          f'>=1100 J (brief\'s lay bar) {pct(sum(e >= 1100 for e in ej), len(ej))}; <200 J {pct(sum(e < 200 for e in ej), len(ej))}')

# the holes on screen at the end: is each 'o' mound cell the cut itself, or reopened since?
last_map = sorted(glob.glob(f'{RUN}/nest_f*.txt'))[-1]
L = open(last_map).read().split('\n')
x0, y0, wd, ht = map(int, L[0].split())
cut_cells = Counter((c['x'], c['y']) for c in cuts)
att = Counter()
for y in range(y0, GY):
    for x in range(NX - MOUND_REACH, NX + MOUND_REACH + 1):
        if L[1 + y - y0][x - x0] != 'o':
            continue
        ev = cell_ev.get((x, y), [])
        lastopen = next((e for e in reversed(ev) if e[2] == 'empty'), None)
        att['last opened by cut' if lastopen and lastopen[3] == 'cut' else
            'last opened by ground falling away' if lastopen else 'no event logged'] += 1
        att['ever cut (mound def)'] += (x, y) in cut_cells
        att['total'] += 1
print(f'{last_map[-11:]}: mound o cells ' + str(dict(att)))

print('\n## extra 2: the dig-down turn and the pocket')
for name, cs in (('mound', mound), ('nest', nest)):
    dy = Counter(max(-1, min(1, c['y'] - int(c['hy']))) for c in cs)
    dn = [c for c in cs if int(c['flags']) & 1]
    enc = [c for c in cs if c.get('curv', '') != '' and float(c['curv']) <= -0.3]
    print(f'{name}: target below head {pct(dy[1], len(cs))}, level {pct(dy[0], len(cs))}, above {pct(dy[-1], len(cs))}; '
          f'DOWN flag {pct(len(dn), len(cs))}; head curvature <= -0.3 (enclosed) {pct(len(enc), len(cs))}; '
          f'dig_p == 0 at cut {pct(sum(float(c["dig_p"]) == 0 for c in cs), len(cs))}')
per = Counter(Counter(c['i'] for c in mound).values())
print('mound cuts per digger: ' + ', '.join(f'{k}:{per[k]}' for k in sorted(per) if k <= 10) + f', >10:{sum(v for k, v in per.items() if k > 10)}')
print('\n## extra 3: every mound cut of the top 3 mound diggers (inputs -> output)')
print('| frame | id | job | head | target | mat | curv | food_adj | at_nest | dig_p | flags | roofed | open8 | refilled after (by) | pellet dropped at |')
print('|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|')
for i, _ in top[:3]:
    for c in sorted(by_ant[i], key=lambda c: c['f']):
        if c['k'] != 'mound':
            continue
        job = ('worker' if c['worker'] == '1' else 'forager') + f" e={c.get('energy_j', '')}J"
        print(f"| {c['f']} | {i} | {job} | {c['hx']},{c['hy']} | {c['x']},{c['y']} | {c['mat']} | {c.get('curv', '')} | {c.get('food_adj', '')} | {c['at_nest']} | {c['dig_p']} | {c['flags']} | {c['roofed']} | {c['open8']} | {c['refill_f']} ({c['refill_by']}) | {c['drop_x']},{c['drop_y']} |")

print('\n## extra 4: the cut-and-put-back loop (pellet dropped into the cell just cut, within 20 frames)')
for name, cs in (('mound', mound), ('nest', nest)):
    sb = [c for c in cs if c['same_cell_back']]
    dr = [c for c in cs if c['drop_f'] != '']
    print(f'{name}: put back in the same cell {len(sb)} = {pct(len(sb), len(cs))} of cuts; spoil_why at those drops ' +
          cnt_str(Counter(c.get('drop_spoil', '?') for c in sb), 4) + '; flags ' + cnt_str(Counter(c.get('drop_sflags', '?') for c in sb), 3))
    print(f'   all next drops: spoil_why ' + cnt_str(Counter(c.get('drop_spoil', '?') for c in dr), 5) + '; head zone at drop ' + cnt_str(Counter(c.get('drop_hz', '?') for c in dr), 4))
sbm = Counter(c['i'] for c in mound if c['same_cell_back'])
print('top-15 mound diggers, put-back cuts / mound cuts: ' + ', '.join(f"{i}:{sbm.get(i, 0)}/{v}" for i, v in top[:15]))
real = Counter(c['i'] for c in mound if not c['same_cell_back'])
rt = sorted(real.values(), reverse=True)
print(f'excluding put-backs: {sum(rt)} mound cuts by {len(rt)} ants, top-10 share {pct(sum(rt[:10]), sum(rt))}')

keys = ['k', 'f', 'i', 'age', 'worker', 'hx', 'hy', 'x', 'y', 'zone', 'mat', 'nearfood', 'flags', 'dig_p', 'at_nest', 'crowding', 'food_adj',
        'curv', 'moist', 'energy', 'hold', 'pull', 'spoil', 'head_zone', 'ground8', 'ahead', 'ret', 'ret_d', 'open8', 'ground24', 'joins', 'roofed', 'home',
        'ants3', 'energy_j', 'run_pos', 'cont_prev', 'refill_f', 'refill_by', 'open_at_end', 'drop_f', 'drop_x', 'drop_y', 'drop_k', 'same_cell_back', 'drop_spoil', 'drop_sflags', 'drop_pull']
with open(f'{OUT}_cuts.csv', 'w', newline='') as fh:
    w = csv.writer(fh)
    w.writerow(keys)
    for c in sorted(cuts, key=lambda c: c['f']):
        w.writerow([c.get(k, '') for k in keys])
print(f'\nwrote {OUT}_diggers.csv and {OUT}_cuts.csv')
