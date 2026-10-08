import csv,collections,sys,os
B='/home/user/runs/boom'
def read(d):
    st=list(csv.DictReader(open(f'{d}/stats.csv'))); late=[int(x['ants']) for x in st if int(x['frame'])>=100000]
    z=collections.Counter()
    for r in csv.DictReader(open(f'{d}/ledger.csv')):
        if r['died'] and r['died']!='0' and 'STARVED' in r['cause']: z[r['zone_end']]+=1
    x=st[-1]
    return min(late),sum(late)//len(late),sum(z.values()),z['surface'],z['nest'],int(x['larvae_starved']),int(x['eggs_laid'])
seeds=[int(a) for a in sys.argv[1:]] or [1,2,3,4]
arms=[('all (playedible)','steady_income-playedible-w0-s{}'),('main','steady_income-main-w0-s{}')]+[(f'minus {d}',f'loo-{d}-s{{}}') for d in ('NEEDS_FIRST','CARRY_HOME','DOOR_COLUMN','LAY_BAR','NEST_STORE','WAY_FOOT')]
print('arm | ' + ' | '.join(f's{s}: min/mean, starved (surface/nest), larvae, eggs' for s in seeds))
for name,pat in arms:
    cells=[]
    for s in seeds:
        d=f'{B}/'+pat.format(s)
        if not os.path.exists(d+'.log') or 'rc=0' not in open(d+'.log').read(): cells.append('-'); continue
        a=read(d); cells.append(f'{a[0]}/{a[1]}, {a[2]} ({a[3]}/{a[4]}), {a[5]}, {a[6]}')
    print(name,'|',' | '.join(cells))
