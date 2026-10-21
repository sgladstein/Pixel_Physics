import re,sys,glob,collections
# usage: summarize.py RUNS_DIR ARM [ARM...]  -> reads RUNS_DIR/ARM_{mix}_s{n}.txt (plant_m0 mode=community logs)
d,arms=sys.argv[1],sys.argv[2:]
pat=re.compile(r' \| (\w+) plants=(\d+) bank=(\d+) leaf=(\d+) est=(\d+)')
for mix in ['roles','roles4']:
  for arm in arms:
    runs={}
    for s in [1,2,3,4]:
      rows={}
      try: lines=open(f'{d}/{arm}_{mix}_s{s}.txt').read().splitlines()
      except FileNotFoundError: continue
      for l in lines:
        if not l.startswith('f='): continue
        f=int(l.split(' ',1)[0][2:]); rows[f]={m[0]:tuple(map(int,m[1:])) for m in pat.findall(l)}
      runs[s]=rows
    if not runs: continue
    print(f'=== {mix} {arm}: seeds {sorted(runs)}  (plants/leaf per seed; est = established in the window ending at the frame, summed over the last 240k)')
    kinds=sorted({k for r in runs.values() for row in r.values() for k in row})
    for F in [240000,480000,720000]:
      if not all(F in r for r in runs.values()): continue
      print(f' f={F}')
      for k in kinds:
        cells=[];ests=[]
        for s,r in sorted(runs.items()):
          p,b,l,e=r[F].get(k,(0,0,0,0)); cells.append(f'{p}/{l}')
          ests.append(sum(r[f].get(k,(0,0,0,0))[3] for f in r if F-240000<f<=F))
        print(f'   {k:8} {"  ".join(f"{c:>10}" for c in cells)}   est {ests}')
      print('   kinds 100+ leaf:', [sum(1 for k in kinds if r[F].get(k,(0,0,0,0))[2]>=100) for s,r in sorted(runs.items())])
