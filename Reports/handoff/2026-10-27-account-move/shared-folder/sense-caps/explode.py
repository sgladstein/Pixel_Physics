import sys
src, out = sys.argv[1], sys.argv[2]
lines=[l for l in open(src) if not l.startswith('#')]
h=lines[0].split(); n=int(h[3]); layout=h[4]
pool=[]
for l in lines[1:]:
    if l.startswith('member'):
        pool.append(list(pool[0]) if pool else [0.0]*n); continue
    i,w=l.split(); pool[-1][int(i)]=float(w)
for k,g in enumerate(pool):
    with open(f"{out}/m{k:02d}.raw.txt","w") as f:
        f.write(f"genome {n} {layout}\n")
        for i,w in enumerate(g): f.write(f"{i} {w!r}\n")
print(len(pool), n)
