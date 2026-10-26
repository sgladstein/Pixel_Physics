# mkarm.py FIT.raw.txt OUTDIR [gatefile_out]: colony 13's members (slot-18 zeroed) with unit 8 replaced by the fit's.
import sys, glob, os
fit, out = sys.argv[1], sys.argv[2]
U=8; rows=set(range(23+35*U,23+35*U+35))|set(range(583+45*U,583+45*U+45))|set(range(1303+9*U,1303+9*U+9))|{1447+U}
f={}
for l in open(fit).read().splitlines()[1:]:
    i,w=l.split(); f[int(i)]=w
os.makedirs(out, exist_ok=True)
base=3285
for p in sorted(glob.glob('c13/m*.raw.txt')):
    L=open(p).read().splitlines()
    keep=[l for l in L[1:] if int(l.split()[0]) not in rows]
    keep+= [f"{i} {w}" for i,w in f.items() if i in rows]
    keep.sort(key=lambda l:int(l.split()[0]))
    open(os.path.join(out,os.path.basename(p)),'w').write(L[0]+"\n"+"\n".join(keep)+"\n")
g=[(i-base,w) for i,w in f.items() if i>=base]
if len(sys.argv)>3:
    open(sys.argv[3],'w').write("".join(f"{i} {w}\n" for i,w in g))
print(out, 'gate weights', len(g))
