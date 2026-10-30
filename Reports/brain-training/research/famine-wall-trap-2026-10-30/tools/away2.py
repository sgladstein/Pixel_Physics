# like away.py but the choice is either the mind's pick (field 7) or, for a shadow log, the hand-built ant's label (field 15 '1|i;j')
import sys, collections
src=sys.argv[2]
c=collections.defaultdict(lambda:[0,0,0]); verbs=collections.defaultdict(collections.Counter)
V=['walk','turn','rest','dig','pick','drop','eat','feed','atk']
for line in open(sys.argv[1]):
    f=line.rstrip('\n').split(',')
    if len(f)<16 or f[1]!='Q' or f[6]=='2': continue
    x=int(f[4]); ctx=[float(v) for v in f[11].split()]
    if src=='label':
        if not f[15].startswith('1|'): continue
        pick=int(f[15][2:].split(';')[0])
    else: pick=int(f[7])
    E=ctx[0]; nest=120 if x<256 else 390
    wall = x<=10 or 246<=x<=266 or x>=500
    band='wall' if wall else ('near' if abs(x-nest)<=30 else 'mid')
    eb='E<.2' if E<.2 else 'E<.4' if E<.4 else 'E>=.4'
    opts=[o.split(':') for o in f[13].split(';')]
    if not 0<=pick<len(opts): continue
    def home(o):
        for a in o[4].split():
            k,v=a.split('=')
            if k=='39': return float(v)
        return 0.0
    verbs[band][V[int(opts[pick][0])]]+=1
    if opts[pick][0]!='0': continue
    hs=[home(o) for o in opts if o[0]=='0']
    if max(hs)>0.3 and min(hs)<-0.3:
        h=home(opts[pick]); k=c[(band,eb)]
        k[0]+=1; k[1]+= h<-0.3; k[2]+= h>0.3
for k in sorted(c): n,o,h=c[k]; print(k,'n=%d outward %.2f homeward %.2f'%(n,o/n,h/n))
for b in verbs: t=sum(verbs[b].values()); print(b,'n=%d'%t,{v:round(n/t,3) for v,n in verbs[b].most_common()})
