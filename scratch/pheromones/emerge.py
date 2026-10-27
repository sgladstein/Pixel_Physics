#!/usr/bin/env python3
"""SCRATCH (evolvable-pheromones lane, 2026-10-25): **can a new scent convention evolve?** The emergence test the
adversarial review asked for (`Reports/evolvable-pheromones-2026-10-25.md`, E2), built on the conditions the
robot-colony studies needed (Floreano et al. 2007; Mitri et al. 2009, 2011):

- **High relatedness.** Every colony is founded from ONE genome (30 copies) and runs with in-run selection off, so
  hatchlings keep their layer's mind exactly: a colony is a clone.
- **Selection at the colony level only.** Colonies are ranked on stored food + nest cells at the end of the round;
  the top half are parents. The next generation is the top 2 unchanged plus mutants of uniformly chosen parents.
- **Something worth saying.** `patches=T:LO:HI`: one food heap that jumps to a random side and distance every T frames.
- **A free channel, free to evolve.** `PP_MIND_ALARM=train` opens the alarm emit row (frozen on the normal bed, and
  carrying nothing on std_pool: decode, 2026-10-25) to mutation.

Arms (same colony seeds per generation and index, so arms pair):
  free   -- as above
  deaf   -- ants cannot read the alarm plane (PP_SCENT_DEAF=alarm): the control that removes the channel
  drift  -- parents chosen at random: what mutation alone does

Every EVERY generations the best colony's genome is replayed on LISTEN seeds normal / deaf to alarm / alarm read 40
columns off; emergence = that gap growing over generations, in free and not in drift.

  python3 scratch/pheromones/emerge.py OUT ARM N GENS FRAMES FOUNDER [P=4] [EVERY=10] [LISTEN=4] [extra inrun args...]
  [EVALS=1] seeds per colony per generation, scores averaged (the review's fix: one seed predicts the next at r=0.23)
  [SCORE=both|stored|kj] stored food + nest cells, or stored food only (nest cells let a 4-ant huddle win the 1st run),
  or the larder's energy in kJ (cells count part-eaten crumbs at full weight: rep 2's 478 vs 334 cells was 157 vs 158 kJ),
  or food delivered home over the round (`home`); every one of these is logged in gens.csv whichever ranks
  [ZERO=LO-HI] zero these genome slots in the founder (2375-2409: the B emit row, for the re-evolve check)
  [FILL=SIGMA] after ZERO, give every zeroed slot a small random weight N(0, SIGMA) (seeded): the operator's
  add events almost never land on the one input a signal needs, so start the row dense and let sigma tune it
  [HARNESS=2] the review's fixes (2026-10-27): gen 0 keeps the unmutated founder, an elite is ranked on the mean of
  every score it has drawn, food stored counts even if the colony died out, listening worlds change per checkpoint
  (resumes no longer duplicate rows, and ZERO/FILL now apply to a .raw.txt founder, in either harness)
  [CHAN=alarm|B] the channel the listening replays deafen and shift; [ALARM=train|frozen] PP_MIND_ALARM
  e.g. python3 scratch/pheromones/emerge.py /tmp/em free 16 200 100000 assets/minds/std_pool.txt P=4

FOUNDER may be a .raw.txt or a pool file (member 0 is used). Resumes from the last complete generation.
"""
import csv, hashlib, json, os, random, shutil, subprocess, sys
from concurrent.futures import ThreadPoolExecutor

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, '..', 'inrun'))
import nestcount  # noqa: E402

BIN = os.environ.get('BIN', 'target/release/examples/inrun')
OUT, ARM, N, GENS, FRAMES, FOUNDER = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4]), int(sys.argv[5]), sys.argv[6]
OPTS = ('P', 'EVERY', 'LISTEN', 'SEED', 'MUT', 'EVALS', 'SCORE', 'ZERO', 'CHAN', 'ALARM', 'FILL', 'HARNESS')
opts = {k: v for k, v in (a.split('=', 1) for a in sys.argv[7:] if a.split('=', 1)[0] in OPTS)}
EXTRA = [a for a in sys.argv[7:] if a.split('=', 1)[0] not in OPTS]
P, EVERY, LISTEN, SEED = int(opts.get('P', 4)), int(opts.get('EVERY', 10)), int(opts.get('LISTEN', 4)), int(opts.get('SEED', 1))
MUT = opts.get('MUT', '0.013:1.0:4')  # pm:sigma:events, the standard line's kpm / ksigma / kevents
EVALS, SCORE, ZERO, CHAN = int(opts.get('EVALS', 1)), opts.get('SCORE', 'both'), opts.get('ZERO'), opts.get('CHAN', 'alarm')
assert ARM in ('free', 'deaf', 'drift'), ARM
H2 = opts.get('HARNESS', '1') == '2'
assert SCORE in ('both', 'stored', 'kj', 'home') and CHAN in ('alarm', 'B'), (SCORE, CHAN)
BED = ['sel=off', f'frames={FRAMES}', 'dist=100', 'food=mix', 'patches=20000:30:110', 'maps=0'] + EXTRA
ENV = {**os.environ, 'RAYON_NUM_THREADS': '1', **({'PP_MIND_ALARM': 'train'} if opts.get('ALARM', 'train') == 'train' else {})}
RUN_ENV = {**ENV, **({'PP_SCENT_DEAF': 'alarm'} if ARM == 'deaf' else {})}
os.makedirs(OUT, exist_ok=True)


def founder_raw():
    if FOUNDER.endswith('.raw.txt'):
        lines = [l for l in open(FOUNDER).read().split('\n') if l.strip()]
        header = lines[0]
        m = {int(l.split()[0]): l.split()[1] for l in lines[1:]}
        if not ZERO and 'FILL' not in opts:
            return FOUNDER
    else:
        lines = [l for l in open(FOUNDER).read().split('\n') if l.strip() and not l.startswith('#')]
        head = lines[0].split()
        header = f'genome {head[3]} {head[4]}'
        m, seen = {}, 0
        for l in lines[1:]:
            if l.startswith('member '):
                seen += 1
                if seen > 1:
                    break
                continue
            i, w = l.split()
            m[int(i)] = w
    fp = f'{OUT}/founder.raw.txt'
    with open(fp, 'w') as f:
        f.write(header + '\n')
        lo, hi = (int(x) for x in ZERO.split('-')) if ZERO else (1, 0)
        f.writelines(f'{i} {w}\n' for i, w in sorted(m.items()) if float(w) != 0.0 and not lo <= i <= hi)
        if 'FILL' in opts:
            fr = random.Random(SEED * 31 + 7)
            f.writelines(f'{i} {fr.gauss(0.0, float(opts["FILL"])):.6f}\n' for i in range(lo, hi + 1))
    return fp


def mutant(parent, dest, mseed):
    pm, sigma, events = MUT.split(':')
    r = subprocess.run([BIN, f'founders={parent}:1', f'out={OUT}/_mut', f'mutate_to={dest}:{pm}:{sigma}:{events}:{mseed}'],
                       capture_output=True, text=True, env=ENV)
    assert os.path.exists(dest), r.stdout + r.stderr


def run(genome, seed, out, env):
    os.makedirs(out, exist_ok=True)
    with open(f'{out}.log', 'w') as lf:
        subprocess.run([BIN, f'founders={genome}:30', f'seed={seed}', f'out={out}'] + BED, stdout=lf, stderr=subprocess.STDOUT, env=env)
    return score(out)


def score(d):
    r = dict(stored=0, nest=0, alive=0, starved=0, layA=0.0, layB=0.0, layAl=0.0, kj=0.0, tank=0.0, deliver=0, churn=0, home=0)
    try:
        last = list(csv.DictReader(open(f'{d}/stock.csv')))[-1]
        r['stored'] = int(last['nest_food']) + int(last['door_food'])
        r['alive'] = int(last['alive'])
        r['tank'] = float(last['tank_mean'])
    except (OSError, IndexError):
        pass
    try:  # the larder's ENERGY: at frame 100k nearly every stored cell is a part-eaten crumb, so cells overstate
        last = list(csv.DictReader(open(f'{d}/larder.csv')))[-1]  # food (E4 readiness review, 2026-10-27)
        r['kj'] = sum(float(v) for k, v in last.items() if k.endswith('_kj'))
    except (OSError, IndexError):
        pass
    try:  # food delivered home over the round (nest.csv food_home)
        r['home'] = int(list(csv.DictReader(open(f'{d}/nest.csv')))[-1]['food_home'])
    except (OSError, IndexError, KeyError):
        pass
    if r['alive'] and os.path.exists(f'{d}/nestmap.txt'):
        r['nest'] = nestcount.count(d, FRAMES).get('nest', 0)
    try:
        with open(f'{d}/events.csv') as f:
            next(f)
            for line in f:  # counted here because events.csv is deleted below and cannot be re-scored later
                k = line.split(',', 3)[2]
                if k == 'death' and line.rstrip().split(',')[5] == '1':
                    r['starved'] += 1
                elif k in ('deliver', 'churn'):
                    r[k] += 1
    except (OSError, StopIteration):
        pass
    try:
        for line in open(f'{d}.log'):
            if 'scent laid' in line:
                t = line.split('scent laid')[1].split()
                r['layA'], r['layB'], r['layAl'] = float(t[1]), float(t[3]), float(t[5])
    except OSError:
        pass
    food = {'kj': r['kj'], 'home': r['home']}.get(SCORE, r['stored'])
    if H2:  # food stored counts even if the colony died out: removes the extinction cliff (review 2026-10-27)
        r['score'] = food + (r['nest'] if SCORE == 'both' and r['alive'] else 0)
    else:
        r['score'] = (food + (r['nest'] if SCORE == 'both' else 0)) if r['alive'] else 0
    for fn in os.listdir(d):  # keep the small files only
        if fn.startswith(('map_f', 'ants_f', 'nestmap_f', 'occupants')) or fn in ('events.csv', 'births.csv', 'age30.csv'):
            os.remove(f'{d}/{fn}')
    return r


gens_csv = f'{OUT}/gens.csv'
listen_csv = f'{OUT}/listen.csv'
g0 = 0
while os.path.exists(f'{OUT}/g{g0:03}/done'):
    g0 += 1
print(f'emerge: arm {ARM}, {N} colonies, gens {g0}..{GENS - 1}, {FRAMES} frames, EVALS={EVALS} SCORE={SCORE} ZERO={ZERO} CHAN={CHAN} ALARM={opts.get("ALARM", "train")}, bed {" ".join(BED)}', flush=True)
if g0 > 0:  # a resumed run re-writes the generation it stopped in: drop that generation's partial rows
    for fn in (gens_csv, listen_csv):
        if os.path.exists(fn):
            L = open(fn).read().split('\n')
            open(fn, 'w').write('\n'.join([L[0]] + [l for l in L[1:] if l.strip() and int(l.split(',')[0]) < g0]) + '\n')
hist_json = f'{OUT}/hist.json'
hist = json.load(open(hist_json)) if H2 and os.path.exists(hist_json) else {}
# hist.json is written before a generation's `done`, so a resume would count that generation twice: entries are
# (gen, score) and every one from a generation being re-run is dropped
hist = {k: [e for e in v if isinstance(e, list) and e[0] < g0] for k, v in hist.items()}
if g0 == 0:
    os.makedirs(f'{OUT}/g000', exist_ok=True)
    base = founder_raw()
    for c in range(N):
        if H2 and c == 0:  # gen 0 keeps one unmutated founder
            shutil.copy(base, f'{OUT}/g000/c00.raw.txt')
        else:
            mutant(base, f'{OUT}/g000/c{c:02}.raw.txt', SEED * 100000 + c)
    with open(gens_csv, 'w') as f:
        f.write('gen,colony,parent,score,stored,nest,alive,starved,layA,layB,layAl,kj,tank,deliver,churn,home\n')
    with open(listen_csv, 'w') as f:
        f.write('gen,genome,seed,normal_score,deaf_score,shift_score,normal_stored,deaf_stored,shift_stored\n')
parents_of = {}
for g in range(g0, GENS):
    gd = f'{OUT}/g{g:03}'
    os.makedirs(gd, exist_ok=True)
    genomes = [f'{gd}/c{c:02}.raw.txt' for c in range(N)]
    with ThreadPoolExecutor(P) as ex:
        flat = list(ex.map(lambda j: run(genomes[j[0]], SEED * 1000003 + g * 1000 + j[0] + 100 * j[1],
                                         f'{gd}/r{j[0]:02}' + (f'_{j[1]}' if j[1] else ''), RUN_ENV),
                           [(c, e) for c in range(N) for e in range(EVALS)]))
    res = [{k: sum(r[k] for r in flat[c * EVALS:(c + 1) * EVALS]) / EVALS for k in flat[0]} for c in range(N)]
    for r in res:
        for k in ('score', 'stored', 'nest', 'alive', 'starved'):
            r[k] = round(r[k])
    rng = random.Random(SEED * 7919 + g)
    rank = [r['score'] for r in res]
    if H2:  # an elite carries its score history: a lucky mutant must beat its running mean, not one draw
        keys = [hashlib.sha1(open(gf, 'rb').read()).hexdigest()[:16] for gf in genomes]
        for c, k in enumerate(keys):
            hist.setdefault(k, []).append([g, res[c]['score']])
            rank[c] = sum(e[1] for e in hist[k]) / len(hist[k])
        hist = {k: hist[k] for k in set(keys)}
        json.dump(hist, open(hist_json, 'w'))
    order = sorted(range(N), key=lambda c: (-rank[c], c))
    if ARM == 'drift':
        order = list(range(N))
        rng.shuffle(order)
    par = order[:max(2, N // 2)]
    with open(gens_csv, 'a') as f:
        for c in range(N):
            r = res[c]
            f.write(f"{g},{c},{parents_of.get((g, c), '')},{r['score']},{r['stored']},{r['nest']},{r['alive']},{r['starved']},{r['layA']:.0f},{r['layB']:.0f},{r['layAl']:.0f},{r['kj']:.1f},{r['tank']:.2f},{r['deliver']:.1f},{r['churn']:.1f},{r['home']:.0f}\n")
    best = order[0] if ARM != 'drift' else max(range(N), key=lambda c: (res[c]['score'], -c))
    if EVERY and (g % EVERY == EVERY - 1 or g == GENS - 1):
        ld = f'{gd}/listen'
        jobs = []
        for k in range(LISTEN):
            s = 900000 + k + (1000 * g if H2 else 0)  # HARNESS=2: fresh listening worlds per checkpoint
            jobs += [(s, 'normal', ENV), (s, 'deaf', {**ENV, 'PP_SCENT_DEAF': CHAN}), (s, 'shift', {**ENV, 'PP_SCENT_SHIFT': f'{CHAN}:40'})]
        with ThreadPoolExecutor(P) as ex:
            lr = list(ex.map(lambda j: run(genomes[best], j[0], f'{ld}/{j[1]}_s{j[0]}', j[2]), jobs))
        with open(listen_csv, 'a') as f:
            for k in range(LISTEN):
                n, d, sh = lr[3 * k], lr[3 * k + 1], lr[3 * k + 2]
                f.write(f"{g},c{best:02},{900000 + k + (1000 * g if H2 else 0)},{n['score']},{d['score']},{sh['score']},{n['stored']},{d['stored']},{sh['stored']}\n")
    nd = f'{OUT}/g{g + 1:03}'
    if True:  # always seed the next generation, so a finished run can be extended by raising GENS
        os.makedirs(nd, exist_ok=True)
        for k, c in enumerate(order[:2]):
            shutil.copy(genomes[c], f'{nd}/c{k:02}.raw.txt')
            parents_of[(g + 1, k)] = f'g{g}c{c}'
        for k in range(2, N):
            p = par[rng.randrange(len(par))]
            mutant(genomes[p], f'{nd}/c{k:02}.raw.txt', SEED * 100000 + (g + 1) * 1000 + k)
            parents_of[(g + 1, k)] = f'g{g}c{p}'
    open(f'{gd}/done', 'w').close()
    sc = sorted((r['score'] for r in res), reverse=True)
    print(f'gen {g}: best {sc[0]} median {sc[N // 2]} alive {sum(1 for r in res if r["alive"])}/{N} alarm laid median {sorted(r["layAl"] for r in res)[N // 2]:.0f} B laid max {max(r["layB"] for r in res):.0f}', flush=True)
