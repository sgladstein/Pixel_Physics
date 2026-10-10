#!/usr/bin/env python3
"""summary.py RUN [RUN...] [--from F] [--to F] -- one line per run of the food trace's headline numbers.

Columns (window 100-200k unless given): ants (mean of stats.csv), adults
starved by place (events.txt, needs census=1: surface+food / mound / nest),
eggs, larvae starved and per egg, food bitten at the heap (kJ face), the
share of it its carriers digested before putting it down, trip food put
down by place (kJ: mound top / mound tunnels / door 0-9 / nest deeper),
loads carried into the store (cells), the store's standing cells (mean of
the 1,000-frame censuses), what the deep nest (way 20+) digested (kJ, all
adults / nest workers), and larva meals by source (kJ gained: ate off the
floor / a nestmate's bank / a brain's share / a carrier's crop).
"""
import csv, gzip, sys, re, collections, statistics, os

args = [a for a in sys.argv[1:] if not a.startswith('--')]
opt = {sys.argv[i][2:]: sys.argv[i + 1] for i in range(len(sys.argv) - 1) if sys.argv[i].startswith('--')}
F0 = int(opt.get('from', 100000)); F1 = int(opt.get('to', 200000))
runs = [a for a in args if os.path.isdir(a)]

hdr = ['run', 'ants', 'starved s/m/n', 'eggs', 'larv.starved', '/egg', 'heap kJ', 'carrier ate%',
       'put down top/in/door/deep kJ', 'store loads', 'store cells', 'deep ate all/nw kJ', 'larva meals ate/bank/share/crop kJ']
print('| ' + ' | '.join(hdr) + ' |')
print('|' + '---|' * len(hdr))
for run in runs:
    st = list(csv.DictReader(open(f'{run}/stats.csv')))
    win = [r for r in st if F0 <= int(r['frame']) < F1]
    s0 = next(r for r in reversed(st) if int(r['frame']) <= F0)
    s1 = next(r for r in reversed(st) if int(r['frame']) <= F1 - 1)
    ants = statistics.mean(int(r['ants']) for r in win)
    eggs = int(s1['eggs_laid']) - int(s0['eggs_laid'])
    ls = int(s1['larvae_starved']) - int(s0['larvae_starved'])
    sz = collections.Counter()
    for l in open(f'{run}/events.txt'):
        if ' DIED ' in l and 'STARVED' in l and F0 <= int(l.split()[0]) < F1:
            z = re.search(r'zone=(\S+)', l).group(1)
            sz['s' if z in ('surface', 'food') else 'm' if z.startswith('mound') else 'n'] += 1
    heap = 0.0; put = collections.Counter(); loads = 0; meals = collections.Counter()
    for r in csv.DictReader(gzip.open(f'{run}/food.csv.gz', 'rt')):
        f = int(r['frame'])
        if f >= F1:
            break
        if f < F0:
            continue
        k = r['kind']
        if k == 'swallow' and r['zone'] == 'food':
            heap += float(r['worth'])
        elif k == 'drop' and r['flag'] == '1':
            z = r['zone']
            b = 'top' if z == 'mound_top' else 'in' if z == 'mound_in' else ('door' if int(r['depth']) < 10 else 'deep') if z == 'nest' else 'other'
            put[b] += float(r['worth'])
        elif k == 'jaws_down' and r['flag'] == '1':
            loads += 1
        elif k.startswith('feed_'):
            meals[k[5:]] += float(r['worth'])
    tripdig = 0.0; deep = 0.0; deepnw = 0.0
    for r in csv.DictReader(open(f'{run}/digest.csv')):
        if not F0 <= int(r['frame']) < F1:
            continue
        w = float(r['worth'])
        if r['trip'] == '1':
            tripdig += w
        if r['zone'] == 'nest' and r['depth'] == '2':
            deep += w
            if r['worker'] == '1':
                deepnw += w
    cells = collections.Counter()
    frames = set()
    for r in csv.DictReader(open(f'{run}/foodcells.csv')):
        f = int(r['frame'])
        if F0 <= f < F1:
            frames.add(f)
            if r['store'] == '1':
                cells[f] += 1
    store = statistics.mean(cells.get(f, 0) for f in frames) if frames else 0
    k = lambda v: f'{v / 1000:,.0f}'
    print(f"| {os.path.basename(run.rstrip('/'))} | {ants:.0f} | {sz['s']}/{sz['m']}/{sz['n']} | {eggs} | {ls} | {ls / max(eggs, 1):.2f} | {k(heap)} | "
          f"{100 * tripdig / max(heap, 1):.0f}% | {k(put['top'])}/{k(put['in'])}/{k(put['door'])}/{k(put['deep'])} | {loads} | {store:.0f} | "
          f"{k(deep)}/{k(deepnw)} | {k(meals['ate'])}/{k(meals['bank'])}/{k(meals['share'])}/{k(meals['crop'])} |")
