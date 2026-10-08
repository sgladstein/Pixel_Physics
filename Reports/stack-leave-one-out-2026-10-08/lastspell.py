# Each surface starver's final hungry spell (rows with gaps <= GAP frames): did it reach the heap zone, bite, how long, where did it start.
import csv,gzip,collections,sys,re
d=sys.argv[1]; GAP=40
m=re.search(r'nest_x=(\d+) food_x=(\d+) ground_y=(\d+)',open(f'{d}/events.txt').read()); NX,FX,GY=map(int,m.groups())
dead={}
for r in csv.DictReader(open(f'{d}/ledger.csv')):
    if r['died'] and r['died']!='0' and 'STARVED' in r['cause'] and r['zone_end']=='surface': dead[r['id']]=int(r['end'])
sp={}
for r in csv.DictReader(gzip.open(f'{d}/hungry.csv.gz','rt')):
    i=r['id']
    if i not in dead: continue
    f=int(r['frame'])
    s=sp.get(i)
    if s is None or f-s['last']>GAP: s=sp[i]={'start':f,'last':f,'food':0,'bite':0,'x0':int(r['hx']),'z0':r['zone'],'minfd':999}
    s['last']=f; s['food']+=r['zone']=='food'; s['bite']+=bool(r['bite']); s['minfd']=min(s['minfd'],abs(int(r['hx'])-FX))
L=[s for i,s in sp.items()]
n=len(L); q=lambda v:sorted(v)[len(v)//2]
print(f'{d}: {n} surface starvers with a final spell')
print(f'  final spell length median {q([s["last"]-s["start"] for s in L])} frames')
print(f'  started in zone: {collections.Counter(s["z0"] for s in L).most_common(5)}')
print(f'  reached the heap zone in it: {sum(1 for s in L if s["food"])}; came within 12 cols of the heap: {sum(1 for s in L if s["minfd"]<=12)}; bit: {sum(1 for s in L if s["bite"])}')
print(f'  start x from door: median {q([s["x0"]-NX for s in L])}; |start x - door| > 100: {sum(1 for s in L if abs(s["x0"]-NX)>100)}')
