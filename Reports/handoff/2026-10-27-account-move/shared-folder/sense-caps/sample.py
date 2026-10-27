import sys
n=int(sys.argv[1]); keep=False; m=0
out=open(sys.argv[2],'w')
for l in sys.stdin:
    if l.startswith('M,'):
        m+=1; keep = (m%n==0)
    if keep: out.write(l)
