import csv,sys,statistics as st
from collections import defaultdict
runs=dict(a.split('=') for a in sys.argv[1:])
for name,d in runs.items():
    G=defaultdict(list)
    for r in csv.DictReader(open(d+'/gens.csv')): G[int(r['gen'])].append(int(r['score']))
    blocks=defaultdict(list)
    for g,v in G.items(): blocks[g//25].append(st.median(v))
    L=defaultdict(lambda:[0,0,0])
    for r in csv.DictReader(open(d+'/listen.csv')):
        b=int(r['gen'])//25; n,dd,s=int(r['normal_stored']),int(r['deaf_stored']),int(r['shift_stored'])
        L[b][0]+=n>dd; L[b][1]+=n>s; L[b][2]+=1
    print(name,'gens',max(G)+1,' | '.join(f"g{b*25}-: med {st.median(blocks[b]):.0f} hear>deaf {L[b][0]}/{L[b][2]} hear>shift {L[b][1]}/{L[b][2]}" for b in sorted(blocks)))
