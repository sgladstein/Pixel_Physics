import sys
def load(p):
    d={}
    for line in open(p).read().split('\n')[1:]:
        if line.strip():
            i,v=line.split(); d[int(i)]=float(v)
    return d
def save(a,out):
    with open(out,'w') as f:
        f.write('genome 2797 layout=4\n'); [f.write(f'{k} {a[k]}\n') for k in sorted(a) if a[k]!=0]
c1=load('/tmp/claude-0/maze/c1.raw.txt')
U=15; F=583+U*45; CTXB=23+U*35+34; OUT=1447+U; SB=1463
LADEN,HUNGRY=32,33
G=200.0; WIN=16.0; WOUT=20.0
def make(trail=True, hungry=False, comp=True):
    a=dict(c1)
    if trail:
        a[F+13]=WIN; a[F+14]=WIN
    a[F+LADEN]=G
    if hungry: a[F+HUNGRY]=G
    a[CTXB]=-G
    a[OUT]=WOUT
    if comp: a[SB]=a.get(SB,0.0)+WOUT*G/(1+G)
    return a
save(make(),'c1-gate-laden.txt')
save(make(hungry=True),'c1-gate-laden-or-hungry.txt')
save(make(comp=False),'c1-gate-laden-nocomp.txt')
save(make(trail=False),'c1-gate-shift-only.txt')
print('score bias c1', c1.get(SB,0.0))
