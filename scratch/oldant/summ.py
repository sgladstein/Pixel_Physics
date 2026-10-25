"""SCRATCH (old-ant thread): summarise ablation RESULT lines: arm, n, median alive, median lived, median starved, alive per seed."""
import sys, re, glob, os, statistics as st
d = sys.argv[1]; sc = sys.argv[2] if len(sys.argv) > 2 else ''
rows = {}
for f in glob.glob(os.path.join(d, f'*.{sc}*.txt') if sc else os.path.join(d, '*.txt')):
    arm, scn, seed = os.path.basename(f)[:-4].split('.')
    t = open(f).read()
    m = re.search(r'RESULT alive=(\d+) lived=([\d.]+) peak=(\d+) starved=(\d+) births=(\d+)', t)
    if not m: print('MISSING', f); continue
    rows.setdefault((arm, scn), {})[int(seed[1:])] = tuple(float(x) for x in m.groups())
base = {k[1]: v for k, v in rows.items() if k[0] == 'base'}
print(f"{'arm':12} {'scen':18} n  alive  lived starved births  worse_than_base  alive_by_seed")
for (arm, scn), v in sorted(rows.items(), key=lambda kv: (kv[0][1], st.median([x[0] for x in kv[1].values()]))):
    seeds = sorted(v)
    al = [v[s][0] for s in seeds]
    b = base.get(scn, {})
    worse = sum(1 for s in seeds if s in b and v[s][0] < b[s][0])
    print(f"{arm:12} {scn:18} {len(seeds)} {st.median(al):6.0f} {st.median([v[s][1] for s in seeds]):6.1f} {st.median([v[s][3] for s in seeds]):6.0f} {st.median([v[s][4] for s in seeds]):6.0f}  {worse}/{len(seeds)}  {' '.join(str(int(x)) for x in al)}")
