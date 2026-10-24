"""judge.py RUNSDIR ARM... : per arm over the seeds present, mean (median); paired vs first arm (wins/losses)."""
import sys, csv, os, glob, statistics as st
sys.path.insert(0, '/home/user/cw/scratch/inrun'); import nestcount
R = sys.argv[1]; arms = sys.argv[2:]
def one(d):
    if not os.path.exists(d + '/nestmap.txt'): return None
    r = dict(home=0, died=0, starved=0)
    for line in open(d + '/events.csv'):
        p = line.split(',')
        if p[2] == 'deliver' and abs(int(p[3]) - 128) <= 8: r['home'] += 1
        elif p[2] == 'death':
            r['died'] += 1; r['starved'] += p[5].strip() == '1'
    s = list(csv.DictReader(open(d + '/stock.csv')))[-1]
    r['alive'] = int(s['alive']); r['nest_food'] = int(s['nest_food']); r['tank'] = float(s['tank_mean'])
    n = nestcount.count(d, 200000)
    r['nest'] = n['nest']; r['crater'] = n['crater']; r['nestcuts'] = n.get('nestcuts') or 0
    return r
res = {a: {os.path.basename(d): one(d) for d in glob.glob(f'{R}/{a}/*') if os.path.isdir(d)} for a in arms}
keys = ['home', 'died', 'starved', 'alive', 'nest', 'crater', 'nestcuts', 'nest_food', 'tank']
b = arms[0]
for a in arms:
    ok = {s: v for s, v in res[a].items() if v}
    print(f'{a}: n={len(ok)}  collapsed(alive<12)={sum(1 for v in ok.values() if v["alive"] < 12)}')
    for k in keys:
        xs = [v[k] for v in ok.values()]
        pw = [(v[k], res[b][s][k]) for s, v in ok.items() if res[b].get(s)]
        hi = sum(1 for x, y in pw if x > y); lo = sum(1 for x, y in pw if x < y)
        print(f'   {k:10s} {st.mean(xs):9.1f} ({st.median(xs):8.1f})   vs {b}: higher {hi:2d} lower {lo:2d}')
