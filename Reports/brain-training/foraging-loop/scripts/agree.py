# Exact agreement (copy's pick in the teacher's label set) by tank band and spoil, on a PP_CARRY=0 log.
import sys,collections,csv,bisect
log,trace=sys.argv[1],sys.argv[2]
tk=collections.defaultdict(list)
for r in csv.DictReader(open(trace)): tk[int(r['id'])].append((int(r['frame']),float(r['tank']),int(r['spoil'])))
c=collections.Counter(); a=collections.Counter()
for line in open(log):
    p=line.rstrip("\n").split(",")
    if len(p)<16 or p[1]!="Q" or "|" not in p[15]: continue
    v=tk.get(int(p[3]));
    if not v: continue
    j=bisect.bisect_right(v,(int(p[2]),9e9))-1
    if j<0: continue
    t=v[j][1]; band="<1" if t<1 else "<2.5" if t<2.5 else ">=2.5"
    ix=set(int(i) for i in p[15].split("|")[1].split(";") if i)
    key=(band,"spoil" if v[j][2] else "free"); c[key]+=1; a[key]+= int(p[7]) in ix
for k in sorted(c): print(k, c[k], f"agree {100*a[k]/c[k]:.0f}%")
