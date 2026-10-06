"""scorecard.py RUN_DIR... [--from F] [--to T] -- one column per deeptrace run of the goal box (deep trace lane, 2026-10-06)

The measuring lane's shared baseline card. Point it at the baseline's runs and at your own arm's runs (same seeds, same
`deeptrace` arguments: `scenario=nest_goal founder=evolved ants=0 mapevery=1000 hungry=1`, `RAYON_NUM_THREADS=1`) and
compare within seed. A run is a deeptrace `out=` directory: `stats.csv` (running totals every 1,000 frames), `colony.csv`
(every live ant every 1,000 frames), `events.txt` (the FOUNDED line) and `map_f*.txt` (the ground round the nest).

Windows: means and sums over --from..--to (100k-300k by default), starved from 20k (the founding die-off is before it).
Rows:
- TIME IN THE DUG NEST, the headline (Scott's number one issue, 2026-10-06: the ants do not spend time in the nest): the
  share of ant-time, live ants x census samples over the window, with the head below the old ground line (colony.csv
  zone `nest`, `zone()` in examples/deeptrace.rs), and the mean number of ants there; the same share for nest workers
  (colony.csv `worker` = 1, nest-bound) and for the other ants; and how it is spread over the ants sampled 10+ times in
  the window: never in the dug nest at a sample, under half their samples, half or more.
- where in the dug nest (added 2026-10-06 15:00, after the redesign thread found most of the headline is ants bunched
  inside the door): four bands that add up to the headline, as shares of all ant-time -- the door knot (within 4
  columns of the door and 10 rows of the old ground line), the rest of the top 10 rows, the door column deeper than 10
  rows, and deeper than 10 rows off the door column; then DEEPER THAN 10 ROWS on its own line (share of ant-time, mean
  ants there), the number to score stay-in work on.
- digs over the window (the engine's `digs`) and pellets dumped (`spoil_dumped`).
- chambers at 100/200/300k by the owner's rule (`chambers.py` in this folder, `examples/digbox.rs`'s `chambers_of`
  ported): count, each chamber tall x wide, how many are spec-shaped (8-16 tall, wider than tall), the passage bore and
  the contrast. The old room rule's rows stay so earlier numbers still compare.
- ants at 100/200/300k, the window's mean and its lowest (frame), births, eggs laid, deaths of old age and starved, larvae
  starved; trip deliveries (a forager's crop emptied at the nest after a trip; misses nurse hand-offs) and all deliveries.
- where the ants are (colony.csv zones: nest = in the dug nest, mound_in = in the spoil mound's tunnels, mound_top, food =
  at the heap, surface), the window's mean share; nest workers in the nest; ants holding a soil pellet.
- the door: maps on which it is cut off from the open air by the walk's rule (`doorseal.py` in this folder: brood and
  crumbs open, soil, corpses and provisions walls) and the longest spell, over 6k..--to.
- the nest from the maps: rooms under the old ground line (a room cell is open with 7 of its 3x3 open, a room is a
  4-connected piece of 30+ cells; `scripts/deeptrace_dig.py rooms`' rule), the biggest, and how many others; food lying
  under the ground line (`f` other food; crumbs `c` and corpses `x` apart, on maps written since 2026-10-06); soil in the
  mound (spoil and ground above the old ground line within 40 columns of the door).
Maps written before 2026-10-06 draw every food `f`: the door count is then an upper bound (doorseal.py's docstring).
`map_f` reaches 70 rows under the ground line; a nest dug deeper is cut off there (seed 2's deepest open row was 62 at
300k). Checked: the biggest room from `map_f` equals `scripts/deeptrace_dig.py rooms` from `nest_f` on the same frames
(seed 2, `CARRY_HOME=on`, 100-300k: 516/802/1,041/1,188/1,048 both); a block carved into a map is counted when it holds
30+ room cells (8x8 -> 36) and not below (7x7 -> 25); ants at 300k, starved and trip deliveries reproduce §13's table.
"""
import sys, os, csv, glob, re, collections

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import doorseal  # noqa: E402
import chambers as chamber_rule  # noqa: E402

OPEN = set("o.aelpfcx~?b")
MARKS = (100_000, 200_000, 300_000)


def founded(run):
    for line in open(f"{run}/events.txt"):
        p = line.split()
        if len(p) > 1 and p[1] == "FOUNDED":
            kv = dict(t.split("=") for t in p[2:] if "=" in t)
            return int(kv["nest_x"]), int(kv["ground_y"])
    sys.exit(f"{run}: no FOUNDED line (still writing?)")


def stats(run):
    rows = {int(r["frame"]): r for r in csv.DictReader(open(f"{run}/stats.csv"))}
    return rows


def at(rows, f, k):
    if f not in rows:
        f = max((g for g in rows if g <= f), default=None)
        if f is None:
            return 0.0
    return float(rows[f][k] or 0)


def window(rows, a, b, *keys):
    return sum(at(rows, b, k) - at(rows, a, k) for k in keys)


def colony(run, a, b):
    zones = collections.Counter()
    frames = set()
    nw_in = nw_all = held = 0
    for r in csv.DictReader(open(f"{run}/colony.csv")):
        f = int(r["frame"])
        if f < a or f > b:
            continue
        frames.add(f)
        zones[r["zone"]] += 1
        if r["worker"] == "1":
            nw_all += 1
            nw_in += r["zone"] == "nest"
        held += r["spoil"] not in ("0", "")
    n = sum(zones.values())
    k = max(1, len(frames))
    return {z: zones[z] / max(1, n) for z in ("nest", "mound_in", "mound_top", "food", "surface")}, nw_in / max(1, nw_all), held / k


def nest_time(run, a, b):
    """The headline: time in the dug nest over a..b (see the module docstring)."""
    n = inside = 0
    caste, caste_in = collections.Counter(), collections.Counter()
    per = collections.defaultdict(lambda: [0, 0])
    frames = set()
    for r in csv.DictReader(open(f"{run}/colony.csv")):
        f = int(r["frame"])
        if f < a or f > b:
            continue
        frames.add(f)
        here = r["zone"] == "nest"
        n += 1
        inside += here
        caste[r["worker"]] += 1
        caste_in[r["worker"]] += here
        per[r["id"]][0] += 1
        per[r["id"]][1] += here
    ants = [p for p in per.values() if p[0] >= 10]
    never = sum(p[1] == 0 for p in ants)
    most = sum(2 * p[1] >= p[0] for p in ants)
    m = max(1, len(ants))
    return (inside / max(1, n), inside / max(1, len(frames)), caste_in["1"] / max(1, caste["1"]),
            caste_in["0"] / max(1, caste["0"]), (never / m, (len(ants) - never - most) / m, most / m), len(ants))


KNOT_COLS, KNOT_ROWS = 4, 10


def nest_bands(run, a, b):
    """Where in the dug nest over a..b, as shares of all ant-time: the door knot, the rest of the top KNOT_ROWS rows,
    the door column deeper, and deeper off it (see the module docstring); and the mean number of ants deeper."""
    nx, gy = founded(run)
    n = 0
    band = collections.Counter()
    frames = set()
    for r in csv.DictReader(open(f"{run}/colony.csv")):
        f = int(r["frame"])
        if f < a or f > b:
            continue
        frames.add(f)
        n += 1
        if r["zone"] != "nest":
            continue
        door = abs(int(r["hx"]) - nx) <= KNOT_COLS
        if int(r["hy"]) - gy <= KNOT_ROWS:
            band["knot" if door else "top"] += 1
        else:
            band["shaft" if door else "deep"] += 1
    shares = {k: band[k] / max(1, n) for k in ("knot", "top", "shaft", "deep")}
    return shares, (band["shaft"] + band["deep"]) / max(1, len(frames))


def rooms(path, gy):
    with open(path) as fh:
        x0, y0, wd, ht = map(int, fh.readline().split())
        rows = [ln.rstrip("\n").ljust(wd, "#")[:wd] for ln in fh][:ht]
    op = [[rows[y][x] in OPEN and y0 + y > gy for x in range(wd)] for y in range(ht)]
    room = [[False] * wd for _ in range(ht)]
    for y in range(1, ht - 1):
        for x in range(1, wd - 1):
            if op[y][x] and sum(op[y + dy][x + dx] for dy in (-1, 0, 1) for dx in (-1, 0, 1)) >= 7:
                room[y][x] = True
    seen, comps = set(), []
    for y in range(ht):
        for x in range(wd):
            if room[y][x] and (x, y) not in seen:
                seen.add((x, y))
                stack, n = [(x, y)], 0
                while stack:
                    cx, cy = stack.pop()
                    n += 1
                    for nx, ny in ((cx + 1, cy), (cx - 1, cy), (cx, cy + 1), (cx, cy - 1)):
                        if 0 <= nx < wd and 0 <= ny < ht and room[ny][nx] and (nx, ny) not in seen:
                            seen.add((nx, ny))
                            stack.append((nx, ny))
                comps.append(n)
    comps = sorted((c for c in comps if c >= 30), reverse=True)
    food = collections.Counter(rows[y][x] for y in range(ht) for x in range(wd) if y0 + y > gy and rows[y][x] in "fcx")
    return comps, food


def mound(path, nx, gy):
    at_ = doorseal.load(path)
    return sum(at_(x, y) in "s#=" for x in range(nx - 40, nx + 41) for y in range(gy - 60, gy))


def card(run, a, b):
    nx, gy = founded(run)
    st = stats(run)
    end = max(st)
    c = {}
    share, there, nw, rest, spread, n_ants = nest_time(run, a, b)
    c[f"TIME IN THE DUG NEST {a // 1000}-{b // 1000}k: ant-time (mean ants there)"] = f"{100 * share:.1f}% ({there:.0f})"
    bands, deep_ants = nest_bands(run, a, b)
    c["  door knot / rest of top 10 rows / deeper: door column, off it"] = " / ".join(f"{100 * bands[k]:.2f}" for k in ("knot", "top", "shaft", "deep")) + "%"
    c["  nest workers / other ants"] = f"{100 * nw:.0f}% / {100 * rest:.0f}%"
    c["  ants sampled 10+ times: never / under half / half+"] = " / ".join(f"{100 * x:.0f}%" for x in spread) + f" of {n_ants}"
    c["DEEPER THAN 10 ROWS: ant-time (mean ants there)"] = f"{100 * (bands['shaft'] + bands['deep']):.2f}% ({deep_ants:.1f})"
    c["frames recorded"] = f"{end // 1000}k"
    c["ants at 100k / 200k / 300k"] = " / ".join(f"{at(st, m, 'ants'):.0f}" if m <= end else "-" for m in MARKS)
    span = [f for f in st if a <= f <= b]
    if span:
        lo = min(span, key=lambda f: at(st, f, "ants"))
        c[f"mean ants {a // 1000}-{b // 1000}k (lowest, at)"] = f"{sum(at(st, f, 'ants') for f in span) / len(span):.0f} ({at(st, lo, 'ants'):.0f} at {lo // 1000}k)"
    c["births / eggs laid"] = f"{window(st, a, b, 'births'):.0f} / {window(st, a, b, 'eggs_laid'):.0f}"
    c["died old age"] = f"{window(st, a, b, 'died_old_age'):.0f}"
    c[f"starved (20-{b // 1000}k / in window)"] = f"{window(st, 20_000, b, 'died_starved', 'died_starved_aloft'):.0f} / {window(st, a, b, 'died_starved', 'died_starved_aloft'):.0f}"
    c["larvae starved"] = f"{window(st, a, b, 'larvae_starved'):.0f}"
    c["trip deliveries / all deliveries"] = f"{window(st, a, b, 'trip_deliveries'):.0f} / {window(st, a, b, 'deliveries'):.0f}"
    c["digs / pellets dumped"] = f"{window(st, a, b, 'digs'):.0f} / {window(st, a, b, 'spoil_dumped'):.0f}"
    zones, _, held = colony(run, a, b)
    c["ants in nest / mound tunnels / mound top / heap / surface"] = " / ".join(f"{100 * zones[z]:.0f}%" for z in ("nest", "mound_in", "mound_top", "food", "surface"))
    c["ants holding a soil pellet (mean)"] = f"{held:.0f}"
    maps = sorted(glob.glob(f"{run}/map_f*.txt"), key=lambda p: int(re.search(r"map_f(\d+)", p).group(1)))
    fr = [(int(re.search(r"map_f(\d+)", p).group(1)), p) for p in maps]
    shut = [f for f, p in fr if f <= b and not doorseal.measure(p)[0]]
    allm = [f for f, _ in fr if f <= b]
    longest, cur, prev = 0, 0, None
    step = (allm[1] - allm[0]) if len(allm) > 1 else 1000
    for f in shut:
        cur = cur + 1 if prev is not None and f - prev == step else 1
        longest, prev = max(longest, cur), f
    c[f"door shut (maps of {len(allm)}; longest spell)"] = f"{len(shut)}; {longest}"
    for m in MARKS:
        p = dict(fr).get(m)
        if not p:
            continue
        comps, food = rooms(p, gy)
        c[f"{m // 1000}k: rooms 30+ (biggest; others)"] = f"{len(comps)} ({comps[0] if comps else 0}; {' '.join(map(str, comps[1:])) or '-'})"
        c[f"{m // 1000}k: food under ground f/c/x; mound soil"] = f"{food['f']}/{food['c']}/{food['x']}; {mound(p, nx, gy)}"
        c[f"{m // 1000}k: chambers, owner's rule (tall x wide)"] = chamber_rule.describe(chamber_rule.measure(p, gy))
    return c


def main():
    args = sys.argv[1:]
    a, b = 100_000, 300_000
    if "--from" in args:
        i = args.index("--from")
        a = int(args[i + 1])
        del args[i:i + 2]
    if "--to" in args:
        i = args.index("--to")
        b = int(args[i + 1])
        del args[i:i + 2]
    cards = [(r.rstrip("/").split("/")[-1], card(r, a, b)) for r in args]
    keys = list(dict.fromkeys(k for _, c in cards for k in c))
    w = max(len(k) for k in keys)
    cw = max(14, *(len(n) for n, _ in cards), *(len(v) for _, c in cards for v in c.values()))
    print(f"{'':{w}}  " + "  ".join(f"{n:>{cw}}" for n, _ in cards))
    for k in keys:
        print(f"{k:{w}}  " + "  ".join(f"{c.get(k, '-'):>{cw}}" for _, c in cards))


if __name__ == "__main__":
    main()
