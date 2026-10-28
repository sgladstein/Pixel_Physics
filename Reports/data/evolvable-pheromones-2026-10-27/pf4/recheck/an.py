#!/usr/bin/env python3
"""Per replay: food delivered home, and where every field pickup happened (at the heap vs near the nest)."""
import csv, glob, os, sys, json, statistics as st, math
R = sys.argv[1]
NEST = 128
def one(d):
    heaps = [(int(r['frame']), int(r['x'])) for r in csv.DictReader(open(f'{d}/patches.csv'))]
    def heap_at(f):
        x = heaps[0][1]
        for fr, hx in heaps:
            if fr <= f: x = hx
        return x
    o = dict(deliver=0, deliver_kj=0.0, churn=0, fdrop=0, fdrop_near=0, field=0, f_heap=0, f_near=0, f_other=0,
             heap_kj=0.0, near_kj=0.0)
    for r in csv.reader(open(f'{d}/events.csv')):
        if len(r) < 6 or r[0] == 'frame': continue
        k = r[2]
        if k not in ('deliver', 'churn', 'fdrop', 'field'): continue
        f, x, v = int(r[0]), int(r[3]), float(r[5])
        if k == 'deliver': o['deliver'] += 1; o['deliver_kj'] += v / 1000
        elif k == 'churn': o['churn'] += 1
        elif k == 'fdrop':
            o['fdrop'] += 1; o['fdrop_near'] += abs(x - NEST) <= 30
        else:
            o['field'] += 1
            if abs(x - heap_at(f)) <= 15: o['f_heap'] += 1; o['heap_kj'] += v / 1000
            elif abs(x - NEST) <= 30: o['f_near'] += 1; o['near_kj'] += v / 1000
            else: o['f_other'] += 1
    last = list(csv.DictReader(open(f'{d}/nest.csv')))[-1]
    o['home'] = int(last['food_home'])
    s = list(csv.DictReader(open(f'{d}/stock.csv')))[-1]
    o['alive'] = int(s['alive'])
    o['kj'] = sum(float(v) for k, v in list(csv.DictReader(open(f'{d}/larder.csv')))[-1].items() if k.endswith('_kj'))
    return o
res = {}
for arm in sorted(os.listdir(R)):
    if not os.path.isdir(f'{R}/{arm}'): continue
    for d in sorted(glob.glob(f'{R}/{arm}/s*')):
        if os.path.exists(f'{d}/fin') and os.path.isdir(d):
            res.setdefault(arm, {})[os.path.basename(d)] = one(d)
json.dump(res, open(f'{R}/an.json', 'w'), indent=1)
keys = ['home', 'deliver_kj', 'churn', 'fdrop', 'fdrop_near', 'f_heap', 'f_near', 'f_other', 'heap_kj', 'near_kj', 'kj', 'alive']
print('arm n ' + ' '.join(keys) + '  (medians)')
for arm, runs in res.items():
    print(arm, len(runs), ' '.join(f"{st.median(r[k] for r in runs.values()):.0f}" for k in keys))
