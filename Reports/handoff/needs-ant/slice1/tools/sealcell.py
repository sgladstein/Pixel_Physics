#!/usr/bin/env python3
"""sealcell.py RUN [--cells DIGREC] [--walk WALKREC] [--from F] [--to F] -- who shut the nest's door, and who opened it.

For every pair of consecutive maps (map_f*.txt in RUN) where doorseal.py's verdict flips -- open -> sealed (a seal) or
sealed -> open (an opening) -- finds the cells that made the difference, then names what put them there.

The door test is doorseal.py's own (its PASS/SOLID sets and sky rule, imported from the deep trace lane's tool).

Seal (open at A, sealed at B): the "thinnest new wall". A 0-1 search from the shut-in space at B to any sky cell at B,
where a cell costs 0 if passable at B, 1 if it was passable at A and is not at B (newly closed), and cannot be crossed
otherwise. D is the fewest newly closed cells that, reopened, would open the door; every newly closed cell on some
cheapest path is listed. With D = 1 each listed cell alone shut the door.

Opening (sealed at A, open at B): the same search from the door's space at A to sky at A, costing 1 for cells that are
passable at B but not at A (newly opened) -- the cells whose opening let the door out.

For each listed cell, the last change to it inside (A, B] from the digging record's cells.csv.gz (frame, from, to,
cause, ant): a `drop` names the ant that let the pellet go; a blank cause is a fall or slide, traced back grain by grain
(the matching ground->open change one row up or diagonally up in the same frame) to the drop that started it, or to
ground already standing at A. A `cut` names the ant. With --walk (a needs-walk record with walk.csv.gz), the drive and
job of that ant at that frame.

Map letters (deeptrace write_map): . empty, a ant, b brood, c crumbs, x corpse, f other food, s powder, # solid.
cells.csv.gz logs only ground <-> not-ground changes (ground = unowned powder or solid holding no food), so a cell shut
by food, a corpse or an ant standing still is listed with no event.

Caveat (2026-10-07): the digging record names a cut only for the brain's own dig (`act`). A needs-walk escape cut,
pack or door cut is logged with a blank cause, so this tool prints it as 'fell' / 'fell-away'. On a run whose walk.csv
has the `cut` column, check the walk rows beside the cell (cut 2 escape, 3 pack, 4 door cut) before calling an opening
soil falling away.
"""
import sys, glob, re, gzip, csv, collections, heapq

sys.path.insert(0, '/mnt/project-files/deep-trace/tools')
import doorseal as ds  # noqa: E402

DX, DY, YMAX = ds.DX, ds.DY, 175


def load(p):
    return ds.load(p)


def flood(at, start, passable=None):
    passable = passable or (lambda x, y: at(x, y) in ds.PASS)
    seen = {start}
    q = collections.deque([start])
    while q:
        x, y = q.popleft()
        for dx in (-1, 0, 1):
            for dy in (-1, 0, 1):
                n = (x + dx, y + dy)
                if n in seen or n[1] > YMAX or not passable(*n):
                    continue
                seen.add(n)
                q.append(n)
    return seen


def is_sky(at, x, y):
    return y <= DY and not any(at(x, y - d) in ds.SOLID for d in range(1, 21))


def bounds(p):
    x0, y0, w, h = map(int, open(p).readline().split())
    return x0, y0, w, h


def zero_one(at_base, at_other, start_cells, newly, box):
    """0-1 search from start_cells; cost 0 through cells passable in at_base, 1 through `newly` cells, else blocked.
    Returns dist dict."""
    x0, y0, w, h = box
    INF = 10 ** 9
    dist = {}
    dq = collections.deque()
    for c in start_cells:
        dist[c] = 0
        dq.append(c)
    while dq:
        c = dq.popleft()
        dc = dist[c]
        x, y = c
        for dx in (-1, 0, 1):
            for dy in (-1, 0, 1):
                n = (x + dx, y + dy)
                if not (x0 <= n[0] < x0 + w and y0 <= n[1] < y0 + h) or n[1] > YMAX:
                    continue
                if at_base(*n) in ds.PASS:
                    cost = 0
                elif n in newly:
                    cost = 1
                else:
                    continue
                nd = dc + cost
                if nd < dist.get(n, INF):
                    dist[n] = nd
                    if cost == 0:
                        dq.appendleft(n)
                    else:
                        dq.append(n)
    return dist


def wall(at_closed, at_open_ref, box, closed_is_b):
    """Cells that separate the door from the sky in at_closed and are passable in at_open_ref."""
    x0, y0, w, h = box
    shut = flood(at_closed, (DX, DY))
    newly = set()
    for y in range(y0, min(y0 + h, YMAX + 1)):
        for x in range(x0, x0 + w):
            if at_closed(x, y) not in ds.PASS and at_open_ref(x, y) in ds.PASS:
                newly.add((x, y))
    sky = [(x, y) for y in range(y0, min(y0 + h, DY + 1)) for x in range(x0, x0 + w)
           if at_closed(x, y) in ds.PASS and is_sky(at_closed, x, y)]
    d1 = zero_one(at_closed, at_open_ref, shut, newly, box)
    d2 = zero_one(at_closed, at_open_ref, sky, newly, box)
    reach = [d1[c] for c in sky if c in d1]
    if not reach:
        return None, [], len(shut)
    D = min(reach)
    on = sorted(c for c in newly if c in d1 and c in d2 and d1[c] + d2[c] - 1 == D)
    return D, on, len(shut)


def load_cells(path, a, b):
    rows = []
    with gzip.open(path, 'rt') as fh:
        r = csv.reader(fh)
        next(r)
        for row in r:
            f = int(row[0])
            if f <= a - 5000:
                continue
            if f > b:
                break
            rows.append((f, int(row[1]), int(row[2]), row[3], row[4], row[5], row[6]))
    return rows


def last_change(cells, x, y, a, b):
    hits = [r for r in cells if r[1] == x and r[2] == y and a < r[0] <= b]
    return hits


def trace_grain(cells_by_frame, cells_at, f, x, y, depth=0):
    """A grain arrived at (x,y) at frame f with no cause. Find the cell it left in the same frame (one up or diagonal
    up, or sideways), then when that cell last became ground, recursively."""
    if depth > 400:
        return ('lost', f, x, y)
    for dx, dy in ((0, -1), (-1, -1), (1, -1), (-1, 0), (1, 0), (0, -2)):
        sx, sy = x + dx, y + dy
        for r in cells_by_frame.get(f, ()):
            if r[1] == sx and r[2] == sy and r[4] in ('empty', 'ant') and r[3] not in ('empty', 'ant'):
                # source cell (sx,sy) emptied at f: when did it last become ground?
                prior = [q for q in cells_at.get((sx, sy), ()) if q[0] < f and q[4] not in ('empty', 'ant')]
                if not prior:
                    return ('standing', f, sx, sy)
                q = prior[-1]
                if q[5] == 'drop':
                    return ('drop', q[0], sx, sy, q[6], depth + 1)
                if q[5] == '':
                    return trace_grain(cells_by_frame, cells_at, q[0], sx, sy, depth + 1)
                return (q[5] or 'other', q[0], sx, sy, q[6], depth + 1)
    return ('unmatched', f, x, y)


def walk_drives(path, want):
    """want: set of (frame, id). Returns {(frame,id): (drive, job, energy, hold)} using the walk row at frame+1 or the
    nearest earlier one for that ant."""
    if not want:
        return {}
    ids = {i for _, i in want}
    lo = min(f for f, _ in want) - 200
    hi = max(f for f, _ in want) + 2
    last = {}
    out = {}
    targets = collections.defaultdict(list)
    for f, i in want:
        targets[i].append(f)
    with gzip.open(path, 'rt') as fh:
        r = csv.reader(fh)
        h = next(r)
        ix = {k: j for j, k in enumerate(h)}
        for row in r:
            f = int(row[0])
            if f < lo:
                continue
            if f > hi:
                break
            i = row[1]
            if i not in ids:
                continue
            for tf in targets[i]:
                if f <= tf + 1:
                    out[(tf, i)] = (row[ix['drive']], row[ix['job']], row[ix['energy']], row[ix['hold']], f)
    return out


def main():
    args = sys.argv[1:]
    run = args[0]
    opt = {}
    i = 1
    while i < len(args):
        opt[args[i].lstrip('-')] = args[i + 1]
        i += 2
    lo = int(opt.get('from', 50000))
    hi = int(opt.get('to', 100000))
    maps = sorted(glob.glob(run + '/map_f*.txt'), key=lambda p: int(re.search(r'map_f(\d+)', p).group(1)))
    maps = [p for p in maps if lo <= int(re.search(r'map_f(\d+)', p).group(1)) <= hi]
    verdict = [(int(re.search(r'map_f(\d+)', p).group(1)), p, ds.measure(p)[0]) for p in maps]
    flips = [(verdict[k - 1], verdict[k]) for k in range(1, len(verdict)) if verdict[k - 1][2] != verdict[k][2]]
    cells = load_cells(opt['cells'], lo, hi) if 'cells' in opt else []
    by_frame = collections.defaultdict(list)
    at_cell = collections.defaultdict(list)
    for r in cells:
        by_frame[r[0]].append(r)
        at_cell[(r[1], r[2])].append(r)
    report = []
    want = set()
    for (fa, pa, oa), (fb, pb, ob) in flips:
        A, B = load(pa), load(pb)
        box = bounds(pb)
        kind = 'SEAL' if oa and not ob else 'OPEN'
        if kind == 'SEAL':
            D, on, nshut = wall(B, A, box, True)
        else:
            D, on, nshut = wall(A, B, box, False)
        items = []
        for (x, y) in on:
            hist = last_change(cells, x, y, fa, fb)
            item = {'xy': (x, y), 'a': A(x, y), 'b': B(x, y), 'hist': hist, 'origin': None}
            # the change that made it what it is at B
            if hist:
                last = hist[-1]
                if kind == 'SEAL':
                    if last[5] == 'drop':
                        item['origin'] = ('drop', last[0], x, y, last[6], 0)
                    elif last[5] == '':
                        item['origin'] = trace_grain(by_frame, at_cell, last[0], x, y)
                    else:
                        item['origin'] = (last[5], last[0], x, y, last[6], 0)
                else:
                    item['origin'] = (last[5] or 'fell-away', last[0], x, y, last[6], 0)
            o = item['origin']
            if o and o[0] in ('drop', 'cut', 'lift') and len(o) > 4 and o[4]:
                want.add((o[1], o[4]))
            items.append(item)
        report.append((kind, fa, fb, D, nshut, items))
    drives = walk_drives(opt['walk'], want) if 'walk' in opt else {}
    for kind, fa, fb, D, nshut, items in report:
        print(f"{kind} {fa // 1000}k -> {fb // 1000}k: wall {D} cell(s) thick; shut-in space {nshut} cells; "
              f"{len(items)} cell(s) on a thinnest wall")
        for it in items:
            x, y = it['xy']
            o = it['origin']
            line = f"   ({x},{y}) row {y - 160:+d} col {x - DX:+d}: '{it['a']}' -> '{it['b']}'"
            if it['hist']:
                last = it['hist'][-1]
                line += f"; last change f{last[0]} {last[3]}>{last[4]} {last[5] or 'fell'} id {last[6] or '-'}"
                line += f" ({len(it['hist'])} changes in window)"
            else:
                line += "; no ground change logged (food, corpse, ant or brood)"
            if o:
                if o[0] in ('drop', 'cut', 'lift') and len(o) > 4:
                    dv = drives.get((o[1], o[4]))
                    dtxt = f" drive {dv[0]} job {dv[1]} energy {dv[2]} hold {dv[3]}" if dv else ''
                    line += f"\n       origin: {o[0]} by ant {o[4]} at f{o[1]} ({o[2]},{o[3]}){dtxt}"
                else:
                    line += f"\n       origin: {o[0]} f{o[1]} ({o[2]},{o[3]})"
            print(line)


if __name__ == '__main__':
    main()
