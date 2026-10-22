import sys
def load(p):
    d={}
    for line in open(p).read().split('\n')[1:]:
        if line.strip():
            i,v=line.split(); d[int(i)]=float(v)
    return d
src,out,w_in,w_out,fall=sys.argv[1],sys.argv[2],float(sys.argv[3]),float(sys.argv[4]),float(sys.argv[5])
a=load(src); u=15; F=583+u*45
a[F+13]=w_in; a[F+14]=w_in   # B, stronger there / two cells on
if fall: a[F+20]=fall        # fall
a[1447+u]=w_out
with open(out,'w') as f:
    f.write('genome 2797 layout=4\n'); [f.write(f'{k} {a[k]}\n') for k in sorted(a) if a[k]!=0]
