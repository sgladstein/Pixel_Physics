#!/usr/bin/env python3
"""data.py OUT.json -- per run: every 2,000 frames from 50k, ants alive, food carried in over the last 5,000 frames
(trip deliveries), ants starved over the last 5,000 frames, ants deeper than 10 rows (colony.csv); plus the gate
numbers per run (lowest ants after 100k, starved 20k-end, trip deliveries 100k-end, deep mean 100k-300k)."""
import csv, json, sys, os, collections
RUNS = {'all': '/home/claude/runs/fix/F-s{}', 'p1': '/home/claude/runs/fix/P1-s{}', 'p2': '/home/claude/runs/fix/P2-s{}',
        'p3': '/home/claude/runs/fix/P3-s{}', 'p4': '/home/claude/runs/fix/P4-s{}', 'slice1': '/home/claude/runs/walk/W-s{}',
        'stack': '/home/claude/runs/walk/S-s{}', 'noants': '/home/claude/runs/walk/O-s{}'}
out, gate = {}, {}
for arm, pat in RUNS.items():
    for s in (1, 2, 3, 4):
        run = pat.format(s)
        if not os.path.exists(run + '/stats.csv'):
            continue
        r = list(csv.reader(open(run + '/stats.csv'))); h = r[0]
        st = {int(x[0]): dict(zip(h, x)) for x in r[1:] if len(x) == len(h)}
        last = max(f for f in st if f % 1000 == 0)
        deep = collections.Counter(); rows_at = collections.Counter()
        with open(run + '/colony.csv') as fh:
            cr = csv.reader(fh); ch = next(cr); ix = {k: j for j, k in enumerate(ch)}
            for row in cr:
                if len(row) != len(ch): continue
                f = int(row[0])
                if f >= 48000: rows_at[f] += 1
                if f >= 48000 and int(row[ix['hy']]) - 160 > 10:
                    deep[f] += 1
        val = lambda f, k: float(st[f][k]) if f in st else None
        rows = []
        for f in range(50000, last + 1, 2000):
            if f not in st or (f - 5000) not in st:
                continue
            rows.append({'f': f, 'ants': int(st[f]['ants']),
                         'food': int(val(f, 'trip_deliveries') - val(f - 5000, 'trip_deliveries')),
                         'starved': int(val(f, 'died_starved') - val(f - 5000, 'died_starved')),
                         'deep': deep.get(f, 0)})
        ants = {f: int(st[f]['ants']) for f in st if f % 1000 == 0}
        low = min(((v, f) for f, v in ants.items() if f >= 100000), default=(0, 0))
        dfs = [f for f in range(100000, 300000, 1000) if rows_at.get(f, 0) > 0]
        out[f'{arm}-{s}'] = {'done': last >= 300000, 'rows': rows}
        gate[f'{arm}-{s}'] = {'last': last, 'end': ants.get(last, 0), 'low': low[0], 'lowf': low[1],
                              'starved': int(val(last, 'died_starved') - val(20000, 'died_starved')),
                              'food': int(val(last, 'trip_deliveries') - val(100000, 'trip_deliveries')) if last > 100000 else None,
                              'births': int(val(last, 'births') - val(100000, 'births')) if last > 100000 else None,
                              'deep': round(sum(deep.get(f, 0) for f in dfs) / max(len(dfs), 1), 1)}
json.dump({'data': out, 'gate': gate}, open(sys.argv[1], 'w'), separators=(',', ':'))
print('runs', len(out), 'bytes', os.path.getsize(sys.argv[1]))
for k in sorted(gate): print(k, gate[k])
