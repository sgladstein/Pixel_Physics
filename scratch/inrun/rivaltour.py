#!/usr/bin/env python3
"""SCRATCH (colony-vs-colony thread, 2026-10-24): **a tournament of colonies on the two-colony bed.**

The owner asked whether selecting colonies for how they do beside a rival would just breed peaceful pairs. This tries
it. Each round the colonies are paired at random; each pair plays two matches on `examples/rivalbed.rs` (sides swapped,
different seeds); a colony scores, per match, its share of the pair's ants over the run (`lived`) plus half its share
of the pair's nest cuts (`score=` sets the terms) -- a score against the rival, so two colonies at peace tie and whichever takes the other's
ground wins. Inside a match every hatchling is a mutant of its layer (`mutate=`), so each colony is under natural
selection too. The winner's survivors (`save=`) found both of the pair's colonies next round, one from each match.

    python3 scratch/inrun/rivaltour.py OUT [ncol=12] [rounds=16] [frames=60000] [P=4] [bin=PATH] [seed0=0] -- rivalbed args...

The args after `--` go to every match (e.g. `gap=40 food=split bite=keep,keep mutate=0.013:1:4`). Writes
OUT/tour.csv (one row per colony per match) and OUT/rR/... (each match's out= and save= dirs). Resumes from the last
finished round's state.json.
"""
import json, os, random, re, subprocess, sys
from concurrent.futures import ThreadPoolExecutor

OUT = sys.argv[1]
own, extra = (sys.argv[2:sys.argv.index('--')], sys.argv[sys.argv.index('--') + 1:]) if '--' in sys.argv else (sys.argv[2:], [])
opt = dict(a.split('=', 1) for a in own)
NCOL, ROUNDS, FRAMES, P = int(opt.get('ncol', 12)), int(opt.get('rounds', 16)), int(opt.get('frames', 60000)), int(opt.get('P', 4))
BIN = opt.get('bin', 'target/release/examples/rivalbed')
SEED0 = int(opt.get('seed0', 0))
# score=KEY:W,...: a colony's match score is the sum of W x its share of the pair's KEY (a RESULT field). Default the
# first tournament's, lived:1,nest_cuts:0.5 -- whose champions stopped carrying food home and cut 5,000 cells a match.
SCORE = [(k, float(w)) for k, w in (p.split(':') for p in opt.get('score', 'lived:1,nest_cuts:0.5').split(','))]
assert NCOL % 2 == 0, 'ncol must be even'
os.makedirs(OUT, exist_ok=True)
state_p = f'{OUT}/state.json'
if os.path.exists(state_p):
    st = json.load(open(state_p))
else:
    st = {'round': 0, 'carry': {str(c): 'std' for c in range(NCOL)}}
log = open(f'{OUT}/tour.csv', 'a')
if os.path.getsize(f'{OUT}/tour.csv') == 0:
    log.write('round,pair,match,colony,side,seed,lived,alive,nest_cuts,delivered,kills_of_foe,killed,starved,bites_given,score,won\n')


def match(r, pi, m, west, east, seed):
    d = f'{OUT}/r{r}/p{pi}m{m}'
    os.makedirs(d, exist_ok=True)
    args = [BIN, f'seed={seed}', f'frames={FRAMES}', f'pool_a={st["carry"][str(west)]}', f'pool_b={st["carry"][str(east)]}', f'out={d}', f'save={d}/save'] + extra
    res = subprocess.run(args, capture_output=True, text=True)
    open(f'{d}/run.log', 'w').write(' '.join(args) + '\n' + res.stdout + res.stderr)
    rows = {}
    for l in res.stdout.splitlines():
        if l.startswith('RESULT'):
            kv = dict(x.split('=', 1) for x in l.split()[1:] if '=' in x)
            rows[kv['side']] = kv
    assert len(rows) == 2, f'{d}: match failed ({res.returncode}): {res.stderr[-400:]}'
    return d, rows


def share(a, b):
    return 0.5 if a + b == 0 else a / (a + b)


for r in range(st['round'], ROUNDS):
    rng = random.Random(1000 * SEED0 + r)
    order = list(range(NCOL))
    rng.shuffle(order)
    pairs = [(order[2 * k], order[2 * k + 1]) for k in range(NCOL // 2)]
    jobs = []
    for pi, (i, j) in enumerate(pairs):
        jobs.append((pi, 0, i, j, 100000 * SEED0 + 1000 * r + 10 * pi + 1))
        jobs.append((pi, 1, j, i, 100000 * SEED0 + 1000 * r + 10 * pi + 2))
    with ThreadPoolExecutor(P) as ex:
        done = list(ex.map(lambda jb: (jb, match(r, *jb)), jobs))
    score = {c: 0.0 for c in range(NCOL)}
    best = {}  # colony -> [(match score, its save dir)]
    rows_out = []
    for (pi, m, west, east, seed), (d, rows) in done:
        for side, c, other in (('west', west, 'east'), ('east', east, 'west')):
            a, b = rows[side], rows[other]
            s = sum(wt * share(float(a[k]), float(b[k])) for k, wt in SCORE)
            score[c] += s
            best.setdefault(c, []).append((s, f'{d}/save/{side}', int(a['alive'])))
            rows_out.append([r, pi, m, c, side, seed, a['lived'], a['alive'], a['nest_cuts'], a['delivered'], a['kills_of_foe'], a['killed'], a['starved'], a.get('bites_given', ''), f'{s:.4f}'])
    nxt = dict(st['carry'])
    winners = {}
    for pi, (i, j) in enumerate(pairs):
        w = i if (score[i], rng.random()) > (score[j], rng.random()) else j
        winners[pi] = w
        # The winner's survivors from each match it played: one founds the winner, the other the loser's slot.
        saves = [p for s, p, alive in sorted(best[w], reverse=True) if alive > 0 and os.path.isdir(p) and os.listdir(p)]
        if saves:
            nxt[str(w)] = saves[0]
            nxt[str(j if w == i else i)] = saves[-1]
        else:
            nxt[str(j if w == i else i)] = st['carry'][str(w)]
    for row in rows_out:
        log.write(','.join(map(str, row + [int(winners[row[1]] == row[3])])) + '\n')
    log.flush()
    st = {'round': r + 1, 'carry': nxt}
    json.dump(st, open(state_p, 'w'))
    # Saves nobody carries are 0.9 MB a side; keep only the carried ones.
    import shutil
    keep = set(nxt.values())
    for (pi, m, west, east, seed), (d, rows) in done:
        for side in ('west', 'east'):
            if f'{d}/save/{side}' not in keep:
                shutil.rmtree(f'{d}/save/{side}', ignore_errors=True)
    kills = sum(int(x[10]) for x in rows_out)
    print(f'round {r}: kills {kills}, mean lived {sum(float(x[6]) for x in rows_out) / len(rows_out):.1f}, winners {sorted(winners.values())}', flush=True)
