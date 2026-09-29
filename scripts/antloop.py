#!/usr/bin/env python3
"""The foraging loop, ant by ant: `trailfollow decisioncsv` traces -> the funnel,
loops per ant, who starved and where, the colony's time budget, and (with the
run's log) its food economy.

Built 2026-09-25 at the owner's request ("we should do this more often or maybe
even build it into a skill") from the analysis in
`Reports/ant-scenes-2026-09-23.md` §16. The method is the `funnel` skill's; this
is its instrument for the ant, so the next session runs one command instead of
rebuilding four scripts.

    # 1. run the colony bed with the per-decision trace ON (rows are written to
    #    the temp dir as trailfollow-decisions-seed<S>-gap<G>-<arm>-<dtag>.csv):
    cargo run --release --example trailfollow -- mode=gap arms=self gaps=90 \\
        seeds=24 frames=24000 ants=20 food=400 refill=400 decisioncsv dtag=mine > run.log
    # 2. read it:
    python3 scripts/antloop.py /tmp --tag mine --log run.log
    python3 scripts/antloop.py --selftest      # the positive control
    # 3. against a baseline run, seed by seed (food taken, food at the nest,
    #    bodies, born, starved by / after frame 6,000, old age; the logs alone
    #    are enough, so drop the CSV directory for a run without decisioncsv):
    python3 scripts/antloop.py /tmp --tag mine --log run.log --vs base.log
    python3 scripts/antloop.py --log run.log --vs base.log

Sections: the loop funnel; GOING OUT (the colony's first delivery, who ever
left home toward the food, when reachers last set out, who ever stepped onto a
road); who starved and where; HUNGRY AT HOME (hungry empty ants at home, how
often beside food and whether they ate it, and -- from the log -- how far a
hungry ant at home stands from the nearest food there, and the net food into
home); the time budget; the economy.

**A full loop** is: reach the food (within `near` cells, the harness's own
`near=`) -> pick food up there (the crop turns laden after reaching it) -> get
back into the nest band (+/-26 cells, the harness's band) still holding it ->
put it down there (the engine's own `delivered` drop outcome, which only fires
at the nest). The next loop starts only when the ant is back at the food, so a
pick-up/put-down cycle at the nest cannot count twice. A loop can also BREAK:
the load is eaten before the nest ("ate it on the way") or brought home and
eaten there with no drop ("ate it at home").

**Starved** = the ant's last decision row is before the run's end and its
`Energy` input was <= 0.02 there. With `--log`, this is reconciled against the
harness's own `DEATHS BY CAUSE` per run, and a mismatch is printed -- it is the
check that the definition is counting the engine's deaths, not inventing its own.

Joules in the economy are what an ant ABSORBS: face value x diet quality, 0.25
for plant food at the shipped ant's neutral gut (`creature::diet_quality`).
"""
import argparse
import collections as C
import csv
import glob
import os
import re
import statistics as st
import sys
import tempfile

BAND = 26          # trailfollow's nest band, +/- cells around nest_x
UP = 10            # rows above the walking surface that count as "up a wall or ceiling"
PLANT_Q = 0.25     # diet quality of plant food at the ant's neutral gut
CAP = 5760.0       # ant.ron `crop_capacity` since 2026-09-25 (2880 before), face J (fill = worth / CAP); read from --log
START_J = 200.0    # ant.ron `start_energy`; `founder_reserve` staggers it per founder but the cohort sums to exactly this x n
BUCKETS = ['carrying food', 'digging / hauling dirt', 'up a wall or ceiling',
           'off the nest, moving (exploring)', 'off the nest, standing',
           'on the nest, moving', 'on the nest, standing still']


def walk(rs, gap, near):
    """One ant's decision rows (sorted by frame) -> its loop record."""
    nest_x = int(rs[0]['nest_x'])
    food_x = nest_x + gap
    surf = int(rs[0]['y2'])
    a = dict(loops=0, reached=0, picked=0, home=0, ate_way=0, ate_home=0, eaten_face=0.0,
             last_loop_i=-1, budget=C.Counter(), could_not_move=0, paused=0, rows=len(rs),
             first_reach=None, set_out=None, left_home=False, on_road=False, first_delivery=None,
             hungry_home=0, hungry_home_food=0, hungry_home_took=0)
    phase = 'out'
    prev_fill = None
    for i, r in enumerate(rs):
        x, y = int(r['x2']), int(r['y2'])
        laden = r['leg'] == 'laden'
        fill = float(r['fill'])
        if prev_fill is not None and fill < prev_fill and r['drop'] not in ('delivered', 'placed'):
            a['eaten_face'] += (prev_fill - fill) * CAP
        prev_fill = fill
        at_food = abs(x - food_x) <= near
        at_nest = abs(x - nest_x) <= BAND
        if phase == 'out' and at_food:
            phase = 'atfood'
            a['reached'] = max(a['reached'], a['loops'] + 1)
        if phase == 'atfood' and laden:
            phase = 'carrying'
            a['picked'] = max(a['picked'], a['loops'] + 1)
        if phase == 'carrying':
            if not laden:
                a['ate_way'] += 1
                phase = 'out'
            elif at_nest:
                phase = 'home'
                a['home'] = max(a['home'], a['loops'] + 1)
        if phase == 'home':
            if r['drop'] == 'delivered':
                a['loops'] += 1
                a['last_loop_i'] = i
                phase = 'out'
            elif not laden:
                a['ate_home'] += 1
                phase = 'out'
        # **Going out** (`ant-scenes-2026-09-23.md` §20): the first time at
        # the food, the last decision in the home band before it, whether the
        # ant ever went more than a band's width toward the food, and whether
        # it ever chose a step onto a road (trail presence >= 0.5).
        if at_food and a['first_reach'] is None:
            a['first_reach'] = int(r['frame'])
        if at_nest and a['first_reach'] is None:
            a['set_out'] = int(r['frame'])
        if x - nest_x > BAND:
            a['left_home'] = True
        cr = r.get('chosen_route', 'NaN')
        if cr not in ('NaN', '', None) and float(cr) >= 0.5:
            a['on_road'] = True
        if r['drop'] == 'delivered' and a['first_delivery'] is None:
            a['first_delivery'] = int(r['frame'])
        # **Hungry at home** (§21): empty, under a quarter of start, in the
        # band; beside food there; and whether it took or ate it within the
        # next two decisions (the crop fills, it turns laden, or its energy
        # rises).
        if not laden and at_nest and float(r['energy']) < 0.25:
            a['hungry_home'] += 1
            if float(r.get('food_adjacent', 0) or 0) > 0:
                a['hungry_home_food'] += 1
                nxt = rs[i + 1:i + 3]
                if any(float(n['fill']) > fill or n['leg'] == 'laden' or float(n['energy']) > float(r['energy']) + 0.002 for n in nxt):
                    a['hungry_home_took'] += 1
        moved = r['outcome'] == 'stepped'
        if laden:
            b = BUCKETS[0]
        elif r['leg'] == 'spoil':
            b = BUCKETS[1]
        elif y < surf - UP:
            b = BUCKETS[2]
        elif not at_nest:
            b = BUCKETS[3] if moved else BUCKETS[4]
        else:
            b = BUCKETS[5] if moved else BUCKETS[6]
        a['budget'][b] += 1
        if not moved and r['p_move'] not in ('NaN', ''):
            if float(r['p_move']) <= 0.001:
                a['could_not_move'] += 1
            else:
                a['paused'] += 1
    last = rs[-1]
    a['last'] = int(last['frame'])
    a['energy_last'] = float(last['energy'])
    a['fill_last'] = float(last['fill'])
    tail = rs[-167:]   # ~1,000 frames at the ant's 6-frame decision
    ex = st.median(int(r['x2']) - nest_x for r in tail)
    a['died_where'] = ('west of the nest' if ex < -BAND else 'on the nest' if ex <= BAND
                       else 'between the nest and the food' if ex < gap - near
                       else 'at the food' if ex <= gap + near else 'beyond the food')
    a['died_up_a_wall'] = sum(int(r['y2']) < surf - UP for r in tail) / len(tail) > 0.5
    return a


def read(paths, near, end):
    """Every ant in every file, keyed (gap, seed, arm, tag)."""
    ants = []
    for path in paths:
        by = C.defaultdict(list)
        meta = None
        with open(path) as fh:
            for r in csv.DictReader(fh):
                by[r['id']].append(r)
                if meta is None:
                    meta = (int(r['gap']), int(r['seed']), r['arm'], r['tag'])
        if meta is None:
            continue
        for aid, rs in by.items():
            rs.sort(key=lambda r: int(r['frame']))
            a = walk(rs, meta[0], near)
            a.update(gap=meta[0], seed=meta[1], arm=meta[2], tag=meta[3], id=int(aid),
                     founder=int(aid) < 1048576)
            a['died'] = a['last'] < end
            a['starved'] = a['died'] and a['energy_last'] <= 0.02
            ants.append(a)
    return ants


def causes(s):
    """A `DEATHS BY CAUSE` list -> {label: count}: 'STARVED 2, OLD AGE 1' ->
    {'STARVED': 2, 'OLD AGE': 1}, '' -> {}. 'not reached' (a run shorter than
    6,000 frames) -> None, so a missing count is never read as a zero."""
    if s.strip() == 'not reached':
        return None
    return {lab: int(n) for lab, n in re.findall(r'([A-Z][A-Z ]*?) (\d+)', s)}


def harness(logs):
    """Per (gap, seed): starved count, absorbed J, food on the nest series, near=, frames=.

    **Which line belongs to which run.** A run's block -- FOOD STORE, BIRTHS,
    DEATHS BY CAUSE, FOOD BUDGET -- is printed BEFORE that run's table row
    (`   90    1  self ...`), so it is held in `pend` until the row names the
    run; the `founded ... born N died N (starved N)` and `food into home` lines
    come AFTER the row and go to the row just read. Getting this backwards
    gives every seed its neighbour's numbers, and the table still looks right
    (`--selftest` checks it both ways). `params` also carries what the pairing
    needs to say whether it pooled anything: keys read twice (`dups`), the arms
    and founder counts seen, and a trailing block with no row after it
    (`orphans`: a log still being written, or cut short)."""
    out, params = {}, {'dups': [], 'orphans': 0}
    last = None
    for log in logs:
        pend = {}
        last = None
        founders = None
        for line in open(log):
            m = re.search(r'frames=(\d+) .*near=(\d+)', line)
            if m and 'trailfollow: mode=' in line:
                # Founders are the header's `ants=`, per segment (a log is often
                # three `seed0=` batches concatenated, each with its own header).
                # NOT the 'N of M founders born on it' line: its M is
                # `ants_seen`, every ant alive at any sample -- founders + born,
                # equal in 96 of 96 runs checked 2026-09-29.
                a_ = re.search(r' ants=(\d+)', line)
                founders = int(a_.group(1)) if a_ else None
                params['frames'], params['near'] = int(m.group(1)), int(m.group(2))
                # **The crop size is read, never assumed.** Fill is worth over
                # capacity, so a run at `cropcap=5760` read at the shipped 2880
                # halves every joule the "who ate it" table books -- found the
                # first time it was run on one (54,746 J against the harness's
                # 113,508 J, exactly half).
                c = re.search(r' cropcap=([0-9.]+)', line)
                if c:
                    params['cropcap'] = float(c.group(1))
            # The species' own crop, echoed by `trailfollow` since the crop
            # doubled. A `cropcap=` rider, parsed above, overrides it. A log
            # older than that echo holds neither, and its crop was 2,880:
            # pass `--cropcap 2880`.
            m = re.search(r'ant\.ron: crop_capacity=([0-9.]+)', line)
            if m and 'cropcap' not in params:
                params['shipped_cropcap'] = float(m.group(1))
            m = re.search(r'DEATHS BY CAUSE -- by frame 6000: \[([^\]]*)\] \| whole run: \[([^\]]*)\]', line)
            if m:
                pend['causes_6000'] = causes(m.group(1))
                pend['causes'] = causes(m.group(2)) or {}
                pend['starved'] = pend['causes'].get('STARVED', 0)
            m = re.search(r'BIRTHS (\d+) \| buds held', line)
            if m:
                pend['births'] = int(m.group(1))
            m = re.search(r'FOOD STORE \(larder cells\) -- (.*)', line)
            if m:
                pend['store'] = [(int(f), int(a), float(j), int(c), float(b)) for f, a, j, c, b in re.findall(
                    r'(\d+): nest ground \d+, crops \d+, elsewhere \d+, crumbs \d+, ants (\d+), '
                    r'nest food (\d+) J \((\d+) crumbs\), ant bodies (\d+) J', m.group(1))]
            m = re.search(r'FOOD BUDGET \(cells, one cell = (\d+) face\): taken from the pile (\d+)', line)
            if m:
                pend['face'], pend['taken'] = int(m.group(1)), int(m.group(2))
            m = re.search(r'HUNGRY AT HOME frame \d+: \d+ empty ants under a quarter full, cells to the nearest food at home \[([^\]]*)\]; food cells at home (\d+)', line)
            if m:
                pend.setdefault('hungry_d', []).extend(m.group(1).split(','))
                pend.setdefault('home_food', []).append(int(m.group(2)))
            m = re.match(r'^\s+(\d+)\s+(\d+)\s+(\w+)\s+\S+\s+(\d+)', line)
            if m and m.group(3) in ('hand', 'self', 'hmute', 'mute', 'homeA', 'flatN', 'flatF'):
                last = (int(m.group(1)), int(m.group(2)))
                if last in out:
                    params['dups'].append(last)
                out[last] = dict(pend, ate=int(m.group(4)), arm=m.group(3), founders=founders,
                                 log=os.path.basename(log))
                pend = {}
            # Printed after its run's table row, so they belong to that row.
            m = re.search(r'food into home: delivered (\d+) picked up at home (\d+) -> net (-?\d+)(?: \| turned home hungry (\d+))?', line)
            if m and last is not None:
                out[last].update(net_home=int(m.group(3)), hungry_turns=int(m.group(4) or 0))
            m = re.search(r'founded x .*?(\d+) of (\d+) founders born on it.*born\s+(\d+) died\s+(\d+) \(starved\s+(\d+)\)', line)
            if m and last is not None:
                out[last].update(seen=int(m.group(2)), born=int(m.group(3)), died=int(m.group(4)),
                                 starved_line=int(m.group(5)))
        if pend:
            params['orphans'] += 1
    return out, params


def pct(k, n):
    return f"{100 * k / n:5.1f}%" if n else "   - "


def report(ants, runs, gap):
    f = [a for a in ants if a['founder'] and a['gap'] == gap]
    n = len(f)
    seeds = sorted({a['seed'] for a in f})
    born = sum(1 for a in ants if not a['founder'] and a['gap'] == gap)
    print(f"\n######## gap {gap}: {len(seeds)} runs, {n} founders ({n / max(1, len(seeds)):.1f} a run), {born} born")
    if runs:
        mine = C.Counter(a['seed'] for a in f if a['starved'])
        diff = [s for s in seeds if (gap, s) in runs and mine[s] != runs[(gap, s)].get('starved')]
        th = sum(runs[(gap, s)].get('starved', 0) for s in seeds if (gap, s) in runs)
        print(f"starved by this trace {sum(mine.values())}, by the harness {th}; runs that differ: {diff or 'none'}")

    print("\nTHE LOOP, ANT BY ANT (booked at the furthest point each ant reached)")
    print(f"  {'':<42} {'ants':>5}  {'of prev':>8}  {'of all':>7}")
    rows = [("reached the food", sum(a['reached'] >= 1 for a in f)),
            ("picked food up there", sum(a['picked'] >= 1 for a in f)),
            ("got back to the nest still holding it", sum(a['home'] >= 1 for a in f)),
            ("put it down at the nest = 1 full loop", sum(a['loops'] >= 1 for a in f)),
            ("reached the food a 2nd time", sum(a['reached'] >= 2 for a in f))]
    rows += [(f"{k}+ full loops", sum(a['loops'] >= k for a in f)) for k in (2, 3, 4, 5, 10)]
    prev = n
    for label, k in rows:
        print(f"  {label:<42} {k:>5}  {pct(k, prev):>8}  {pct(k, n):>7}")
        prev = k
    hist = C.Counter(min(a['loops'], 10) for a in f)
    print("  loops per ant: " + "  ".join(f"{k if k < 10 else '10+'}: {hist[k]}" for k in range(11) if hist[k]))
    tot = sum(a['loops'] for a in f)
    top = sorted((a['loops'] for a in f), reverse=True)[:max(1, n // 10)]
    print(f"  {tot} full loops; the busiest 10% of ants made {sum(top)} ({pct(sum(top), max(1, tot)).strip()})")
    print(f"  broken loops: ate the load on the way home {sum(a['ate_way'] for a in f)}; "
          f"brought it home and ate it there {sum(a['ate_home'] for a in f)}")

    print("\nGOING OUT: when and whether ants leave home for the food")
    firsts = []
    for sd in seeds:
        fd = [a['first_delivery'] for a in ants if a['gap'] == gap and a['seed'] == sd and a['first_delivery'] is not None]
        if fd:
            firsts.append(min(fd))
    q = lambda xs, p: sorted(xs)[min(len(xs) - 1, int(p * len(xs)))] if xs else float('nan')
    print(f"  the colony's first delivery, median frame {q(firsts, .5):.0f} (p25 {q(firsts, .25):.0f}, p75 {q(firsts, .75):.0f}); "
          f"runs with none {len(seeds) - len(firsts)}")
    left = [a for a in f if a['left_home']]
    print(f"  founders who ever went more than {BAND} cells toward the food: {len(left)} ({pct(len(left), n).strip()})")
    rch = [a for a in f if a['first_reach'] is not None]
    fr = [a['first_reach'] for a in rch]
    so = [a['set_out'] for a in rch if a['set_out'] is not None]
    print(f"  reached the food {len(rch)}: first reach median frame {q(fr, .5):.0f}; "
          f"last set out from home before it, median frame {q(so, .5):.0f} (p25 {q(so, .25):.0f}, p75 {q(so, .75):.0f})")
    never = [a for a in f if a['first_reach'] is None]
    print(f"  never reached it {len(never)}, of whom never left home toward it {sum(not a['left_home'] for a in never)}")
    print(f"  ever chose a step onto a road (trail presence >= 0.5): reachers {pct(sum(a['on_road'] for a in rch), len(rch)).strip()}, "
          f"never-reachers {pct(sum(a['on_road'] for a in never), len(never)).strip()}")

    print("\nWHO STARVED, by how far they got (and where they spent their last ~1,000 frames)")
    groups = [("never reached the food", lambda a: a['reached'] == 0),
              ("reached it, never completed a loop", lambda a: a['reached'] > 0 and a['loops'] == 0),
              ("exactly 1 full loop", lambda a: a['loops'] == 1),
              ("2-3 full loops", lambda a: 2 <= a['loops'] <= 3),
              ("4+ full loops", lambda a: a['loops'] >= 4)]
    sv_all = sum(a['starved'] for a in f)
    for name, test in groups:
        g = [a for a in f if test(a)]
        if not g:
            continue
        sv = [a for a in g if a['starved']]
        where = C.Counter(a['died_where'] for a in sv)
        up = sum(a['died_up_a_wall'] for a in sv)
        med = f"{st.median(a['last'] for a in sv):.0f}" if sv else "-"
        print(f"  {name:<36} {len(g):>4} ants, starved {len(sv):>4} ({pct(len(sv), len(g)).strip()}), "
              f"{pct(len(sv), max(1, sv_all)).strip()} of all the starved; median frame of death {med}")
        if sv:
            print("      died: " + ", ".join(f"{k} {v}" for k, v in where.most_common()) + f"; up a wall or ceiling {up}")
    print(f"  all starved: {sv_all} of {n} ({pct(sv_all, n).strip()})")
    # **Starving with food in the crop is an economy defect, not a loop one**:
    # the carrier cannot digest as fast as carrying the load costs. 2% of the
    # dead on the shipped default; 34% at `cropcap=5760`, which is how it was
    # found (`ant-scenes-2026-09-23.md` §17).
    fed = [a for a in f if a['starved'] and a['fill_last'] > 0.25]
    print(f"  starved with the crop over a quarter full: {len(fed)} ({pct(len(fed), max(1, sv_all)).strip()} of the starved)")

    print("\nHUNGRY AT HOME: empty ants under a quarter full, in the home band")
    hh = [a for a in f if a['hungry_home']]
    fed = [a for a in hh if a['hungry_home_food']]
    took = sum(a['hungry_home_took'] for a in f)
    beside = sum(a['hungry_home_food'] for a in f)
    print(f"  ants ever hungry at home {len(hh)}; ever beside food while so {len(fed)} ({pct(len(fed), len(hh)).strip()}); "
          f"took or ate it within two decisions {took} of {beside} times ({pct(took, beside).strip()})")
    ds = [d for s_ in seeds for d in runs.get((gap, s_), {}).get('hungry_d', [])]
    if ds:
        num = [int(d) for d in ds if d != 'none']
        food = [c for s_ in seeds for c in runs.get((gap, s_), {}).get('home_food', [])]
        print(f"  census (every 3,000 frames): {len(ds)} hungry ants at home, a median {q(num, .5):.0f} cells from the nearest "
              f"food at home; no food at home at all for {pct(len(ds) - len(num), len(ds)).strip()}; food cells at home, median {q(food, .5):.0f}")
    nets = [runs[(gap, s_)]['net_home'] for s_ in seeds if 'net_home' in runs.get((gap, s_), {})]
    if nets:
        turns = sum(runs[(gap, s_)].get('hungry_turns', 0) for s_ in seeds if (gap, s_) in runs)
        print(f"  net food into home (delivered minus picked up at home): {sum(nets)} cells over {len(nets)} runs; "
              f"turned home hungry {turns}")

    print("\nTIME BUDGET: where each group's decisions went (pooled; typical ant in brackets)")
    for name, test in (("never looped", lambda a: a['loops'] == 0), ("looped at least once", lambda a: a['loops'] >= 1)):
        g = [a for a in f if test(a)]
        if not g:
            continue
        pool = C.Counter()
        for a in g:
            pool.update(a['budget'])
        total = sum(pool.values())
        print(f"  {name}: {len(g)} ants")
        for b in BUCKETS:
            typ = st.median(a['budget'][b] / max(1, a['rows']) for a in g)
            print(f"      {b:<36} {pct(pool[b], total)}  ({100 * typ:3.0f}%)")
        rows_n = sum(a['rows'] for a in g)
        print(f"      could not move (P(move) exactly 0) {pct(sum(a['could_not_move'] for a in g), rows_n).strip()} "
              f"of decisions; paused (lost the roll) {pct(sum(a['paused'] for a in g), rows_n).strip()}")

    keys = [(gap, s) for s in seeds if (gap, s) in runs]
    if keys:
        print("\nECONOMY (J an ant absorbs; plant food at diet quality 0.25)")
        frames = max(a['last'] for a in f) + 10
        ate = sum(runs[k]['ate'] for k in keys)
        end_bodies = sum(runs[k]['store'][-1][4] for k in keys if runs[k].get('store'))
        ant_frames = sum(a['last'] - 0 for a in f)
        burned = START_J * n + ate - end_bodies
        rate = burned / max(1, ant_frames)
        need = rate * frames * n / len(seeds)
        print(f"  an ant burns {rate:.4f} J a frame: {rate * frames:.0f} J over the whole run; the colony would need {need:.0f} J a run")
        print(f"  the colony absorbed a median {st.median(runs[k]['ate'] for k in keys):.0f} J a run: "
              f"{pct(ate, need * len(keys)).strip()} of that need")
        taken = sum(runs[k].get('taken', 0) for k in keys)
        if tot:
            face = runs[keys[0]].get('face', 960)
            print(f"  taken from the pile per full loop: {taken / tot:.1f} cells ({taken * face * PLANT_Q / tot:.0f} J to an ant)")
        eaten = C.defaultdict(float)
        cnt = C.Counter()
        for a in f:
            k = '4+' if a['loops'] >= 4 else str(a['loops'])
            eaten[k] += a['eaten_face'] * PLANT_Q
            cnt[k] += 1
        et = sum(eaten.values())
        print(f"  who ate it (crop digestion in the trace, {et:.0f} J against the harness's {ate} J):")
        for k in ('0', '1', '2', '3', '4+'):
            if cnt[k]:
                print(f"      ants with {k:>2} full loops: {cnt[k]:>4} ants ({pct(cnt[k], n).strip()}) ate "
                      f"{pct(eaten[k], et).strip()} of the food, {eaten[k] / cnt[k]:.0f} J each")
        stores = [runs[k]['store'] for k in keys if runs[k].get('store')]
        if stores:
            print("  food standing ON THE NEST (fruit + crumbs), median over runs:")
            for i in range(min(len(s) for s in stores)):
                fr = stores[0][i][0]
                j = [s[i][2] for s in stores]
                print(f"      frame {fr:>6}: {st.median(s[i][3] for s in stores):>4.0f} crumbs, "
                      f"{st.median(j) * PLANT_Q:>5.0f} J to an ant (max {max(j) * PLANT_Q:.0f}); "
                      f"ants alive {st.median(s[i][1] for s in stores):.1f}; in their bodies {st.median(s[i][4] for s in stores):.0f} J")


def sign_p(better, worse):
    """Two-sided sign test on paired seeds, ties dropped."""
    import math
    n, k = better + worse, min(better, worse)
    return min(1.0, 2 * sum(math.comb(n, i) for i in range(k + 1)) / 2 ** n) if n else 1.0


def _nest_mean(r, i):
    """Mean of one FOOD STORE column over the samples at frame >= 6,000 (the
    first 6,000 are the founding, before the colony has a store to keep)."""
    v = [smp[i] for smp in r.get('store', []) if smp[0] >= 6000]
    return st.mean(v) if v else None


def _lived(r):
    """Ants that lived: founders (the header's `ants=`) + born."""
    return r['founders'] + r['born'] if r.get('founders') is not None and 'born' in r else None


def _cause(r, label, when='causes'):
    c = r.get(when)
    return None if c is None else c.get(label, 0)


def _starved_after(r):
    a, b = _cause(r, 'STARVED'), _cause(r, 'STARVED', 'causes_6000')
    return None if a is None or b is None else a - b


# (label, per-run value, how runs add up: 'sum' a count, 'mean' a per-run mean,
# 'share' a count also read over the ants that lived). Order is the printed order;
# net food into home stays last, labelled, because it counts the same crumbs
# again (ant-scenes-2026-09-23.md §22j).
PAIRED = [
    ("food taken from the pile, cells", lambda r: r.get('taken'), 'sum'),
    ("food standing at the nest, face J", lambda r: _nest_mean(r, 2), 'mean'),
    ("in the ants' bodies, J", lambda r: _nest_mean(r, 4), 'mean'),
    ("born", lambda r: r.get('born'), 'sum'),
    ("ants that lived (founders + born)", _lived, 'sum'),
    ("starved, whole run", lambda r: _cause(r, 'STARVED'), 'share'),
    ("  starved by frame 6,000", lambda r: _cause(r, 'STARVED', 'causes_6000'), 'share'),
    ("  starved after frame 6,000", _starved_after, 'share'),
    ("  died of old age", lambda r: _cause(r, 'OLD AGE'), 'share'),
    ("net food into home, cells -- OVERCOUNTS 3.4-4.6x, s22j", lambda r: r.get('net_home'), 'sum'),
]


def pair_rows(runs, base, keys):
    """The paired table as data, so `--selftest` can check the arithmetic.
    One dict per PAIRED row (and a share row under each 'share' row): the
    per-seed values of the baseline `a` and this run `b`, and the sign counts."""
    rows = []
    for label, get, kind in PAIRED:
        ks = [k for k in keys if get(base[k]) is not None and get(runs[k]) is not None]
        a = [get(base[k]) for k in ks]
        b = [get(runs[k]) for k in ks]
        rows.append(dict(label=label, kind=kind, n=len(ks), a=a, b=b,
                         hi=sum(y > x for x, y in zip(a, b)), lo=sum(y < x for x, y in zip(a, b))))
        if kind == 'share':
            ks = [k for k in ks if _lived(base[k]) and _lived(runs[k])]
            ca, cb = [get(base[k]) for k in ks], [get(runs[k]) for k in ks]
            la, lb = [_lived(base[k]) for k in ks], [_lived(runs[k]) for k in ks]
            a = [c / n for c, n in zip(ca, la)]
            b = [c / n for c, n in zip(cb, lb)]
            rows.append(dict(label="    as a share of ants that lived", kind='pct', n=len(ks), a=a, b=b,
                             pooled=(sum(ca) / sum(la) if la else None, sum(cb) / sum(lb) if lb else None),
                             hi=sum(y > x for x, y in zip(a, b)), lo=sum(y < x for x, y in zip(a, b))))
    return rows


def versus(runs, rparams, base, bparams, gap=None):
    """Paired by (gap, seed) against a baseline run's logs, from the two logs
    alone: food taken from the pile, food standing at the nest, the ants'
    bodies, born, and the deaths split three ways (starved by frame 6,000,
    starved after it, old age), each dead count also as a share of the ants
    that lived -- a colony eleven times the size starves more ants and a
    smaller share of them, and only the share says which. Net food into home
    last, because it overcounts (§22j)."""
    ra = {k: v for k, v in runs.items() if gap is None or k[0] == gap}
    rb = {k: v for k, v in base.items() if gap is None or k[0] == gap}
    keys = sorted(set(ra) & set(rb))
    logs = lambda d: ", ".join(sorted({v['log'] for v in d.values()})) or "-"
    print(f"\nPAIRED AGAINST THE BASELINE, seed by seed, from the two logs"
          + (f" (gap {gap})" if gap is not None else ""))
    # A parse is a measurement: print the key's cardinality against what the runs swept.
    print(f"  keyed (gap, seed): baseline {len(rb)} runs [{logs(rb)}], this {len(ra)} runs [{logs(ra)}]; "
          f"paired {len(keys)} = " + ", ".join(f"gap {g} x {n} seeds" for g, n in
                                                 sorted(C.Counter(k[0] for k in keys).items())))
    for name, d in (("baseline", rb), ("this", ra)):
        only = sorted(set(d) - set(keys))
        if only:
            print(f"  UNPAIRED in {name}: {len(only)} runs {only[:6]}{' ...' if len(only) > 6 else ''}")
    print(f"  arms: baseline {sorted({v['arm'] for v in rb.values()})}, this {sorted({v['arm'] for v in ra.values()})}; "
          f"founders (header ants=): baseline {sorted({v['founders'] for v in rb.values()}, key=str)}, "
          f"this {sorted({v['founders'] for v in ra.values()}, key=str)}")
    for name, p in (("baseline", bparams), ("this", rparams)):
        if p and p.get('dups'):
            print(f"  WARNING: the {name} log(s) hold {len(p['dups'])} (gap, seed) keys twice -- last write wins, "
                  f"so those runs are POOLED: {sorted(set(p['dups']))[:6]}")
        if p and p.get('orphans'):
            print(f"  WARNING: the {name} log(s) end with {p['orphans']} run block(s) and no table row -- "
                  f"still being written, or cut short")
    both = [d[k] for d in (ra, rb) for k in keys]
    seen = [r for r in both if 'seen' in r and _lived(r) is not None]
    births = [r for r in both if 'births' in r and 'born' in r]
    sl = [r for r in both if 'starved_line' in r and 'causes' in r]
    print(f"  reconciled per run: the founded line's 'of M founders' = header founders + born in "
          f"{sum(r['seen'] == _lived(r) for r in seen)} of {len(seen)} (so M counts every ant seen; "
          f"founders come from ants=); BIRTHS = its born in {sum(r['births'] == r['born'] for r in births)} "
          f"of {len(births)}; DEATHS BY CAUSE STARVED = its starved in "
          f"{sum(r['starved_line'] == r['causes'].get('STARVED', 0) for r in sl)} of {len(sl)}")
    print(f"  FOOD STORE samples at frame >= 6,000 per run: "
          f"{sorted(C.Counter(sum(s_[0] >= 6000 for s_ in r.get('store', [])) for r in both).items())} "
          f"(count: runs); nest food and bodies are each run's mean over them")
    other = C.Counter()
    for r in both:
        other.update({c: n for c, n in (r.get('causes') or {}).items() if c not in ('STARVED', 'OLD AGE')})
    if other:
        print(f"  NOTE: other causes of death in these runs, not in the split: {dict(other)}")
    print(f"  {'':<54} {'':<6} {'baseline':>10}    {'this':>10}   {'median by seed':>21}   this higher / lower by seed")
    for r in pair_rows(ra, rb, keys):
        if not r['n']:
            print(f"  {r['label']:<54} not in both logs")
            continue
        tail = (f"{r['hi']:>3} / {r['lo']:<3} tie {r['n'] - r['hi'] - r['lo']:<3} "
                f"sign p {sign_p(r['hi'], r['lo']):.3f}  (n {r['n']})")
        if r['kind'] == 'pct':
            word, fmt = "pooled", (lambda x: f"{100 * x:.1f}%")
            ta, tb = r['pooled']
        else:
            word, fmt = ("mean", "total")[r['kind'] != 'mean'], (lambda x: f"{x:,.0f}")
            agg = st.mean if r['kind'] == 'mean' else sum
            ta, tb = agg(r['a']), agg(r['b'])
        print(f"  {r['label']:<54} {word:<6} {fmt(ta):>10} -> {fmt(tb):>10}   "
              f"{fmt(st.median(r['a'])):>9} -> {fmt(st.median(r['b'])):>9}   {tail}")


def selftest():
    """Hand-built ants whose answers are known. Exits non-zero on any miss."""
    cols = ['seed', 'gap', 'arm', 'tag', 'frame', 'id', 'leg', 'fill', 'x2', 'y2', 'nest_x', 'energy',
            'outcome', 'p_move', 'drop', 'food_adjacent', 'chosen_route']
    rows = []

    def ant(aid, steps):
        for i, st_ in enumerate(steps):
            x, leg, fill, drop, e = st_[:5]
            food = st_[5] if len(st_) > 5 else 0
            rows.append(dict(seed=1, gap=50, arm='self', tag='t', frame=6 * (i + 1), id=aid, leg=leg,
                             fill=fill, x2=x, y2=40, nest_x=10, energy=e, outcome='stepped', p_move=0.5,
                             drop=drop, food_adjacent=food, chosen_route='NaN'))
    # food at x=60. Two full loops, then alive at the end.
    trip = [(30, 'empty', 0, 'not_asked', 1), (60, 'empty', 0, 'not_asked', 1), (60, 'laden', .33, 'not_asked', 1),
            (35, 'laden', .30, 'not_asked', 1), (10, 'laden', .30, 'roll_lost', 1), (10, 'empty', 0, 'delivered', 1)]
    ant(1, trip + trip + [(10, 'empty', 0, 'not_asked', 1)] * 20)
    # A pick-up/put-down cycle at the nest must NOT count as loops: one real loop, then churn.
    ant(2, trip + [(10, 'laden', .3, 'roll_lost', 1), (10, 'empty', 0, 'delivered', 1)] * 5 + [(10, 'empty', 0, 'not_asked', 1)] * 10)
    # Reached the food, ate the load on the way, starved beyond the food.
    ant(3, [(60, 'empty', 0, 'not_asked', .5), (60, 'laden', .33, 'not_asked', .5), (70, 'empty', 0, 'not_asked', .1)]
        + [(95, 'empty', 0, 'not_asked', .05)] * 6 + [(95, 'empty', 0, 'not_asked', 0.0)])
    # Never reached the food, starved on the nest.
    ant(4, [(12, 'empty', 0, 'not_asked', .5)] * 5 + [(12, 'empty', 0, 'not_asked', 0.0)])
    # Hungry at home beside food, and eats it (its energy rises next decision); alive at the end.
    ant(5, [(12, 'empty', 0, 'not_asked', .2, 1), (12, 'empty', 0, 'not_asked', .3)] + [(12, 'empty', 0, 'not_asked', .3)] * 30)
    d = tempfile.mkdtemp()
    p = os.path.join(d, 'trailfollow-decisions-seed1-gap50-self-t.csv')
    with open(p, 'w', newline='') as fh:
        w = csv.DictWriter(fh, fieldnames=cols)
        w.writeheader()
        w.writerows(rows)
    ants = {a['id']: a for a in read([p], near=10, end=150)}
    checks = [
        (ants[1]['loops'] == 2, f"ant 1 made 2 loops, read {ants[1]['loops']}"),
        (ants[2]['loops'] == 1, f"ant 2's nest churn counted as loops: {ants[2]['loops']}"),
        (ants[3]['loops'] == 0 and ants[3]['ate_way'] == 1 and ants[3]['starved'], f"ant 3: {ants[3]}"),
        (ants[3]['died_where'] == 'beyond the food', f"ant 3 died {ants[3]['died_where']}"),
        (ants[4]['reached'] == 0 and ants[4]['starved'] and ants[4]['died_where'] == 'on the nest', f"ant 4: {ants[4]}"),
        (not ants[1]['starved'], "ant 1 is alive at the end"),
        (ants[1]['first_reach'] == 12 and ants[1]['set_out'] == 6 and ants[1]['left_home'] and ants[1]['first_delivery'] == 36,
         f"ant 1 going out: first_reach {ants[1]['first_reach']} set_out {ants[1]['set_out']} left {ants[1]['left_home']} first_delivery {ants[1]['first_delivery']}"),
        (not ants[4]['left_home'] and ants[4]['hungry_home'] == 1 and ants[4]['hungry_home_food'] == 0,
         f"ant 4 hungry at home, no food: {ants[4]['hungry_home']}, {ants[4]['hungry_home_food']}"),
        (ants[5]['hungry_home'] == 1 and ants[5]['hungry_home_food'] == 1 and ants[5]['hungry_home_took'] == 1,
         f"ant 5 hungry at home beside food, ate it: {ants[5]['hungry_home']}, {ants[5]['hungry_home_food']}, {ants[5]['hungry_home_took']}"),
    ]
    # The logs: two runs whose blocks differ, so a block given to the wrong row
    # is caught. A run's block is printed BEFORE its table row and the founded
    # line AFTER it; seed 1 took 100 cells, seed 2 took 300.
    def run(seed, taken, births, by6, whole, nest, net):
        store = " | ".join(f"{f}: nest ground 0, crops 1, elsewhere 1, crumbs 1, ants 20, nest food {j} J (1 crumbs), "
                           f"ant bodies {j // 2} J" for f, j in zip((3000, 6000, 9000), nest))
        return (f"    FOOD STORE (larder cells) -- {store}\n"
                f"    BIRTHS {births} | buds held for the nest 0 (PIXEL_PHYSICS_BUD_SITE=nest) | parents overdrawn by a birth 0\n"
                f"    DEATHS BY CAUSE -- by frame 6000: [{by6}] | whole run: [{whole}]\n"
                f"    FOOD BUDGET (cells, one cell = 960 face): taken from the pile {taken}; chewed 1\n"
                f"   90 {seed:>4}  self   23/19       38007     1500        0\n"
                f"                founded x   30..68   (nest cursor 48 MATERIAL x 26..70 19 of {20 + births} founders born on it; "
                f"food 138)  occupancy/1k [1]  carry->nest 1  born {births:>4} died    6 (starved {causes(whole).get('STARVED', 0)})\n"
                f"                food into home: delivered 10 picked up at home 4 -> net {net} | turned home hungry 0\n")
    hdr = "trailfollow: mode=gap gate=shipped frames=24000 seeds=2 seed0=1 ants=20 relay=60 near=10 food=400\n"
    lb = os.path.join(d, 'base.log')
    lt = os.path.join(d, 'this.log')
    with open(lb, 'w') as fh:
        fh.write(hdr + run(1, 100, 5, "STARVED 1", "STARVED 4, OLD AGE 2", (100, 200, 400), 6)
                 + run(2, 300, 9, "", "STARVED 1", (900, 900, 900), 8))
    with open(lt, 'w') as fh:
        fh.write(hdr + run(1, 200, 30, "", "STARVED 5, OLD AGE 1", (100, 100, 100), 7)
                 + run(2, 600, 50, "STARVED 2", "STARVED 2, OLD AGE 3", (900, 900, 900), 9))
    base, _ = harness([lb])
    this, _ = harness([lt])
    b1, b2 = base[(90, 1)], base[(90, 2)]
    checks.append((b1.get('taken') == 100 and b2.get('taken') == 300 and b1.get('born') == 5 and b2.get('born') == 9
                   and b1.get('causes_6000') == {'STARVED': 1} and _starved_after(b1) == 3 and _cause(b1, 'OLD AGE') == 2
                   and _nest_mean(b1, 2) == 300 and _lived(b1) == 25 and b1.get('net_home') == 6,
                   "log attribution: a pre-row block belongs to the NEXT row, the founded line to the one before: "
                   f"seed 1 {b1}"))
    rows = {r['label']: r for r in pair_rows(this, base, [(90, 1), (90, 2)])}
    sh = [r for r in pair_rows(this, base, [(90, 1), (90, 2)]) if r['kind'] == 'pct'][1]   # starved by 6,000
    checks.append((rows["food taken from the pile, cells"]['hi'] == 2 and rows["born"]['b'] == [30, 50]
                   and rows["  starved by frame 6,000"]['a'] == [1, 0] and rows["  starved by frame 6,000"]['b'] == [0, 2]
                   and sh['pooled'] == (1 / 54, 2 / 120) and (sh['hi'], sh['lo']) == (1, 1),
                   f"pairing: taken {rows['food taken from the pile, cells']}, starved by 6,000 share {sh}"))
    bad = [msg for ok, msg in checks if not ok]
    for msg in bad:
        print("SELFTEST FAIL:", msg)
    print("antloop selftest:", "FAILED" if bad else f"all {len(checks)} checks passed")
    return 1 if bad else 0


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument('csv_dir', nargs='?', help="directory holding trailfollow-decisions-*.csv")
    ap.add_argument('--tag', default=None, help="only files whose dtag is this")
    ap.add_argument('--gap', type=int, default=None, help="only this food distance")
    ap.add_argument('--log', nargs='*', default=[], help="the trailfollow run log(s), for the economy and the death reconciliation")
    ap.add_argument('--near', type=int, default=None, help="cells from the food that count as at it (default: from --log, else 10)")
    ap.add_argument('--cropcap', type=float, default=None, help="crop capacity in face J (default: from --log, else ant.ron's 5760; a log from before 2026-09-25 needs --cropcap 2880)")
    ap.add_argument('--vs', nargs='*', default=[], help="a baseline run's trailfollow log(s): pairs, by (gap, seed), food taken from the pile, food at the nest, "
                    "the ants' bodies, born, and starved by / after frame 6,000 and old age (counts and shares of ants that lived). "
                    "Needs only the logs: `--log this.log --vs base.log` with no CSV directory")
    ap.add_argument('--selftest', action='store_true')
    args = ap.parse_args()
    if args.selftest:
        sys.exit(selftest())
    global CAP
    runs, params = harness(args.log)
    if not args.csv_dir:
        # The paired table needs only the two logs: a bed run without
        # `decisioncsv` can still be compared seed by seed.
        if args.vs and args.log:
            base, bparams = harness(args.vs)
            versus(runs, params, base, bparams, args.gap)
            return
        ap.error("give the directory of decision CSVs (or --log X --vs BASE for the paired table alone), or --selftest")
    CAP = args.cropcap or params.get('cropcap', params.get('shipped_cropcap', CAP))
    near = args.near or params.get('near', 10)
    end = params.get('frames', 24000) - 10
    paths = sorted(glob.glob(os.path.join(args.csv_dir, 'trailfollow-decisions-*.csv')))
    if args.tag:
        paths = [p for p in paths if p.endswith(f"-{args.tag}.csv")]
    if args.gap:
        paths = [p for p in paths if f"-gap{args.gap}-" in p]
    if not paths:
        sys.exit(f"no decision CSVs matched in {args.csv_dir}")
    ants = read(paths, near, end)
    keys = sorted({(a['gap'], a['seed'], a['arm'], a['tag']) for a in ants})
    # The funnel skill's rule: print the key's cardinality against what the run swept.
    by_gap = C.Counter(k[0] for k in keys)
    print(f"antloop: {len(paths)} files, {len(keys)} runs keyed (gap, seed, arm, tag): "
          + ", ".join(f"gap {g} x {n} runs" for g, n in sorted(by_gap.items()))
          + f"; arms {sorted({k[2] for k in keys})}; near={near}; crop {CAP:.0f} face J")
    if len({k[2] for k in keys}) > 1 or len({k[3] for k in keys}) > 1:
        print("antloop: WARNING -- more than one arm or tag in these files; they are pooled below. Pass --tag.")
    base, bparams = harness(args.vs) if args.vs else (None, None)
    for g in sorted(by_gap):
        report(ants, runs, g)
        if base:
            versus(runs, params, base, bparams, g)


if __name__ == '__main__':
    main()
