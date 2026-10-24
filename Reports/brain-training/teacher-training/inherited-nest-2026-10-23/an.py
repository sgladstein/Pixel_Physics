import sys, os, statistics as st
NEST=128
def load(d):
    L=open(f'{d}/nestmap.txt').read().split('\n'); h=L[0].split(' '); leg=h[2].split(',')
    g=[[leg[ord(c)-65] if c!='?' else '?' for c in r] for r in L[1:] if r]
    surf=[int(l) for l in open(f'{d}/surface.txt') if l.strip()]
    return int(h[0]),int(h[1]),g,surf
GND={'soil','packedsoil','spoil','stone'}
def stats(d0,d1):
    x0,y0,a,surf=load(d0); _,_,b,_=load(d1)
    ref=emp=top6=top6b=kept=0
    for y in range(len(a)):
        for x in range(len(a[0])):
            X,Y=x0+x,y0+y
            if abs(X-NEST)>60 or Y<surf[X]: continue
            oa=a[y][x] not in GND and a[y][x]!='?'; ob=b[y][x] not in GND and b[y][x]!='?'
            kept+=oa
            if oa and not ob and b[y][x]!='?': ref+=1
            if not oa and ob and a[y][x]!='?': emp+=1
            if abs(X-NEST)<=20 and Y<surf[X]+6:
                top6+=oa; top6b+=ob
    # joined to door at f0
    door=NEST-x0
    opn=lambda x,y: 0<=y<len(a) and 0<=x<len(a[0]) and a[y][x] not in GND and y+y0>=surf[x0+x]
    seen=set(); st_=[(door,y) for y in range(len(a)) if opn(door,y)][:1]  # from the mouth only
    while st_:
        p=st_.pop()
        if p in seen: continue
        seen.add(p); x,y=p
        for q in ((x-1,y),(x+1,y),(x,y-1),(x,y+1)):
            if opn(*q) and q not in seen: st_.append(q)
    joined=sum(1 for (x,y) in seen if abs(x0+x-NEST)<=60)
    return kept,joined,ref,emp,top6,top6b
root=sys.argv[1]; N=sys.argv[2]
rows={}
for t in sorted(os.listdir(root)):
    for L in ('0','1'):
        rows[(t,L)]=stats(f'{root}/{t}/L{L}/f0',f'{root}/{t}/L{L}/f{N}')
for L in ('0','1'):
    v=[rows[(t,L)] for t in sorted(os.listdir(root))]
    names=['open below surface at start','of those joined to door','refilled by f'+N,'ground emptied by f'+N,'top6 open start','top6 open end']
    print(f'INHERIT_LINE={L}  n={len(v)}')
    for i,nm in enumerate(names):
        col=sorted(r[i] for r in v)
        print(f'  {nm:32} median {st.median(col):6.0f}  p90 {col[int(.9*len(col))-1]:5}  max {col[-1]:5}  sum {sum(col)}')
with open(f'{root}.tsv','w') as f:
    f.write('map\tL\tkept\tjoined\trefilled\temptied\ttop6_0\ttop6_N\n')
    for (t,L),r in rows.items(): f.write('\t'.join([t,L]+[str(x) for x in r])+'\n')
