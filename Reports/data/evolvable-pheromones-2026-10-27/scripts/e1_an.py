import json,statistics as st
from collections import defaultdict
R=json.load(open('/tmp/claude-0/-home-user-Pixel-Physics/2ef7e65c-531a-5db4-866b-18b502ff6e79/scratchpad/e1/runs/results.json'))
by=defaultdict(dict)
for r in R: by[r['arm']][r['seed']]=r
def mean(a): return sum(by[a][s]['stored'] for s in by[a])/len(by[a])
def ext(a): return sum(by[a][s]['alive']==0 for s in by[a])
print(f"{'genome':5} {'parent':>7} | median mutant/parent stored ratio (extinct runs of 48)")
for g in ['F1','F1b','F2','D1','D2','D2b']:
    p=mean(f'{g}_parent'); row=[f'{g:5} {p:7.0f}']
    for k in ['std','chan','other','gentle']:
        rs=[mean(f'{g}_{k}{i:02}')/p for i in range(12)]
        row.append(f"{k} {st.median(rs):.2f} ({sum(ext(f'{g}_{k}{i:02}') for i in range(12))})")
    print(' | '.join(row))
