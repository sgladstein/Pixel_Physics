#!/usr/bin/env python3
"""foodflow.py RUN [RUN...] [--from F] [--to F] -- where food goes once it reaches home.

Reads `deeptrace foodlog=1` output (food.csv.gz, digest.csv, foodcells.csv,
larvae.csv, stats.csv). One column per run. Worth is face value (J of food
before the gut's filter); a provisions cell is 960.

Sections:
  intake    food bitten at the heap, and what its carriers did with it:
            digested on the road / at home, put down by zone
  home      every put-down at home followed to its next mouth: swallowed by
            whom, carried into the store, eaten by a larva, or still lying
  store     the store's inflow and outflow, and its standing stock
  larvae    larva meals by source, larvae starved per egg, where hungry
            larvae lie against the food
"""
import csv, gzip, sys, os, statistics
from collections import defaultdict, Counter

args = [a for a in sys.argv[1:] if not a.startswith('--')]
opt = {}
for i, a in enumerate(sys.argv):
    if a.startswith('--') and i + 1 < len(sys.argv):
        opt[a[2:]] = sys.argv[i + 1]
F0 = int(opt.get('from', 100000)); F1 = int(opt.get('to', 200000))
runs = [a for a in args if os.path.isdir(a) and not a.lstrip('-').isdigit()]


def band(depth, zone):
    if zone == 'nest':
        if depth < 0:
            return 'nest off way'
        if depth < 10:
            return 'door (way 0-9)'
        if depth < 20:
            return 'nest way 10-19'
        return 'deep (way 20+)'
    return zone


def read(run):
    R = {}
    food = gzip.open(f'{run}/food.csv.gz', 'rt')
    rows = []
    try:
        for r in csv.DictReader(food):
            f = int(r['frame'])
            if f >= F1:
                break
            if f >= F0:
                rows.append(r)
    except (EOFError, ValueError, TypeError):
        pass  # a run still writing
    R['rows'] = rows
    R['digest'] = [r for r in csv.DictReader(open(f'{run}/digest.csv')) if F0 <= int(r['frame']) < F1]
    R['cells'] = [r for r in csv.DictReader(open(f'{run}/foodcells.csv')) if F0 <= int(r['frame']) < F1]
    R['larvae'] = [r for r in csv.DictReader(open(f'{run}/larvae.csv')) if F0 <= int(r['frame']) < F1]
    st = list(csv.DictReader(open(f'{run}/stats.csv')))
    def at(f):
        best = None
        for r in st:
            if int(r['frame']) <= f:
                best = r
        return best
    R['s0'], R['s1'] = at(F0), at(F1 - 1) or st[-1]
    R['stats'] = [r for r in st if F0 <= int(r['frame']) < F1]
    return R


def k(v):
    return f'{v/1000:,.0f}k'


def table(title, names, data, keys, fmt=lambda v: f'{v:,.0f}'):
    print(f'\n## {title}')
    w = max(len(x) for x in keys) + 2
    print(' ' * w + ''.join(f'{n[-18:]:>20}' for n in names))
    for key in keys:
        print(f'{key:<{w}}' + ''.join(f'{(fmt(d.get(key, 0)) if not isinstance(d.get(key, 0), str) else d.get(key)):>20}' for d in data))


names = [os.path.basename(r.rstrip('/')) for r in runs]
Rs = [read(r) for r in runs]

# ---- intake
out = []
for R in Rs:
    d = Counter()
    for r in R['rows']:
        w = float(r['worth'])
        if r['kind'] == 'swallow' and r['zone'] == 'food':
            d['bitten at the heap'] += w
        if r['kind'] == 'drop' and r['flag'] == '1':
            d['trip put down: ' + band(int(r['depth']), r['zone'])] += w
            d['trip put down, all'] += w
    for r in R['digest']:
        if r['trip'] == '1':
            d['trip digested: ' + r['zone']] += float(r['worth'])
            d['trip digested, all'] += float(r['worth'])
        else:
            d['home food digested: ' + r['zone']] += float(r['worth'])
            d['home food digested, all'] += float(r['worth'])
    for r in R['rows']:
        if r['kind'] == 'feed_crop':
            d['crop to larvae (gain)'] += float(r['worth'])
    out.append(d)
keys = sorted({x for d in out for x in d}, key=lambda s: (s.split(':')[0], s))
table(f'Food from the heap and what became of it, {F0//1000}-{F1//1000}k (face J)', names, out, keys, k)

# ---- home: follow every put-down at home to its next event at that cell
out = []
for R in Rs:
    lying = {}
    fate = Counter(); life = defaultdict(list)
    for r in R['rows']:
        pos = (r['x'], r['y'])
        kind = r['kind']
        if kind in ('drop', 'jaws_down') and r['zone'] != 'food' and r['brood'] == '0':
            lying[pos] = (int(r['frame']), kind, band(int(r['depth']), r['zone']), float(r['worth']))
        elif kind in ('swallow', 'jaws', 'eaten') and pos in lying:
            f0, k0, b0, w0 = lying.pop(pos)
            e = float(r['energy'])
            if kind == 'swallow':
                who = 'swallowed, fed ant' if e >= 1.0 else ('swallowed, lean ant' if e < 0.5 else 'swallowed, mid ant')
            elif kind == 'jaws':
                who = 'into jaws for store'
            else:
                who = 'eaten by larva' if r['brood'] == '1' else 'eaten for an egg'
            src = 'from jaws' if k0 == 'jaws_down' else 'from crop'
            fate[f'{src} -> {who}'] += w0
            fate[f'{src}, put down'] += w0
            life[src].append(int(r['frame']) - f0)
    for pos, (f0, k0, b0, w0) in lying.items():
        src = 'from jaws' if k0 == 'jaws_down' else 'from crop'
        fate[f'{src} -> still there or moved'] += w0
        fate[f'{src}, put down'] += w0
    for src, l in life.items():
        fate[f'{src}: median frames lying'] = statistics.median(l) if l else 0
    out.append(fate)
keys = sorted({x for d in out for x in d})
table('Food put down at home (crop or jaws, not at the heap), followed to its next mouth (face J)', names, out, keys, k)

# ---- where home food is put down and swallowed, by place
out = []
for R in Rs:
    d = Counter()
    for r in R['rows']:
        if r['zone'] == 'food' or r['brood'] == '1':
            continue
        b = band(int(r['depth']), r['zone'])
        if r['kind'] == 'drop':
            d[f'put down from crop: {b}'] += float(r['worth'])
        elif r['kind'] == 'swallow':
            d[f'swallowed: {b}'] += float(r['worth'])
    out.append(d)
keys = sorted({x for d in out for x in d})
table('Home food by place (face J)', names, out, keys, k)

# ---- store
out = []
for R in Rs:
    d = Counter()
    for r in R['rows']:
        w = float(r['worth'])
        if r['kind'] == 'jaws':
            d['jaws: taken (cells)'] += 1
            d['jaws: taken from ' + band(int(r['depth']), r['zone'])] += 1
        elif r['kind'] == 'jaws_down':
            d['jaws: down at store' if r['flag'] == '1' else 'jaws: let go on the way'] += 1
            if r['flag'] == '0':
                d['jaws: let go at ' + band(int(r['depth']), r['zone'])] += 1
        elif r['store'] == '1' and r['kind'] == 'swallow':
            e = float(r['energy'])
            d['store eaten by adult: ' + ('fed (>=1)' if e >= 1 else 'lean (<0.5)' if e < 0.5 else 'mid')] += 1
        elif r['store'] == '1' and r['kind'] == 'eaten':
            d['store eaten by larva' if r['brood'] == '1' else 'store eaten for an egg'] += 1
    frames = defaultdict(int)
    for c in R['cells']:
        if c['store'] == '1':
            frames[c['frame']] += 1
    nf = len({c['frame'] for c in R['larvae']}) or 1
    allf = sorted({int(c['frame']) for c in R['larvae']})
    vals = [frames.get(str(f), 0) for f in allf]
    d['store cells standing: mean'] = statistics.mean(vals) if vals else 0
    d['store cells standing: median'] = statistics.median(vals) if vals else 0
    d['store cells standing: max'] = max(vals) if vals else 0
    out.append(d)
keys = sorted({x for d in out for x in d})
table('The store (cells)', names, out, keys, lambda v: f'{v:,.1f}' if isinstance(v, float) else f'{v:,}')

# ---- larvae
out = []
for R in Rs:
    d = Counter()
    for r in R['rows']:
        if r['kind'].startswith('feed_'):
            d['meal J: ' + r['kind'][5:]] += float(r['worth'])
    s0, s1 = R['s0'], R['s1']
    eggs = int(s1['eggs_laid']) - int(s0['eggs_laid'])
    starved = int(s1['larvae_starved']) - int(s0['larvae_starved'])
    d['eggs laid'] = eggs
    d['larvae starved'] = starved
    d['larvae starved per egg'] = starved / eggs if eggs else 0
    L = R['larvae']
    n = len(L) or 1
    d['larva-censuses'] = len(L)
    d['share < 0.25 of target (lean)'] = sum(float(r['share']) < 0.25 for r in L) / n
    d['with food beside'] = sum(int(r['food8']) > 0 for r in L) / n
    d['with an adult beside'] = sum(int(r['adults8']) > 0 for r in L) / n
    d['with a rich adult (>=1) beside'] = sum(float(r['best_adult']) >= 1 for r in L) / n
    lean = [r for r in L if float(r['share']) < 0.25]
    m = len(lean) or 1
    d['lean: food beside'] = sum(int(r['food8']) > 0 for r in lean) / m
    d['lean: rich adult beside'] = sum(float(r['best_adult']) >= 1 for r in lean) / m
    bands = Counter(band(int(r['depth']), r['zone']) for r in L)
    for b, c in bands.items():
        d['larvae at: ' + b] = c / n
    out.append(d)
keys = sorted({x for d in out for x in d})
table('Larvae', names, out, keys, lambda v: f'{v:,.3f}' if isinstance(v, float) and v < 10 else f'{v:,.0f}')
