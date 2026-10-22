"""SCRATCH: per-arm summary of an arms batch dir (teacher lane, 2026-10-21). usage: armsum.py DIR
Per arm: food delivered and starved per seed (all rounds), extinct seeds and the round they died in, colonies healthy
(12+ ants) at the end of their last round, and food delivered per round among seeds still alive in that round."""
import gzip, sys, os, csv, glob, re
D = sys.argv[1]
arms = sorted({re.sub(r'_\d+$', '', os.path.basename(p)) for p in glob.glob(D + '/*_*') if os.path.isdir(p)})
for a in arms:
    food = st = ext = ok = n = 0; died = []; per = {}; ns = []; pit = []
    for d in sorted(glob.glob(f'{D}/{a}_*')):
        n += 1
        rs = sorted(glob.glob(d + '/r*'), key=lambda p: int(p.rsplit('r', 1)[1]))
        for r in rs:
            fr = 0
            evf = r + '/events.csv' if os.path.exists(r + '/events.csv') else (r + '/events.csv.gz' if os.path.exists(r + '/events.csv.gz') else None)
            for e in csv.DictReader(gzip.open(evf, 'rt') if evf and evf.endswith('.gz') else open(evf)) if evf else []:
                fr += e['kind'] == 'deliver'
                st += e['kind'] == 'death' and e['v'] == '1'
            food += fr; per.setdefault(int(r.rsplit('r', 1)[1]), []).append(fr)
        if os.path.exists(d + '/EXTINCT'):
            ext += 1; died.append(len(rs) - 1)
        elif rs:
            f = rs[-1] + '/ants_f0200000.txt'
            if os.path.exists(f) and sum(1 for _ in open(f)) >= 12: ok += 1
            nf = rs[-1] + '/nest.csv'
            rows = list(csv.DictReader(open(nf))) if os.path.exists(nf) else []
            if rows:
                ns.append(float(rows[-1]['nest_score'])); pit.append(int(rows[-1]['pit']))
    mean = lambda v: sum(v) / len(v) if v else float('nan')
    pr = ' '.join(f'r{k}:{sum(v)/len(v):.0f}(n{len(v)})' for k, v in sorted(per.items()))
    print(f'{a:8s} seeds {n}: food/seed {food/max(n,1):7.0f}  starved/seed {st/max(n,1):5.0f}  extinct {ext} (rounds {sorted(died)})  healthy {ok}  final nest score {mean(ns):.1f} pit {mean(pit):.1f} (n{len(ns)}) | food per round {pr}')
