"""Which ledger parts pick parents. usage: picks.py ROUND_DIR... (each with pick.csv)
Per part: adults' mean rate (points per 1k frames), parents' mean rate, and the selection differential (parent - adults),
plus each part's share of the total differential. Split by first/second half of the round."""
import csv, sys
parts = ['food', 'cut', 'exit', 'clear', 'meal', 'hungry', 'field']
rows = []
F = 200000
for d in sys.argv[1:]:
    try:
        rows += [(d, r) for r in csv.DictReader(open(f'{d}/pick.csv'))]
    except FileNotFoundError:
        pass
for label, sel in [('all', lambda f: True), ('first half', lambda f: f <= F // 2), ('second half', lambda f: f > F // 2)]:
    rs = [r for _, r in rows if sel(int(r['frame']))]
    if not rs:
        continue
    n = len(rs)
    a = {p: sum(float(r['a_' + p]) for r in rs) / n for p in parts}
    pr = {p: sum(float(r['p_' + p]) for r in rs) / n for p in parts}
    tot = sum(pr[p] - a[p] for p in parts)
    print(f'{label}: {n} births; total differential {tot:.3f} points/1k frames')
    print(f'  {"part":7s} {"adults":>8s} {"parents":>8s} {"diff":>8s} {"share":>6s}')
    for p in parts:
        dd = pr[p] - a[p]
        print(f'  {p:7s} {a[p]:8.3f} {pr[p]:8.3f} {dd:8.3f} {100 * dd / tot if tot else 0:5.0f}%')
