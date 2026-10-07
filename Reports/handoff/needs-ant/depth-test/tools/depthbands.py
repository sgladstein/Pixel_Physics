"""depthbands.py RUN... [--from F] [--to T]: where the ants stand in the dug nest, as MEAN ANTS (about N of M), split by
caste, for the depth test (redesign thread, 2026-10-06).

Reads a deeptrace run's colony.csv (every live ant every 1,000 frames), events.txt (FOUNDED line) and stats.csv.
Bands are the scorecard's (deep-trace/tools/scorecard.py): door knot = in the dug nest within 4 columns of the door and
10 rows of the old ground line; top = the rest of the top 10 rows; door column deep = within 4 columns, deeper than 10
rows; off column deep = more than 4 columns off, deeper than 10 rows. "Idle" = crop 0, no pellet, energy at or above
the grant (start_j): the ants the depth rule may touch, if nest workers.
"""
import sys, csv, collections

args = sys.argv[1:]
LEAN = 0.5   # creature::lean_forage_of(world).line, as a share of the grant (100 J of 200)
a, b = 100_000, 300_000
if "--from" in args:
    i = args.index("--from"); a = int(args[i + 1]); del args[i:i + 2]
if "--to" in args:
    i = args.index("--to"); b = int(args[i + 1]); del args[i:i + 2]


def founded(run):
    for line in open(f"{run}/events.txt"):
        p = line.split()
        if len(p) > 1 and p[1] == "FOUNDED":
            kv = dict(t.split("=") for t in p[2:] if "=" in t)
            return int(kv["nest_x"]), int(kv["ground_y"]), float(kv.get("start_j", 200))
    sys.exit(f"{run}: no FOUNDED line")


def bands(run):
    nx, gy, start = founded(run)
    frames = set()
    n = collections.Counter()           # (caste) -> ant-samples
    band = collections.Counter()        # (caste, band) -> ant-samples
    idle_band = collections.Counter()   # (caste, band) idle ant-samples
    depth_bins = collections.Counter()  # nest workers in the nest, by 10-row bin
    ecls = collections.Counter()        # (band, energy class) ant-samples, all castes: under the lean line / to the grant / fed
    for r in csv.DictReader(open(f"{run}/colony.csv")):
        f = int(r["frame"])
        if f < a or f > b:
            continue
        frames.add(f)
        c = "nw" if r["worker"] == "1" else "other"
        n[c] += 1
        if r["zone"] != "nest":
            continue
        dx, dy = abs(int(r["hx"]) - nx), int(r["hy"]) - gy
        door = dx <= 4
        k = ("knot" if door else "top") if dy <= 10 else ("door_deep" if door else "off_deep")
        band[c, k] += 1
        idle = r["crop_cells"] == "0" and r["spoil"] in ("0", "") and float(r["energy_j"]) >= start
        idle_band[c, k] += idle
        e = float(r["energy_j"])
        ecls[k, "lean" if e < LEAN * start else ("half" if e < start else "fed")] += 1
        if c == "nw":
            depth_bins[min(dy // 10, 6)] += 1
    k = max(1, len(frames))
    return n, band, idle_band, depth_bins, k, ecls


def stats(run):
    rows = {int(r["frame"]): r for r in csv.DictReader(open(f"{run}/stats.csv"))}
    def at(f, key):
        g = max((h for h in rows if h <= f), default=None)
        return float(rows[g].get(key) or 0) if g is not None else 0.0
    out = {}
    for key in ("depth_slowed", "depth_pauses", "depth_unknown", "depth_rows", "depth_ground_rows", "depth_err_rows", "rest_pulls", "hungry_out_pulls"):
        out[key] = at(b, key) - at(a, key)
    return out


for run in args:
    n, band, idle_band, depth_bins, k, ecls = bands(run)
    M = (n["nw"] + n["other"]) / k
    print(f"== {run}  ({a // 1000}-{b // 1000}k, {k} samples)")
    print(f"  live ants (mean): {M:.0f}  (nest workers {n['nw'] / k:.0f}, others {n['other'] / k:.0f})")
    for c, name in (("nw", "nest workers"), ("other", "other ants")):
        cells = " / ".join(f"{band[c, z] / k:.1f}" for z in ("knot", "top", "door_deep", "off_deep"))
        idle = " / ".join(f"{idle_band[c, z] / k:.1f}" for z in ("knot", "top", "door_deep", "off_deep"))
        print(f"  {name}: mean ants at door knot / rest of top 10 rows / deeper in door column / deeper off it: {cells}   (idle: {idle})")
    deep_door = (band["nw", "door_deep"] + band["other", "door_deep"]) / k
    deep_off = (band["nw", "off_deep"] + band["other", "off_deep"]) / k
    print(f"  DEEPER THAN 10 ROWS: about {deep_door + deep_off:.1f} of {M:.0f} ants ({100 * (deep_door + deep_off) / max(1, M):.2f}%): door column {deep_door:.1f}, off it {deep_off:.1f}")
    nb = sum(depth_bins.values())
    print("  ants by energy (under the lean line {:.0f} J / lean line to grant / at or over the grant): ".format(LEAN * 200)
          + "; ".join(f"{name} {ecls[z, 'lean'] / k:.1f} / {ecls[z, 'half'] / k:.1f} / {ecls[z, 'fed'] / k:.1f}"
                      for z, name in (("knot", "door knot"), ("top", "rest of top"), ("door_deep", "door column deep"), ("off_deep", "off column deep"))))
    print("  nest workers in the dug nest by rows below ground (0-9,10-19,...,60+): " + " / ".join(f"{depth_bins[i] / k:.1f}" for i in range(7)))
    s = stats(run)
    if s["depth_slowed"] > 0:
        d = s["depth_slowed"]
        print(f"  depth rule over the window: slowed {d:.0f} decisions, pauses it made {s['depth_pauses']:.0f}, never-out decisions {s['depth_unknown']:.0f}; "
              f"mean depth remembered {s['depth_rows'] / d:.1f} rows vs below founding ground {s['depth_ground_rows'] / d:.1f} (mean gap {s['depth_err_rows'] / d:.1f})")
    print(f"  rest pulls {s['rest_pulls']:.0f}, hungry way-out pulls {s['hungry_out_pulls']:.0f}")
