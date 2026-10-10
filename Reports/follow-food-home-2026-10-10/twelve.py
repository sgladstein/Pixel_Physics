#!/usr/bin/env python3
"""twelve.py DIR [--from F] [--to F] [--seeds] -- the default flip's 12-seed table.

DIR holds runs named BED-ARM-sSEED (as `deeptrace ... out=DIR/BED-ARM-sSEED`,
each with stats.csv, events.txt from census=1, and colony.csv). For each bed
and arm, one row per seed and a summary: adults alive (mean over the window),
adults starved (events.txt), larvae starved per egg (stats.csv), the share of
adult rows in the dug nest (`nest` zone of colony.csv), and the share of nest
time spent in stays over 5,000 frames. Then each arm against main and against
`stack`, seed by seed: how many seeds it is higher on.

Runs still writing (no `rc=` line in DIR/NAME.log) are skipped and counted.
"""
import csv, os, re, sys, collections, statistics

args = [a for i, a in enumerate(sys.argv[1:], 1) if not a.startswith('--') and not sys.argv[i - 1] in ('--from', '--to')]
opt = {sys.argv[i][2:]: sys.argv[i + 1] for i in range(len(sys.argv) - 1) if sys.argv[i].startswith('--')}
F0 = int(opt.get('from', 100000)); F1 = int(opt.get('to', 200000))
D = args[0]


def one(run):
    st = [r for r in csv.DictReader(open(f'{run}/stats.csv'))]
    win = [r for r in st if F0 <= int(r['frame']) < F1]
    s0 = next(r for r in reversed(st) if int(r['frame']) <= F0)
    s1 = next(r for r in reversed(st) if int(r['frame']) <= F1 - 1)
    eggs = int(s1['eggs_laid']) - int(s0['eggs_laid'])
    ls = int(s1['larvae_starved']) - int(s0['larvae_starved'])
    starved = 0
    for l in open(f'{run}/events.txt'):
        if ' DIED ' in l and 'STARVED' in l and F0 <= int(l.split()[0]) < F1:
            starved += 1
    z = collections.Counter(); seq = collections.defaultdict(list); fr = set()
    for r in csv.DictReader(open(f'{run}/colony.csv')):
        f = int(r['frame'])
        if F0 <= f < F1:
            z[r['zone']] += 1; seq[r['id']].append(r['zone'] == 'nest'); fr.add(f)
    fs = sorted(fr)
    step = min(b - a for a, b in zip(fs, fs[1:])) if len(fs) > 1 else 1000
    long_ = tot = 0
    for s in seq.values():
        run_ = 0
        for inn in s + [False]:
            if inn:
                run_ += 1
            else:
                tot += run_
                if run_ * step > 5000:
                    long_ += run_
                run_ = 0
    n = sum(z.values()) or 1
    return dict(ants=statistics.mean(int(r['ants']) for r in win) if win else 0, starved=starved,
                lpe=ls / max(eggs, 1), nest=100 * z['nest'] / n, stays=100 * long_ / max(tot, 1),
                alive=int(s1['ants']) > 0)


R = collections.defaultdict(dict); skipped = 0
for name in sorted(os.listdir(D)):
    m = re.fullmatch(r'(\w+)-(\w+)-s(\d+)', name)
    if not m or not os.path.isdir(f'{D}/{name}'):
        continue
    log = f'{D}/{name}.log'
    if not os.path.exists(log) or 'rc=0' not in open(log).read():
        skipped += 1
        continue
    R[(m.group(1), m.group(2))][int(m.group(3))] = one(f'{D}/{name}')

print(f'window {F0//1000}-{F1//1000}k; runs still writing or failed: {skipped}')
beds = sorted({b for b, _ in R})
for bed in beds:
    arms = sorted({a for b, a in R if b == bed}, key=lambda a: (a != 'main', a))
    print(f'\n## {bed}\n')
    print('| arm | seeds | ants (median, range) | adults starved | larvae starved / egg | in dug nest % | nest time in stays >5k % | colonies dead |')
    print('|---|---|---|---|---|---|---|---|')
    for arm in arms:
        rows = R[(bed, arm)]
        def rng(k, f='{:.0f}'):
            v = [r[k] for r in rows.values()]
            return f'{f.format(statistics.median(v))} ({f.format(min(v))}-{f.format(max(v))})'
        print(f"| {arm} | {len(rows)} | {rng('ants')} | {rng('starved')} | {rng('lpe', '{:.2f}')} | {rng('nest', '{:.1f}')} | {rng('stays', '{:.1f}')} | {sum(not r['alive'] for r in rows.values())} |")
    for ref in ('main', 'stack'):
        if (bed, ref) not in R:
            continue
        for arm in arms:
            if arm == ref:
                continue
            a, b = R[(bed, arm)], R[(bed, ref)]
            seeds = sorted(set(a) & set(b))
            if not seeds:
                continue
            hi = lambda k: sum(a[s][k] > b[s][k] for s in seeds)
            print(f'- {arm} vs {ref}, {len(seeds)} paired seeds: ants higher on {hi("ants")}, adults starved higher on {hi("starved")}, '
                  f'larvae/egg higher on {hi("lpe")}, in-nest higher on {hi("nest")}')

# ---- per seed (`--seeds`): every arm's numbers side by side, seed by seed
if '--seeds' in sys.argv:
    for bed in beds:
        arms = sorted({a for b, a in R if b == bed}, key=lambda a: (a != 'main', a))
        print(f'\n### {bed}, per seed ({" / ".join(arms)})\n')
        print('| seed | ants | adults starved | larvae starved / egg | in dug nest % |')
        print('|---|---|---|---|---|')
        seeds = sorted(set().union(*(R[(bed, a)].keys() for a in arms)))
        for s in seeds:
            cell = lambda k, f: ' / '.join(f.format(R[(bed, a)][s][k]) if s in R[(bed, a)] else '-' for a in arms)
            print(f"| {s} | {cell('ants', '{:.0f}')} | {cell('starved', '{}')} | {cell('lpe', '{:.2f}')} | {cell('nest', '{:.1f}')} |")
