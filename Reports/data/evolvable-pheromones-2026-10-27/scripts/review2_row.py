import sys
def load(p):
    L=[l.split() for l in open(p) if l.strip()]
    return L[0], {int(a):float(b) for a,b in L[1:]}
h0,f=load(sys.argv[1])
for p in sys.argv[2:]:
    h,g=load(p)
    row=[g.get(i,0.0) for i in range(2410,2445)]; frow=[f.get(i,0.0) for i in range(2410,2445)]
    big=[(i-2410,round(w,2)) for i,w in zip(range(2410,2445),row) if abs(w)>0.5]
    print(p.split('scratchpad/')[-1], h, 'live',sum(1 for v in g.values() if v), 'alarm-row nonzero',sum(1 for w in row if w),'|w|>0.5:',big, 'L1 change',round(sum(abs(a-b) for a,b in zip(row,frow)),2))
