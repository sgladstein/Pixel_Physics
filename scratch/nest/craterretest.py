"""SCRATCH (nest lane, 2026-10-28): does the whole-crater count (nestcount `crater_full`) repeat from world to world
as well as the empty-only `crater` that colsel's +crater case uses?

Reads a finished colsel arm (colsel.csv + c<k>/r<r>/nestmap.txt, surface.txt), recounts each colony-round's end map
with nestcount, and scores both counts with selaudit's own retest: Spearman across colonies kept unmutated (elite=1)
in rounds r and r+1, two fresh worlds, averaged over rounds with a 90% bootstrap interval. Same statistic and same
colony filter as the selection review's table (clean1 k3: crater 0.11), so the numbers compare.

Also prints how much of the hole the empty-only count sees, and the check that the recount reproduces colsel.csv's
own `crater` column (a recount that does not is measuring a different thing).

usage: craterretest.py ARM_DIR
"""
import os, random, statistics as st, sys
here = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(here, '..', 'inrun'))
from selaudit import load, spearman, ci, kept_twice  # noqa: E402
import nestcount  # noqa: E402

arm = sys.argv[1]
d = load(f'{arm}/colsel.csv')
R = sorted(d)
cnt = {}
for r in R:
    for c in d[r]:
        p = f'{arm}/c{c}/r{r}'
        if os.path.exists(f'{p}/nestmap.txt'):
            n = nestcount.count(p)
            cnt[r, c] = (n['crater'], n['crater_full'])

same = sum(1 for (r, c), v in cnt.items() if v[0] == d[r][c]['crater'])
print(f'{arm}: {len(cnt)} colony-rounds recounted; recount crater == colsel.csv crater in {same}')
seen = [v[0] / v[1] for v in cnt.values() if v[1] >= 20]
print(f'share of the whole crater the empty-only count sees (crater_full >= 20): median {st.median(seen):.2f} (n {len(seen)})')
print('median crater / crater_full by round:', ' '.join(
    f'r{r}:{st.median(cnt[r, c][0] for c in d[r] if (r, c) in cnt):.0f}/{st.median(cnt[r, c][1] for c in d[r] if (r, c) in cnt):.0f}'
    for r in R[::4] if any((r, c) in cnt for c in d[r])))

rng = random.Random(1)
rt = {'crater': [], 'crater_full': []}
ns = []
for r in R:
    if r - 1 not in d or r + 1 not in d:
        continue
    kept = [c for c in kept_twice(d, r) if (r, c) in cnt and (r + 1, c) in cnt]
    if len(kept) < 6:
        continue
    ns.append(len(kept))
    for i, k in enumerate(rt):
        rt[k].append(spearman([cnt[r, c][i] for c in kept], [cnt[r + 1, c][i] for c in kept]))
print(f'retest over {len(ns)} round pairs, colonies kept twice median {st.median(ns)}:')
for k, v in rt.items():
    print(f'  {k:12s} {ci(v, rng)}')
