"""starvewhere.py RUN... [--from F] [--to T] | --selftest -- where were the ants that starved, and since when? (deep trace lane, 2026-10-06)

Needs a run with the census on (deeptrace `hungry=1`: death lines in events.txt, every ant in colony.csv every 1k) and
maps every 1k (`mapevery=1000`), as in deep-trace/baseline/. A digging record without hungry=1 has no death lines.

For each death with cause=STARVED alone in the window (default 20k-300k; STARVED+OLD AGE is left out, and stats.csv's
died_starved books some of those as starved, so the count runs a few under it: 11 against 12 on ecca174e6 seed 1, 145
against 152 on seed 8) it takes the ant's last census sample (hx, hy, pellet, energy) and the map written on that same frame (so up to 999 frames before the death), and asks which space
its head was in. Spaces are the connected walkable cells by doorseal.py's rule (air, dug, ants, brood and crumbs
open; soil, corpses and other food walls; 8-way; to the bottom of the map):
  door system, door open    -- the space beside the door anchor, and doorseal.py reads the door open: with the
                               door open this is also the open ground, so it means "free to walk out and in"
  door system, door SEALED  -- the same space with the door sealed: shut in behind the door
  elsewhere, open air       -- a space that reaches the open air but not the door (the open ground while the door is shut)
  closed pocket             -- a space joined to neither, with at least one free cell (air, dug, brood or crumbs)
  encased                   -- a space joined to neither and made only of ant cells: the ant, alone or packed
                               with others, with soil or food all round and no free cell to step into
  off the map               -- outside the map's box (the open ground far from the nest)
Beside the starvers it prints the same buckets for EVERY ant, and every ant in the mound's tunnels, on the same
frames: the base rate the starvers are compared with.

Then, for each starver trapped at its last sample (sealed, pocket or encased), it walks back through its census
samples while it stayed trapped, and prints how long that was, what its energy was when it was first found trapped,
and whether it held a pellet then.

The flood is 8-way with no corner rule, so it joins diagonal gaps the walk cannot: 'pocket' and 'encased' are lower
bounds. Checked by eye on ecca174e6 seed 2 at 34k (an encased starver: four ant cells in solid soil; another packed
with a second ant, eight ant cells). On that baseline the starvers' closed spaces of 7-10 cells were all ant cells.

**Corrected 2026-10-07** (deep trace lane): until then the flood stopped at row 175, 15 rows under the ground line,
the floor of doorseal.py's door walk, where it does no harm. A head deeper than that joined no space, and `bucket`
read it "encased" whatever was round it, so in a nest dug deeper than 15 rows "encased" counted depth. On
LAY_BAR=body seed 1 with the smell store (95d65cd66, starvers of 55-70k) 50 of 51 starvers read encased and, with
the flood to the map bottom, 0: all 50 "door system, door open", their last census row a median 192. On arm 2b seeds
1-4 (c9e8a860, 20k to the end) 433 / 360 / 153 / 291 encased starvers became 0 / 1 / 0 / 11, and the all-ant base rate
went from 6-14% to 0%. An "encased" read made before this date with starvers deeper than row 175 needs re-reading.
"door system, door open" says an unbroken 8-way path of air, ants, brood or crumbs joins the head to the door. It does
not say an ant can climb it: a body with nothing solid, powdery or plant beside it and no grip on a grounded nestmate
falls a cell instead of stepping (creature.rs `fall_if_unsupported`).
`--selftest` builds a room reaching 45 rows under the ground line and two sealed pockets, checks each bucket, and shows
the old floor reading the deep room's ant "encased".
"""
import os, sys, re, csv, collections
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import doorseal as D

ORDER = ['door system, door open', 'door system, door SEALED', 'elsewhere, open air', 'closed pocket', 'encased',
         'off the map']
TRAPPED = {'door system, door SEALED', 'closed pocket', 'encased'}


def spaces(path, floor=None):
    """The map's spaces. `floor`: flood only the rows above it (None, the default, is the whole map; 176 is the floor
    this tool had until 2026-10-07, kept for --selftest to show the fault)."""
    L = open(path).read().split('\n')
    x0, y0, w, h = map(int, L[0].split())
    at = D.load(path)
    comp, info = {}, []
    bottom = y0 + h if floor is None else min(y0 + h, floor)
    inside = lambda p: x0 <= p[0] < x0 + w and y0 <= p[1] < bottom
    for yy in range(y0, bottom):
        for xx in range(x0, x0 + w):
            if (xx, yy) in comp or at(xx, yy) not in D.PASS:
                continue
            cid = len(info); comp[(xx, yy)] = cid
            q = collections.deque([(xx, yy)]); n = 0; free = 0; sky = False
            while q:
                x, y = q.popleft(); n += 1; free += at(x, y) != 'a'
                if y <= D.DY and not any(at(x, y - d) in D.SOLID for d in range(1, 21)):
                    sky = True
                for dx in (-1, 0, 1):
                    for dy in (-1, 0, 1):
                        m = (x + dx, y + dy)
                        if m in comp or not inside(m) or at(*m) not in D.PASS:
                            continue
                        comp[m] = cid; q.append(m)
            info.append((n, sky, free))
    # doorseal.py floods from the anchor whatever it holds, so the door system is every space beside the anchor
    door = {comp[(D.DX + dx, D.DY + dy)] for dx in (-1, 0, 1) for dy in (-1, 0, 1) if (D.DX + dx, D.DY + dy) in comp}
    return comp, info, door, any(info[c][1] for c in door), (x0, y0, w, h)


def bucket(sp, x, y):
    comp, info, door, dsky, (x0, y0, w, h) = sp
    if not (x0 + 1 <= x < x0 + w - 1 and y0 + 1 <= y < y0 + h - 1):
        return 'off the map'
    cs = {comp[(x + dx, y + dy)] for dx in (-1, 0, 1) for dy in (-1, 0, 1) if (x + dx, y + dy) in comp}
    if cs & door:
        return 'door system, door ' + ('open' if dsky else 'SEALED')
    if any(info[c][1] for c in cs):
        return 'elsewhere, open air'
    return 'closed pocket' if sum(info[c][2] for c in cs) else 'encased'


def selftest():
    """A goal-box map (door anchor (256,159), ground row 160) with a shaft, a room 35 rows deep with ants at its floor,
    and two pockets of ants sealed in soil, one above row 175 and one below. Each head must land in its bucket, and the
    old floor (176) must read the deep room's ant encased -- the fault this test exists for."""
    import tempfile
    x0, y0, w, h = 176, 100, 151, 131
    g = [['.' if y0 + j < 160 else 's' for i in range(w)] for j in range(h)]
    def put(xa, xb, ya, yb, c):
        for y in range(ya, yb + 1):
            for x in range(xa, xb + 1):
                g[y - y0][x - x0] = c
    put(254, 258, 160, 169, '.')    # the shaft under the door
    put(240, 270, 170, 205, '.')    # a room from 10 to 45 rows under the ground line
    put(244, 252, 203, 205, 'a')    # a crowd on its floor
    put(280, 283, 190, 191, 'a')    # a pocket of ants sealed in soil, 30 rows down
    put(290, 292, 165, 166, 'a')    # and one 5 rows down
    with tempfile.TemporaryDirectory() as d:
        path = f'{d}/map_f000000.txt'
        open(path, 'w').write(f'{x0} {y0} {w} {h}\n' + '\n'.join(''.join(r) for r in g) + '\n')
        new, old = spaces(path), spaces(path, floor=176)
        rows = [('the shaft', 256, 165, new, 'door system, door open'),
                ('the deep room, 44 rows down', 248, 204, new, 'door system, door open'),
                ('the deep pocket', 281, 190, new, 'encased'),
                ('the shallow pocket', 291, 165, new, 'encased'),
                ('the shallow pocket, old floor', 291, 165, old, 'encased'),
                ('the deep room, old floor (the fault)', 248, 204, old, 'encased')]
        bad = 0
        for name, x, y, sp, want in rows:
            got = bucket(sp, x, y)
            bad += got != want
            print(f"  {'ok ' if got == want else 'BAD'} {name:38s} -> {got}" + ('' if got == want else f' (want {want})'))
    print('selftest:', 'PASS' if not bad else f'FAIL ({bad})')
    return 1 if bad else 0


def main():
    a = sys.argv[1:]
    if '--selftest' in a:
        sys.exit(selftest())
    lo = int(a[a.index('--from') + 1]) if '--from' in a else 20000
    hi = int(a[a.index('--to') + 1]) if '--to' in a else 300000
    runs = [x for j, x in enumerate(a) if not x.startswith('--') and (j == 0 or a[j - 1] not in ('--from', '--to'))]
    for run in runs:
        deaths = {}
        if ' DIED ' not in open(run + '/events.txt').read():
            print(f"{run}: no death lines in events.txt -- this tool needs a run with hungry=1 (the census)")
            continue
        for L in open(run + '/events.txt'):
            # cause=STARVED alone; the few STARVED+OLD AGE deaths are left out (see the docstring)
            if L.rstrip().endswith('cause=STARVED'):
                f = int(L.split()[0])
                if lo <= f <= hi:
                    deaths[re.search(r' id=(\d+)', L).group(1)] = f
        byframe = collections.defaultdict(list); hist = collections.defaultdict(list)
        for r in csv.DictReader(open(run + '/colony.csv')):
            byframe[int(r['frame'])].append(r)
            if r['id'] in deaths and int(r['frame']) <= deaths[r['id']]:
                hist[r['id']].append(r)
        cache = {}
        sp = lambda f: cache[f] if f in cache else cache.setdefault(f, spaces(f'{run}/map_f{f:06d}.txt'))
        st, al, mo = collections.Counter(), collections.Counter(), collections.Counter()
        trapped = []
        for i in deaths:
            if not hist[i]:
                st['no census sample'] += 1; continue
            r = hist[i][-1]; f = int(r['frame'])
            b = bucket(sp(f), int(r['hx']), int(r['hy'])); st[b] += 1
            if b in TRAPPED:
                k = len(hist[i]) - 1
                while k > 0 and bucket(sp(int(hist[i][k - 1]['frame'])), int(hist[i][k - 1]['hx']), int(hist[i][k - 1]['hy'])) in TRAPPED:
                    k -= 1
                first = hist[i][k]
                trapped.append((deaths[i] - int(first['frame']), int(first['energy_j']), first['spoil'] != '0', int(first['crop_cells'])))
        for f in sorted({int(h[-1]['frame']) for h in hist.values() if h}):
            for r in byframe[f]:
                b = bucket(sp(f), int(r['hx']), int(r['hy'])); al[b] += 1
                if r['zone'] == 'mound_in':
                    mo[b] += 1
        pct = lambda c, k: f"{100 * c[k] / max(1, sum(c.values())):3.0f}%"
        print(f"{'/'.join(run.rstrip('/').split('/')[-2:])}: {len(deaths)} starved {lo // 1000}-{hi // 1000}k; "
              f"base rate over the same {len({int(h[-1]['frame']) for h in hist.values() if h})} census frames: "
              f"{sum(al.values())} ant samples, {sum(mo.values())} of them in the mound's tunnels")
        print(f"    {'last sample before death':28s} {'starvers':>12s} {'all ants':>9s} {'mound tunnels':>14s}")
        for k in ORDER + ['no census sample']:
            if st[k] or al[k]:
                print(f"    {k:28s} {st[k]:5d} {pct(st, k)}  {pct(al, k):>9s} {pct(mo, k):>14s}")
        if trapped:
            d = sorted(t[0] for t in trapped); e = sorted(t[1] for t in trapped)
            print(f"    trapped at the end: {len(trapped)}; trapped for (frames before death, from the first trapped sample) "
                  f"median {d[len(d) // 2]}, range {d[0]}-{d[-1]}; energy when first found trapped median {e[len(e) // 2]} J "
                  f"(range {e[0]}-{e[-1]}), with food still in the crop for {sum(t[3] > 0 for t in trapped)} of them; "
                  f"holding a pellet then {sum(t[2] for t in trapped)}")


if __name__ == '__main__':
    main()
