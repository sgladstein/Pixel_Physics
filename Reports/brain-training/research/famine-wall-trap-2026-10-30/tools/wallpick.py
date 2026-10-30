import sys, collections
src=sys.argv[2]
V=['walk','turn','rest','dig','pick','drop','eat','feed','atk']
st=collections.defaultdict(collections.Counter)
for line in open(sys.argv[1]):
    f=line.rstrip('\n').split(',')
    if len(f)<16 or f[1]!='Q' or f[6]=='2': continue
    x=int(f[4])
    if src=='label':
        if not f[15].startswith('1|'): continue
        pick=int(f[15][2:].split(';')[0])
    else: pick=int(f[7])
    wall = x<=10 or 246<=x<=266 or x>=500
    nest=120 if x<256 else 390
    band='wall' if wall else ('near' if abs(x-nest)<=30 else 'mid')
    opts=[o.split(':') for o in f[13].split(';')]
    if not 0<=pick<len(opts): continue
    o=opts[pick]; fe={a.split('=')[0]:float(a.split('=')[1]) for a in o[4].split()}
    c=st[band]; c['n']+=1
    v=V[int(o[0])]; c[v]+=1
    if v=='walk':
        c['walk:back']+= fe.get('42',0)>0
        c['walk:climb']+= fe.get('21',0)>0
        c['walk:up']+= fe.get('25',0)>0
        c['walk:onto ant']+= fe.get('8',0)>0
        c['walk:homeward']+= fe.get('39',0)>0.3
    c['offered turn']+= any(p[0]=='1' for p in opts)
for b,c in st.items():
    n=c['n']; print(b,'n=%d'%n,' '.join('%s %.3f'%(k,c[k]/n) for k in ['walk','rest','turn','walk:back','walk:homeward','walk:climb','walk:up','walk:onto ant','offered turn']))
