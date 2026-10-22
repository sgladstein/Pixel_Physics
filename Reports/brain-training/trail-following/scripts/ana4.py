import gzip, collections, statistics as st
exec(open('ana1.py').read().split('for run in sys.argv')[0])
import sys
arms=sys.argv[1:]
def home_food(run):
    n=0
    for l in open(f'{run}/events.csv'):
        p=l.split(',')
        if len(p)>3 and p[2]=='deliver':
            try:
                if abs(int(p[3])-128)<=8: n+=1
            except: pass
    return n
def field(run): return sum(1 for l in open(f'{run}/events.csv') if ',field,' in l)
print('arm         | left nest early (of 24, median) | alive at 200k (median, seeds with 0) | field bites (median) | deliveries home (median)')
for a in arms:
    left=[];alive=[];fb=[];hf=[]
    for s in range(1,17):
        far=collections.defaultdict(int)
        for r in rows(f'{a}-{s}',2200): far[r['id']]=max(far[r['id']],abs(r['x']-128))
        left.append(sum(v>15 for v in far.values()))
        alive.append(int([l for l in open(f'{a}-{s}.log') if l.startswith('frame  200000')][0].split()[3]))
        fb.append(field(f'{a}-{s}')); hf.append(home_food(f'{a}-{s}'))
    print(f"{a:11s} | {st.median(left):5.1f} | {st.median(alive):5.1f}  dead out {sum(v==0 for v in alive):2d}/16 | {st.median(fb):7.0f} | {st.median(hf):6.0f}")
