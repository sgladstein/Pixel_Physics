"""usage: judge.py ARM... -- per arm over bed seeds 1-32 (200k frames, sel=off): home food (|x-128|<=8), deaths,
collapsed (>=12 of 24 dead), crater near the nest (nearpit.py's 'near': sky-open cells below ground within 20 cols,
mouth +-4 excluded, final map), nest dug/chambers (nest.csv final row), new cuts ('dig'), re-cuts, spoil let go:
outside within 5 cols of the door vs farther, and inside ('in'). Paired by seed vs the first arm."""
import sys, csv, os, glob, statistics as st
S = os.path.dirname(os.path.abspath(__file__)) + "/l"
def near_pit(run):
    surf = [int(l) for l in open(run + '/surface.txt')]
    m = sorted(glob.glob(run + '/map_f*.txt'))[-1]
    L = open(m).read().split('\n'); x0, y0, w, h = map(int, L[0].split())
    rows = [(r + ' ' * w)[:w] for r in L[1:1 + h]]
    near = 0
    for x in range(w):
        X = x + x0
        if abs(X - 128) <= 4: continue
        for y in range(h):
            if rows[y][x] not in '.af': break
            if y + y0 >= surf[X] and abs(X - 128) <= 20: near += 1
    return near
def one(a, s):
    d = f"{S}/{a}{s}"
    if not os.path.exists(d + "/events.csv") or not glob.glob(d + '/map_f0200000.txt'): return None
    r = dict(home=0, died=0, cuts=0, recut=0, out_door=0, out_far=0, inside=0)
    for e in csv.DictReader(open(d + "/events.csv")):
        k = e['kind']; x = int(e['x'])
        if k == 'deliver' and abs(x - 128) <= 8: r['home'] += 1
        elif k == 'death': r['died'] += 1
        elif k == 'dig': r['cuts'] += 1
        elif k == 'recut': r['recut'] += 1
        elif k == 'out': r['out_door' if abs(x - 128) <= 5 else 'out_far'] += 1
        elif k == 'in': r['inside'] += 1
    last = list(csv.DictReader(open(d + "/nest.csv")))[-1]
    r['dug'] = int(last['dug']); r['chambers'] = int(last['chambers']); r['crater'] = near_pit(d)
    return r
arms = sys.argv[1:]
res = {a: {s: one(a, s) for s in range(1, 33)} for a in arms}
base = arms[0]
keys = ['home', 'died', 'crater', 'dug', 'chambers', 'cuts', 'recut', 'out_door', 'out_far', 'inside']
print("per colony, mean (median) over seeds; 'paired' = seeds where this arm is lower / higher than", base)
for a in arms:
    ok = [s for s in res[a] if res[a][s] and res[base].get(s)]
    if not ok: print(a, "no seeds"); continue
    col = sum(res[a][s]['died'] >= 12 for s in ok)
    print(f"\n{a}: {len(ok)} seeds, collapsed (>=12 of 24 dead) {col}")
    for k in keys:
        v = [res[a][s][k] for s in ok]; b = [res[base][s][k] for s in ok]
        lo = sum(x < y for x, y in zip(v, b)); hi = sum(x > y for x, y in zip(v, b))
        print(f"  {k:9s} {st.mean(v):8.1f} ({st.median(v):6.0f})   paired lower/higher {lo:2d}/{hi:2d}")
    od = sum(res[a][s]['out_door'] for s in ok); of = sum(res[a][s]['out_far'] for s in ok)
    print(f"  share of spoil let go outside within 5 cols of the door: {od / max(1, od + of):.0%}")
