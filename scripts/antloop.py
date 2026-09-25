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
             last_loop_i=-1, budget=C.Counter(), could_not_move=0, paused=0, rows=len(rs))
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


def harness(logs):
    """Per (gap, seed): starved count, absorbed J, food on the nest series, near=, frames=."""
    out, params = {}, {}
    for log in logs:
        pend = {}
        for line in open(log):
            m = re.search(r'frames=(\d+) .*near=(\d+)', line)
            if m and 'trailfollow: mode=' in line:
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
            m = re.search(r'DEATHS BY CAUSE -- by frame 6000: \[[^\]]*\] \| whole run: \[([^\]]*)\]', line)
            if m:
                mm = re.search(r'STARVED (\d+)', m.group(1))
                pend['starved'] = int(mm.group(1)) if mm else 0
            m = re.search(r'FOOD STORE \(larder cells\) -- (.*)', line)
            if m:
                pend['store'] = [(int(f), int(a), float(j), int(c), float(b)) for f, a, j, c, b in re.findall(
                    r'(\d+): nest ground \d+, crops \d+, elsewhere \d+, crumbs \d+, ants (\d+), '
                    r'nest food (\d+) J \((\d+) crumbs\), ant bodies (\d+) J', m.group(1))]
            m = re.search(r'FOOD BUDGET \(cells, one cell = (\d+) face\): taken from the pile (\d+)', line)
            if m:
                pend['face'], pend['taken'] = int(m.group(1)), int(m.group(2))
            m = re.match(r'^\s+(\d+)\s+(\d+)\s+(\w+)\s+\S+\s+(\d+)', line)
            if m and m.group(3) in ('hand', 'self', 'hmute', 'mute', 'homeA', 'flatN', 'flatF'):
                out[(int(m.group(1)), int(m.group(2)))] = dict(pend, ate=int(m.group(4)))
                pend = {}
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


def selftest():
    """Hand-built ants whose answers are known. Exits non-zero on any miss."""
    cols = ['seed', 'gap', 'arm', 'tag', 'frame', 'id', 'leg', 'fill', 'x2', 'y2', 'nest_x', 'energy',
            'outcome', 'p_move', 'drop', 'food_adjacent', 'chosen_route']
    rows = []

    def ant(aid, steps):
        for i, (x, leg, fill, drop, e) in enumerate(steps):
            rows.append(dict(seed=1, gap=50, arm='self', tag='t', frame=6 * (i + 1), id=aid, leg=leg,
                             fill=fill, x2=x, y2=40, nest_x=10, energy=e, outcome='stepped', p_move=0.5,
                             drop=drop, food_adjacent=0, chosen_route='NaN'))
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
    ]
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
    ap.add_argument('--selftest', action='store_true')
    args = ap.parse_args()
    if args.selftest:
        sys.exit(selftest())
    if not args.csv_dir:
        ap.error("give the directory of decision CSVs, or --selftest")
    global CAP
    runs, params = harness(args.log)
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
    for g in sorted(by_gap):
        report(ants, runs, g)


if __name__ == '__main__':
    main()
