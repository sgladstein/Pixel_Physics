# Surface starvers' last LAST frames: what pulled them, where they were, whether they bit.
import csv,gzip,collections,sys,re
d=sys.argv[1]; LAST=int(sys.argv[2]) if len(sys.argv)>2 else 2000
m=re.search(r'nest_x=(\d+) food_x=(\d+) ground_y=(\d+)',open(f'{d}/events.txt').read()); NX,FX,GY=map(int,m.groups())
dead={}
for r in csv.DictReader(open(f'{d}/ledger.csv')):
    if r['died'] and r['died']!='0' and 'STARVED' in r['cause'] and r['zone_end']=='surface':
        dead[r['id']]=int(r['end'])
pull=collections.Counter(); zone=collections.Counter(); leg=collections.Counter(); bites=collections.Counter(); out=collections.Counter()
xs=collections.Counter(); per=collections.defaultdict(lambda: collections.Counter()); n=0
for r in csv.DictReader(gzip.open(f'{d}/hungry.csv.gz','rt')):
    i=r['id']
    if i not in dead: continue
    f=int(r['frame'])
    if f<dead[i]-LAST or f>dead[i]: continue
    n+=1; pull[r['pull']]+=1; zone[r['zone']]+=1; leg[r['leg']]+=1; out[r['outcome']]+=1
    if r['bite']: bites[r['bite']]+=1; per[i]['bite']+=1
    x=int(r['hx']); xs[(x-NX)//10*10]+=1
    per[i][r['pull']]+=1
print(f'{d}: surface starvers {len(dead)}, rows in last {LAST} frames {n}')
for name,c in (('pull',pull),('zone',zone),('leg',leg),('outcome',out),('bite',bites)):
    print(f'  {name}:', ', '.join(f'{k} {v*100/max(n,1):.0f}%' if name!='bite' else f'{k} {v}' for k,v in c.most_common(8)))
print('  x from door (10-col bins):', ', '.join(f'{k}:{v*100/max(n,1):.0f}%' for k,v in sorted(xs.items()) if v*100/max(n,1)>=2))
top=collections.Counter({i:max(c, key=lambda k:c[k] if k!='bite' else -1) for i,c in per.items()}.values())
print('  each starver\'s commonest pull:', dict(top.most_common(6)))
print('  starvers that bit at all in the window:', sum(1 for i in per if per[i]['bite']>0), 'of', len(per))
