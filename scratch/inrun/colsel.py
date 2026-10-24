"""SCRATCH (teacher lane): colony-level selection with digging as a gate, for the VM.

Grown from the scoring review lane's prototype (/mnt/project-files/scoring-review/colsel.py, 2026-10-22). Its test, 12
colonies x 6 rounds, gave 3x the food and a third of the starvation, but zero digging: its rank-sum included the raw nest
score, which is negative for every crater-digging colony and ~+1 for a colony that never digs, so it paid not digging.

usage: colsel.py OUT MODE NCOL ROUNDS FRAMES GENOME [inrun args...]
  GENOME a .raw.txt (round 0 = it and 29 mutants), or a pool file such as assets/minds/std_pool.txt (round 0 = the pool)
  MODE plain   every colony runs rounds.sh's loop on its own (the control).
  MODE gate    after each round, a colony may be in the top half only if it passed the dig gate (GATE or more cuts of
               original ground within 20 columns in the round's second half).
               Gate-passers rank by rank-sum of (home8, starved fewer, late cuts, rooms); the rest below them by late cuts.
               The bottom half is refounded from the top half's carried ants (best donates to worst).
  MODE lex     the same gate; the bottom half's donors are picked by epsilon-lexicase over the gate-passers on the cases
               (home8, starved fewer, late cuts, rooms), a fresh case order per pick, so different kinds of good colony
               keep donating. No weights.
Round r of colony c runs in world seed c (r=0) or c*1000+r, as rounds.sh does, so arms are paired by colony.
env: BIN, P (parallel runs), GATE (default 20), CG_EVERY (default 2: a common garden of every colony's carry after
rounds 1, 3, 5...; 0 = off), INHERIT=1 (each round after the first starts in the nest its colony's ants
came from: the colony's own last round, or its donor's when it was refounded; inrun's `inherit=`), CG_ARGS (the inrun args for the garden, default = the run's own args with sel=off).
Writes OUT/colsel.csv (one row per colony-round) and OUT/cg.csv (one row per colony per garden).
"""
import csv, os, subprocess, sys, gzip, random, shutil
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import nestcount
import roomshape
from concurrent.futures import ThreadPoolExecutor

OUT, MODE, NCOL, R, F, G = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4]), int(sys.argv[5]), sys.argv[6]
EXTRA = sys.argv[7:]
# dists=A,B,...: the food heaps' distance from the door, drawn per colony and round from the list (owner, 2026-10-22:
# make foraging harder by moving the food farther, and switch its location so ants don't learn where the pile is).
# The common garden keeps the first. A plain dist=N in EXTRA overrides the default 25 everywhere.
DISTS = next((x.split('=', 1)[1].split(',') for x in EXTRA if x.startswith('dists=')), None)
EXTRA = [x for x in EXTRA if not x.startswith('dists=')]
BASE_DIST = next((x.split('=', 1)[1] for x in EXTRA if x.startswith('dist=')), DISTS[0] if DISTS else '25')
EXTRA = [x for x in EXTRA if not x.startswith('dist=')]
# wean=R0:R1 (owner, 2026-10-23: "No permanent body rule from the old ant!"): the body rules (setenv PP_BODYRULES) are a
# scaffold. Each round's setenv gets PP_BODYRULES_P=p, p = 1 up to round R0, falling linearly to 0 at round R1 and 0
# after; the common garden always runs at p = 0, so the colony is judged with the scaffold off.
# distramp=START:STEP:STARVED[:MAX] (teacher lane, 2026-10-24; owner: "the ants are just so specialized to a box that is
# just slightly different"): the food heaps' distance from the door starts at START and, like pilecap, moves on the
# colonies' median starved: STARVED or fewer -> STEP farther (never past MAX, default 110: the bed is 256 wide with the
# door at 128), more than 3 x STARVED -> STEP back (never nearer than START), otherwise it holds. Replaces dist= and
# dists= for the rounds; the common garden keeps BASE_DIST. Logged per round (colsel.csv's last field).
# Why: the shipped pool (std_pool, 2026-10-24) was only ever fed 25 columns out. In the training bed itself, fruit,
# seed 1-2, 20k frames, sel=off: food at 25 -> 58-65 alive; at 60 -> 0, even on 3 tanks. Traced: an empty ant steps
# outward 41% and homeward 35% at every distance from the door (a near-random walk); none got past 49 columns, median
# 30. Food at 25 is found by wandering, so nothing ever selected for going farther, and the lab box (food 100+ out,
# one tank) kills every ant. A ramp keeps the bed survivable while it asks for a little more range each round.
DISTRAMP = next((tuple(int(v) for v in x.split('=', 1)[1].split(':')) for x in EXTRA if x.startswith('distramp=')), None)
EXTRA = [x for x in EXTRA if not x.startswith('distramp=')]
if DISTRAMP and len(DISTRAMP) == 3:
    DISTRAMP = DISTRAMP + (110,)
dist_now = DISTRAMP[0] if DISTRAMP else None
WEAN = next((tuple(int(v) for v in x.split('=', 1)[1].split(':')) for x in EXTRA if x.startswith('wean=')), None)
EXTRA = [x for x in EXTRA if not x.startswith('wean=')]
def wean_p(r):
    return 1.0 if r <= WEAN[0] else max(0.0, 1.0 - (r - WEAN[0]) / max(1, WEAN[1] - WEAN[0]))
# The same schedule weans the brood lane's home-eat crutch (setenv PP_HOMEEAT, 2026-10-23): PP_HOMEEAT_P rides along,
# and is read only when PP_HOMEEAT is set.
def with_p(args, p):
    return [f'{x},PP_BODYRULES_P={p:.3f},PP_HOMEEAT_P={p:.3f}' if x.startswith('setenv=') else x for x in args]
# pilecap=START:STEP:STARVED[:FLOOR] (plant-food lane, 2026-10-23; owner chose the weaning bed): the pile's cap (inrun
# `cap=`) starts at START. After a round whose colonies' median starved is STARVED or fewer it falls by STEP, never below
# FLOOR (default 10); after one with more than 3 x STARVED it rises by STEP, never above START; otherwise it holds. Run
# with the garden's args (`plants=... weed=30 fjoule=1`, setenv PP_MIND_GRAZE=1). The cap used is logged per round
# (colsel.csv, cg.csv `pilecap`).
# Why starved and not alive: plantwean1-1023 stepped on median end-alive, which in-run births hold at 24 whatever
# happens, so the cap fell every round (30 -> 0 by round 6) while starved per round ran 47, 9, 3, 6.5, 52, 195; at cap 0
# 6 of 64 colonies lived and all were extinct by round 23. Why a floor: no common garden has yet shown a colony living on
# plants alone (cap 0: median alive 0 in every garden), while cap 15 after three rounds held 19 of 24 (hcs at round 0,
# sel=off: 1.5 of 24). Lower FLOOR to 0 only once a garden at cap 0 holds.
PILECAP = next((tuple(int(v) for v in x.split('=', 1)[1].split(':')) for x in EXTRA if x.startswith('pilecap=')), None)
if PILECAP:
    assert len(PILECAP) in (3, 4), 'pilecap=START:STEP:STARVED[:FLOOR]'
    PILECAP = PILECAP + (10,) if len(PILECAP) == 3 else PILECAP
EXTRA = [x for x in EXTRA if not x.startswith('pilecap=')]
cap_now = PILECAP[0] if PILECAP else None
# pilegate=extinct:LOW:HIGH (plant-food lane, 2026-10-24): on a breeding bed, step pilecap on the share of colonies that
# died out in the round (no ant left to carry, as the common garden skips), in percent: at most LOW% -> the cap falls by STEP, more than
# HIGH% -> it rises by STEP, otherwise it holds. pilecap's STARVED is then unused. Why: with breeding a colony grows to what
# its food feeds, so any cut to the pile starves its surplus and starved counts the cut, not the foraging. plantwean4-1024
# (std_pool, breeding on, pilecap=45:5:10:5) went 45 -> 25 by round 8 and then bounced 25 <-> 20 for 23 rounds: at 25
# median starved 6-17 (step down), at 20 43-65 (> 3 x 10, step straight back up), so no round ran twice at 20 and the
# common garden at 20 stood still (median alive, extinct as 0: r9 62, r31 65.5). Extinctions at 20 were 10-16 of 64
# colonies a round: pressure selection can act on, which a starved rule threw away after one round.
PILEGATE = next((tuple(int(v) for v in x.split('=', 1)[1].split(':')[1:]) for x in EXTRA if x.startswith('pilegate=')), None)
if PILEGATE:
    assert PILECAP and len(PILEGATE) == 2, 'pilegate=extinct:LOW:HIGH needs pilecap='
EXTRA = [x for x in EXTRA if not x.startswith('pilegate=')]
# spiderramp=START:STEP:K[:FROM[:FLOOR]] (lone predator thread, 2026-10-24, at the teacher lane's ask: "a predator a
# decent colony can just beat"): with inrun's `spiders=POOL:N` in the args, the spiders arrive at frame START of each
# round (inrun `spider_start=`). After a round whose colonies' median killed (ants a spider took) is K or fewer they
# arrive STEP frames earlier, never before FLOOR (default 0); after one above 3 x K, STEP later, never after START.
# Rounds before FROM (default 0) run with no spiders at all, so the colony is established first (brood1ctl's round 0
# held a median of 17.5 ants and 11 of 64 colonies extinct with no predator; round 2, 83). The common garden uses the
# round's arrival. Logged per round (colsel.csv, cg.csv `spider_at`; empty = no spiders that round).
# Why: one spider from frame 20,000 ended 15 of 16 round-0 colonies whether they could bite or not; one from 100,000
# with a 50,000-frame refill gap killed 0-30 ants a run, and the ants killed it in 5 of 6 runs when they could bite,
# 0 of 6 when they could not.
SPIDERRAMP = next((tuple(int(v) for v in x.split('=', 1)[1].split(':')) for x in EXTRA if x.startswith('spiderramp=')), None)
if SPIDERRAMP:
    assert len(SPIDERRAMP) in (3, 4, 5), 'spiderramp=START:STEP:K[:FROM[:FLOOR]]'
    SPIDERRAMP = SPIDERRAMP + (0, 0)[len(SPIDERRAMP) - 3:]
    assert any(x.startswith('spiders=') for x in EXTRA), 'spiderramp= needs inrun spiders=POOL:N in the args'
EXTRA = [x for x in EXTRA if not x.startswith('spiderramp=')]
spider_at = SPIDERRAMP[0] if SPIDERRAMP else None
# spiderevo=KEEP:PM:SIGMA:EVENTS (lone predator thread, stage three, 2026-10-24: the owner's co-evolution, "train the
# ant colony to survive and defend while training the predator to eat the ants"): the spider pool evolves beside the
# colonies. Every spider placed in a round's colony runs is a size-neutral mutant of a pool mind (inrun `spider_mut=`),
# scored on the flesh it ate per thousand frames alive over at least 20,000; after the round the KEEP best placements
# over every colony are the next round's pool (OUT/spool/rN, logged in OUT/spool.csv). The garden faces the current
# pool unmutated. Needs inrun `spiders=POOL:N` in the args (POOL a directory of .raw.txt, the round-0 pool).
SPIDEREVO = next((x.split('=', 1)[1].split(':') for x in EXTRA if x.startswith('spiderevo=')), None)
EXTRA = [x for x in EXTRA if not x.startswith('spiderevo=')]
spool = next((x.split('=', 1)[1].rsplit(':', 1)[0] for x in EXTRA if x.startswith('spiders=')), None)
if SPIDEREVO:
    assert len(SPIDEREVO) == 4 and spool, 'spiderevo=KEEP:PM:SIGMA:EVENTS with spiders=POOL:N'
def spider_args(args, out=None):
    """The current pool in `spiders=`; with `out`, the round's mutation and the placements' log there too."""
    if not SPIDEREVO or not any(x.startswith('spiders=') for x in args):
        return args
    a = [f"spiders={spool}:{x.rsplit(':', 1)[1]}" if x.startswith('spiders=') else x for x in args]
    return a + ([f'spider_mut={SPIDEREVO[1]}:{SPIDEREVO[2]}:{SPIDEREVO[3]}', f'spider_out={out}'] if out else [])
def evolve_spiders(r, dirs):
    """The KEEP best spider placements of round r over every colony, as round r+1's pool."""
    global spool
    rows = []
    for c, d in sorted(dirs.items()):
        f = f'{d}/spiders/scores.csv'
        if os.path.exists(f):
            rows += [(float(x['score']), c, int(x['k']), f"{d}/spiders/k{int(x['k']):03}.raw.txt") for x in csv.DictReader(open(f))]
    if not rows:
        return
    rows.sort(key=lambda x: (-x[0], x[1], x[2]))
    keep = rows[:int(SPIDEREVO[0])]
    nd = f'{OUT}/spool/r{r + 1}'
    os.makedirs(nd, exist_ok=True)
    new = not os.path.exists(f'{OUT}/spool.csv')
    with open(f'{OUT}/spool.csv', 'a') as f:
        if new:
            f.write('round,rank,score,colony,k,placements,median_score\n')
        med = sorted(x[0] for x in rows)[len(rows) // 2]
        for i, (sc, c, k, fp) in enumerate(keep):
            shutil.copy(fp, f'{nd}/{i:02}.raw.txt')
            f.write(f'{r},{i},{sc:.4f},{c},{k},{len(rows)},{med:.4f}\n')
    spool = nd
    print(f'round {r}: {len(rows)} spiders placed, median score {med:.2f}, best {keep[0][0]:.2f}; pool -> {nd}', flush=True)
def with_spiders(args, r):
    """The round's spider args: none before FROM, otherwise `spider_start=` the ramp's current arrival."""
    if not SPIDERRAMP:
        return args
    if r < SPIDERRAMP[3]:
        return [x for x in args if not x.startswith('spider')]
    return [x for x in args if not x.startswith('spider_start=')] + [f'spider_start={spider_at}']
def spider_log(r):
    return '' if not SPIDERRAMP or r < SPIDERRAMP[3] else str(spider_at)
def with_cap(args):
    return args if cap_now is None else [x for x in args if not x.startswith('cap=')] + [f'cap={cap_now}']
BIN = os.environ.get('BIN', 'target/release/examples/inrun')
P = int(os.environ.get('P', '4'))
GATE = int(os.environ.get('GATE', '20'))
# DIG=nest (default since 2026-10-22 17:00Z): the dig gate and the lexicase dig cases read the nest the colony actually
# kept (nestcount.py: late cuts whose cell is still roofed nest joined to the door at the end, and the nest's size)
# instead of raw late cuts and nest-score rooms. Owner asked for cuts that expand the nest; measured the same day, the
# replay's "best" colony made 272 cuts near the door and kept a nest of 19 cells (old ant 278), its cuts all crater.
# DIG=cuts is colsel-1022's rule. GATE then defaults to 5 kept late cuts (one round of hA1: 19; old ant: 48).
DIG = os.environ.get('DIG', 'nest')
# SPOIL=1 (or MODE lex+spoil): a fifth lexicase case, less loose spoil left below the datum within 20 columns of the
# door at the end. The kept-nest rule (colsel3-1022, 16 rounds, lesson + nest-only inheritance) reached a median nest of 225 cells against the
# old ant's 302, but left 88 spoil cells in and around it (old ant 0) and let 437 pellets go below the datum.
if DIG == 'nest' and 'GATE' not in os.environ:
    GATE = 5
CG_EVERY = int(os.environ.get('CG_EVERY', '2'))
INHERIT = os.environ.get('INHERIT', '0') == '1' or MODE.endswith('+inherit')  # MODE 'lex+inherit' = INHERIT=1
MODE = MODE.removesuffix('+inherit')
# Extra lexicase cases as MODE suffixes, in any order before +inherit: +spoil (less spoil left, see SPOIL), +crater
# (fewer sky-open cells within 20 columns of the door: owner 2026-10-22, "not digging the top layer near the nest"),
# +rooms (more chambers + niches by the nest score), +shape (roomshape.py, the dig-scoring thread's graded "on the way to
# multiple rooms": a second room or the start of a side tunnel scores, the main room and the crater never do; old ant
# median 20 over 12 garden worlds, colsel3's arms 3-6). colsel3-1022's best arm: crater 114 vs the old ant's 4, rooms 0-1.
# +guard (lone predator thread, 2026-10-24): fewer ants killed in the round (inrun's `death` rows of cause 3, killed:
# on a bed with `spiders=POOL:N`, only a spider kills). Run it with inrun's `spiders=` in the args; on a bed without
# spiders every colony scores 0 and the case drops nobody. **Measured to pay hiding** (16 colonies x 10 rounds, one
# spider from round 2, 2026-10-24): against the same bed without the case, ants killed per round fell (median 29.5 vs
# 31.5 at round 9, about half in rounds 3-7) but food home fell with it (5,553 vs 9,235), ants killed per 1,000 deliveries
# did not (5.3 vs 4.7), and the common garden of its colonies collapsed (median 16 alive vs 70, 6 of 15 extinct vs 2).
# +slay (same day): the colony's damage to spiders, spider cells bitten off (inrun `wound`) plus six for each spider
# killed. A colony that hides scores 0 on it: the defence case that pays the verb, not the absence.
EXTRA_CASES = []
while '+' in MODE:
    MODE, k = MODE.rsplit('+', 1)
    assert k in ('spoil', 'crater', 'rooms', 'shape', 'guard', 'slay', 'lab'), k
    EXTRA_CASES.append(k)
SPOIL = os.environ.get('SPOIL') == '1' or 'spoil' in EXTRA_CASES
# +lab (teacher lane, 2026-10-24): each round, every colony's carry also lands in lab boxes through the lab's own code
# (examples/labsurvive.rs), and its mean headcount over the LAB_AFTER frames after landing (labsurvive `lived`, in tenths), summed over LAB_SCENS, is a case. **Why**: the
# owner's playtest found trained colonies starving in the lab within ~5k frames; labsurvive put it on food distance --
# the pool lived with food 10-50 columns out and every ant starved at 100-240 (the hand-built ant lived at all of
# them), because every training bed had dropped its food 25 columns from the door. The scenarios live in LAB_DIR.
LAB = 'lab' in EXTRA_CASES
LAB_BIN = os.environ.get('LAB_BIN', 'target/release/examples/labsurvive')
LAB_DIR = os.environ.get('LAB_DIR', 'scratch/inrun/labscen')
LAB_SCENS = os.environ.get('LAB_SCENS', 'lab_far_mix,lab_mid_fruit').split(',')
LAB_AFTER = os.environ.get('LAB_AFTER', '15000')
# The arm's own sense switches reach the lab box too (a lab case judged on another mind's senses judges nothing): every
# setenv= pair but the training scaffolds (PP_BODYRULES*, PP_HOMEEAT*, which the lab never runs).
LAB_ENV = {k: v for x in EXTRA if x.startswith('setenv=') for k, v in (kv.split('=', 1) for kv in x[len('setenv='):].split(','))
           if not k.startswith(('PP_BODYRULES', 'PP_HOMEEAT'))}


def lab_alive(c, r, carry_c):
    import re
    tot = 0
    for k, sc in enumerate(LAB_SCENS):
        out = subprocess.run([LAB_BIN, f'founders={carry_c}', f'scenario={sc}', f'seed={c * 1000 + r * 10 + k}', f'after={LAB_AFTER}'],
                             capture_output=True, text=True, env={**os.environ, **LAB_ENV, 'PIXEL_PHYSICS_LAB_SCENARIOS': LAB_DIR}).stdout
        m = re.search(r'RESULT alive=\d+ lived=([\d.]+)', out)  # mean headcount after landing: graded, unlike alive
        tot += round(10 * float(m.group(1))) if m else 0  # tenths: whole ants rounded every early-dying colony to one value
    return tot
C = [f'frames={F}', f'dist={BASE_DIST}', 'spread=10', 'grants=3', 'window=4000', 'sel=score'] + EXTRA
CGC = [a if not a.startswith('sel=') else 'sel=off' for a in C if not a.startswith('frames=')] + ['frames=200000']
CGC = with_p(CGC, 0.0) if WEAN else CGC
os.makedirs(OUT, exist_ok=True)


def run(args, out):
    os.makedirs(out, exist_ok=True)
    with open(f'{out}.log', 'w') as lf:
        subprocess.run([BIN] + args + [f'out={out}'], stdout=lf, stderr=subprocess.STDOUT, check=False)
    for fn in os.listdir(out):  # maps and per-window ant lists: not read here
        if fn.startswith('map_f') or fn.startswith('ants_f'):
            os.remove(f'{out}/{fn}')
    return out


def carried(rdir):
    """rounds.sh's carry: top 30 by ant score, topped up to 30 requests by repeating."""
    idx = f'{rdir}/final/index.csv'
    if not os.path.exists(idx):
        return None
    r = list(csv.DictReader(open(idx)))
    r.sort(key=lambda x: int(x['id']))  # index.csv is in HashMap order: sort first so ties keep a fixed order
    r.sort(key=lambda x: (-float(x['score']), int(x['id'])))
    r = r[:30]
    n = len(r)
    if n == 0:
        return None
    return ','.join(f"{rdir}/final/{x['id']}.raw.txt@{x['family']}.{x['id']}:{30 // n + (1 if i < 30 % n else 0)}" for i, x in enumerate(r))


def result(rdir, frames):
    """(home8, late cuts, starved, rooms, alive, cuts, dug, pit) of one run; late = original-ground cuts within 20
    columns in the second half; rooms = chambers + niches in the last nest.csv row."""
    ev = f'{rdir}/events.csv'
    home = late = starved = cuts = killed = slay = 0
    if os.path.exists(ev):
        with open(ev) as f:
            next(f)
            for line in f:
                p = line.split(',')
                k = p[2]
                if k == 'deliver' and abs(int(p[3]) - 128) <= 8:
                    home += 1
                elif k == 'dig' and abs(int(p[3]) - 128) <= 20:
                    cuts += 1
                    late += int(p[0]) > frames // 2
                elif k == 'death' and p[5] == '1':
                    starved += 1
                elif k == 'death' and p[5] == '3':
                    killed += 1
                elif k == 'wound':
                    slay += int(p[5])
                elif k == 'spdeath' and p[5] == '3':
                    slay += 6  # a spider killed: its whole six-cell body
    rooms = alive = dug = pit = 0
    nf = f'{rdir}/nest.csv'
    if os.path.exists(nf):
        rows = list(csv.DictReader(open(nf)))
        if rows:
            n = rows[-1]
            rooms, alive = int(n['chambers']) + int(n['niches']), int(n['alive'])
            dug, pit = int(n['dug']), int(n['pit'])
    n = nestcount.count(rdir, frames) if os.path.exists(f'{rdir}/nestmap.txt') else {}
    return (home, late, starved, rooms, alive, cuts, dug, pit, n.get('nest', 0), n.get('nestcuts') or 0,
            n.get('nestcuts_late') or 0, n.get('crater', 0), n.get('spoil', 0), n.get('spoil_in') or 0,
            roomshape.score(rdir) if n else 0.0, killed, slay)


def ranks(vals, higher_better=True):
    order = sorted(range(len(vals)), key=lambda i: vals[i], reverse=higher_better)
    rk = [0.0] * len(vals)
    i = 0
    while i < len(order):  # average ranks over ties
        j = i
        while j + 1 < len(order) and vals[order[j + 1]] == vals[order[i]]:
            j += 1
        for t in range(i, j + 1):
            rk[order[t]] = (i + j) / 2
        i = j + 1
    return rk


def cases(v):
    """The lexicase/rank cases of a colony result, every one higher-is-better: food home, fewer starved, then digging
    (DIG=nest: kept late cuts and nest size; DIG=cuts: raw late cuts and rooms)."""
    c = (v[0], -v[2], v[10], v[8]) if DIG == 'nest' else (v[0], -v[2], v[1], v[3])
    c = c + (-v[12],) if SPOIL else c
    c = c + (-v[11],) if 'crater' in EXTRA_CASES else c
    c = c + (v[3],) if 'rooms' in EXTRA_CASES else c
    c = c + (v[14],) if 'shape' in EXTRA_CASES else c
    c = c + (-v[15],) if 'guard' in EXTRA_CASES else c
    c = c + (v[16],) if 'slay' in EXTRA_CASES else c
    return c + (v[17],) if LAB else c


def dig_passed(v):
    return (v[10] if DIG == 'nest' else v[1]) >= GATE


def lexicase(pool, res, rng):
    """Epsilon-lexicase: per case (random order) keep the colonies within one median absolute deviation of the best."""
    cand = list(pool)
    # Every case cases() returns. Was range(4) until 2026-10-22 19:00Z, so colsel4-1022's +spoil arm never read its
    # fifth case and came out identical to its control in every round (a change that moved nothing).
    order = list(range(len(cases(res[cand[0]]))))
    rng.shuffle(order)
    for k in order:
        vals = [cases(res[c])[k] for c in cand]
        best = max(vals)
        med = sorted(vals)[len(vals) // 2]
        mad = sorted(abs(x - med) for x in vals)[len(vals) // 2]
        cand = [c for c, x in zip(cand, vals) if x >= best - mad]
        if len(cand) == 1:
            break
    return rng.choice(sorted(cand))


def garden(r, carry):
    """Common garden of every live colony's carry: fresh world c*1000+99, 200k frames, selection off."""
    live = [c for c in carry if carry[c] is not None]
    def one(c):
        d = run([f'founders={carry[c]}', f'seed={c * 1000 + 99}'] + spider_args(with_spiders(with_cap(CGC), r)), f'{OUT}/cg/r{r}/c{c}')
        v = result(d, 200000)
        shutil.rmtree(d, ignore_errors=True)
        return c, v
    with ThreadPoolExecutor(P) as ex:
        got = dict(ex.map(one, live))
    new = not os.path.exists(f'{OUT}/cg.csv')
    with open(f'{OUT}/cg.csv', 'a') as f:
        if new:
            f.write('round,colony,home8,late_cuts,starved,rooms,alive,cuts,dug,pit,nest,nestcuts,nestcuts_late,crater,spoil,spoil_in,shape,killed,slay,pilecap,spider_at,train_dist\n')
        for c in sorted(got):
            f.write(f'{r},{c},' + ','.join(str(x) for x in got[c]) + f',{"" if cap_now is None else cap_now},{spider_log(r)},{"" if dist_now is None else dist_now}\n')


# LEXSEED (default 12345): the lexicase draws' seed, for a replicate of an arm on the same colony seeds.
rng = random.Random(int(os.environ.get('LEXSEED', '12345')))
# Resume (teacher lane, 2026-10-24): spot VMs were preempted five times on 2026-10-23, and a colsel job only reached the
# bucket when it finished, so each preemption lost the whole batch (broodplant1b: 1 h 40 min). The driver now writes
# state.json at the end of every round (everything the next round reads: founders, inherited nests, the last round's
# dirs, the lexicase rng, the pile cap); started on an OUT that holds one, it drops the rows and dirs of any round it
# had not finished and carries on from the next. inrun_cloud.py uploads OUT as partial/ while the job runs and restores
# it with `--resume OLDNAME`. A finished run is the same as an unbroken one: rounds are seeded per colony and round.
statep = f'{OUT}/state.json'
R0 = 0
if os.path.exists(statep):
    import json
    st0 = json.load(open(statep))
    R0 = st0['r']
    for fn in ('colsel.csv', 'cg.csv'):
        fp = f'{OUT}/{fn}'
        if os.path.exists(fp):
            rows = open(fp).read().splitlines(True)
            open(fp, 'w').writelines(rows[:1] + [x for x in rows[1:] if int(x.split(',', 1)[0]) < R0])
    for c in range(1, NCOL + 1):
        shutil.rmtree(f'{OUT}/c{c}/r{R0}', ignore_errors=True)
    shutil.rmtree(f'{OUT}/cg/r{R0}', ignore_errors=True)
    print(f'resuming at round {R0}', flush=True)
logp = f'{OUT}/colsel.csv'
log = open(logp, 'a')
if os.path.getsize(logp) == 0:
    log.write('round,colony,home8,late_cuts,starved,rooms,alive,cuts,dug,pit,nest,nestcuts,nestcuts_late,crater,spoil,spoil_in,shape,killed,slay,' + ('lab,' if LAB else '') + 'passed_gate,rank,refounded_from,pilecap,spider_at,dist\n')
def expand_pool(path):
    """A pool file (assets/minds/std_pool.txt: member 0 whole, later members as their differences from it) written
    out as one .raw.txt per member under OUT/pool, and round 0's founders string: each member once, as a carry is."""
    lines = [l for l in open(path).read().split('\n') if l.strip() and not l.startswith('#')]
    head = lines[0].split()
    assert head[0] == 'pool' and head[2] == 'genome', head
    members = []
    for l in lines[1:]:
        if l.startswith('member '):
            members.append(dict(members[0]) if members else {})
            continue
        i, w = l.split()
        members[-1][int(i)] = w
    os.makedirs(f'{OUT}/pool', exist_ok=True)
    out = []
    for k, m in enumerate(members):
        fp = f'{OUT}/pool/p{k:02}.raw.txt'
        with open(fp, 'w') as f:
            f.write(f'genome {head[3]} {head[4]}\n')
            f.writelines(f'{i} {w}\n' for i, w in sorted(m.items()) if float(w) != 0.0)
        out.append(f'{fp}@p{k:02}:1')
    return [f"founders={','.join(out)}"]
# G a pool file (the standard pool, since 2026-10-24): round 0 is the pool itself, no mutants
founders = {c: (expand_pool(G) if G.endswith('pool.txt') else [f'founders={G}:1', 'mutants=30:0.03:1.0:7']) for c in range(1, NCOL + 1)}
prev_dirs = {}
nest = {c: None for c in range(1, NCOL + 1)}
if R0:
    founders = {int(c): v for c, v in st0['founders'].items()}
    nest = {int(c): v for c, v in st0['nest'].items()}
    prev_dirs = {int(c): v for c, v in st0['prev_dirs'].items()}
    rng.setstate((st0['rng'][0], tuple(st0['rng'][1]), st0['rng'][2]))
    cap_now = st0['cap_now']
    spider_at = st0.get('spider_at', spider_at)
    spool = st0.get('spool', spool)
    dist_now = st0.get('dist_now', dist_now)
for r in range(R0, R):
    live = [c for c in founders if founders[c] is not None]
    def go(c):
        inh = [f'inherit={nest[c]}'] if INHERIT and nest[c] else []
        cr = C if not DISTS else [f'dist={random.Random(c * 7919 + r).choice(DISTS)}' if x.startswith('dist=') else x for x in C]
        cr = [f'dist={dist_now}' if x.startswith('dist=') else x for x in cr] if DISTRAMP else cr
        cr = with_p(cr, wean_p(r)) if WEAN else cr
        cr = spider_args(with_spiders(with_cap(cr), r), f'{OUT}/c{c}/r{r}/spiders')
        return run(founders[c] + [f'seed={c if r == 0 else c * 1000 + r}'] + inh + cr, f'{OUT}/c{c}/r{r}')
    with ThreadPoolExecutor(P) as ex:
        dirs = dict(zip(live, ex.map(go, live)))
    cols = list(range(1, NCOL + 1))
    res = {c: result(dirs[c], F) if c in dirs else None for c in cols}
    carry = {c: carried(dirs[c]) if c in dirs else None for c in cols}
    ok = [c for c in cols if carry[c] is not None]
    if LAB:
        with ThreadPoolExecutor(P) as ex:
            labv = dict(zip(ok, ex.map(lambda c: lab_alive(c, r, carry[c]), ok)))
        res = {c: (v + (labv.get(c, 0),) if v else v) for c, v in res.items()}
    # The gate is late original-ground cuts only. A room in the final nest map no longer passes it: colsel-1022 found
    # 47 of 64 gate-arm colonies passing on a "room" with 0-1 late cuts (the nest score finds a chamber or niche in
    # nearly any nest, dug or not), and that arm's digging went to 0 by round 7. Rooms stay a lexicase case.
    passed = {c: c in ok and dig_passed(res[c]) for c in cols}
    if MODE == 'plain':
        order = sorted(ok, key=lambda c: c)
    else:
        gp = [c for c in ok if passed[c]]
        rs = {}
        if gp:
            nk = len(cases(res[gp[0]]))
            rk = [ranks([cases(res[c])[k] for c in gp]) for k in range(nk)]
            rs = {c: sum(rk[k][i] for k in range(nk)) for i, c in enumerate(gp)}
        order = sorted(gp, key=lambda c: (rs[c], c)) + sorted([c for c in ok if not passed[c]], key=lambda c: (-res[c][1], c))
    order += [c for c in cols if c not in ok]  # extinct last
    src = {c: '' for c in cols}
    nxt = {c: carry[c] for c in cols}
    if MODE in ('gate', 'lex') and ok:
        half = NCOL // 2
        top = [c for c in order[:half] if carry[c] is not None]
        donors = [c for c in top if passed[c]] or top
        for i, b in enumerate(order[half:][::-1]):  # worst first
            if not donors:
                break
            d = donors[i % len(donors)] if MODE == 'gate' else lexicase(donors, res, rng)
            nxt[b], src[b] = carry[d], str(d)
    for c in cols:
        v = res[c] or (0,) * (18 if LAB else 17)
        log.write(f'{r},{c},' + ','.join(str(x) for x in v) + f',{int(passed[c])},{order.index(c)},{src[c]},{"" if cap_now is None else cap_now},{spider_log(r)},{"" if dist_now is None else dist_now}\n')
    log.flush()
    if CG_EVERY and (r % CG_EVERY == CG_EVERY - 1 or r == R - 1):
        garden(r, carry)
    if SPIDEREVO and (not SPIDERRAMP or r >= SPIDERRAMP[3]):
        evolve_spiders(r, dirs)  # after the garden, which faces the pool this round's colonies faced
    if PILECAP:
        # A colony with no run this round (extinct, not refounded) counts as starving, never as a colony that starved none.
        st = sorted(res[c][2] if res[c] else 10**9 for c in cols)
        med = st[len(st) // 2]
        if PILEGATE:
            ext = 100 * sum(1 for c in cols if carry[c] is None) / len(cols)  # no ant left to carry: the colony died out
            down, up = ext <= PILEGATE[0], ext > PILEGATE[1]
        else:
            down, up = med <= PILECAP[2], med > 3 * PILECAP[2]
        if down:
            cap_now = max(PILECAP[3], cap_now - PILECAP[1])
        elif up:
            cap_now = min(PILECAP[0], cap_now + PILECAP[1])
        print(f'round {r}: median starved {med}' + (f', {ext:.0f}% of colonies extinct' if PILEGATE else '') + f', pile cap next round {cap_now}', flush=True)
    if SPIDERRAMP and r >= SPIDERRAMP[3]:
        # An extinct colony counts as one that lost everything, never as one that lost none.
        kl = sorted(res[c][15] if res[c] else 10**9 for c in cols)
        med = kl[len(kl) // 2]
        if med <= SPIDERRAMP[2]:
            spider_at = max(SPIDERRAMP[4], spider_at - SPIDERRAMP[1])
        elif med > 3 * SPIDERRAMP[2]:
            spider_at = min(SPIDERRAMP[0], spider_at + SPIDERRAMP[1])
        print(f'round {r}: median killed {med}, spiders arrive at frame {spider_at} next round', flush=True)
    if DISTRAMP:
        st = sorted(res[c][2] if res[c] else 10**9 for c in cols)
        med = st[len(st) // 2]
        if med <= DISTRAMP[2]:
            dist_now = min(DISTRAMP[3], dist_now + DISTRAMP[1])
        elif med > 3 * DISTRAMP[2]:
            dist_now = max(DISTRAMP[0], dist_now - DISTRAMP[1])
        print(f'round {r}: median starved {med}, food distance next round {dist_now}', flush=True)
    for c in cols:
        founders[c] = None if nxt[c] is None else [f'founders={nxt[c]}']
        home = int(src[c]) if src[c] else c
        nest[c] = f'{dirs[home]}/nestmap.txt' if home in dirs else None
    # the round before is no longer referenced by any carry: drop its genomes and event log, keep its csv summaries
    for c, d in prev_dirs.items():
        shutil.rmtree(f'{d}/final', ignore_errors=True)
        if os.path.exists(f'{d}/events.csv'):
            os.remove(f'{d}/events.csv')
    prev_dirs = dirs
    import json
    json.dump({'r': r + 1, 'founders': founders, 'nest': nest, 'prev_dirs': prev_dirs, 'rng': rng.getstate(), 'cap_now': cap_now, 'spider_at': spider_at, 'dist_now': dist_now, 'spool': spool},
              open(statep + '.tmp', 'w'))
    os.replace(statep + '.tmp', statep)
    print(f'round {r} done: {sum(passed.values())} passed the gate, {len(ok)} alive', flush=True)
for d in prev_dirs.values():  # the last round keeps its genomes (the product) and gzips its log
    if os.path.exists(f'{d}/events.csv'):
        subprocess.run(['gzip', '-f', f'{d}/events.csv'])
print('DONE', flush=True)
