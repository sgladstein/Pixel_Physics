import json, sys
rows=[]
for p in ('results.jsonl',):
    for l in open(p):
        if l.startswith('{'):
            rows.append(json.loads(l))
seen=set(); R=[]
for r in rows:
    k=(r['seed'],r['frame'])
    if k in seen: continue
    seen.add(k); R.append(r)
R.sort(key=lambda r:(r['seed'],r['frame']))
print('maps', len(R))
for r in R:
    print(r['seed'], r['frame'], 'nest cells', r['nest_cells'], 'unreach', r['nest_unreachable'], 'ants nest/mound', r['ants_in_nest'], r['ants_in_mound'], 'brood', r['brood'], r.get('note',''))
names=['air_sealed','air_p02','air_plane','larva_L3','larva_L10']
keys=['flat_1pct','flat_1pct_room','flat_1pct_tunnel','flat_0p1pct','contrast_p50','out','agree','wallward','tops','range_max_over_min','step_under_1_65536','step_under_1_256']
import statistics as st
for n in names:
    print('\n==', n)
    for k in keys:
        vals=[r['fields'][n].get(k) for r in R if r['fields'] and n in r['fields']]
        vals=[v for v in vals if v is not None]
        if not vals: print(f'  {k:22s} -'); continue
        print(f'  {k:22s} min {min(vals):8.4g}  med {st.median(vals):8.4g}  max {max(vals):8.4g}  (n={len(vals)})')
    tops=[r['fields'][n]['top3'][0] for r in R if r['fields'] and n in r['fields']]
    print('  top1 share/brood/ants/row_below:', [(round(t['share'],2), t['brood'], t['ants'], t['row_below_ground']) for t in tops])
