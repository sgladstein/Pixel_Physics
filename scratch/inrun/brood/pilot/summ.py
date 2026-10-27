import csv,sys,os,statistics as st
def rows(p): return list(csv.DictReader(open(p))) if os.path.exists(p) else []
print(f"{'run':7} {'wall':>5} {'alive_end':>9} {'alive min/max(after20k)':>22} {'store_mean':>10} {'store_end':>9} {'tank':>6} {'starved':>7} {'culled':>6} {'hatch':>6} {'lstarv':>6} {'food_home':>9} {'dug':>5} {'pit':>4} {'nscore':>7} {'churn':>6} {'homeeat':>7} {'deliv':>6}")
for arm in sys.argv[1:] or ['ctl','lay','cap','sb','fb']:
  for s in '123':
    d=f'{arm}{s}'
    if not os.path.isdir(d): continue
    stock=rows(f'{d}/stock.csv'); nest=rows(f'{d}/nest.csv'); br=rows(f'{d}/brood.csv')
    late=[int(r['alive']) for r in stock if int(r['frame'])>=20000]
    sm=st.mean(int(r['nest_food']) for r in stock if int(r['frame'])>=20000)
    dc={}; ev={}
    for l in open(f'{d}/events.csv'):
      p=l.split(',')
      if len(p)>5 and p[2]=='death': dc[p[5]]=dc.get(p[5],0)+1
      if len(p)>2: ev[p[2]]=ev.get(p[2],0)+1
    wall=[l for l in open(f'{d}.log') if l.startswith('wall')]
    hatch=br[-1]['hatched'] if br else '-'; ls=br[-1]['larvae_starved'] if br else '-'
    n=nest[-1]
    print(f"{d:7} {wall[0].split()[1] if wall else '?':>5} {stock[-1]['alive']:>9} {str(min(late))+'/'+str(max(late)):>22} {sm:10.1f} {stock[-1]['nest_food']:>9} {float(stock[-1]['tank_mean']):6.1f} {dc.get('1',0):>7} {dc.get('4',0):>6} {hatch:>6} {ls:>6} {n['food_home']:>9} {n['dug']:>5} {n['pit']:>4} {float(n['nest_score']):7.1f} {ev.get('churn',0):6} {ev.get('homeeat',0):7} {ev.get('deliver',0):6}")
