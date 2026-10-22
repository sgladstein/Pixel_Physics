import sys, math
exec(open('ana4.py').read().split("print('arm")[0])
def alive(run): return int([l for l in open(f'{run}.log') if l.startswith('frame  200000')][0].split()[3])
def sign_p(u,d):
    n=u+d
    if n==0: return 1.0
    k=min(u,d); return min(1.0, 2*sum(math.comb(n,i) for i in range(k+1))/2**n)
for base,new in [('c1','c1-noB'),('c1','c1g'),('c1-ladenB','c1g-ladenB'),('c1-ladenB','c1ge-ladenB'),('hA1','hA1-noB'),('hA1','hA1-ladenB')]:
    out=[]
    for nm,fn in (('alive',alive),('home',home_food),('field',field)):
        b=[fn(f'{base}-{s}') for s in range(1,17)]; n=[fn(f'{new}-{s}') for s in range(1,17)]
        u=sum(x>y for x,y in zip(n,b)); d=sum(x<y for x,y in zip(n,b))
        out.append(f"{nm} {sum(b)/16:.0f}->{sum(n)/16:.0f} up {u}/down {d} p {sign_p(u,d):.3f}")
    print(f"{new} vs {base}: "+' | '.join(out))
