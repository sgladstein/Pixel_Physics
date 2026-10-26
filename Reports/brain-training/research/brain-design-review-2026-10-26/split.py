import sys
src, out = sys.argv[1], sys.argv[2]
lines=[l.rstrip('\n') for l in open(src) if not l.startswith('#')]
h=lines[0].split(); n=int(h[3]); 
pool=[]; ids=[]
for l in lines[1:]:
    if l.startswith('member '):
        pool.append(list(pool[0]) if pool else [0.0]*n); ids.append(l.split()[2]); continue
    i,w=l.split(' '); pool[-1][int(i)]=float(w)
for k,(g,i) in enumerate(zip(pool,ids)):
    with open(f"{out}/m{k:02d}.raw.txt","w") as f:
        f.write(f"genome {n} layout=5\n"); f.write("".join(f"{j} {x!r}\n" for j,x in enumerate(g) if x!=0.0))
print(len(pool), n)
