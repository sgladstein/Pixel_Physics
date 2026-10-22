import gzip, collections
exec(open('ana1.py').read().split('for run in sys.argv')[0])
print('seed | c1: ants past 15 cols by frame 2,200, alive at 200k | graft: same')
for s in range(1,17):
    out=[]
    for arm in ('c1','c1g'):
        far=collections.defaultdict(int)
        for r in rows(f'{arm}-{s}',2200):
            far[r['id']]=max(far[r['id']],abs(r['x']-128))
        alive=[l for l in open(f'{arm}-{s}.log') if l.startswith('frame  200000')][0].split()[3]
        out.append(f"{sum(v>15 for v in far.values()):2d}/{len(far)} alive {alive:>2}")
    print(f'{s:2d} | {out[0]} | {out[1]}')
