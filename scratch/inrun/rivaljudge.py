#!/usr/bin/env python3
"""SCRATCH (colony-vs-colony thread, 2026-10-24): **judge pools head to head on the two-colony bed.**

Each pool in POOLS meets OPP on `seeds` fresh seeds, once on each side (so 2 x seeds matches a pool), with no
mutation: the pools as carried, judged on a bed none of them trained on.

    python3 scratch/inrun/rivaljudge.py OUT OPP POOL [POOL...] [seeds=6] [seed0=9000] [frames=60000] [P=4] [bin=PATH] -- rivalbed args...

Prints per pool: matches won on the tournament's own score (share of ants lived + half the share of nest cuts),
median share of ants lived, nest cuts and food home against the opponent's, kills both ways, colonies died out.
Writes OUT/judge.csv.
"""
import os, statistics as st, subprocess, sys
from concurrent.futures import ThreadPoolExecutor

OUT, OPP = sys.argv[1], sys.argv[2]
own, extra = (sys.argv[3:sys.argv.index('--')], sys.argv[sys.argv.index('--') + 1:]) if '--' in sys.argv else (sys.argv[3:], [])
pools = [a for a in own if '=' not in a]
opt = dict(a.split('=', 1) for a in own if '=' in a)
SEEDS, SEED0, FRAMES, P = int(opt.get('seeds', 6)), int(opt.get('seed0', 9000)), int(opt.get('frames', 60000)), int(opt.get('P', 4))
BIN = opt.get('bin', 'target/release/examples/rivalbed')
os.makedirs(OUT, exist_ok=True)


def run(job):
    k, pool, seed, me = job
    a, b = (pool, OPP) if me == 'west' else (OPP, pool)
    res = subprocess.run([BIN, f'seed={seed}', f'frames={FRAMES}', f'pool_a={a}', f'pool_b={b}', f'out={OUT}/p{k}-s{seed}-{me}'] + extra, capture_output=True, text=True)
    rows = {}
    for l in res.stdout.splitlines():
        if l.startswith('RESULT'):
            kv = dict(x.split('=', 1) for x in l.split()[1:] if '=' in x)
            rows[kv['side']] = kv
    assert len(rows) == 2, res.stderr[-400:]
    return job, rows[me], rows['east' if me == 'west' else 'west']


jobs = [(k, p, SEED0 + s, me) for k, p in enumerate(pools) for s in range(SEEDS) for me in ('west', 'east')]
with ThreadPoolExecutor(P) as ex:
    done = list(ex.map(run, jobs))
f = open(f'{OUT}/judge.csv', 'w')
f.write('pool,seed,side,lived,opp_lived,nest,opp_nest,delivered,opp_delivered,kills,killed,alive,opp_alive\n')


def share(x, y):
    return 0.5 if x + y == 0 else x / (x + y)


for k, pool in enumerate(pools):
    mine = [(j, a, b) for j, a, b in done if j[0] == k]
    for j, a, b in mine:
        f.write(','.join(map(str, [k, j[2], j[3], a['lived'], b['lived'], a['nest_cuts'], b['nest_cuts'], a['delivered'], b['delivered'], a['kills_of_foe'], a['killed'], a['alive'], b['alive']])) + '\n')
    sc = [share(float(a['lived']), float(b['lived'])) + 0.5 * share(float(a['nest_cuts']), float(b['nest_cuts'])) for _, a, b in mine]
    med = lambda fn: st.median(fn(a, b) for _, a, b in mine)
    print(f"pool {k} {pool}\n  won {sum(s > 0.75 for s in sc)}/{len(sc)} | lived share {med(lambda a, b: share(float(a['lived']), float(b['lived']))):.2f} | "
          f"nest {med(lambda a, b: float(a['nest_cuts'])):.0f} vs {med(lambda a, b: float(b['nest_cuts'])):.0f} | food home {med(lambda a, b: float(a['delivered'])):.0f} vs {med(lambda a, b: float(b['delivered'])):.0f} | "
          f"kills {med(lambda a, b: float(a['kills_of_foe'])):.0f} vs {med(lambda a, b: float(a['killed'])):.0f} | died out {sum(int(a['alive']) <= 2 for _, a, _ in mine)} vs {sum(int(b['alive']) <= 2 for _, _, b in mine)}", flush=True)
