# keep the old ant's dig / carry-spoil / drop labels; tag the kind in the label's first field: 1 cut, 2 no-cut, 3 spoil
import sys
src,dst=sys.argv[1],sys.argv[2]; n={1:0,2:0,3:0,'drop':0}
with open(dst,'w') as out:
    for line in open(src):
        f=line.rstrip('\n').split(',')
        if len(f)<17 or f[1]!='Q' or f[15]=='-':
            out.write(line); continue
        opts=[o.split(':',2) for o in f[13].split(';')]
        idx=[int(i) for i in f[15].split('|')[1].split(';')]
        verbs={int(opts[i][0]) for i in idx}; pick=int(f[7]); pv=int(opts[pick][0]); carrying=f[9]=='1'
        k=None
        if 3 in verbs: k=1; idx=[i for i in idx if int(opts[i][0])==3]
        elif carrying and verbs & {0,1,5}: k=3; idx=[i for i in idx if int(opts[i][0]) in (0,1,5)]
        elif not carrying and pv==3: k=2
        if k is None: f[15]='-'; n['drop']+=1
        else: f[15]=f'{k}|'+';'.join(map(str,idx)); n[k]+=1
        out.write(','.join(f)+'\n')
print(src.split('/')[-1], 'cut',n[1],'no-cut',n[2],'spoil',n[3],'dropped',n['drop'])
