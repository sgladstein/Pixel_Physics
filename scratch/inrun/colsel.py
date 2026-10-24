"""SCRATCH (teacher lane): colony-level selection with digging as a gate, for the VM.

Grown from the scoring review lane's prototype (/mnt/project-files/scoring-review/colsel.py, 2026-10-22). Its test, 12
colonies x 6 rounds, gave 3x the food and a third of the starvation, but zero digging: its rank-sum included the raw nest
score, which is negative for every crater-digging colony and ~+1 for a colony that never digs, so it paid not digging.

usage: colsel.py OUT MODE NCOL ROUNDS FRAMES GENOME [inrun args...]
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
# spiders every colony scores 0 and the case drops nobody.
EXTRA_CASES = []
while '+' in MODE:
    MODE, k = MODE.rsplit('+', 1)
    assert k in ('spoil', 'crater', 'rooms', 'shape', 'guard'), k
    EXTRA_CASES.append(k)
SPOIL = os.environ.get('SPOIL') == '1' or 'spoil' in EXTRA_CASES
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
    home = late = starved = cuts = killed = 0
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
            roomshape.score(rdir) if n else 0.0, killed)


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
    return c + (-v[15],) if 'guard' in EXTRA_CASES else c


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
        d = run([f'founders={carry[c]}', f'seed={c * 1000 + 99}'] + with_spiders(with_cap(CGC), r), f'{OUT}/cg/r{r}/c{c}')
        v = result(d, 200000)
        shutil.rmtree(d, ignore_errors=True)
        return c, v
    with ThreadPoolExecutor(P) as ex:
        got = dict(ex.map(one, live))
    new = not os.path.exists(f'{OUT}/cg.csv')
    with open(f'{OUT}/cg.csv', 'a') as f:
        if new:
            f.write('round,colony,home8,late_cuts,starved,rooms,alive,cuts,dug,pit,nest,nestcuts,nestcuts_late,crater,spoil,spoil_in,shape,killed,pilecap,spider_at\n')
        for c in sorted(got):
            f.write(f'{r},{c},' + ','.join(str(x) for x in got[c]) + f',{"" if cap_now is None else cap_now},{spider_log(r)}\n')


rng = random.Random(12345)
logp = f'{OUT}/colsel.csv'
log = open(logp, 'a')
if os.path.getsize(logp) == 0:
    log.write('round,colony,home8,late_cuts,starved,rooms,alive,cuts,dug,pit,nest,nestcuts,nestcuts_late,crater,spoil,spoil_in,shape,killed,passed_gate,rank,refounded_from,pilecap,spider_at\n')
founders = {c: [f'founders={G}:1', 'mutants=30:0.03:1.0:7'] for c in range(1, NCOL + 1)}
prev_dirs = {}
nest = {c: None for c in range(1, NCOL + 1)}
for r in range(R):
    live = [c for c in founders if founders[c] is not None]
    def go(c):
        inh = [f'inherit={nest[c]}'] if INHERIT and nest[c] else []
        cr = C if not DISTS else [f'dist={random.Random(c * 7919 + r).choice(DISTS)}' if x.startswith('dist=') else x for x in C]
        cr = with_p(cr, wean_p(r)) if WEAN else cr
        cr = with_spiders(with_cap(cr), r)
        return run(founders[c] + [f'seed={c if r == 0 else c * 1000 + r}'] + inh + cr, f'{OUT}/c{c}/r{r}')
    with ThreadPoolExecutor(P) as ex:
        dirs = dict(zip(live, ex.map(go, live)))
    cols = list(range(1, NCOL + 1))
    res = {c: result(dirs[c], F) if c in dirs else None for c in cols}
    carry = {c: carried(dirs[c]) if c in dirs else None for c in cols}
    ok = [c for c in cols if carry[c] is not None]
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
        v = res[c] or (0,) * 16
        log.write(f'{r},{c},' + ','.join(str(x) for x in v) + f',{int(passed[c])},{order.index(c)},{src[c]},{"" if cap_now is None else cap_now},{spider_log(r)}\n')
    log.flush()
    if CG_EVERY and (r % CG_EVERY == CG_EVERY - 1 or r == R - 1):
        garden(r, carry)
    if PILECAP:
        # A colony with no run this round (extinct, not refounded) counts as starving, never as a colony that starved none.
        st = sorted(res[c][2] if res[c] else 10**9 for c in cols)
        med = st[len(st) // 2]
        if med <= PILECAP[2]:
            cap_now = max(PILECAP[3], cap_now - PILECAP[1])
        elif med > 3 * PILECAP[2]:
            cap_now = min(PILECAP[0], cap_now + PILECAP[1])
        print(f'round {r}: median starved {med}, pile cap next round {cap_now}', flush=True)
    if SPIDERRAMP and r >= SPIDERRAMP[3]:
        # An extinct colony counts as one that lost everything, never as one that lost none.
        kl = sorted(res[c][15] if res[c] else 10**9 for c in cols)
        med = kl[len(kl) // 2]
        if med <= SPIDERRAMP[2]:
            spider_at = max(SPIDERRAMP[4], spider_at - SPIDERRAMP[1])
        elif med > 3 * SPIDERRAMP[2]:
            spider_at = min(SPIDERRAMP[0], spider_at + SPIDERRAMP[1])
        print(f'round {r}: median killed {med}, spiders arrive at frame {spider_at} next round', flush=True)
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
    print(f'round {r} done: {sum(passed.values())} passed the gate, {len(ok)} alive', flush=True)
for d in prev_dirs.values():  # the last round keeps its genomes (the product) and gzips its log
    if os.path.exists(f'{d}/events.csv'):
        subprocess.run(['gzip', '-f', f'{d}/events.csv'])
print('DONE', flush=True)
