exec(open('gate.py').read().split("save(make()")[0])
MEM1 = 23+U*35+22+1   # unit 15 context row, memory 1 (c1's laden flag: mem0 <- crop*36.7, mem1 <- mem0*39.6)
def make4(out, win, K=103.0, comp=True):
    a=dict(c1)
    a[F+13]=win; a[F+14]=win
    a[MEM1]=K
    a[CTXB]=-0.97*K
    a[OUT]=out
    if comp: a[SB]=a.get(SB,0.0)+out*(1.94*K)/(1+1.94*K)
    return a
for o in (5,10,20):
    save(make4(o,320.0/o),f'c1-mem1-o{o}.txt')
save(make4(20,16.0,comp=False),'c1-mem1-nocomp-o20.txt')
