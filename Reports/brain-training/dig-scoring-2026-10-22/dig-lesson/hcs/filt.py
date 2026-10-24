# hcs logs (PP_OLDLAB_EMPTY): kind in the label's first field -- 1 the old ant's cut, 2 it would not cut where the
# brain cut (must-not-dig), 4 its step / turn / rest underground with empty jaws. Food labels dropped.
import sys
src,dst=sys.argv[1],sys.argv[2]; n={1:0,2:0,4:0,'drop':0}
with open(dst,'w') as out:
    for line in open(src):
        f=line.rstrip('\n').split(',')
        if len(f)<17 or f[1]!='Q' or f[15]=='-' or f[6]=='2':
            out.write(line); continue
        opts=[o.split(':',2) for o in f[13].split(';')]
        idx=[int(i) for i in f[15].split('|')[1].split(';')]
        verbs={int(opts[i][0]) for i in idx}; pv=int(opts[int(f[7])][0]); carrying=f[9]=='1'
        k=None
        if carrying: k=None
        elif 3 in verbs: k=1; idx=[i for i in idx if int(opts[i][0])==3]
        elif pv==3: k=2
        elif verbs <= {0,1,2}: k=4
        if k is None: f[15]='-'; n['drop']+=1
        else: f[15]=f'{k}|'+';'.join(map(str,idx)); n[k]+=1
        out.write(','.join(f)+'\n')
print(src.split('/')[-1], 'cut',n[1],'no-cut',n[2],'under',n[4],'dropped',n['drop'])
