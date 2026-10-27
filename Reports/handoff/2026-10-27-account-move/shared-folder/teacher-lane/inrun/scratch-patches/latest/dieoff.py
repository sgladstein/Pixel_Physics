import csv,sys,glob,collections
# For each die-off (alive drops >=12 within 2 judgements of 1k), trace the food deliverers before it.
agg=collections.Counter(); rows_out=[]
for r in sorted(glob.glob(sys.argv[1])):
    try: sel=list(csv.DictReader(open(r+'/sel.csv')))
    except: continue
    F=None
    for i in range(2,len(sel)):
        if int(sel[i-2]['alive'])-int(sel[i]['alive'])>=12: F=int(sel[i-2]['frame']); break
    if F is None: continue
    W=F-30000
    dl=collections.Counter(); dl_all=collections.Counter(); field=collections.Counter(); home=collections.Counter(); fate={}; per=collections.Counter()
    for e in csv.DictReader(open(r+'/events.csv')):
        f=int(e['frame']); i=e['id']
        if f>F: break
        if e['kind']=='deliver':
            dl_all[i]+=1; per[f//5000]+=1
            if f>=W: dl[i]+=1
        elif e['kind']=='death' and i not in fate: fate[i]=('starved' if e['v']=='1' else 'cause'+e['v'], f)
    for b in csv.DictReader(open(r+'/births.csv')):
        f=int(b['frame'])
        if f<=F and b['kind'] in ('cull','retire'): fate[b['id']]=(b['kind'],f)
    # deliverers = ants with >=2 deliveries ever before F
    good=[i for i,c in dl_all.items() if c>=2]
    c=collections.Counter()
    for i in good:
        k,f=fate.get(i,('alive',0))
        if k=='starved' and f>=F-3000: k='starved_in_dieoff'
        c[k]+=1
    agg.update(c)
    trend=' '.join(str(per[k]) for k in range(0,F//5000+1))
    print(f"{r}: dieoff {F//1000}k; ants with 2+ deliveries: {len(good)} -> {dict(c)}; deliveries per 5k: {trend}")
print('ALL',dict(agg))
