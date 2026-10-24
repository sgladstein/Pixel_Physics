"""summ.py OUTDIR ARM... : one row per run and per-arm medians."""
import csv, sys, os, statistics as st, collections
sys.path.insert(0, '/home/user/Pixel_Physics/scratch/inrun')
import nestcount, roomshape
out = sys.argv[1]; arms = sys.argv[2:]
def row(d):
    s = list(csv.DictReader(open(f'{d}/stock.csv')))
    late = [r for r in s if int(r['frame']) > 100000]
    c = collections.Counter(); fb = collections.Counter()
    for e in csv.DictReader(open(f'{d}/events.csv')):
        k = e['kind']
        if k == 'deliver': c['home'] += abs(int(e['x']) - 128) <= 8
        elif k == 'death': c['starved'] += e['v'] == '1'
        elif k == 'field':
            c['field'] += 1; v = float(e['v'])
            if v != 960.0: c['pfield'] += 1; c['pJ'] += v
            else: c['pileJ'] += v
        elif k == 'dig': c['dig'] += 1
        elif k == 'eat': c['eat'] += 1
    n = nestcount.count(d, 200000)
    r = dict(alive=int(s[-1]['alive']), minalive=min(int(x['alive']) for x in s), starved=c['starved'], field=c['field'], home=c['home'], dig=c['dig'],
             nestfood=int(s[-1]['nest_food']), nestfood_late=st.median(int(x['nest_food']) for x in late) if late else 0,
             tank_late=round(st.median(float(x['tank_mean']) for x in late), 1) if late else 0,
             plant_bites=c['pfield'], plant_J_share=round(c['pJ'] / max(1, c['pJ'] + c['pileJ']), 2),
             nest=n['nest'], crater=n['crater'], nestcuts=n['nestcuts'], late_cuts=n['nestcuts_late'], shape=round(roomshape.score(d), 2))
    if os.path.exists(f'{d}/brood.csv'):
        b = list(csv.DictReader(open(f'{d}/brood.csv')))
        r['brood_cols'] = '|'.join(b[0].keys()) if False else None
        r.pop('brood_cols')
        r['peak_ants'] = max(int(x['adults']) for x in b)
        r['hatched'] = int(b[-1]['hatched']) if 'hatched' in b[-1] else -1
    if os.path.exists(f'{d}/garden.csv'):
        g = list(csv.DictReader(open(f'{d}/garden.csv')))
        r['plants_end'] = int(g[-1]['plants']); r['seeds_end'] = int(g[-1]['seeds'])
        r['leaf_end'] = int(g[-1]['live_leaf']) + int(g[-1]['live_grassblade']); r['fruit_end'] = int(g[-1]['live_fruit']) + int(g[-1]['loose_fruit']) + int(g[-1]['loose_windfall'])
    return r
allrows = {}
for a in arms:
    rows = []
    for k in sorted(os.listdir(out)):
        if k.startswith(a + '_s') and os.path.isdir(f'{out}/{k}') and os.path.exists(f'{out}/{k}/nestmap.txt'):
            try: rows.append((k, row(f'{out}/{k}')))
            except Exception as ex: print(k, 'ERR', ex)
    allrows[a] = rows
    for k, r in rows: print(k, ' '.join(f'{x}={y}' for x, y in r.items()))
    if rows:
        keys = rows[0][1].keys()
        print(f'== {a} median of {len(rows)}:', ' '.join(f'{x}={st.median(r[x] for _, r in rows)}' for x in keys))
