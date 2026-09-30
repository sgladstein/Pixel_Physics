#!/usr/bin/env python3
"""Departures from the nest and the scout's give-up, ant by ant, from `trailfollow` decision CSVs.

The Stage 3 (`giveup`) instrument of `Reports/food-trail-reader-design-2026-09-30.md` (its must-fix M7):
ported from the Stage 1 panel's `departures.py` + `giveup.py`, which only ever lived in a run directory
and are archived in `Reports/data/food-trail-lay-2026-09-29.tar.gz`. The arithmetic is theirs, unchanged;
the two passes are one command and the parse is keyed on (gap, seed, arm).

usage:  giveup.py CSV [CSV ...] [--tsv OUT.tsv] [--example]
        giveup.py --selftest

The CSVs need `decisioncsv dwide` (for chosen_cos, scout_w, scout_patience).

A DEPARTURE is a row of one ant whose previous row read at_nest 1 and this one reads at_nest 0. Class, at that row:
  worker  id % 4 == 0 (the nest-worker caste, Storeroom::SHIPPED caste=4); not a forager
  fed     leg empty, energy_j >= 200
  hungry  leg empty, energy_j < 200
  lunch   leg laden and drive finite (a packed-lunch carrier: CarryingFood 1, but the drive is felt)
  laden   leg laden, drive NaN (a true load); spoil  leg spoil
The EXCURSION runs from the departure to the next row reading at_nest 1, or the ant's last row.
  dir30   E/W/- : the sign of x - x_dep once the summed |dx| of the excursion reaches 30 cells
  outcome pile (x >= nest_x + gap - 10 at any row), returned, or end

THE GIVE-UP, over forager excursions (fed, hungry, lunch) with dir30 E or W that got 10 cells out:
  gave up          the excursion has a row with scout_home 1 (the scout's patience ran out; it is pulled home)
  frames to give up  departure -> that row;  give-up -> back  that row -> the excursion's last row
  dark walk        cells walked (summed |dx|) after the last step onto trail (chosen_route > 0) and up to
                   the give-up: how far a follower goes past a dead trail before patience ends (M4)
  after give-up    over stepped rows that read trail B (empty with fill 0, or a lunch carrier):
                   on route = chosen_route >= 0.5; heading outward = chosen_cos < 0 (the picked heading
                   points away from home)
The Stage 1 figures this must reproduce (the positive control, M7), `lay`, seeds 1-8, unlimited pile:
  E all: heading outward 0.295 (90 cells) / 0.300 (140); give-up -> back median 1,253 / 1,086 frames.

The panel also quoted "give-ups after a stray reversal" and "east legs that load". Neither was ever
defined in code, so neither is reported here (M7: define them or drop them; dropped).
"""
import sys, csv, gzip, math, collections, statistics as st

NEED = ['seed', 'gap', 'arm', 'frame', 'id', 'leg', 'x', 'y', 'x2', 'at_nest', 'nest_x', 'outcome', 'chosen_route',
        'chosen_cos', 'energy_j', 'drive', 'scout_home', 'fill', 'scout_w', 'scout_patience']


def opener(path):
    return gzip.open(path, 'rt') if path.endswith('.gz') else open(path)


def reads_b(p, I):
    # The chooser read trail B on this row: empty after act (leg empty, fill 0), or a lunch carrier (leg laden,
    # drive finite). A row where the ant loaded food in `act` reads trail A and is excluded.
    return (p[I['leg']] == 'empty' and float(p[I['fill']]) == 0.0) or (p[I['leg']] == 'laden' and p[I['drive']] != 'NaN')


def excursions(path):
    """Yield (key, departure dict, excursion rows, index map) for every departure in one CSV."""
    with opener(path) as f:
        hdr = f.readline().rstrip('\n').split(',')
        I = {k: i for i, k in enumerate(hdr)}
        miss = [k for k in NEED if k not in I]
        if miss:
            sys.exit(f'giveup: {path} lacks {miss} -- run trailfollow with `decisioncsv dwide`')
        by_id = collections.defaultdict(list)
        key = None
        for line in f:
            p = line.rstrip('\n').split(',')
            by_id[int(p[I['id']])].append(p)
            if key is None:
                key = (int(p[I['gap']]), int(p[I['seed']]), p[I['arm']])
    for aid, rows in by_id.items():
        for i in range(1, len(rows)):
            a, b = rows[i - 1], rows[i]
            if not (float(a[I['at_nest']]) >= 0.5 and float(b[I['at_nest']]) < 0.5):
                continue
            ej, dr = float(b[I['energy_j']]), float(b[I['drive']])
            if aid % 4 == 0: cls = 'worker'
            elif b[I['leg']] == 'empty': cls = 'fed' if ej >= 200 else 'hungry'
            elif b[I['leg']] == 'laden': cls = 'lunch' if not math.isnan(dr) else 'laden'
            else: cls = 'spoil'
            exc = [b]
            j = i + 1
            while j < len(rows) and float(rows[j][I['at_nest']]) < 0.5:
                exc.append(rows[j]); j += 1
            nx, x0 = int(b[I['nest_x']]), int(b[I['x']])
            moved, dir30 = 0, '-'
            for r in exc:
                moved += abs(int(r[I['x2']]) - int(r[I['x']]))
                if moved >= 30:
                    d = int(r[I['x2']]) - x0
                    dir30 = 'E' if d > 0 else 'W' if d < 0 else '0'
                    break
            gap = key[0]
            outcome = 'end'
            if any(int(r[I['x2']]) >= nx + gap - 10 for r in exc): outcome = 'pile'
            elif j < len(rows): outcome = 'returned'
            dep = dict(id=aid, cls=cls, frame=int(b[I['frame']]), dir30=dir30, outcome=outcome,
                       max_e=max(int(r[I['x2']]) for r in exc) - nx, min_w=min(int(r[I['x2']]) for r in exc) - nx)
            yield key, dep, exc, I


def trace(exc, I):
    gi = next((j for j, p in enumerate(exc) if p[I['scout_home']] == '1'), None)
    after = [p for p in exc[gi:] if p[I['outcome']] == 'stepped' and p[I['chosen_route']] != 'NaN' and reads_b(p, I)] if gi is not None else []
    f0 = int(exc[0][I['frame']])
    # M4: cells walked past the trail's last lit cell before the give-up -- the dark walk a wall would cut
    # short. "Lit" is a step whose picked heading carried trail (chosen_route > 0); NaN if never lit.
    dark = math.nan
    if gi is not None:
        lit = [j for j in range(gi) if exc[j][I['chosen_route']] not in ('NaN', 'nan') and float(exc[j][I['chosen_route']]) > 0]
        if lit:
            dark = sum(abs(int(p[I['x2']]) - int(p[I['x']])) for p in exc[lit[-1] + 1:gi + 1])
    return dict(gave_up=gi is not None, dark=dark,
                t_give=int(exc[gi][I['frame']]) - f0 if gi is not None else math.nan,
                t_after=int(exc[-1][I['frame']]) - int(exc[gi][I['frame']]) if gi is not None else math.nan,
                n_after=len(after), on_after=sum(float(p[I['chosen_route']]) >= 0.5 for p in after),
                out_after=sum(float(p[I['chosen_cos']]) < 0 for p in after if p[I['chosen_cos']] != 'NaN'), gi=gi)


def run(paths, tsv=None, example=False, out=sys.stdout):
    res = []
    keys = set()
    shown = False
    for path in paths:
        for key, dep, exc, I in excursions(path):
            keys.add(key)
            if dep['cls'] not in ('fed', 'hungry', 'lunch') or dep['dir30'] not in ('E', 'W'):
                continue
            if not (dep['max_e'] >= 10 or dep['min_w'] <= -10):
                continue
            t = trace(exc, I)
            res.append(dict(gap=key[0], seed=key[1], arm=key[2], **dep, **{k: v for k, v in t.items() if k != 'gi'}))
            if example and not shown and dep['cls'] == 'fed' and dep['dir30'] == 'W' and t['gave_up'] and t['n_after'] > 30:
                shown = True
                print(f"EXAMPLE gap {key[0]} seed {key[1]} arm {key[2]} ant {dep['id']}, fed, left at frame {dep['frame']}; "
                      f"every 5th row, ! marks scout_home", file=out)
                print('  frame   dx   y  leg   outcome        route   chosen_cos scout_w patience drive  energy_j', file=out)
                for j, p in enumerate(exc):
                    if j % 5 and j != t['gi']: continue
                    print(f"  {p[I['frame']]:>6} {int(p[I['x']]) - int(p[I['nest_x']]):+4d} {p[I['y']]:>3} {p[I['leg']]:5} "
                          f"{p[I['outcome']]:15} {p[I['chosen_route']]:>7} {p[I['chosen_cos']]:>8} {p[I['scout_w']]:>7} "
                          f"{p[I['scout_patience']]:>7} {p[I['drive']]:>6} {p[I['energy_j']]:>8} {'!' if p[I['scout_home']] == '1' else ''}", file=out)
                    if j > 400: print('  ...', file=out); break
    by = collections.defaultdict(list)
    for k in keys: by[(k[0], k[2])].append(k[1])
    for (gap, arm), seeds in sorted(by.items()):
        print(f'giveup: gap {gap} arm {arm}: {len(seeds)} seeds {sorted(seeds)}', file=out)
    for (gap, arm) in sorted(by):
        R = [o for o in res if o['gap'] == gap and o['arm'] == arm]
        print(f'gap {gap} arm {arm}: {len(R)} forager excursions', file=out)
        for d in ('W', 'E'):
            for cls in ('fed', 'hungry', 'lunch', 'all'):
                O = [o for o in R if o['dir30'] == d and (cls == 'all' or o['cls'] == cls)]
                G = [o for o in O if o['gave_up']]
                if not O: continue
                na = sum(o['n_after'] for o in G)
                med = lambda xs: st.median(xs) if xs else math.nan
                print(f"  {d} {cls:6} n {len(O):4}  gave up {len(G):4} ({len(G)/len(O):.2f})  frames to give up median "
                      f"{med([o['t_give'] for o in G]):6.0f}  give-up -> back median {med([o['t_after'] for o in G]):6.0f}  "
                      f"after give-up: on route {sum(o['on_after'] for o in G)/max(1, na):.3f}  heading outward "
                      f"{sum(o['out_after'] for o in G)/max(1, na):.3f} (rows {na})", file=out)
                D = sorted(o['dark'] for o in G if not math.isnan(o['dark']))
                if D:
                    print(f"           dark walk before give-up (cells past the last lit step, {len(D)} that were ever lit): "
                          f"median {st.median(D):.0f}  p90 {D[int(0.9 * (len(D) - 1))]:.0f}  max {D[-1]:.0f}", file=out)
    if tsv and res:
        with open(tsv, 'w') as f:
            f.write('\t'.join(res[0].keys()) + '\n')
            for o in res: f.write('\t'.join(str(v) for v in o.values()) + '\n')
    return res


def selftest():
    """Positive and negative control on a hand-built CSV: one fed ant goes 40 cells east, gives up, and steps
    back with 3 of its 4 B-reading rows heading home; a second identical ant never gives up. A fault that
    reads chosen_cos with the wrong sign, or never finds scout_home, must move the numbers."""
    import io, os, tempfile
    hdr = NEED
    rows = []
    def row(fr, aid, x, x2, at_nest, sh, cos, leg='empty'):
        v = dict(seed=1, gap=90, arm='self', frame=fr, id=aid, leg=leg, x=x, y=90, x2=x2, at_nest=at_nest, nest_x=100,
                 outcome='stepped', chosen_route=0.6, chosen_cos=cos, energy_j=300, drive='NaN', scout_home=sh, fill=0.0,
                 scout_w=0.5, scout_patience=0)
        rows.append(','.join(str(v[k]) for k in hdr))
    for aid, gives in ((1, True), (2, False)):
        row(0, aid, 100, 100, 1, 0, 1)
        fr = 1
        for s in range(40):
            row(fr, aid, 100 + s, 101 + s, 0, 0, -1); fr += 1
        for s in range(4):
            row(fr, aid, 140 - s, 139 - s, 0, 1 if gives else 0, -1 if s == 0 else 1); fr += 1
        row(fr, aid, 136, 136, 1, 0, 1)
    fd, path = tempfile.mkstemp(suffix='.csv')
    with os.fdopen(fd, 'w') as f:
        f.write(','.join(hdr) + '\n' + '\n'.join(rows) + '\n')
    try:
        res = run([path], out=io.StringIO())
    finally:
        os.unlink(path)
    by = {o['id']: o for o in res}
    ok = True
    def check(cond, msg):
        nonlocal ok
        print(('ok   ' if cond else 'FAIL ') + msg)
        ok &= cond
    check(len(res) == 2 and all(o['dir30'] == 'E' for o in res), 'both ants are east forager excursions')
    check(by[1]['gave_up'] and not by[2]['gave_up'], 'ant 1 gave up, ant 2 did not')
    check(by[1]['t_give'] == 40 and by[1]['t_after'] == 3, f"frames to give up 40 and back 3 (got {by[1]['t_give']}, {by[1]['t_after']})")
    check(by[1]['dark'] == 1, f"dark walk 1 cell: lit up to the row before give-up (got {by[1]['dark']})")
    check(by[1]['n_after'] == 4 and by[1]['out_after'] == 1, f"1 of 4 rows after give-up heads outward (got {by[1]['out_after']}/{by[1]['n_after']})")
    sys.exit(0 if ok else 1)


def main():
    args = sys.argv[1:]
    if '--selftest' in args: selftest()
    tsv = None
    if '--tsv' in args:
        i = args.index('--tsv'); tsv = args[i + 1]; del args[i:i + 2]
    example = '--example' in args
    paths = [a for a in args if not a.startswith('--')]
    if not paths: sys.exit(__doc__)
    run(paths, tsv, example)


if __name__ == '__main__':
    main()
