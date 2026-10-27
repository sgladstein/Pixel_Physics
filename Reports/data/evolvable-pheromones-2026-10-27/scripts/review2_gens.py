import csv, statistics as st, sys
for path in sys.argv[1:]:
    R=list(csv.DictReader(open(path)))
    by={}
    for r in R: by.setdefault(int(r['gen']),[]).append(int(r['stored']))
    gs=sorted(by)
    out=[]
    for lo,hi in [(0,10),(20,30),(40,50),(50,60),(90,100),(140,150)]:
        med=[st.median(by[g]) for g in range(lo,hi) if g in by]
        top=[st.mean(sorted(by[g])[-4:]) for g in range(lo,hi) if g in by]
        if med: out.append(f'g{lo}-{hi-1}: med {st.mean(med):.0f} top4 {st.mean(top):.0f}')
    # duplicates check
    from collections import Counter
    c=Counter((r['gen'],r['colony']) for r in R); dup=sum(v-1 for v in c.values() if v>1)
    print(path.split('/')[-2], 'gens', len(gs), 'dups', dup, ' | '.join(out))
