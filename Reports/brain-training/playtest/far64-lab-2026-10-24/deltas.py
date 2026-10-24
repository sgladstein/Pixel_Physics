import csv,sys
r=list(csv.DictReader(open(sys.argv[1])))
cols=sys.argv[2].split(',')
cum=set("births deaths starved killed oldage_deaths eats digs deliveries forage_trips forage_returns nest_visits at_nest_ticks starved_under starved_near starved_afield pickups drops eggs_laid larvae_starved spoil_dumped hatches_denied".split())
print('frame '+' '.join(('d' if c in cum else '')+c for c in cols))
prev=None
for x in r:
    out=[x['frame']]
    for c in cols:
        v=float(x[c])
        if c in cum and prev: v-=float(prev[c])
        out.append(f"{v:.0f}" if abs(v)>=10 or v==int(v) else f"{v:.2f}")
    print(' '.join(out)); prev=x
