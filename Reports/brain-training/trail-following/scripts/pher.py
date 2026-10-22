import sys
def load(p):
    L=open(p).read().split('\n'); x0,y0,w,h=map(int,L[0].split())
    A=[];B=[]
    for row in L[1:1+h]:
        cs=[c.split('/') for c in row.split()]
        A.append([int(a) for a,b in cs]); B.append([int(b) for a,b in cs])
    return x0,y0,w,h,A,B
for p in sys.argv[1:]:
    x0,y0,w,h,A,B=load(p)
    print(p)
    for ch,M in (('A',A),('B',B)):
        line=[]
        for dx in range(-45,46,5):
            i=128+dx-x0
            col=[M[y][i] for y in range(h)]
            line.append(f'{dx:+d}:{max(col)}')
        print(' ',ch,' '.join(line))
