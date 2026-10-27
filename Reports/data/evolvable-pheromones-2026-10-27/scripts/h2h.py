import json,sys,statistics as st
from collections import defaultdict
R=json.load(open(sys.argv[1])); by=defaultdict(dict)
for r in R: by[r['arm']][r['seed']]=r
ss=sorted(next(iter(by.values())))
ref=sys.argv[2] if len(sys.argv)>2 else 'F_hear'
for a in by:
    v=by[a]; print(f"{a:8} stored med {st.median(v[s]['stored'] for s in ss):6.1f}  extinct {sum(v[s]['alive']==0 for s in ss):2}/{len(ss)}  {ref}>this {sum(by[ref][s]['stored']>v[s]['stored'] for s in ss)}/{len(ss)}")
