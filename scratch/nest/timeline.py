"""SCRATCH (nest lane): nest measures every 25k frames for every world of a baseline.py arm -> ARM/timeline.csv
usage: timeline.py ARMDIR"""
import csv, glob, os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from baseline import measure  # noqa: E402

D = sys.argv[1]
rows = []
for o in sorted(glob.glob(f'{D}/w*/')) + sorted(glob.glob(f'{D}/lab_*/')):
    o = o.rstrip('/')
    nest = list(csv.DictReader(open(f'{o}/nest.csv'))) if os.path.exists(f'{o}/nest.csv') else None
    st = {}
    if os.path.exists(f'{o}/log.txt'):
        for l in open(f'{o}/log.txt'):
            if l.startswith('NESTAT'):
                kv = dict(x.split('=', 1) for x in l.split()[1:])
                st[int(kv['f'])] = int(kv['alive'])
    for p in sorted(glob.glob(f'{o}/nestmap_f*.txt')):
        f = int(os.path.basename(p)[9:16])
        alive = st.get(f) if not nest else int([x for x in nest if int(x['frame']) <= f][-1]['alive'])
        rows.append(measure(o, p, p.replace('nestmap_', 'occupants_'), dict(run=os.path.basename(o), frame=f, alive=alive)))
with open(f'{D}/timeline.csv', 'w') as fh:
    w = csv.DictWriter(fh, list(rows[0]))
    w.writeheader()
    w.writerows(rows)
print(len(rows), 'rows ->', f'{D}/timeline.csv')
