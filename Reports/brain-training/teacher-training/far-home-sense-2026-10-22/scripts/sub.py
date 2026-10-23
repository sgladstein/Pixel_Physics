# usage: sub.py IN OUT P -- keep each Q row with probability P (seeded), every non-Q row
import sys,random
r=random.Random(7); P=float(sys.argv[3]); n=k=lab=0
with open(sys.argv[2],'w') as o:
    for line in open(sys.argv[1]):
        p=line.split(',',16)
        if p[1]!='Q': o.write(line); continue
        n+=1
        if r.random()<P:
            o.write(line); k+=1; lab+= p[15]!='-'
print(sys.argv[1],n,k,'labelled',lab)
