#!/usr/bin/env python3
"""The drop-and-repick loop, counted from events.csv (plain or .gz). Every field pickup is tagged by what last put
food on that exact cell: a delivery ('redeliver' if it is then delivered again), a churn or field drop, or nothing an
ant logged ('source'). Each ant's pickups queue in order; each deliver/churn/fdrop pops its oldest. Cells that slide
after a drop look like 'source', so the loop count is a LOWER bound.
usage: loop.py RUNDIR... -> one line per run, and medians."""
import csv, gzip, sys, collections, statistics as st, json
def loop(d):
    fn = f'{d}/events.csv'
    try: fh = open(fn)
    except OSError: fh = gzip.open(fn + '.gz', 'rt')
    last = {}; q = collections.defaultdict(collections.deque)
    o = collections.Counter()
    for r in csv.reader(fh):
        if len(r) < 6 or r[0] == 'frame': continue
        k = r[2]
        if k == 'field':
            tag = last.pop((r[3], r[4]), 'source'); q[r[1]].append(tag); o['pick_' + tag] += 1
        elif k in ('deliver', 'churn', 'fdrop'):
            last[(r[3], r[4])] = k
            tag = q[r[1]].popleft() if q[r[1]] else 'none'
            if k == 'deliver':
                o['deliver'] += 1; o['deliver_from_' + tag] += 1
    return o
if __name__ == '__main__':
    rows = {d: loop(d) for d in sys.argv[1:]}
    keys = ['deliver', 'deliver_from_source', 'deliver_from_deliver', 'deliver_from_churn', 'deliver_from_fdrop', 'pick_deliver']
    for d, o in rows.items():
        print(d.rsplit('/', 2)[-2:], ' '.join(f'{k[13:] or k}={o[k]}' for k in keys), f"loop_share={o['deliver_from_deliver']/max(1,o['deliver']):.2f}")
    print('MEDIAN', ' '.join(f'{k}={st.median(o[k] for o in rows.values()):.0f}' for k in keys),
          f"loop_share={st.median(o['deliver_from_deliver']/max(1,o['deliver']) for o in rows.values()):.2f}")
