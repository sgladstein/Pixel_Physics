#!/usr/bin/env python3
"""SCRATCH (score-gaming watchdog, 2026-10-28): does selection pay the gap between a case and its twin?

Reads twins.py tables. For each gap (the part of a voted score its ground-truth twin does not back), per arm:
  donors   rounds where the donors' median gap beats the other surviving colonies' (sign test over rounds)
  trend    population median gap, first 4 rounds against last 4
  agree    Spearman, within round, between the voted number and its twin (low = the case ranks on the gap)
usage: gap.py TABLE.csv [TABLE.csv ...]
"""
import csv, sys, statistics as st, math, collections


def f(x):
    try:
        return float(x)
    except (TypeError, ValueError):
        return None


def sign_p(k, n):
    if n == 0:
        return 1.0
    t = sum(math.comb(n, i) for i in range(0, min(k, n - k) + 1)) / 2 ** n
    return min(1.0, 2 * t)


def rank(v):
    o = sorted(range(len(v)), key=lambda i: v[i])
    r = [0.0] * len(v)
    i = 0
    while i < len(o):
        j = i
        while j + 1 < len(o) and v[o[j + 1]] == v[o[i]]:
            j += 1
        for t in range(i, j + 1):
            r[o[t]] = (i + j) / 2
        i = j + 1
    return r


def spearman(a, b):
    if len(a) < 4:
        return None
    ra, rb = rank(a), rank(b)
    ma, mb = st.mean(ra), st.mean(rb)
    sa = math.sqrt(sum((x - ma) ** 2 for x in ra)); sb = math.sqrt(sum((x - mb) ** 2 for x in rb))
    return None if sa == 0 or sb == 0 else sum((x - ma) * (y - mb) for x, y in zip(ra, rb)) / (sa * sb)


# name: (voted column, twin column, gap(row) -> float or None, higher gap = more score without outcome)
GAPS = {
    'home8 re-carried share': ('home8', 'home8_source',
                               lambda r: f(r['home8_recarry']) / f(r['home8_traced']) if f(r['home8_traced']) else None),
    'home8 not-from-source share': ('home8', 'home8_source',
                                    lambda r: 1 - f(r['home8_source']) / f(r['home8_traced']) if f(r['home8_traced']) else None),
    'nest share that is food': ('nest', 'nest_open',
                                lambda r: 1 - f(r['nest_open']) / f(r['nest_all']) if f(r['nest_all']) else None),
    'crater hidden by fill (cells)': ('crater', 'crater_full',
                                      lambda r: f(r['crater_full']) - f(r['crater_empty']) if f(r['crater_full']) is not None else None),
    'shape points from food (shape - open)': ('shape', 'shape_open',
                                              lambda r: f(r['shape']) - f(r['shape_open']) if f(r['shape_open']) is not None else None),
    'starved per egg laid': ('starved', 'laid',
                             lambda r: f(r['starved']) / (f(r['laid']) + 30) if f(r['laid']) is not None else None),
    'eggs laid (fewer eggs, fewer starve)': ('starved', 'laid', lambda r: -f(r['laid']) if f(r['laid']) is not None else None),
    'scale per open cell x1000 (small need)': ('scale', 'nest_open',
                                               lambda r: 1000 * f(r['scale']) / f(r['nest_open']) if f(r['nest_open']) else None),
}

rows = [r for p in sys.argv[1:] for r in csv.DictReader(open(p))]
arms = sorted({r['arm'] for r in rows})
for name, (vote, twin, g) in GAPS.items():
    print(f'\n== {name}   (voted {vote}, twin {twin})')
    for a in arms:
        by = collections.defaultdict(list)
        for r in rows:
            if r['arm'] == a and r['rank'] != '' and int(r['alive'] or 0) > 0:
                by[int(r['round'])].append(r)
        wins = n = 0
        med = {}
        agree = []
        for rd, rs in sorted(by.items()):
            vals = [(g(r), int(r['children']) > 0) for r in rs]
            vals = [(x, d) for x, d in vals if x is not None]
            if not vals:
                continue
            med[rd] = st.median(x for x, _ in vals)
            dn = [x for x, d in vals if d]; ot = [x for x, d in vals if not d]
            if dn and ot and st.median(dn) != st.median(ot):
                n += 1; wins += st.median(dn) > st.median(ot)
            pr = [(f(r[vote]), f(r[twin])) for r in rs if f(r[vote]) is not None and f(r[twin]) is not None]
            s = spearman([x for x, _ in pr], [y for _, y in pr])
            if s is not None:
                agree.append(s)
        ks = sorted(med)
        early = st.median(med[k] for k in ks[:4]) if len(ks) >= 4 else None
        late = st.median(med[k] for k in ks[-4:]) if len(ks) >= 4 else None
        tr = f'trend r{ks[0]}-{ks[3]} {early:.3g} -> r{ks[-4]}-{ks[-1]} {late:.3g}' if early is not None else \
            f"r{ks[0] if ks else '-'} median {med[ks[0]]:.3g}" if ks else 'no data'
        print(f'  {a:3} donor gap higher in {wins}/{n} rounds (p={sign_p(wins, n):.3g}); {tr}; '
              f'agree {st.mean(agree):.2f}' if agree else f'  {a:3} {wins}/{n} {tr}')
