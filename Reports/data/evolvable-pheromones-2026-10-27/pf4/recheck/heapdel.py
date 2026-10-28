"""Deliveries whose load began with a pickup AT THE HEAP (within 15 columns of the current patch), any cell history."""
import csv, collections, sys
def heapdel(d):
    heaps = [(int(r['frame']), int(r['x'])) for r in csv.DictReader(open(f'{d}/patches.csv'))]
    q = collections.defaultdict(collections.deque); n = collections.Counter()
    for r in csv.reader(open(f'{d}/events.csv')):
        if len(r) < 6 or r[0] == 'frame': continue
        k = r[2]
        if k == 'field':
            f = int(r[0]); hx = [h for fr, h in heaps if fr <= f][-1]
            q[r[1]].append(abs(int(r[3]) - hx) <= 15)
        elif k in ('deliver', 'churn', 'fdrop'):
            t = q[r[1]].popleft() if q[r[1]] else False
            if k == 'deliver': n['deliver'] += 1; n['from_heap'] += t
    return n
