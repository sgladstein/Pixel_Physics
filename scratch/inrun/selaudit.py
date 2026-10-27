"""SCRATCH (selection review, 2026-10-27): what a colony-selection batch's own colsel.csv says about its selection.

Reads only what colsel.py already writes, so it can audit any finished batch with no new run. Built for the review in
Reports/brain-training/research/selection-review-2026-10-27.md; every table there comes from this script.

usage: selaudit.py ARM_DIR [ARM_DIR ...]          each ARM_DIR holds colsel.csv (and cg.csv, and optionally rN/cM/)
       selaudit.py --pair ARM_DIR ARM_DIR           the world effect: two arms run on the same seeds (colony c, round r
                                                    plays world c*1000+r in every arm), so their within-round colony
                                                    rankings agree only as far as the world, not the colony, sets them

Per arm it prints:
  donors     distinct donor colonies per round (refounded_from), and how many colonies passed the dig gate.
  retest     repeatability: a colony kept after round r-1 and after round r runs its unmutated carry (elite=1) in two
             fresh worlds, r and r+1; Spearman of each case across those colonies, averaged over rounds, with a 90%
             bootstrap interval. `ranksum` is the survival order itself. Needs elite=1 to mean "the same colony twice".
  weight     each case's share of the survival rank-sum, cov(rank_k, sum) / var(sum): the weight it really carries.
  pressure   standardised selection differential (kept half minus all, in sd; donors minus all), signed + = better.
  child      a child and its donor in the SAME round: both start from the donor's carry, the donor (kept) runs with no
             mutation, the child mutates. Counts of pairs where the child is better / worse, and a two-sided sign test.
             This is what one round of mutation plus in-run selection does to each case.
  luck       share of the top half taken by children (colonies that mutated last round). 50% = survival cannot tell a
             child from a kept colony.
  lineage    from rN/cM/index.csv (the last round's per-ant files): founder lines left per colony, and how many rounds
             back every ant in every colony shares one ancestor ant (the carry appends each round's ant id to `family`).

Positive control for `retest` (built in): the same statistic over two arms' kept colonies pooled (`--pair`), where the
arms differ genuinely (e.g. the instinct arm against std_pool) -- it must come out high. Negative control: an arm run
with kpm=0 kevents=0 from a converged pool (drift1's lnomut) must come out near 0.
"""
import csv, math, os, random, statistics as st, sys
from math import comb

CASES = [('home8', 1), ('starved', -1), ('nestcuts_late', 1), ('nest', 1), ('spoil', -1), ('crater', -1),
         ('shape', 1), ('scale', 1), ('lab', 1), ('lab2_lab_herbs', 1)]


def load(path):
    out = {}
    if not os.path.exists(path):
        return out
    for row in csv.DictReader(open(path)):
        d = {}
        for k, v in row.items():
            try:
                d[k] = float(v)
            except (TypeError, ValueError):
                d[k] = v
        out.setdefault(int(d['round']), {})[int(d['colony'])] = d
    return out


def cases_of(d):
    any_row = next(iter(next(iter(d.values())).values()))
    return [(k, s) for k, s in CASES if k in any_row]


def rankdata(x):
    o = sorted(range(len(x)), key=lambda i: x[i])
    rk = [0.0] * len(x)
    i = 0
    while i < len(o):
        j = i
        while j + 1 < len(o) and x[o[j + 1]] == x[o[i]]:
            j += 1
        for t in range(i, j + 1):
            rk[o[t]] = (i + j) / 2 + 1
        i = j + 1
    return rk


def spearman(a, b):
    if len(a) < 3:
        return float('nan')
    ra, rb = rankdata(a), rankdata(b)
    ma, mb = st.mean(ra), st.mean(rb)
    den = math.sqrt(sum((x - ma) ** 2 for x in ra) * sum((y - mb) ** 2 for y in rb))
    return sum((x - ma) * (y - mb) for x, y in zip(ra, rb)) / den if den else float('nan')


def signp(w, l):
    n, k = w + l, min(w, l)
    return 1.0 if n == 0 else min(1.0, 2 * sum(comb(n, i) for i in range(k + 1)) / 2 ** n)


def ci(v, rng):
    v = [x for x in v if x == x]
    if not v:
        return '  n/a'
    bs = sorted(st.mean(rng.choices(v, k=len(v))) for _ in range(2000))
    return f'{st.mean(v):5.2f} [{bs[100]:5.2f},{bs[1900]:5.2f}]'


def ranksum(rows, cs, cases):
    tot = {c: 0.0 for c in cs}
    for k, s in cases:
        for c, x in zip(cs, rankdata([s * rows[c][k] for c in cs])):
            tot[c] += x
    return [tot[c] for c in cs]


def kept_twice(d, r):
    return [c for c, v in d[r].items() if v['refounded_from'] == '' and d[r - 1][c]['refounded_from'] == ''
            and v['alive'] > 0 and d[r + 1][c]['alive'] > 0]


def audit(arm):
    d = load(f'{arm}/colsel.csv')
    rng = random.Random(1)
    cases = cases_of(d)
    R = sorted(d)
    print(f'\n=== {arm}: {len(R)} rounds, {len(d[R[0]])} colonies, cases {[k for k, _ in cases]}')
    nd = [len({v['refounded_from'] for v in d[r].values() if v['refounded_from'] != ''}) for r in R]
    gp = [sum(1 for v in d[r].values() if v['passed_gate'] == 1) for r in R]
    print(f'donors: distinct donors per round median {st.median(nd)} (range {min(nd)}-{max(nd)}); passed dig gate median {st.median(gp)}')

    rt = {k: [] for k, _ in cases}
    rt['ranksum'] = []
    for r in R:
        if r - 1 not in d or r + 1 not in d:
            continue
        kept = kept_twice(d, r)
        if len(kept) < 6:
            continue
        for k, _ in cases:
            rt[k].append(spearman([d[r][c][k] for c in kept], [d[r + 1][c][k] for c in kept]))
        rt['ranksum'].append(spearman(ranksum(d[r], kept, cases), ranksum(d[r + 1], kept, cases)))
    print(f'retest (kept-twice colonies, {len(rt["ranksum"])} round pairs): Spearman r -> r+1 [90% bootstrap]')
    for k in rt:
        print(f'   {k:15s} {ci(rt[k], rng)}')

    wt = {k: [] for k, _ in cases}
    pr = {k: ([], []) for k, _ in cases}
    for r in R:
        live = [c for c, v in d[r].items() if v['alive'] > 0]
        gate = [c for c in live if d[r][c]['passed_gate'] == 1]
        if len(gate) >= 10:
            rk = {k: rankdata([s * d[r][c][k] for c in gate]) for k, s in cases}
            tot = [sum(rk[k][i] for k, _ in cases) for i in range(len(gate))]
            mt = st.mean(tot)
            vt = sum((t - mt) ** 2 for t in tot)
            for k, _ in cases:
                m = st.mean(rk[k])
                wt[k].append(sum((a - m) * (t - mt) for a, t in zip(rk[k], tot)) / vt)
        if len(live) < 8:
            continue
        donors = {int(v['refounded_from']) for v in d[r].values() if v['refounded_from'] != ''}
        kept = [c for c in live if d[r][c]['rank'] < len(d[r]) // 2]
        for k, s in cases:
            x = [s * d[r][c][k] for c in live]
            m, sd = st.mean(x), st.pstdev(x)
            if sd == 0:
                continue
            pr[k][0].append((st.mean(s * d[r][c][k] for c in kept) - m) / sd)
            if donors:
                pr[k][1].append((st.mean(s * d[r][c][k] for c in donors) - m) / sd)
    print('weight (share of the survival rank-sum) | pressure on kept half, on donors (sd, + = better)')
    for k, _ in cases:
        print(f'   {k:15s} {st.mean(wt[k]):5.0%} | {st.median(pr[k][0]):+5.2f} {st.median(pr[k][1]):+5.2f}')

    diff = {k: [] for k, _ in cases}
    for r in R:
        if r + 1 not in d:
            continue
        for c, v in d[r].items():
            if v['refounded_from'] == '':
                continue
            dn = int(v['refounded_from'])
            if d[r][dn]['refounded_from'] != '':
                continue
            for k, s in cases:
                diff[k].append(s * (d[r + 1][c][k] - d[r + 1][dn][k]))
    print(f'child vs donor, same round ({len(diff[cases[0][0]])} pairs; pairs share donors and rounds, so p is optimistic)')
    for k, _ in cases:
        w = sum(1 for x in diff[k] if x > 0)
        l = sum(1 for x in diff[k] if x < 0)
        print(f'   {k:15s} child better {w:5d} worse {l:5d}  median {st.median(diff[k]):+9.1f}  sign p={signp(w, l):.1e}')

    share = []
    for r in R:
        if r == 0:
            continue
        fresh = {c for c, v in d[r - 1].items() if v['refounded_from'] != ''}
        top = [c for c, v in d[r].items() if v['rank'] < len(d[r]) // 2]
        if top:
            share.append(sum(1 for c in top if c in fresh) / len(top))
    if share:
        print(f'luck: children take {st.mean(share):.0%} of the top half (50% = no information)')

    last = [x for x in os.listdir(arm) if x.startswith('r') and x[1:].isdigit()] if os.path.isdir(arm) else []
    for rd in sorted(last, key=lambda x: int(x[1:])):
        fams, lines = [], []
        for c in sorted(os.listdir(f'{arm}/{rd}')):
            p = f'{arm}/{rd}/{c}/index.csv'
            if not os.path.exists(p):
                continue
            rows = list(csv.DictReader(open(p)))
            if not rows:
                continue
            lines.append(len({x['family'] for x in rows}))
            fams += [x['family'].split('.') for x in rows]
        if not fams:
            continue
        pre = 0
        while all(len(f) > pre for f in fams) and len({f[pre] for f in fams}) == 1:
            pre += 1
        depth = st.median(len(f) for f in fams)
        print(f'lineage ({rd}): founder lines left per colony median {st.median(lines)}; founding pool members left '
              f'{len({f[0] for f in fams})}; every ant shares one ancestor ant {depth - pre:.0f} rounds back')


def pair(a, b):
    da, db = load(f'{a}/colsel.csv'), load(f'{b}/colsel.csv')
    cases = cases_of(da)
    acc = {k: [] for k, _ in cases}
    acc['ranksum'] = []
    for r in sorted(da):
        if r == 0 or r not in db:
            continue
        cs = [c for c in da[r] if c in db[r] and da[r][c]['alive'] > 0 and db[r][c]['alive'] > 0]
        if len(cs) < 10:
            continue
        for k, _ in cases:
            acc[k].append(spearman([da[r][c][k] for c in cs], [db[r][c][k] for c in cs]))
        acc['ranksum'].append(spearman(ranksum(da[r], cs, cases), ranksum(db[r], cs, cases)))
    print(f'\n=== world effect {a} ~ {b} ({len(acc["ranksum"])} rounds): agreement of within-round colony ranks')
    for k, v in acc.items():
        v = [x for x in v if x == x]
        print(f'   {k:15s} {st.mean(v):5.2f}')
    # positive control for retest: kept-twice colonies of both arms pooled
    pc = []
    for r in sorted(da):
        if r - 1 not in da or r + 1 not in da or r + 1 not in db or r - 1 not in db:
            continue
        x, y = [], []
        for dd in (da, db):
            for c in kept_twice(dd, r):
                x.append(dd[r][c]['starved'])
                y.append(dd[r + 1][c]['starved'])
        pc.append(spearman(x, y))
    pc = [v for v in pc if v == v]
    if pc:
        print(f'   positive control: starvation retest over both arms pooled {st.mean(pc):.2f}')


if __name__ == '__main__':
    if sys.argv[1] == '--pair':
        pair(sys.argv[2], sys.argv[3])
    else:
        for arm in sys.argv[1:]:
            audit(arm)
