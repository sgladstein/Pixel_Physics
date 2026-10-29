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
# heaps=LO:HI (teacher lane, 2026-10-25; owner: "should we always just be randomly moving the distance ... so the ants
# never just learn one location?"): each colony-round draws the east heap's distance and the west heap's (inrun near=)
# independently, in steps of 5 from LO to HI. The garden keeps BASE_DIST for both. **Why**: from the ramp on, every
# heap sat 100-120 columns out, and on the herb bed the trained ants walk away from food at the nest.
HEAPS = next((tuple(int(v) for v in x.split('=', 1)[1].split(':')) for x in EXTRA if x.startswith('heaps=')), None)
EXTRA = [x for x in EXTRA if not x.startswith('heaps=')]
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
# tramp=START:STARVED:MIN (teacher lane, 2026-10-25): with patches=T:LO:HI, ramp how often the training heap moves
# instead of how far it is. T starts at START frames; a round whose median starved is STARVED or fewer multiplies it by
# 0.8 (never below MIN), one over 3 x STARVED divides it by 0.8 (never above START). The garden keeps the job's own
# patches=. **Why**: on moving food the standard pool dies out even with the heap 20-40 columns out (3 of 4 test
# worlds), starving beside 300+ cells of stored food it never eats -- the moves kill it, not the distance. A heap that
# moves rarely at first is the gentlest way to make eating the store pay.
TRAMP = next((tuple(int(v) for v in x.split('=', 1)[1].split(':')) for x in EXTRA if x.startswith('tramp=')), None)
EXTRA = [x for x in EXTRA if not x.startswith('tramp=')]
move_t = TRAMP[0] if TRAMP else None
WEAN = next((tuple(int(v) for v in x.split('=', 1)[1].split(':')) for x in EXTRA if x.startswith('wean=')), None)
EXTRA = [x for x in EXTRA if not x.startswith('wean=')]
def wean_p(r):
    return 1.0 if r <= WEAN[0] else max(0.0, 1.0 - (r - WEAN[0]) / max(1, WEAN[1] - WEAN[0]))
# The same schedule weans the brood lane's home-eat crutch (setenv PP_HOMEEAT, 2026-10-23): PP_HOMEEAT_P rides along,
# and is read only when PP_HOMEEAT is set.
# And the come-home teacher (setenv PP_CARRY=q with PP_HOMEFAR_HUNGRY, teacher lane 2026-10-25): its q IS the share of
# decisions it takes over, so the schedule rewrites PP_CARRY itself. Acting at q=1 on the c10 pool with food far and
# moving, it took starvation from 145-476 a run to 14-31; the garden runs at q=0, so a colony is judged on its own feet.
def with_p(args, p):
    def one(x):
        if not x.startswith('setenv='):
            return x
        kv = [f'PP_CARRY={p:.3f}' if k.startswith('PP_CARRY=') else k for k in x[len('setenv='):].split(',')]
        return 'setenv=' + ','.join(kv) + f',PP_BODYRULES_P={p:.3f},PP_HOMEEAT_P={p:.3f}'
    return [one(x) for x in args]
# capwean=R0:R1 (brood lane, 2026-10-24; owner: brood rules should "just balance through training"): inrun's size
# brake lay_cap is a scaffold too -- the lab has none, and far64 boomed there to ~710 ants and starved 817 in 10k frames.
# Each round gets lay_cap_p=p on wean_p's shape (1 up to round R0, linear to 0 at R1); the common garden runs at
# lay_cap_p=0, so a colony is judged on its own brake (PP_LAYGENE=1 lets the genome learn one).
CAPWEAN = next((tuple(int(v) for v in x.split('=', 1)[1].split(':')) for x in EXTRA if x.startswith('capwean=')), None)
EXTRA = [x for x in EXTRA if not x.startswith('capwean=')]
def capwean_p(r):
    return 1.0 if r <= CAPWEAN[0] else max(0.0, 1.0 - (r - CAPWEAN[0]) / max(1, CAPWEAN[1] - CAPWEAN[0]))
def with_capp(args, p):
    return [x for x in args if not x.startswith('lay_cap_p=')] + [f'lay_cap_p={p:.3f}']
# pilecap=START:STEP:STARVED[:FLOOR] (plant-food lane, 2026-10-23; owner chose the weaning bed): the pile's cap (inrun
# `cap=`) starts at START. After a round whose colonies' median starved is STARVED or fewer it falls by STEP, never below
# FLOOR (default 10); after one with more than 3 x STARVED it rises by STEP, never above START; otherwise it holds. Run
# with the garden's args (`plants=...`; `weed=30 fjoule=1` and PP_MIND_GRAZE come with it by default since 2026-10-24).
# The cap used is logged per round (colsel.csv, cg.csv `pilecap`). On a breeding bed the step rule is pilegate's below.
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
# **On by default on a breeding bed** (owner, 2026-10-24: "The default should be on unless there is uncertainty"):
# breeding is inrun's default, and there the starved rule is the one measured broken, so `pilecap=` without `pilegate=`
# steps on extinct:10:25 unless the args carry `brood=off`; `pilegate=off` keeps the starved rule.
_pg = next((x.split('=', 1)[1] for x in EXTRA if x.startswith('pilegate=')), None)
if _pg is None and PILECAP and 'brood=off' not in EXTRA:
    _pg = 'extinct:10:25'
PILEGATE = tuple(int(v) for v in _pg.split(':')[1:]) if _pg and _pg != 'off' else None
if PILEGATE:
    assert PILECAP and len(PILEGATE) == 2, 'pilegate=extinct:LOW:HIGH needs pilecap='
if PILECAP:
    print(f'pilecap {PILECAP} steps on ' + (f'extinct colonies {PILEGATE[0]}:{PILEGATE[1]}%' if PILEGATE else 'median starved'), flush=True)
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
# spiderevo=KEEP:PM:SIGMA:EVENTS:ELITE (the fix, same day): the first batch made the pool hunt WORSE (fewer ants in 49
# of 77 paired judgings) -- each placement scored on one life, every placement a mutant, and the luckiest kept. With
# ELITE that share of placements is its pool mind unmutated (inrun `spider_mut`'s 4th field), every pool mind keeps
# the scores of all its lives across rounds (`spool_hist`, in state.json), and candidates -- the pool and the round's
# mutants -- rank on a mean shrunk toward the round's median (PRIOR lives' worth), so one lucky life cannot outrank
# a parent with five good ones.
SPIDEREVO = next((x.split('=', 1)[1].split(':') for x in EXTRA if x.startswith('spiderevo=')), None)
PRIOR = 2.0
spool_hist = {}
# spiderhold=K (owner, 2026-10-24 23:18Z: "If the spiders start doing too well then we need to hold their evolution
# while selecting for the best ants until the ants survive"): the spider pool evolves after a round only when that
# round's median colony lost K ants or fewer to spiders (an extinct colony counts as having lost everything); above
# it the pool is held and only the ants are selected, until the colonies catch up. Logged per round.
SPIDERHOLD = next((int(x.split('=', 1)[1]) for x in EXTRA if x.startswith('spiderhold=')), None)
EXTRA = [x for x in EXTRA if not x.startswith('spiderhold=')]
# spiderlab=SCEN:FRAMES:N:TOP (owner, same evening: "select for a spider colony that's more sustainable instead of one
# that wipes out"): the TOP candidates of the in-run ranking each land as N spiders of their one mind in the lab
# scenario SCEN (examples/spiderlab.rs: the lab's own ant, the game's own breeding, nothing refilled) for FRAMES, and
# the pool is the KEEP of those whose population lived longest beside the ants (`coexist`, the share of samples with
# both alive; then the spiders' mean headcount; then the in-run rank). The lab's first look: one spider bred to 11,
# wiped a colony of 181 and starved out. SPIDERLAB_BIN / SPIDERLAB_DIR as LAB_BIN / LAB_DIR.
SPIDERLAB = next((x.split('=', 1)[1].split(':') for x in EXTRA if x.startswith('spiderlab=')), None)
EXTRA = [x for x in EXTRA if not x.startswith('spiderlab=')]
SPIDERLAB_BIN = os.environ.get('SPIDERLAB_BIN', 'target/release/examples/spiderlab')
SPIDERLAB_DIR = os.environ.get('SPIDERLAB_DIR', 'scratch/inrun/labscen')


def spider_sustain(r, i, fp):
    """(coexist, spider_mean) of one spider mind as a lab population (`spiderlab=`)."""
    import re
    d = f'{OUT}/spiderlab/r{r}/c{i:02}'
    os.makedirs(d, exist_ok=True)
    shutil.copy(fp, f'{d}/m.raw.txt')
    if os.path.exists(fp[:-len('.raw.txt')] + '.body.txt'):
        shutil.copy(fp[:-len('.raw.txt')] + '.body.txt', f'{d}/m.body.txt')
    sc, frames, n, _ = SPIDERLAB
    out = subprocess.run([SPIDERLAB_BIN, f'scenario={sc}', f'pool={d}', f'spiders={n}', f'frames={frames}', f'seed={r * 100 + i + 1}', 'every=5000'],
                         capture_output=True, text=True, env={**os.environ, 'PIXEL_PHYSICS_LAB_SCENARIOS': SPIDERLAB_DIR}).stdout
    open(f'{d}.log', 'w').write(out)
    m = re.search(r'coexist=([\d.]+) spider_mean=([\d.]+)', out)
    return (float(m.group(1)), float(m.group(2))) if m else (0.0, 0.0)
EXTRA = [x for x in EXTRA if not x.startswith('spiderevo=')]
spool = next((x.split('=', 1)[1].rsplit(':', 1)[0] for x in EXTRA if x.startswith('spiders=')), None)
if SPIDEREVO:
    assert len(SPIDEREVO) in (4, 5) and spool, 'spiderevo=KEEP:PM:SIGMA:EVENTS[:ELITE] with spiders=POOL:N'
def spider_args(args, out=None):
    """The current pool in `spiders=`; with `out`, the round's mutation and the placements' log there too."""
    if not SPIDEREVO or not any(x.startswith('spiders=') for x in args):
        return args
    a = [f"spiders={spool}:{x.rsplit(':', 1)[1]}" if x.startswith('spiders=') else x for x in args]
    return a + ([f"spider_mut={':'.join(SPIDEREVO[1:])}", f'spider_out={out}'] if out else [])
def evolve_spiders(r, dirs):
    """Round r+1's pool from round r's placements over every colony. Without ELITE: the KEEP best placements, each on
    its one life. With it: the pool minds (every life they have had, this round's unmutated placements added) and the
    round's mutants (one life each) rank together on a mean shrunk PRIOR lives toward the round's median."""
    global spool, spool_hist
    rows = []
    for c, d in sorted(dirs.items()):
        f = f'{d}/spiders/scores.csv'
        if os.path.exists(f):
            rows += [(float(x['score']), c, int(x['k']), f"{d}/spiders/k{int(x['k']):03}.raw.txt", int(x['parent']), x.get('mutant', '1') == '1')
                     for x in csv.DictReader(open(f))]
    if not rows:
        return
    med = sorted(x[0] for x in rows)[len(rows) // 2]
    if len(SPIDEREVO) == 5:
        members = sorted(fn for fn in os.listdir(spool) if fn.endswith('.raw.txt'))
        hist = {m: list(spool_hist.get(f'{spool}/{m}', [])) for m in members}
        for sc, c, k, fp, p, mutant in rows:
            if not mutant:
                hist[members[p]].append(sc)
        shrunk = lambda v: (sum(v) + PRIOR * med) / (len(v) + PRIOR)
        cands = [(shrunk(v), -1, i, f'{spool}/{m}', v) for i, (m, v) in enumerate(sorted(hist.items()))]
        cands += [(shrunk([sc]), c, k, fp, [sc]) for sc, c, k, fp, p, mutant in rows if mutant]
    else:
        cands = [(sc, c, k, fp, [sc]) for sc, c, k, fp, p, mutant in rows]
    cands.sort(key=lambda x: (-x[0], x[1], x[2]))
    if SPIDERLAB:
        top = cands[:int(SPIDERLAB[3])]
        with ThreadPoolExecutor(P) as ex:
            lab = list(ex.map(lambda t: spider_sustain(r, t[0], t[1][3]), enumerate(top)))
        order = sorted(range(len(top)), key=lambda i: (-lab[i][0], -lab[i][1], i))
        print(f'round {r}: spiderlab coexist of the top {len(top)}: ' + ' '.join(f'{lab[i][0]:.2f}' for i in order), flush=True)
        cands = [top[i] for i in order] + cands[len(top):]
    keep = cands[:int(SPIDEREVO[0])]
    nd = f'{OUT}/spool/r{r + 1}'
    os.makedirs(nd, exist_ok=True)
    new = not os.path.exists(f'{OUT}/spool.csv')
    new_hist = {}
    with open(f'{OUT}/spool.csv', 'a') as f:
        if new:
            f.write('round,rank,score,colony,k,placements,median_score,lives\n')
        for i, (sc, c, k, fp, v) in enumerate(keep):
            shutil.copy(fp, f'{nd}/{i:02}.raw.txt')
            # The breeding genes ride with the mind (`predator::BREEDING`, owner 2026-10-25).
            if os.path.exists(fp[:-len('.raw.txt')] + '.body.txt'):
                shutil.copy(fp[:-len('.raw.txt')] + '.body.txt', f'{nd}/{i:02}.body.txt')
            new_hist[f'{nd}/{i:02}.raw.txt'] = v
            f.write(f'{r},{i},{sc:.4f},{c},{k},{len(rows)},{med:.4f},{len(v)}\n')
    spool, spool_hist = nd, new_hist
    kept_parents = sum(1 for x in keep if x[1] == -1)
    print(f'round {r}: {len(rows)} spiders placed, median score {med:.2f}, best {keep[0][0]:.2f}; {kept_parents} parents kept; pool -> {nd}', flush=True)
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
# cg_every=N as a jobs-line arg (the cloud runner passes only P to the driver's environment). The training compute
# audit (2026-10-24) measured the garden at 30% of a batch's CPU at every 2 rounds while decisions read one or two of
# them: 8 saves ~22%.
CG_EVERY = next((int(x.split('=', 1)[1]) for x in EXTRA if x.startswith('cg_every=')), CG_EVERY)
EXTRA = [x for x in EXTRA if not x.startswith('cg_every=')]
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
    assert k in ('spoil', 'crater', 'craterfull', 'rooms', 'shape', 'guard', 'slay', 'lab', 'scale'), k
    EXTRA_CASES.append(k)
# +craterfull (nest lane, 2026-10-28): the crater case, counting the hole with what fills it (nestcount `crater_full`:
# the crumbs and food in the sky-open space under a widened mouth, as well as its open air). Use it in place of +crater.
# **Why**: the trained ant cuts the door's rim and its mouth widens from 5 to 9-12 cells, but the crumb pile fills the
# hole, so at the 200k judge `crater` sees ~42 cells of a ~190-cell crater (std_pool, 16 exam worlds; the hand-built
# ant 0 both ways). The colsel.csv/cg.csv `crater` column then holds the full count. Unset: the run unchanged.
CRATER_FULL = 'craterfull' in EXTRA_CASES
SPOIL = os.environ.get('SPOIL') == '1' or 'spoil' in EXTRA_CASES
# +lab (teacher lane, 2026-10-24): each round, every colony's carry also lands in lab boxes through the lab's own code
# (examples/labsurvive.rs), and its mean headcount over the LAB_AFTER frames after landing (labsurvive `lived`, in tenths), summed over LAB_SCENS, is a case. **Why**: the
# owner's playtest found trained colonies starving in the lab within ~5k frames; labsurvive put it on food distance --
# the pool lived with food 10-50 columns out and every ant starved at 100-240 (the hand-built ant lived at all of
# them), because every training bed had dropped its food 25 columns from the door. The scenarios live in LAB_DIR.
LAB = 'lab' in EXTRA_CASES
# labgate=Q (teacher lane, 2026-10-24, with +lab): a donor must also have a lab case at or above the Q-th percentile of
# the round's donors (Q=50: the better half). **Why**: farfood3b's +scale arm won the nest the owner asked for (garden
# nest 318 vs 127, 29 of 32 pairs) and lost the lab case in 30 of 32 -- with nine lexicase cases, lab survival led only
# one pick in nine, and the nest cases outvoted it. A gate makes lab survival a floor every donor clears rather than one
# vote among many, without weighting the lexicase (which keeps "No weights").
LABGATE = next((float(x.split('=', 1)[1]) for x in EXTRA if x.startswith('labgate=')), None)
EXTRA = [x for x in EXTRA if not x.startswith('labgate=')]
# scalegate=Q (teacher lane, 2026-10-25; owner: the nest is "one of the biggest issues"): a donor must also have a nest
# scale (`+scale`'s nest / what the colony needs, v[17]) at or above the Q-th percentile of the round's donors, applied
# after labgate=. **Why**: as one lexicase vote among ten, nest size loses to food; labgate= made lab survival stick
# the same way, and widebed1's scalegate arm (+scale with labgate=50) dug 220 against 165.
SCALEGATE = next((float(x.split('=', 1)[1]) for x in EXTRA if x.startswith('scalegate=')), None)
EXTRA = [x for x in EXTRA if not x.startswith('scalegate=')]
assert LABGATE is None or LAB, 'labgate= needs +lab'
# +scale (owner, 2026-10-24 16:34Z: "select for nests that scale with food, brood, ants"): the nest the colony has at the
# end against the nest it needs, NEED = A x living ants + B x brood (eggs, larvae, pupae) + F x food cells it holds
# (stock.csv nest_food + door_food, wherever they lie), case = min(nest / NEED, SCALE_CAP). Why a needed size and not
# the owner's first idea, open cells over filled ones: a ratio of open to filled scores a colony that leaves its nest
# empty and stores nothing; here food and brood raise the need wherever they sit, so not using the nest cannot pay,
# and a bigger colony or a bigger store needs a bigger nest. F = 2: a stored cell fills its own cell and needs an
# aisle, so storing more must mean digging more (F = 1 would let food fill the nest without ever growing it).
# Measured before it was built (farfood2 ramp2far r23, 64 colonies, A,B,F = 6,3,2): nest 75-396 cells for 64-93 ants
# holding 210-483 food cells, 2.5 nest cells per ant, nest / NEED 0.08-0.32 -- every colony far under, with spread
# for selection to act on. The cap keeps a colony from buying the case with a huge empty bowl over feeding.
# donor600=FRAMES[:Q] (selection review + teacher lane, 2026-10-27; off by default): after labgate= and scalegate=, each
# remaining donor candidate's carry also runs once for FRAMES frames in a fresh world (seed_of('long'), the garden's
# args, selection off), and only candidates whose (alive at the end, mean alive over the run) is at or above the Q-th
# percentile of the candidates' (default 50) stay donors. Logged to OUT/donor600.csv. **Why**: the environment review
# (PR #941) found std_pool's garden 15/16 alive at 200k and 0/16 at 600k on the same bed, while the hand-built ant kept
# 14/16 -- every judge here stops at 200k (the lab case at 15k after landing), so the late game cannot be selected for.
# Only the ~4 post-gate candidates run, not all 32 colonies: about +37% compute a round at FRAMES=600000 against 3x.
# Mean alive (stock.csv, every 1,000 frames) grades colonies that all die out before the end, where alive alone ties.
# With ~4 candidates Q=50 keeps 2; widen the gates before it (labgate=25 scalegate=25) to keep more families. Check it
# repeats (scratch/inrun/selaudit.py, or the log here) before trusting it as a gate.
DONOR_LONG = next((tuple(float(v) for v in x.split('=', 1)[1].split(':')) for x in EXTRA if x.startswith('donor600=')), None)
EXTRA = [x for x in EXTRA if not x.startswith('donor600=')]
# donorlab=FRAMES[:Q[:A,B]] (teacher lane, 2026-10-28; off by default): donor600's check run in lab boxes instead of on the
# bed -- after the gates (and after donor600= if both are set), each remaining candidate's carry lands in each of the
# lab scenarios A,B (default lab_herbs,lab_far_mix from LAB_DIR) on the lab case's own seeds and runs FRAMES frames
# after landing; candidates whose (alive at the end, mean headcount), summed over the boxes, is at or above the Q-th
# percentile (default 50) stay donors. Logged to OUT/donorlab.csv. **Why**: six held-out exams on 2026-10-28 (x1-x6,
# the game's six boxes at 600k): every pool 24 rounds of selection produced landed level with or below its founder --
# k3c23 6/30 against std_pool, env2 s_d6 10/22, env4 f_w6 8/23 against its own founder -- and the watchdog lane found
# no training case, donor600's bed check included, ranks donors by survival (311 donors, every case within +-0.25).
# The exam's boxes are lab boxes; this puts a long lab run between a candidate and the next generation.
# donorlab=FRAMES:Q:A,B,...:lived ranks on mean headcount first (then alive at the end); a box named twice runs twice
# on two seeds. **Why** (2026-10-28, x7 and the rank checks): at the default boxes the check is a random pick -- the
# labscen twins agree with themselves seed to seed at -0.09..+0.27 and with the exam at -0.19..+0.11, because 73-93 of
# 96 trained runs die in them (twinself-1028; twinenv-1028: the box, not the door rule). Two non-exam game boxes do
# rank colonies as the exam does: played_bed_scrambler + played_bed_understory, 3 seeds each at 600k, Spearman of mean
# headcount +0.67 against the exam's plant boxes (+0.59 with played_bed left out; 2 seeds each +0.61, 1 seed +0.45),
# twingame-1028 over the 32 t_fama carries. Both are copied into LAB_DIR for this.
DONOR_LAB = next((x.split('=', 1)[1].split(':') for x in EXTRA if x.startswith('donorlab=')), None)
EXTRA = [x for x in EXTRA if not x.startswith('donorlab=')]
SCALE_W = tuple(float(x) for x in os.environ.get('NEST_NEED', '6,3,2').split(','))
SCALE_CAP = float(os.environ.get('SCALE_CAP', '1.25'))
# scaleopen=1 / SCALE_OPEN=1 (teacher lane, 2026-10-25): score only the nest's OPEN cells against the need. nest1-1025's scalegate
# grew the nest 116 -> 424 cells over random food distances, but the gain was stored food: open room stayed 66-112 cells
# in every arm (food 342-666) and the hand-built ant keeps ~1,100 open. With food out of the numerator, a stored cell
# costs the room it fills, so a colony keeps the case by digging around its store (a second room) or storing less.
# Use with NEST_NEED's F at 1 (an aisle per stored cell), not 2: the food is already charged once in the numerator.
# Job lines cannot set colsel's environment on the VM, so both ride as args: `scaleopen=1`, `nestneed=A,B,F`.
SCALE_OPEN = os.environ.get('SCALE_OPEN', '0') == '1' or 'scaleopen=1' in EXTRA
# elite=1 (teacher lane, 2026-10-27): only a colony refounded last round (and every colony in round 0) breeds with the
# job's mutation; a colony that kept its own place runs with kpm=0 kevents=0, so the top half is carried unchanged and
# only the children that replace the bottom half vary -- the (mu+lambda) elitism a genetic algorithm uses against drift.
# **Why**: drift1 and self1 (2026-10-27) found in-run mutation outruns colony selection. At the default kpm 0.013 a
# hand-laid come-home latch decayed (garden starved 46 -> 311 in 48 rounds) and std_pool traded its nest for food
# (nest 596 -> 129 over 96 rounds); at kpm 0.003 std_pool improved nothing in 96 rounds; with no mutation every trait
# held. inrun's `arg` takes the first match, so the zeros go in front of the job's own kpm.
# **Default on since 2026-10-27** (Teacher lane, plan A1 "unless you object", owner did not): elite1/elite2/clean1 all
# ran it, and clean1's five alternatives (kev1, ktone, kdec, kgoal, kboth) found nothing that starved less (PR #955).
# `elite=0` (or env ELITE=0) turns it off, for a batch that must match one from before the default changed.
ELITE = os.environ.get('ELITE', '1') == '1' and 'elite=0' not in EXTRA
# refound_mut=PM:SIGMA:EVENTS and carry=random (teacher lane, 2026-10-27, from the selection review's arm (b), PR #939):
# mutation happens once, when a child colony is refounded -- each of the donor's 30 carried genomes is written as one
# size-neutral mutant (inrun's `mutate_to`) -- and nowhere else; run the job with kpm=0 kevents=0 and sel=random so
# the round neither mutates nor selects inside the colony, and carry=random takes 30 of the round's ants blind instead
# of the top 30 by ant score. Colony selection is then the only selection there is. **Why**: the review found a child
# worse than its own unmutated donor in the same round on lab and nest in 57-69% of 1,520 pairs (control 54%), an
# unmutated colony's order across two worlds agreeing only 0.05-0.09, and in-round breeding narrowing 24 lines to 2-3
# per round (every ant at r95 descends from one ant 7 rounds back): the in-round score works against the colony cases.
REFOUND_MUT = next((x.split('=', 1)[1] for x in EXTRA if x.startswith('refound_mut=')), None)
CARRY_RANDOM = 'carry=random' in EXTRA
# islands=K[:M] (selection lane, 2026-10-27; off by default, K=1): the island model. The colonies are split into K
# islands of NCOL/K by index (colony c in island (c-1)*K//NCOL), and each round's ranking, gates, kept half and
# refounding happen inside each island, so a family can only take over its own island. Every M rounds (default 4) each
# island's best colony also refounds the next island's worst (a migrant), so a family that wins in one island is tried
# in another without taking it over. colsel.csv's `rank` is then the place within the colony's island. **Why**: every
# clean1 arm, random carry included, was down to ONE round-0 colony by round 15-31 (lineage.py), and among near-clones
# no case can repeat: clean1 k3's starvation retest is 0.03 (measures.py) while two different pools pooled give 0.66
# (the selection review's positive control). Selection among copies of one family is luck by construction.
ISLANDS = next((tuple(int(v) for v in x.split('=', 1)[1].split(':')) for x in EXTRA if x.startswith('islands=')), None)
EXTRA = [x for x in EXTRA if not x.startswith('islands=')]
NISL = ISLANDS[0] if ISLANDS else 1
MIGRATE = ISLANDS[1] if ISLANDS and len(ISLANDS) > 1 else 4
assert NISL >= 1 and NCOL % NISL == 0 and NCOL // NISL >= 4, 'islands=K needs NCOL divisible by K, 4+ colonies each'
assert MIGRATE >= 1, 'islands=K:M needs M >= 1'
def island(c):
    return (c - 1) * NISL // NCOL
SEL_ARG = next((x for x in EXTRA if x.startswith('sel=')), 'sel=score')  # was hardwired: arg() takes the first match
SCALE_W = next((tuple(float(v) for v in x.split('=', 1)[1].split(',')) for x in EXTRA if x.startswith('nestneed=')), SCALE_W)
# shapeopen=1 (teacher lane, 2026-10-25): +shape reads rooms from the nest's open cells only (roomshape.OPEN_ONLY):
# with stored food counted as space, nest1's best "rooms" colony (85) was a lumpy food pile with 17 open cells.
roomshape.OPEN_ONLY = 'shapeopen=1' in EXTRA
EXTRA = [x for x in EXTRA if not x.startswith(('scaleopen=', 'nestneed=', 'shapeopen='))]


def nest_scale(rdir, nest):
    try:
        s = list(csv.DictReader(open(f'{rdir}/stock.csv')))[-1]
        alive, food = int(s['alive']), int(s['nest_food']) + int(s['door_food'])
    except (OSError, IndexError):
        return 0.0
    brood = 0
    if os.path.exists(f'{rdir}/brood.csv'):
        rows = list(csv.DictReader(open(f'{rdir}/brood.csv')))
        brood = sum(int(rows[-1][k]) for k in ('eggs', 'larvae', 'pupae')) if rows else 0
    if alive == 0:
        # A colony that died out needs no nest, so leftover food alone made its need tiny and its case the cap:
        # linkprice1-1025's colony 1 (0 ants, 22 food cells, nest 134) scored 1.25, the best a colony can.
        return 0.0
    need = SCALE_W[0] * alive + SCALE_W[1] * brood + SCALE_W[2] * food
    return round(min(nest / need, SCALE_CAP), 4) if need > 0 else 0.0
LAB_BIN = os.environ.get('LAB_BIN', 'target/release/examples/labsurvive')
LAB_DIR = os.environ.get('LAB_DIR', 'scratch/inrun/labscen')
LAB_SCENS = os.environ.get('LAB_SCENS', 'lab_far_mix,lab_mid_fruit').split(',')
# labscens=A,B (jobs arg; teacher lane, 2026-10-25): the same list from a jobs line, since the cloud runner passes only
# P to the driver's environment -- LAB_SCENS set by hand never reached a VM batch.
LAB_SCENS = next((x.split('=', 1)[1].split(',') for x in EXTRA if x.startswith('labscens=')), LAB_SCENS)
EXTRA = [x for x in EXTRA if not x.startswith('labscens=')]
# labcase=A,B (teacher lane, 2026-10-25, with +lab): lab scenarios scored each as a lexicase case of its own, beside the
# summed LAB_SCENS case (and outside labgate=). **Why**: herbs1 added lab_herbs (the owner's herb-bed opening, plants
# and no heaps) to the summed case and 24 rounds moved nothing -- herb headcount ahead of the control in 15 of 32 r23
# carries, median 21 against 17 -- because ~17 ants on herbs is a sliver of a sum the far-food boxes (~65 each) set.
# Logged as colsel.csv columns lab2_<name> after `lab`.
LAB_CASES = next((x.split('=', 1)[1].split(',') for x in EXTRA if x.startswith('labcase=')), [])
EXTRA = [x for x in EXTRA if not x.startswith('labcase=')]
assert not LAB_CASES or 'lab' in EXTRA_CASES, 'labcase= needs +lab'
LAB_AFTER = os.environ.get('LAB_AFTER', '15000')
# labafter=N (jobs arg; teacher lane, 2026-10-29): LAB_AFTER from a jobs line, as labscens= is, since a VM batch passes
# only P to the driver's environment. **Why**: the rank checks of 2026-10-28 (twinrank, twinself, twingame) found the
# only lab boxes that rank colonies as the exam does are played_bed_scrambler + played_bed_understory at 600k, 2-3
# seeds each (+0.61/+0.67); a 15k headcount there was never checked. Used with weights= to make that the whole score.
LAB_AFTER = next((x.split('=', 1)[1] for x in EXTRA if x.startswith('labafter=')), LAB_AFTER)
EXTRA = [x for x in EXTRA if not x.startswith('labafter=')]
# labscore=alive:AFTER[:K] (teacher lane + watchdog lane, 2026-10-28; off by default): a lab run is scored on the colony
# alive AFTER frames after landing, less K x the ants that starved (default K 0), in place of the mean headcount over
# LAB_AFTER. **Why**: the watchdog lane reran env1 c's 28 r23 carries on their own lab seeds to 300k; the 15k headcount
# ranks them against mean alive at 150-300k at Spearman +0.03 (against alive at 300k -0.14, its rank 0 dead in both
# boxes), because headcount rises with a hatching boom that then starves (a median 67% of what they hatch). Alive at
# 100k ranks them at +0.57 (+0.41), alive100k - 0.1 x starved +0.59 (+0.47); full subtraction (K 1) -0.03, since every
# trained colony starves thousands, though it alone puts all 8 hand-built runs above every trained one. About 13x the
# lab run's cost at 100k. Logged in the same `lab`/`lab2_*` columns (alive in tenths, as lived was).
LAB_SCORE = next((x.split('=', 1)[1].split(':') for x in EXTRA if x.startswith('labscore=')), None)
EXTRA = [x for x in EXTRA if not x.startswith('labscore=')]
if LAB_SCORE:
    assert LAB_SCORE[0] == 'alive' and len(LAB_SCORE) in (2, 3), 'labscore=alive:AFTER[:K]'
    LAB_AFTER = LAB_SCORE[1]
# The arm's own sense switches reach the lab box too (a lab case judged on another mind's senses judges nothing): every
# setenv= pair but the training scaffolds (PP_BODYRULES*, PP_HOMEEAT*, which the lab never runs).
LAB_ENV = {k: v for x in EXTRA if x.startswith('setenv=') for k, v in (kv.split('=', 1) for kv in x[len('setenv='):].split(','))
           if not k.startswith(('PP_BODYRULES', 'PP_HOMEEAT', 'PP_CARRY', 'PP_HOMEFAR'))}


# oneworld=1 (selection lane, 2026-10-28): every colony plays the SAME world in a round -- training seed, food
# distance draws, heaps and the lab case's seeds are colony 1's -- and a new one each round, so a round's ranking
# compares colonies on one test instead of on 32 different ones. **Why**: in clean1 the world sets a colony's
# starvation more than the colony does (Spearman between two arms on the same seeds 0.24; the same colony kept into a
# fresh world 0.03, `measures.py`), so with a world each, starvation ranks worlds. The common garden keeps a world per
# colony (it pairs arms by colony). Costs nothing: the same runs, other seeds. Off, every seed is as before.
# **The births' stream stays each colony's own** (inrun `sseed=`, below in `go`): the first screen (selow1, 2026-10-28)
# ran without it, and colonies sharing a carry in one world were one deterministic run -- one or two distinct outcomes
# a round, and all 32 died out together at round 23. A risk that remains by design: a world lethal to every colony
# now kills every colony in the same round.
ONEWORLD = 'oneworld=1' in EXTRA
EXTRA = [x for x in EXTRA if x != 'oneworld=1']


def wc(c):
    """The colony whose world colony c plays this round: itself, or colony 1's under oneworld=1."""
    return 1 if ONEWORLD else c


def seed_of(kind, c, r=0, k=0, own=False):
    """Every world seed the driver hands out, in one place (selection review, 2026-10-27). The old seeds were sums
    that ran into each other: training c*1000+r met the garden's c*1000+99 at round 99, the lab case's
    c*1000+10r+k met colony c+1's from round 100, and each labcase= scenario (c*1000+10(r+100(k+1))) took colony
    c+k+1's first lab seed every round. Training below round 99 and the lab case below round 100 keep their old
    seeds, so earlier controls stay reusable; past those rounds, and for labcase=, each kind has its own range."""
    if kind in ('train', 'lab', 'labcase', 'labl') and not own:
        c = wc(c)
    if kind == 'train':
        return c if r == 0 else (c * 1000 + r if r < 99 else 10_000_000 + c * 1000 + r)
    if kind == 'garden':
        return c * 1000 + 99
    if kind == 'lab':
        return c * 1000 + r * 10 + k if r < 100 else 20_000_000 + c * 10_000 + r * 10 + k
    if kind == 'labcase':
        return 30_000_000 + c * 100_000 + r * 100 + k
    if kind == 'labl':  # donorlab=
        return 50_000_000 + c * 10_000 + r * 10 + k
    if kind == 'long':
        return 40_000_000 + c * 1000 + r
    raise ValueError(kind)


def lab_alive(c, r, carry_c, scens=None, kind='lab', case=0):
    import re
    tot = 0
    for k, sc in enumerate(scens or LAB_SCENS):
        out = subprocess.run([LAB_BIN, f'founders={carry_c}', f'scenario={sc}', f'seed={seed_of(kind, c, r, case + k)}', f'after={LAB_AFTER}'],
                             capture_output=True, text=True, env={**os.environ, **LAB_ENV, 'PIXEL_PHYSICS_LAB_SCENARIOS': LAB_DIR}).stdout
        if LAB_SCORE:
            m = re.search(r'RESULT alive=(\d+) lived=[\d.]+ peak=\d+ starved=(\d+)', out)
            k = float(LAB_SCORE[2]) if len(LAB_SCORE) > 2 else 0.0
            tot += round(10 * (int(m.group(1)) - k * int(m.group(2)))) if m else 0
            continue
        m = re.search(r'RESULT alive=\d+ lived=([\d.]+)', out)  # mean headcount after landing: graded, unlike alive
        tot += round(10 * float(m.group(1))) if m else 0  # tenths: whole ants rounded every early-dying colony to one value
    return tot
C = [f'frames={F}', f'dist={BASE_DIST}', 'spread=10', 'grants=3', 'window=4000', SEL_ARG] + EXTRA
CGC = [a if not a.startswith('sel=') else 'sel=off' for a in C if not a.startswith('frames=')] + ['frames=200000']
# near=N (inrun's home-side heap) is training only: the garden keeps both heaps at BASE_DIST, so an arm's gardens are
# the same worlds as its control's (teacher lane, 2026-10-25).
CGC = [a for a in CGC if not a.startswith('near=')]
CGC = with_p(CGC, 0.0) if WEAN else CGC
CGC = with_capp(CGC, 0.0) if CAPWEAN else CGC
os.makedirs(OUT, exist_ok=True)


def run(args, out):
    os.makedirs(out, exist_ok=True)
    with open(f'{out}.log', 'w') as lf:
        subprocess.run([BIN] + args + [f'out={out}'], stdout=lf, stderr=subprocess.STDOUT, check=False)
    for fn in os.listdir(out):  # maps and per-window ant lists: not read here
        if fn.startswith('map_f') or fn.startswith('ants_f'):
            os.remove(f'{out}/{fn}')
    return out


def mutate_carry(fs, b, r):
    """refound_mut=: the founders string with every genome replaced by one mutant of it, written under OUT/c{b}/mut{r}."""
    pm, sigma, ev = REFOUND_MUT.split(':')
    d = f'{OUT}/c{b}/mut{r}'
    os.makedirs(d, exist_ok=True)
    out = []
    for k, e in enumerate(fs.split(',')):
        path, rest = e.split('@', 1)
        dst = f'{d}/{k:02}.raw.txt'
        subprocess.run([BIN, f'founders={path}:1', f'out={d}/tmp', f'mutate_to={dst}:{pm}:{sigma}:{ev}:{b * 1000003 + r * 101 + k}'],
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True)
        out.append(f'{dst}@{rest}')
    return ','.join(out)


def long_alive(c, r, carry_c):
    """donor600=: (alive at the end, mean alive) of one carry over DONOR_LONG[0] frames, selection off."""
    args = [a for a in CGC if not a.startswith('frames=')] + [f'frames={int(DONOR_LONG[0])}']
    d = run([f'founders={carry_c}', f'seed={seed_of("long", c, r)}'] + spider_args(with_spiders(with_cap(args), r)), f'{OUT}/long/r{r}/c{c}')
    try:
        al = [int(x['alive']) for x in csv.DictReader(open(f'{d}/stock.csv'))]
    except OSError:
        al = []
    shutil.rmtree(d, ignore_errors=True)
    return (al[-1], round(sum(al) / len(al), 2)) if al else (0, 0.0)


def lab_long(c, r, carry_c):
    """donorlab=: (alive at the end, mean headcount), summed over its boxes, of one carry FRAMES after landing."""
    import re
    scens = DONOR_LAB[2].split(',') if len(DONOR_LAB) > 2 else ['lab_herbs', 'lab_far_mix']
    al = lv = 0.0
    for k, sc in enumerate(scens):
        out = subprocess.run([LAB_BIN, f'founders={carry_c}', f'scenario={sc}', f'seed={seed_of("labl", c, r, k)}',
                              f'after={int(float(DONOR_LAB[0]))}'], capture_output=True, text=True,
                             env={**os.environ, **LAB_ENV, 'PIXEL_PHYSICS_LAB_SCENARIOS': LAB_DIR}).stdout
        m = re.search(r'RESULT alive=(\d+) lived=([\d.]+)', out)
        if m:
            al, lv = al + int(m.group(1)), lv + float(m.group(2))
    return (int(al), round(lv, 2))


def carried(rdir):
    """rounds.sh's carry: top 30 by ant score, topped up to 30 requests by repeating."""
    idx = f'{rdir}/final/index.csv'
    if not os.path.exists(idx):
        return None
    r = list(csv.DictReader(open(idx)))
    r.sort(key=lambda x: int(x['id']))  # index.csv is in HashMap order: sort first so ties keep a fixed order
    if CARRY_RANDOM:
        import zlib
        random.Random(zlib.crc32(rdir.encode())).shuffle(r)
    else:
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
    home_j = 0
    if os.path.exists(ev):
        with open(ev) as f:
            next(f)
            for line in f:
                p = line.split(',')
                k = p[2]
                if k == 'deliver' and abs(int(p[3]) - 128) <= 8:
                    home += 1
                    home_j += int(p[5])
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
    # home8 in energy as well as cells (selection lane, 2026-10-27): inrun's deliver row carries the cell's worth as
    # put down (v; 960 = a whole provisions/fruit/windfall cell), and 61% of the cells delivered in one std_pool round
    # (seed 5, food=mix) were part cells, mean 674. Kept beside the round, not in colsel.csv, whose columns other
    # readers index by position; scratch/inrun/measures.py reads it. Events from before the worth was logged give 0.
    if os.path.exists(ev):
        with open(f'{rdir}/home.csv', 'w') as f:
            f.write(f'home8_cells,home8_kj\n{home},{home_j / 1000:.1f}\n')
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
            n.get('nestcuts_late') or 0, n.get('crater_full' if CRATER_FULL else 'crater', 0), n.get('spoil', 0), n.get('spoil_in') or 0,
            roomshape.score(rdir) if n else 0.0, killed, slay,
            nest_scale(rdir, n.get('nest_open' if SCALE_OPEN else 'nest', 0)) if n else 0.0)


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
    c = c + (-v[11],) if 'crater' in EXTRA_CASES or CRATER_FULL else c
    c = c + (v[3],) if 'rooms' in EXTRA_CASES else c
    c = c + (v[14],) if 'shape' in EXTRA_CASES else c
    c = c + (-v[15],) if 'guard' in EXTRA_CASES else c
    c = c + (v[16],) if 'slay' in EXTRA_CASES else c
    c = c + (v[17],) if 'scale' in EXTRA_CASES else c
    return c + tuple(v[18:19 + len(LAB_CASES)]) if LAB else c


def case_names():
    """The names of cases()'s entries, in its order (colsel.csv's column names where one exists)."""
    n = ['home8', 'starved'] + (['nestcuts_late', 'nest'] if DIG == 'nest' else ['late_cuts', 'rooms'])
    n += ['spoil'] if SPOIL else []
    n += ['crater'] if 'crater' in EXTRA_CASES or CRATER_FULL else []
    n += [k for k in ('rooms', 'shape', 'guard', 'slay', 'scale') if k in EXTRA_CASES]
    return n + (['lab'] + [f'lab2_{sc}' for sc in LAB_CASES] if LAB else [])


# weights=NAME:W,... (selection lane, 2026-10-28; the selection review's "one case per goal, with an explicit weight
# the owner sets"): a case not named keeps weight 1, weight 0 drops it. The survival rank-sum becomes a weighted sum,
# and lexicase draws each pick's case order weighted (a case of weight 3 leads a pick three times as often as one of
# weight 1; Efraimidis-Spirakis keys). **Why**: unweighted, the six nest cases carried 63-71% of the vote by accident
# of how many were written per goal, and cases that do not repeat from world to world (starved 0.03, shape -0.03,
# spoil 0.02 in clean1 k3, `measures.py`) vote as loudly as food home (0.34). Unset: the run, rng draws included,
# as before.
WEIGHTS = next((dict((k, float(w)) for k, w in (kv.split(':') for kv in x.split('=', 1)[1].split(',')))
                for x in EXTRA if x.startswith('weights=')), None)
EXTRA = [x for x in EXTRA if not x.startswith('weights=')]


def case_weights():
    names = case_names()
    assert set(WEIGHTS) <= set(names), f'weights= names a case this run does not have: {set(WEIGHTS) - set(names)} (cases: {names})'
    return [WEIGHTS.get(k, 1.0) for k in names]


def dig_passed(v):
    return (v[10] if DIG == 'nest' else v[1]) >= GATE


def lexicase(pool, res, rng):
    """Epsilon-lexicase: per case (random order) keep the colonies within one median absolute deviation of the best."""
    cand = list(pool)
    # Every case cases() returns. Was range(4) until 2026-10-22 19:00Z, so colsel4-1022's +spoil arm never read its
    # fifth case and came out identical to its control in every round (a change that moved nothing).
    order = list(range(len(cases(res[cand[0]]))))
    if WEIGHTS is None:
        rng.shuffle(order)
    else:
        w = case_weights()
        assert len(w) == len(order), (case_names(), len(order))
        key = {k: rng.random() ** (1.0 / w[k]) for k in order if w[k] > 0}
        order = sorted(key, key=lambda k: -key[k])
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
        d = run([f'founders={carry[c]}', f'seed={seed_of("garden", c)}'] + spider_args(with_spiders(with_cap(CGC), r)), f'{OUT}/cg/r{r}/c{c}')
        v = result(d, 200000)
        shutil.rmtree(d, ignore_errors=True)
        return c, v
    with ThreadPoolExecutor(P) as ex:
        got = dict(ex.map(one, live))
    new = not os.path.exists(f'{OUT}/cg.csv')
    with open(f'{OUT}/cg.csv', 'a') as f:
        if new:
            f.write('round,colony,home8,late_cuts,starved,rooms,alive,cuts,dug,pit,nest,nestcuts,nestcuts_late,crater,spoil,spoil_in,shape,killed,slay,scale,pilecap,spider_at,train_dist\n')
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
    for fn in ('colsel.csv', 'cg.csv', 'donor600.csv'):
        fp = f'{OUT}/{fn}'
        if os.path.exists(fp):
            rows = open(fp).read().splitlines(True)
            open(fp, 'w').writelines(rows[:1] + [x for x in rows[1:] if int(x.split(',', 1)[0]) < R0])
    for c in range(1, NCOL + 1):
        shutil.rmtree(f'{OUT}/c{c}/r{R0}', ignore_errors=True)
    shutil.rmtree(f'{OUT}/cg/r{R0}', ignore_errors=True)
    shutil.rmtree(f'{OUT}/long/r{R0}', ignore_errors=True)
    print(f'resuming at round {R0}', flush=True)
logp = f'{OUT}/colsel.csv'
log = open(logp, 'a')
if os.path.getsize(logp) == 0:
    log.write('round,colony,home8,late_cuts,starved,rooms,alive,cuts,dug,pit,nest,nestcuts,nestcuts_late,crater,spoil,spoil_in,shape,killed,slay,scale,' + ('lab,' + ''.join(f'lab2_{sc},' for sc in LAB_CASES) if LAB else '') + 'passed_gate,rank,refounded_from,pilecap,spider_at,dist\n')
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
fresh = {c: True for c in range(1, NCOL + 1)}  # elite=1: breeds with mutation this round (round 0: all)
if R0:
    founders = {int(c): v for c, v in st0['founders'].items()}
    nest = {int(c): v for c, v in st0['nest'].items()}
    fresh = {int(c): v for c, v in st0.get('fresh', {}).items()} or fresh
    prev_dirs = {int(c): v for c, v in st0['prev_dirs'].items()}
    rng.setstate((st0['rng'][0], tuple(st0['rng'][1]), st0['rng'][2]))
    cap_now = st0['cap_now']
    spider_at = st0.get('spider_at', spider_at)
    spool = st0.get('spool', spool)
    spool_hist = st0.get('spool_hist', {})
    dist_now = st0.get('dist_now', dist_now)
    move_t = st0.get('move_t', move_t)
# DEADLINE (unix seconds; set by scripts/batch_run.sh, 2026-10-27): a GitHub runner stops at 6 h, so a leg ends
# between rounds rather than inside one. A round that would not finish by DEADLINE at the slowest pace this leg has
# seen is not started: the driver exits 75 with state.json at that round, and the next leg resumes there exactly as
# after a preemption. Unset (the VM), it never fires.
DEADLINE = float(os.environ.get('DEADLINE', '0'))
import time
t_round = 0.0
for r in range(R0, R):
    if DEADLINE and time.time() + 1.1 * t_round > DEADLINE:
        print(f'PAUSED before round {r}: the slowest round so far took {t_round:.0f} s and the leg ends in {DEADLINE - time.time():.0f} s', flush=True)
        sys.exit(75)
    t_start = time.time()
    live = [c for c in founders if founders[c] is not None]
    def go(c):
        inh = [f'inherit={nest[c]}'] if INHERIT and nest[c] else []
        cr = C if not DISTS else [f'dist={random.Random(wc(c) * 7919 + r).choice(DISTS)}' if x.startswith('dist=') else x for x in C]
        cr = [f'dist={dist_now}' if x.startswith('dist=') else x for x in cr] if DISTRAMP else cr
        # distramp= with patches=T:LO:HI (teacher lane, 2026-10-25; owner: food "farther from the nest (ideally still
        # always moving)"): the ramp moves the moving heap's range instead, patches=T:max(20, D/2):D for this round's
        # D, so training food keeps jumping while it walks outward on the colonies' starvation. The garden keeps the
        # job's own patches= (the far test). **Why**: nest4-1025 trained straight on patches=20000:40:120 and no arm
        # improved; trained colonies starved 110-280 a run there against the hand-built ant's 11.
        if TRAMP:
            cr = [f'patches={move_t}:{x.split("=", 1)[1].split(":", 1)[1]}' if x.startswith('patches=') else x for x in cr]
        if DISTRAMP:
            cr = [f'patches={x.split("=", 1)[1].split(":")[0]}:{max(20, dist_now // 2)}:{dist_now}' if x.startswith('patches=') else x
                  for x in cr]
        if HEAPS:
            draw = lambda k: HEAPS[0] + 5 * random.Random(wc(c) * 7919 + r * 31 + k).randrange((HEAPS[1] - HEAPS[0]) // 5 + 1)
            cr = [f'dist={draw(0)}' if x.startswith('dist=') else x for x in cr] + [f'near={draw(1)}']
        cr = with_p(cr, wean_p(r)) if WEAN else cr
        cr = with_capp(cr, capwean_p(r)) if CAPWEAN else cr
        cr = spider_args(with_spiders(with_cap(cr), r), f'{OUT}/c{c}/r{r}/spiders')
        cr = ['kpm=0', 'kevents=0'] + cr if ELITE and not fresh[c] else cr
        # under oneworld the births' stream stays the colony's own (inrun sseed=): without it, colonies with one carry
        # in one world are one run (selow1: 1-2 distinct outcomes a round, all 32 died out together at round 23)
        own = [f'sseed={seed_of("train", c, r, own=True)}'] if ONEWORLD else []
        return run(founders[c] + [f'seed={seed_of("train", c, r)}'] + own + inh + cr, f'{OUT}/c{c}/r{r}')
    with ThreadPoolExecutor(P) as ex:
        dirs = dict(zip(live, ex.map(go, live)))
    cols = list(range(1, NCOL + 1))
    res = {c: result(dirs[c], F) if c in dirs else None for c in cols}
    carry = {c: carried(dirs[c]) if c in dirs else None for c in cols}
    ok = [c for c in cols if carry[c] is not None]
    if LAB:
        with ThreadPoolExecutor(P) as ex:
            # one job per colony x box, summed, rather than one per colony looping its boxes: the same seeds (kind
            # 'lab', index k) and the same per-run rounding, so the case is unchanged, but a round of 600k boxes
            # (labafter=600000, env7 2026-10-29) is no longer four runs long end to end on a 64-core VM
            jobs = [(c, k, sc) for c in ok for k, sc in enumerate(LAB_SCENS)]
            got = list(ex.map(lambda j: lab_alive(j[0], r, carry[j[0]], [j[2]], 'lab', j[1]), jobs))
            labv = {c: 0 for c in ok}
            for (c, _, _), v in zip(jobs, got):
                labv[c] += v
            # Each labcase= scenario keeps its seed offset past LAB_SCENS', so adding one never moves the summed case.
            lab2 = [dict(zip(ok, ex.map(lambda c, k=k, sc=sc: lab_alive(c, r, carry[c], [sc], 'labcase', k), ok))) for k, sc in enumerate(LAB_CASES)]
        res = {c: (v + (labv.get(c, 0),) + tuple(d.get(c, 0) for d in lab2) if v else v) for c, v in res.items()}
    # The gate is late original-ground cuts only. A room in the final nest map no longer passes it: colsel-1022 found
    # 47 of 64 gate-arm colonies passing on a "room" with 0-1 late cuts (the nest score finds a chamber or niche in
    # nearly any nest, dug or not), and that arm's digging went to 0 by round 7. Rooms stay a lexicase case.
    passed = {c: c in ok and dig_passed(res[c]) for c in cols}
    src = {c: '' for c in cols}
    nxt = {c: carry[c] for c in cols}

    def select(grp):
        """Rank the colonies in grp and refound grp's worse half from its better half's donors; the order."""
        if MODE == 'plain':
            order = sorted([c for c in ok if c in grp], key=lambda c: c)
        else:
            gp = [c for c in ok if passed[c] and c in grp]
            rs = {}
            if gp:
                nk = len(cases(res[gp[0]]))
                rk = [ranks([cases(res[c])[k] for c in gp]) for k in range(nk)]
                w = case_weights() if WEIGHTS is not None else [1.0] * nk
                assert len(w) == nk, (case_names(), nk)
                rs = {c: sum(w[k] * rk[k][i] for k in range(nk)) for i, c in enumerate(gp)}
            order = sorted(gp, key=lambda c: (rs[c], c)) + sorted([c for c in ok if not passed[c] and c in grp], key=lambda c: (-res[c][1], c))
        order += [c for c in grp if c not in ok]  # extinct last
        if MODE in ('gate', 'lex') and ok:
            half = len(grp) // 2
            top = [c for c in order[:half] if carry[c] is not None]
            donors = [c for c in top if passed[c]] or top
            if LABGATE is not None and len(donors) > 1:
                lv = sorted(res[c][18] for c in donors)
                bar = lv[min(len(lv) - 1, int(len(lv) * LABGATE / 100))]
                donors = [c for c in donors if res[c][18] >= bar] or donors
            if SCALEGATE is not None and len(donors) > 1:
                sv = sorted(res[c][17] for c in donors)
                bar = sv[min(len(sv) - 1, int(len(sv) * SCALEGATE / 100))]
                donors = [c for c in donors if res[c][17] >= bar] or donors
            if DONOR_LONG is not None and len(donors) > 1:
                with ThreadPoolExecutor(P) as ex:
                    lv = dict(zip(donors, ex.map(lambda c: long_alive(c, r, carry[c]), donors)))
                q = DONOR_LONG[1] if len(DONOR_LONG) > 1 else 50.0
                srt = sorted(lv.values())
                bar = srt[min(len(srt) - 1, int(len(srt) * q / 100))]
                kept = [c for c in donors if lv[c] >= bar] or donors
                new_l = not os.path.exists(f'{OUT}/donor600.csv')
                with open(f'{OUT}/donor600.csv', 'a') as f:
                    if new_l:
                        f.write('round,colony,frames,alive_end,mean_alive,kept\n')
                    for c in donors:
                        f.write(f'{r},{c},{int(DONOR_LONG[0])},{lv[c][0]},{lv[c][1]},{int(c in kept)}\n')
                print(f'round {r}: donor600 ' + ' '.join(f'c{c}={lv[c][0]}/{lv[c][1]}' for c in donors) + f' -> {kept}', flush=True)
                donors = kept
            if DONOR_LAB is not None and len(donors) > 1:
                with ThreadPoolExecutor(P) as ex:
                    lv = dict(zip(donors, ex.map(lambda c: lab_long(c, r, carry[c]), donors)))
                q = float(DONOR_LAB[1]) if len(DONOR_LAB) > 1 else 50.0
                key = (lambda t: (t[1], t[0])) if len(DONOR_LAB) > 3 and DONOR_LAB[3] == 'lived' else (lambda t: t)
                srt = sorted(key(v) for v in lv.values())
                bar = srt[min(len(srt) - 1, int(len(srt) * q / 100))]
                kept = [c for c in donors if key(lv[c]) >= bar] or donors
                new_l = not os.path.exists(f'{OUT}/donorlab.csv')
                with open(f'{OUT}/donorlab.csv', 'a') as f:
                    if new_l:
                        f.write('round,colony,frames,alive_end,lived,kept\n')
                    for c in donors:
                        f.write(f'{r},{c},{int(float(DONOR_LAB[0]))},{lv[c][0]},{lv[c][1]},{int(c in kept)}\n')
                print(f'round {r}: donorlab ' + ' '.join(f'c{c}={lv[c][0]}/{lv[c][1]}' for c in donors) + f' -> {kept}', flush=True)
                donors = kept
            for i, b in enumerate(order[half:][::-1]):  # worst first
                if not donors:
                    break
                d = donors[i % len(donors)] if MODE == 'gate' else lexicase(donors, res, rng)
                nxt[b], src[b] = (mutate_carry(carry[d], b, r) if REFOUND_MUT else carry[d]), str(d)
        return order

    order = []
    for g in range(NISL):
        order += select([c for c in cols if island(c) == g])
    if NISL > 1 and MODE in ('gate', 'lex') and (r + 1) % MIGRATE == 0:
        # migration: each island's best colony (with a carry) also refounds the next island's worst, so a family
        # that wins in one island is tried in another without taking it over
        best = {g: next((c for c in order if island(c) == g and carry[c] is not None), None) for g in range(NISL)}
        for g in range(NISL):
            d, b = best[g], [c for c in order if island(c) == (g + 1) % NISL][-1]
            if d is not None and b != d:
                nxt[b], src[b] = (mutate_carry(carry[d], b, r) if REFOUND_MUT else carry[d]), str(d)
        print(f'round {r}: migration ' + ' '.join(f'c{best[g]}->island{(g + 1) % NISL}' for g in range(NISL)), flush=True)
    for c in cols:
        v = res[c] or (0,) * (19 + len(LAB_CASES) if LAB else 18)
        log.write(f'{r},{c},' + ','.join(str(x) for x in v) + f',{int(passed[c])},{order.index(c)},{src[c]},{"" if cap_now is None else cap_now},{spider_log(r)},{"" if dist_now is None else dist_now}\n')
    log.flush()
    if CG_EVERY and (r % CG_EVERY == CG_EVERY - 1 or r == R - 1):
        garden(r, carry)
    if SPIDEREVO and (not SPIDERRAMP or r >= SPIDERRAMP[3]):
        kl = sorted(res[c][15] if res[c] else 10**9 for c in cols)
        if SPIDERHOLD is not None and kl[len(kl) // 2] > SPIDERHOLD:
            print(f'round {r}: median killed {kl[len(kl) // 2]} > {SPIDERHOLD}: spider pool held, ants catch up', flush=True)
        else:
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
    if TRAMP:
        st = sorted(res[c][2] if res[c] else 10**9 for c in cols)
        med = st[len(st) // 2]
        if med <= TRAMP[1]:
            move_t = max(TRAMP[2], int(move_t * 0.8))
        elif med > 3 * TRAMP[1]:
            move_t = min(TRAMP[0], int(move_t / 0.8))
        print(f'round {r}: median starved {med}, food moves every {move_t} frames next round', flush=True)
    for c in cols:
        founders[c] = None if nxt[c] is None else [f'founders={nxt[c]}']
        home = int(src[c]) if src[c] else c
        nest[c] = f'{dirs[home]}/nestmap.txt' if home in dirs else None
        fresh[c] = bool(src[c])
    # the round before is no longer referenced by any carry: drop its genomes and event log, keep its csv summaries
    for c, d in prev_dirs.items():
        shutil.rmtree(f'{d}/final', ignore_errors=True)
        if os.path.exists(f'{d}/events.csv'):
            os.remove(f'{d}/events.csv')
    prev_dirs = dirs
    import json
    json.dump({'r': r + 1, 'founders': founders, 'nest': nest, 'prev_dirs': prev_dirs, 'rng': rng.getstate(), 'cap_now': cap_now, 'spider_at': spider_at, 'dist_now': dist_now, 'move_t': move_t, 'spool': spool, 'spool_hist': spool_hist, 'fresh': fresh},
              open(statep + '.tmp', 'w'))
    os.replace(statep + '.tmp', statep)
    print(f'round {r} done: {sum(passed.values())} passed the gate, {len(ok)} alive', flush=True)
    t_round = max(t_round, time.time() - t_start)
for d in prev_dirs.values():  # the last round keeps its genomes (the product) and gzips its log
    if os.path.exists(f'{d}/events.csv'):
        subprocess.run(['gzip', '-f', f'{d}/events.csv'])
print('DONE', flush=True)
