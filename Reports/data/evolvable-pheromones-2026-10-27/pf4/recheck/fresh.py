#!/usr/bin/env python3
"""'Fresh' deliveries: a delivered cell counts only if the pickup that started it took food no ant had put down
before (first removal from the source). Per ant, pickups queue in order; each deliver pops the oldest."""
import csv, sys, json, os, glob, collections
def fresh(d):
    dropped = set(); q = collections.defaultdict(collections.deque)
    out = dict(deliver=0, fresh=0, fresh_kj=0.0, src_taken=0)
    for r in csv.reader(open(f'{d}/events.csv')):
        if len(r) < 6 or r[0] == 'frame': continue
        k = r[2]
        if k == 'field':
            p = (int(r[3]), int(r[4])); src = p not in dropped; dropped.discard(p)
            out['src_taken'] += src; q[r[1]].append(src)
        elif k in ('churn', 'fdrop'):
            dropped.add((int(r[3]), int(r[4])))
            if q[r[1]]: q[r[1]].popleft()
        elif k == 'deliver':
            dropped.add((int(r[3]), int(r[4]))); out['deliver'] += 1
            src = q[r[1]].popleft() if q[r[1]] else False
            out['fresh'] += src; out['fresh_kj'] += float(r[5]) / 1000 * src
    return out
if __name__ == '__main__':
    R = sys.argv[1]; res = json.load(open(f'{R}/an.json'))
    for arm, runs in res.items():
        for s in runs:
            runs[s].update(fresh(f'{R}/{arm}/{s}'))
    json.dump(res, open(f'{R}/an.json', 'w'), indent=1)
    import statistics as st
    for arm, runs in res.items():
        print(arm, *(f"{k}={st.median(r[k] for r in runs.values()):.0f}" for k in ('deliver', 'fresh', 'fresh_kj', 'src_taken', 'f_heap')))
