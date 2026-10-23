# usage: filt.py IN OUT -- keep Q rows that are labelled (field 16) or far (ctx slot 9 > 0), and every non-Q row
import sys
n=k=0
with open(sys.argv[2],'w') as o:
    for line in open(sys.argv[1]):
        p=line.split(',',16)
        if p[1]!='Q': o.write(line); continue
        n+=1
        if p[6]=='2': continue
        lab=len(p)>15 and p[15].split(',')[0].strip()!='-'
        if lab or float(p[11].split(' ')[9])>0:
            o.write(line); k+=1
print(sys.argv[1],n,k)
