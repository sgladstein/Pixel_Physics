# E4 verdict over garden halves (pheromones lane, §9.15). garden.py's per-run rows depend only on the run (fixed seeds
# 5,000,000+), so rows from separate garden calls combine. Applies garden.py's rule unchanged to the pairs whose run
# name starts with e4_, plus the second pre-registered question: free > its paired deaf in 6/6 (sign test p = 0.031).
import json, math, statistics as st, sys
free, deaf = {}, {}
for f in sys.argv[1:]:
    g = json.load(open(f))
    for arm, dst in (('free', free), ('deaf', deaf)):
        for r in g[arm]:
            name = r['run'].rstrip('/').split('/')[-1]
            dst[name.split('_', 2)[-1] if name.startswith('e4_') else name] = r
reps = sorted(k for k in free if k.isdigit() and k in deaf)
rows = []
for k in reps:
    fr, dr = free[k], deaf[k]
    rows.append(dict(rep=k, free=round(fr['value'], 1), deaf=round(dr['value'], 1), ratio=round(fr['value'] / max(1.0, dr['value']), 3),
                     comm=fr['communicates'], info=round(fr['max_state_ratio'], 2),
                     drop={n: round(1 - fr['C_median'][n] / fr['C_median']['normal'], 3) for n in ('deaf', 'shift', 'const')},
                     lower={n: fr[f'{n}_lower'] for n in ('deaf', 'shift', 'const')}))
    print(rows[-1])
n = len(rows); n_comm = sum(r['comm'] for r in rows)
logr = [math.log(max(1.0, free[k]['value']) / max(1.0, deaf[k]['value'])) for k in reps]
guard = math.exp(st.mean(logr)) if logr else float('nan')
wins = sum(free[k]['value'] > deaf[k]['value'] for k in reps)
p = sum(math.comb(n, i) for i in range(n + 1) if abs(i - n / 2) >= abs(wins - n / 2)) / 2 ** n if n else float('nan')
verdict = 'SIGNALS FORM' if n_comm >= 3 and guard >= 0.90 else 'DEAD END' if n_comm == 0 else 'INCONCLUSIVE'
print(f'pairs {n}; free runs communicating {n_comm}/{n}; guard free/deaf geometric mean {guard:.2f}; free higher {wins}/{n} '
      f'(two-sided sign test p = {p:.3f}; bar 6/6)\nVERDICT: {verdict}; signalling arm forages better: {"YES" if n == 6 and wins == 6 else "NO" if n == 6 else "pending"}')
