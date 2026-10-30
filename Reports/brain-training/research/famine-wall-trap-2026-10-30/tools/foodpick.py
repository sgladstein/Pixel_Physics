import sys, collections
src=sys.argv[2]
V=['walk','turn','rest','dig','pick','drop','eat','feed','atk']
c=collections.defaultdict(collections.Counter)
for line in open(sys.argv[1]):
    f=line.rstrip('\n').split(',')
    if len(f)<16 or f[1]!='Q' or f[6]=='2': continue
    if src=='label':
        if not f[15].startswith('1|'): continue
        pick=int(f[15][2:].split(';')[0])
    else: pick=int(f[7])
    ctx=[float(v) for v in f[11].split()]
    opts=[o.split(':') for o in f[13].split(';')]
    if not 0<=pick<len(opts): continue
    foodpick=[o for o in opts if o[0] in ('4','6') and any(a.startswith('3=') for a in o[4].split())]
    if not foodpick: continue
    eb='E<.3' if ctx[0]<.3 else 'E<.6' if ctx[0]<.6 else 'E>=.6'
    crop='crop' if ctx[1]>0 else 'empty'
    k=(eb,crop); c[k]['n']+=1; c[k][V[int(opts[pick][0])]]+=1
    c[k]['picked food']+= opts[pick][0] in ('4','6') and any(a.startswith('3=') for a in opts[pick][4].split())
for k in sorted(c): n=c[k]['n']; print(k,'n=%d'%n,' '.join('%s %.2f'%(v,c[k][v]/n) for v in ['picked food','walk','rest','pick','eat','turn']))
