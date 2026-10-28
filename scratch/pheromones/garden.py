#!/usr/bin/env python3
"""SCRATCH (evolvable-pheromones lane, 2026-10-27): **the pre-registered judge for E4**, written and committed before
E4 runs (the readiness review's change 6, `Reports/brain-training/evolvable-pheromones-e4-readiness-2026-10-27.md`).
The question (owner, 2026-10-27 18:49Z): *can ants evolve their own signals that communicate information between
ants?* -- with food delivered home as the guard (`evolvable-pheromones-2026-10-25.md` §9.10).

  python3 scratch/pheromones/garden.py OUT free=RUN,RUN,... deaf=RUN,RUN,... [P=4] [DEAFALSO=B] [FRAMES=100000]
      [PATCHES=T:LO:HI] [SCORE=heaphome]

Each RUN is an `emerge.py` OUT directory; its last generation with a `done` file is judged. Run with the same
environment the runs used (BIN, PIXEL_PHYSICS_ALARM_GROUND). Seeds start at 5,000,000: never a training seed
(SEED*1000003 + ...) or a listening seed (900000 + ...).

Stage A  every genome of the last generation, 4 worlds (5,000,000-3): picks each run's top 4 on worlds selection
         never saw, so a lucky mutant is not crowned on its training draw.
Stage B  the top 4, 8 fresh worlds (5,000,100-7): the run's value = mean food delivered home over 32 runs.
Stage C  (free runs only) the run's best from stage B on 16 worlds (5,000,200-15): normal, deaf to the channel,
         the channel read 40 columns off, a constant level 0.02; plus an emit census on one world (5,000,300).

Criteria for "this run evolved a communicating signal" (all fixed here, before the runs):
  1 information  the alarm emit by sender state (empty vs laden, at nest vs away) differs >= 3x, the higher >= 0.25
                 (a founder's random N(0, 0.05) row already differs 2.5x at ~0.2, so 2x/0.05 would pass noise)
  2 response     deaf to the channel: food delivered lower on >= 12/16 and the median lower by >= 20%
  3 meaning      the map 40 columns off AND the constant level: each lower on >= 12/16, median lower by >= 20%
  (4 social, the half-silent chimera, is checked by hand on passing genomes; it is not automated here)

Verdict (E4, 6 free + 6 deaf replicates):
  SIGNALS FORM   >= 3 of 6 free runs meet 1-3, AND the guard holds: the geometric mean of free/deaf stage-B food
                 delivered (runs paired by index) >= 0.90
  DEAD END       0 of 6 free runs meet 1-3
  INCONCLUSIVE   anything else; next step: more replicates of the same harness, not a new design
Reported beside it, not deciding: free vs deaf one-sided paired t on log(stage B), larder kJ, tank, starved, alive.
"""
import csv, json, math, os, statistics as st, subprocess, sys, threading
from concurrent.futures import ThreadPoolExecutor

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from heaphome import heaphome  # noqa: E402
BIN = os.environ.get('BIN', 'target/release/examples/inrun')
OUT = sys.argv[1]
kv = dict(a.split('=', 1) for a in sys.argv[2:])
P = int(kv.get('P', 4))
RUNS = {arm: [r for r in kv.get(arm, '').split(',') if r] for arm in ('free', 'deaf')}
BED = ['sel=off', f"frames={kv.get('FRAMES', 100000)}", 'dist=100', 'food=mix', f"patches={kv.get('PATCHES', '20000:30:110')}", 'maps=0']
# SCORE=heaphome (2026-10-28): judge on deliveries whose trip began at the heap, which the door loop cannot farm (§9.13)
HEAPHOME = kv.get('SCORE') == 'heaphome'
BASE = {**os.environ, 'RAYON_NUM_THREADS': '1', 'PP_MIND_ALARM': 'train'}
DEAFALSO = [kv['DEAFALSO']] if 'DEAFALSO' in kv else []
os.makedirs(OUT, exist_ok=True)


def env(arm, extra=None):
    deaf = DEAFALSO + (['alarm'] if arm == 'deaf' else [])
    e = {**BASE, **({'PP_SCENT_DEAF': ','.join(deaf)} if deaf else {})}
    return {**e, **(extra or {})}


def score(d):
    r = dict(home=0, kj=0.0, tank=0.0, alive=0)
    try:
        last = list(csv.DictReader(open(f'{d}/stock.csv')))[-1]
        r['alive'], r['tank'] = int(last['alive']), float(last['tank_mean'])
        r['kj'] = sum(float(v) for k, v in list(csv.DictReader(open(f'{d}/larder.csv')))[-1].items() if k.endswith('_kj'))
        r['home'] = int(list(csv.DictReader(open(f'{d}/nest.csv')))[-1]['food_home'])
        if HEAPHOME:
            r['home'] = heaphome(d)
    except (OSError, IndexError, KeyError):
        pass
    for fn in os.listdir(d) if os.path.isdir(d) else []:
        if fn.startswith(('map_f', 'ants_f', 'nestmap_f', 'occupants')) or fn in ('events.csv', 'births.csv', 'age30.csv'):
            os.remove(f'{d}/{fn}')
    return r


def run(job):
    genome, seed, out, e = job
    if not os.path.exists(f'{out}/done'):  # a run killed mid-way (container restart) leaves a partial nest.csv: redo it
        os.makedirs(out, exist_ok=True)
        with open(f'{out}.log', 'w') as lf:
            subprocess.run([BIN, f'founders={genome}:30', f'seed={seed}', f'out={out}'] + BED, stdout=lf, stderr=subprocess.STDOUT, env=e)
        r = score(out)
        open(f'{out}/done', 'w').close()
        return r
    return score(out)


def pool(jobs):
    with ThreadPoolExecutor(P) as ex:
        return list(ex.map(run, jobs))


def last_gen(r):
    g = 0
    while os.path.exists(f'{r}/g{g:03}/done'):
        g += 1
    assert g > 0, f'{r}: no finished generation'
    return f'{r}/g{g - 1:03}'


def count(lo, base, k):
    n = sum(1 for a, b in zip(lo, base) if a < b)
    return n, st.median(lo) <= (1 - k) * st.median(base)


res = {'free': [], 'deaf': []}
for arm, runs in RUNS.items():
    for i, r in enumerate(runs):
        gd, tag = last_gen(r), f'{arm}{i}'
        genomes = sorted(f'{gd}/{f}' for f in os.listdir(gd) if f.endswith('.raw.txt'))
        a = pool([(g, 5_000_000 + k, f'{OUT}/{tag}/A/{os.path.basename(g)[:3]}_s{k}', env(arm)) for g in genomes for k in range(4)])
        mean_a = [st.mean(x['home'] for x in a[j * 4:(j + 1) * 4]) for j in range(len(genomes))]
        top = sorted(range(len(genomes)), key=lambda j: (-mean_a[j], j))[:4]
        b = pool([(genomes[j], 5_000_100 + k, f'{OUT}/{tag}/B/{os.path.basename(genomes[j])[:3]}_s{k}', env(arm)) for j in top for k in range(8)])
        best = top[max(range(4), key=lambda t: (st.mean(x['home'] for x in b[t * 8:(t + 1) * 8]), -t))]
        row = dict(run=r, gen=gd, top=[os.path.basename(genomes[j]) for j in top], best=os.path.basename(genomes[best]),
                   value=st.mean(x['home'] for x in b), kj=st.mean(x['kj'] for x in b), tank=st.mean(x['tank'] for x in b),
                   extinct=sum(x['alive'] == 0 for x in b))
        if arm == 'free':
            g = genomes[best]
            C = {}
            for name, extra in (('normal', {}), ('deaf', {'PP_SCENT_DEAF': ','.join(DEAFALSO + ['alarm'])}),
                                ('shift', {'PP_SCENT_SHIFT': 'alarm:40'}), ('const', {'PP_SCENT_CONST': 'alarm:0.02'})):
                C[name] = [x['home'] for x in pool([(g, 5_000_200 + k, f'{OUT}/{tag}/C/{name}_s{k}', env('free', extra)) for k in range(16)])]
            fifo = f'{OUT}/{tag}/census.fifo'
            os.makedirs(os.path.dirname(fifo), exist_ok=True)
            if os.path.exists(fifo):
                os.remove(fifo)
            os.mkfifo(fifo)
            cen = {}

            def read_census():
                out = subprocess.run([sys.executable, f'{HERE}/census.py', fifo], capture_output=True, text=True).stdout
                for line in out.split('\n'):
                    if 'alarm' in line:
                        cen[line[:28].strip()] = float(line.rsplit('alarm', 1)[1])
            t = threading.Thread(target=read_census)
            t.start()
            run((g, 5_000_300, f'{OUT}/{tag}/census', env('free', {'PP_MIND_LOG': fifo, 'PP_MIND_LOG_EMITS': '1'})))
            t.join()
            pairs = [('empty', 'laden'), ('at_nest empty', 'away empty'), ('at_nest laden', 'away laden')]
            ratios = [max(cen.get(p, 0), cen.get(q, 0)) / max(1e-6, min(cen.get(p, 0), cen.get(q, 0))) for p, q in pairs
                      if max(cen.get(p, 0), cen.get(q, 0)) >= 0.25]
            crit = {'information': bool(ratios) and max(ratios) >= 3.0}
            for name in ('deaf', 'shift', 'const'):
                n, big = count(C[name], C['normal'], 0.20)
                row[f'{name}_lower'] = n
                crit[name] = n >= 12 and big
            row.update(census=cen, max_state_ratio=max(ratios) if ratios else 0.0, C_median={k: st.median(v) for k, v in C.items()},
                       criteria=crit, communicates=crit['information'] and crit['deaf'] and crit['shift'] and crit['const'])
        res[arm].append(row)
        json.dump(res, open(f'{OUT}/garden.json', 'w'), indent=1)
        print(json.dumps(row), flush=True)

n_comm = sum(r['communicates'] for r in res['free'])
k = min(len(res['free']), len(res['deaf']))
logr = [math.log(max(1.0, res['free'][i]['value']) / max(1.0, res['deaf'][i]['value'])) for i in range(k)]
guard = math.exp(st.mean(logr)) if logr else float('nan')
t = (st.mean(logr) / (st.stdev(logr) / math.sqrt(k))) if k > 1 and st.stdev(logr) > 0 else float('nan')
verdict = ('SIGNALS FORM' if n_comm >= 3 and guard >= 0.90 else 'DEAD END' if n_comm == 0 else 'INCONCLUSIVE')
print(f'free runs communicating {n_comm}/{len(res["free"])}; guard free/deaf geometric mean {guard:.2f} (>= 0.90); '
      f'paired t on log ratio {t:.2f} over {k} pairs (reported, not deciding)\nVERDICT: {verdict}')
json.dump({**res, 'n_communicating': n_comm, 'guard': guard, 't': t, 'verdict': verdict}, open(f'{OUT}/garden.json', 'w'), indent=1)
