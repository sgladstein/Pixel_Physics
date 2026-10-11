#!/usr/bin/env python3
"""gate.py RUN... -- the kill rule's numbers per run, to the last frame reached: ants at 100/150/200/300k, the lowest count
after 100k, starved 20k-end, trip deliveries 50-100k and 100k-end, births 100k-end, door shut maps."""
import csv, re, sys, glob
sys.path.insert(0, '/mnt/project-files/deep-trace/tools')
import doorseal as ds
for run in sys.argv[1:]:
    r = list(csv.reader(open(run + '/stats.csv'))); h = r[0]
    s = {int(x[0]): dict(zip(h, x)) for x in r[1:] if len(x) == len(h)}
    last = max(f for f in s if f % 1000 == 0)
    ants = {f: int(s[f]['ants']) for f in s if f % 1000 == 0}
    low = min(((v, f) for f, v in ants.items() if f >= 100000), default=(None, None))
    d = lambda k, a, b: int(float(s[b][k]) - float(s[a][k])) if a in s and b in s else None
    shut = n = 0
    for p in glob.glob(run + '/map_f*.txt'):
        f = int(re.search(r'map_f(\d+)', p).group(1))
        if 6000 <= f <= last and f % 1000 == 0:
            n += 1; shut += (not ds.measure(p)[0])
    print(f"{run.split('/')[-1]:8s} to {last//1000}k: ants 100k {ants.get(100000)} 150k {ants.get(150000)} 200k {ants.get(200000)} 300k {ants.get(300000)}; "
          f"low {low[0]}@{(low[1] or 0)//1000}k; starved {d('died_starved', 20000, last)}; trips 50-100k {d('trip_deliveries', 50000, min(100000, last))} "
          f"100k-end {d('trip_deliveries', 100000, last) if last > 100000 else '-'}; births 100k-end {d('births', 100000, last) if last > 100000 else '-'}; shut {shut}/{n}")
