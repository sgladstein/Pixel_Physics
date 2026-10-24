"""Which dig rung is the frontier, for one in-run batch (dig-scoring thread, 2026-10-22).

usage: python3 rungcheck.py BATCH_DIR [--control ARM] [--ev EVREDUCE_CSV] [--nest 128]

Reads the Teacher lane's batch layout, BATCH_DIR/ARM_SEED/rN/{nest.csv, events.csv, ...} with an EXTINCT file in
ARM_SEED for a colony that died out. For each
arm it prints rung 0 (the feeding gate) and rungs 1-5 of "A ladder of dig metrics for the colony score"
(https://claude.ai/artifact/WkdjKUCBtZRsS2mU2HnGWE) with the agreed bars, and names the frontier: the lowest rung not
met. Rungs are judged on each surviving colony's LAST round, from that round's final nest.csv row (200k frames).

Bars (owner: "seems fine", 2026-10-22):
  0 feeding    extinct <= 10 of 64 over the batch (scaled to the seed count), and home food per colony-round
               >= the control arm's, PAIRED BY SEED over the rounds both colonies reached (Teacher lane, 2026-10-22:
               pooled food/seed weights arms by how long they lived, and strict=1 logs field drops as 'tdrop', not
               'deliver'). Home = deliveries within 12 columns of the door. Beside the bar it prints the discordant
               extinctions against the control (exact McNemar p) and on how many seeds the arm's food was higher (sign
               test p): an arm-vs-control gap of 5 extinctions is p~0.4. Food comes from --ev (the Teacher lane's
               evreduce.py CSV, reduced from the bucket's full logs) or else each round's events.csv; with neither, the
               gate's food half prints '?'.
  1 dig down   pit <= 10 cells on 80% of survivors          (printed: share of new-ground cuts 3+ rows deep)
  2 spoil out  >= 50% of spoil let go 20+ columns out, and <= 1 pellet let go inside per 10 new cuts  [events.csv]
  3 door open  open on 95% of checks after 20k frames       (nest.csv door_open; -1 = no nest yet, not counted)
               Provisional: the owner wants only COSTLY closures charged (a forager shut out, hungry ants shut in
               on a low store); until that check exists this prints the plain open share.
  4 a room     100+ roofed cells (nest.csv dug, which already leaves out pit) with at least one niche or chamber, on 80% of survivors
  5 nest       median nest score > 0 (the top rung: printed, no hand-over)
A rung whose data is missing (no events.csv pulled, no door column) prints '?' and does not count as met.
"""
import csv, glob, math, os, re, statistics, sys

args = sys.argv[1:]
def opt(name, default):
    if name in args:
        i = args.index(name); v = args[i + 1]; del args[i:i + 2]; return v
    return default
CONTROL = opt('--control', None)
EV = opt('--ev', None)
ROUNDS_FULL = 199000
NEST = int(opt('--nest', '128'))
if len(args) != 1:
    sys.exit(__doc__)
D = args[0].rstrip('/')

def rounds(seed_dir):
    return sorted(glob.glob(seed_dir + '/r*'), key=lambda p: int(p.rsplit('r', 1)[1]))

def ev_rows():
    """{arm: {seed: {round: home deliveries}}} from the evreduce CSV."""
    d = {}
    for r in csv.DictReader(open(EV)):
        a, sd = r['job'].rsplit('_', 1)
        d.setdefault(a, {}).setdefault(int(sd), {})[int(r['round'])] = int(r['deliver']) - int(r['deliver_far'])
    return d
EVD = ev_rows() if EV else {}

def arm_stats(arm):
    seeds = sorted(glob.glob(f'{D}/{arm}_*'))
    n = len(seeds); extinct = 0
    food = {}  # seed -> {round: home deliveries}; a seed without data is left out
    dead = {}  # seed -> extinct
    surv = []  # per surviving colony: its last round's numbers
    for sd in seeds:
        seed = int(sd.rsplit('_', 1)[1])
        rs = rounds(sd)
        if arm in EVD and seed in EVD[arm]:
            food[seed] = EVD[arm][seed]
        elif rs and all(os.path.exists(r + '/events.csv') for r in rs):
            food[seed] = {int(r.rsplit('r', 1)[1]): sum(1 for e in csv.DictReader(open(r + '/events.csv'))
                                                      if e['kind'] == 'deliver' and abs(int(e['x']) - NEST) <= 12)
                          for r in rs}
        dead[seed] = os.path.exists(sd + '/EXTINCT') or not rs
        if dead[seed]:
            extinct += 1
            continue
        last = rs[-1]
        rows = list(csv.DictReader(open(last + '/nest.csv'))) if os.path.exists(last + '/nest.csv') else []
        if not rows:
            continue
        end = rows[-1]
        c = dict(seed=os.path.basename(sd), pit=int(end['pit']), score=float(end['nest_score']),
                 roofed=int(end['dug']), rooms=int(end['niches']) + int(end['chambers']))
        doors = [int(x['door_open']) for x in rows if 'door_open' in x and int(x['frame']) >= 20000]
        doors = [d for d in doors if d >= 0]
        c['door'] = sum(doors) / len(doors) if doors else None
        ev = last + '/events.csv'
        if os.path.exists(ev):
            new = deep = out = far = inn = 0
            for e in csv.DictReader(open(ev)):
                k = e['kind']; x = int(e['x'])
                if k == 'dig' and abs(x - NEST) <= 20:
                    new += 1; deep += int(e['v']) >= 3
                elif k == 'out':
                    out += 1; far += abs(x - NEST) >= 20
                elif k == 'in':
                    inn += 1
            c.update(deep=deep / new if new else None, far=far / out if out else None,
                     in_per10=10 * inn / new if new else None)
        c['pit_ever'] = max(int(x['pit']) for x in rows)
        surv.append(c)
    return dict(n=n, extinct=extinct, food=food, dead=dead, surv=surv)

def p2(k, n):
    """Exact two-sided sign / McNemar p for k of n."""
    return min(1.0, 2 * sum(math.comb(n, i) for i in range(min(k, n - k) + 1)) / 2 ** n) if n else 1.0

def paired(a, c):
    """Arm a against control c, paired by seed: discordant extinctions, and home food per colony-round over the
    rounds both reached (None where either seed has no food data)."""
    seeds = sorted(set(a['dead']) & set(c['dead']))
    only_a = sum(a['dead'][s] and not c['dead'][s] for s in seeds)
    only_c = sum(c['dead'][s] and not a['dead'][s] for s in seeds)
    fa = fc = 0.0; m = win = lose = 0
    for s in seeds:
        if s not in a['food'] or s not in c['food']:
            continue
        common = sorted(set(a['food'][s]) & set(c['food'][s]))
        if not common:
            continue
        x = sum(a['food'][s][r] for r in common) / len(common); y = sum(c['food'][s][r] for r in common) / len(common)
        fa += x; fc += y; m += 1; win += x > y; lose += x < y
    food = (fa / m, fc / m, win, lose, m) if m else None
    return only_a, only_c, food

def share(surv, ok):
    """Share of survivors meeting `ok`; None if any lacks the data."""
    vals = [ok(c) for c in surv]
    if not vals or any(v is None for v in vals):
        return None
    return sum(vals) / len(vals)

def fmt(x, pct=False):
    if x is None:
        return '?'
    return f'{100 * x:.0f}%' if pct else (f'{x:.1f}' if isinstance(x, float) else str(x))

arms = sorted({re.sub(r'_\d+$', '', os.path.basename(p)) for p in glob.glob(D + '/*_*') if os.path.isdir(p)})
stats = {a: arm_stats(a) for a in arms}
ctl = stats.get(CONTROL)
for a in arms:
    s = stats[a]; sv = s['surv']
    lim = 10 * s['n'] / 64
    if ctl is not None and a != CONTROL:
        only_a, only_c, pf = paired(s, ctl)
        food_ok = None if pf is None else pf[0] >= pf[1]
        food_note = (f"discordant vs control: only arm {only_a}, only control {only_c} (p={p2(only_a, only_a + only_c):.2f}); "
                     + ('home food/colony-round ?' if pf is None else
                        f"home food/colony-round {pf[0]:.0f} vs {pf[1]:.0f}, arm higher on {pf[2]}/{pf[2] + pf[3]} seeds "
                        f"(p={p2(pf[2], pf[2] + pf[3]):.2f}, n={pf[4]})"))
    else:
        food_ok = None
        food_note = 'the control' if a == CONTROL else 'no control given, so food is not judged'
    gate = s['extinct'] <= lim and food_ok is not False and (food_ok is not None or a == CONTROL or ctl is None)
    med = lambda k: (statistics.median([c[k] for c in sv if c.get(k) is not None])
                     if any(c.get(k) is not None for c in sv) else None)
    r1 = share(sv, lambda c: c['pit'] <= 10)
    r2a = share(sv, lambda c: None if c.get('far') is None else c['far'] >= 0.5)
    r2b = share(sv, lambda c: None if c.get('in_per10') is None else c['in_per10'] <= 1)
    r3 = share(sv, lambda c: None if c['door'] is None else c['door'] >= 0.95)
    r4 = share(sv, lambda c: c['roofed'] >= 100 and c['rooms'] >= 1)
    r5 = med('score')
    met = [gate,
           r1 is not None and r1 >= 0.8,
           r2a is not None and r2b is not None and r2a >= 0.8 and r2b >= 0.8,
           r3 is not None and r3 >= 0.8,
           r4 is not None and r4 >= 0.8,
           r5 is not None and r5 > 0]
    frontier = next((i for i, m in enumerate(met) if not m), None)
    tick = lambda i: 'MET ' if met[i] else ('open' if i != frontier else 'NEXT')
    if sv and all(c['pit_ever'] == 0 for c in sv) and med('roofed') and med('roofed') > 50:
        print(f"!! {a}: pit is 0 in every nest.csv row of every survivor. Before PR #780 the nest score read pit 0 in")
        print(f"!! walled beds (the in-run bed is one), so this nest.csv was probably written by an older build; rungs 1, 4, 5 untrusted.")
    print(f"== {a}: {s['n']} seeds, {len(sv)} survivors, frontier = rung {frontier if frontier is not None else 'none (all met)'}")
    print(f"  0 feeding    {tick(0)}  extinct {s['extinct']} of {s['n']} (bar <= {lim:.1f}); {food_note}")
    print(f"  1 dig down   {tick(1)}  pit <= 10 on {fmt(r1, 1)} (bar 80%); median pit {fmt(med('pit'))}, deep cuts {fmt(med('deep'), 1)}")
    print(f"  2 spoil out  {tick(2)}  far >= 50% on {fmt(r2a, 1)}, in <= 1/10 cuts on {fmt(r2b, 1)} (bar 80% each); "
          f"median far {fmt(med('far'), 1)}, in/10 cuts {fmt(med('in_per10'))}")
    print(f"  3 door open  {tick(3)}  open >= 95% on {fmt(r3, 1)} (bar 80%); median open {fmt(med('door'), 1)}  [plain share; costly-closure check pending]")
    print(f"  4 a room     {tick(4)}  100+ roofed with a room on {fmt(r4, 1)} (bar 80%); median roofed {fmt(med('roofed'))}, rooms {fmt(med('rooms'))}")
    print(f"  5 nest       {tick(5)}  median nest score {fmt(r5)} (bar > 0)")
