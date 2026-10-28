#!/usr/bin/env python3
"""SCRATCH (score-gaming watchdog, 2026-10-28): inside one inrun round, does the ant-level pay reward re-carrying?

Per ant, from events.csv(.gz): home deliveries (within 8 of the door) split by what began the trip (loop.py's tags:
'source' = a cell no ant had put down, 'deliver' = a cell another delivery had put down: a re-carry), plus cuts.
Per ant, from births.csv: kids (selected births, parent != 0). From final/index.csv: the in-run score of the ants
alive at the end, and which 30 the carry keeps (colsel's carried(): top 30 by score).
Prints per run: the carried ants' re-carry share against the other finalists', and among ants that delivered, the
re-carry share of parents against non-parents.
usage: antpay.py RUNDIR... (colsel c<k>/r<r> folders with events kept)
"""
import csv, gzip, os, sys, collections, statistics as st


def per_ant(d):
    fn = f'{d}/events.csv'
    fh = open(fn) if os.path.exists(fn) else gzip.open(fn + '.gz', 'rt')
    last, q = {}, collections.defaultdict(collections.deque)
    a = collections.defaultdict(collections.Counter)
    for r in csv.reader(fh):
        if len(r) < 6 or r[0] == 'frame':
            continue
        k = r[2]
        if k == 'field':
            q[r[1]].append(last.pop((r[3], r[4]), 'source'))
        elif k in ('deliver', 'churn', 'fdrop'):
            last[(r[3], r[4])] = k
            tag = q[r[1]].popleft() if q[r[1]] else 'none'
            if k == 'deliver' and abs(int(r[3]) - 128) <= 8:
                a[r[1]]['home'] += 1
                a[r[1]][tag] += 1
        elif k == 'dig':
            a[r[1]]['dig'] += 1
    return a


def share(cs):
    h = sum(c['home'] for c in cs)
    return sum(c['deliver'] for c in cs) / h if h else float('nan')


if __name__ == '__main__':
    print('run,carried_recarry,other_final_recarry,parents_recarry,nonparents_recarry,n_final,n_parents')
    for d in sys.argv[1:]:
        a = per_ant(d)
        kids = collections.Counter()
        for b in csv.DictReader(open(f'{d}/births.csv')):
            if b['parent'] != '0' and b['kind'].startswith('hatch'):
                kids[b['parent']] += 1
        fin = sorted(csv.DictReader(open(f'{d}/final/index.csv')), key=lambda x: int(x['id']))
        fin.sort(key=lambda x: (-float(x['score']), int(x['id'])))
        car, oth = [a[x['id']] for x in fin[:30]], [a[x['id']] for x in fin[30:]]
        dl = [i for i, c in a.items() if c['home'] > 0]
        par, non = [a[i] for i in dl if kids[i] > 0], [a[i] for i in dl if kids[i] == 0]
        print(f'{d},{share(car):.3f},{share(oth):.3f},{share(par):.3f},{share(non):.3f},{len(fin)},{len(par)}')
