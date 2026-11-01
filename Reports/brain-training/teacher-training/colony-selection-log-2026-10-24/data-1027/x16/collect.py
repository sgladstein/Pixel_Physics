import csv,os,re,sys
src,dst=sys.argv[1:]
rows=[];bad=[]
for n in sorted(os.listdir(src)):
    m=re.match(r'x16-(\w+?)-(\w+)-s(\d+)$',n)
    if not m: continue
    p=f'{src}/{n}/lab.csv'
    r=list(csv.DictReader(open(p))) if os.path.exists(p) else []
    if not r or not r[0].get('alive'): bad.append(n); continue
    d=dict(r[0]); d.pop('box',None); d.pop('seed',None)
    rows.append(dict(brain=m[1],box=m[2],seed=m[3],**d))
keys=list(dict.fromkeys(k for r in rows for k in r))
w=csv.DictWriter(open(dst,'w'),keys);w.writeheader();w.writerows(rows)
print(len(rows),'rows; bad',bad)
