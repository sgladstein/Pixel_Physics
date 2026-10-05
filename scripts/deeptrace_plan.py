#!/usr/bin/env python3
"""Do the nest plan's switches do their jobs? One table over `deeptrace walk=1
census=1` runs, one column per run, each switch read on the behaviour it was
built to change -- not on what it costs the colony, which comes last.

    python3 scripts/deeptrace_plan.py OUT [OUT...]          # a table, runs as columns
    python3 scripts/deeptrace_plan.py --json OUT [OUT...]   # the same numbers as JSON
    python3 scripts/deeptrace_plan.py --window=20000-60000 OUT  # another span

Name each OUT `<arm>-s<seed>` (`main-s1`, `soilway-s2`) and the table groups
runs by arm. Every number is over frames 100k-200k unless it says otherwise.
Each run's numbers are cached in `OUT/plan.json`; delete it to read again.

Places, as `deeptrace_dig.py fed` names them: under the old ground line is the
nest; within 3 columns of the door the top five rows are the *shaft* and the
rows under them the *lane*; the rest of the nest is the *room*; anything at or
above the old ground line is *out* (the mound and the surface).

**Soil out** (`SOIL_WAY`, with `WAY_GAPS`): every cell cut in the nest is
followed in `digrows.csv.gz` to the row its digger let go of the pellet: let go
in the nest is soil put back; let go out of it is soil out. How long it was
carried, how many carriers died holding one, and how many of the carriers'
walking decisions in the nest the soil's way out steered. Then the ground
itself, from `cuts.csv`: nest cuts at a cell cut before (re-digs) and at new
ground.

**The way out** (`WAY_GAPS`): of the walking decisions of hungry adults with
nothing carried in the nest (energy under the start), the share the hungry
ant's way out pulled, and the share with no pull at all -- the gap in the map
the switch closes.

**Back to the face** (`FACE_TRIP`): of nest cuts, the share whose digger's
next cut is within 2 cells of it; and how each walk back ended, from the
engine's own record (`trip_end`, since 2026-10-05) and the next cut: got back
and cut at its face, cut somewhere else, gave up, hungry, food, still walking
or died.

**Food down to the brood** (`CROP_DOWN`): what larvae were fed, by place and
by source (`feeds.csv`: a carrier's crop, a nestmate's bank, a brain's share,
food in reach they ate), per larva present (`brood.csv`'s census); larvae
starved per egg laid by place (`broodlog.csv`); and where the ants carrying
crop food walk (`walkrows.csv.gz`), with the share of their decisions below
the old ground that the larva scent steered (`nurse_w`).

**The colony**: mean adults and brood, eggs laid, and every adult death by
cause and place (`events.txt`'s `census=1` lines), with the starvers' energy,
what they carried, and, for those that died in the nest, the pulls on their
last 3,000 frames of walking.

Written 2026-10-05 for the owner's ask to trace whether the nest plan's
switches work at the ant level, whatever they cost; the numbers it produced
are in /mnt/project-files/deep-trace/nest-plan-switches-2026-10-05.md.
"""
import csv
import gzip
import json
import math
import os
import re
import subprocess
import sys
from collections import Counter, defaultdict

F0, F1 = 100000, 200001


def geo(out):
    for e in open(f"{out}/events.txt"):
        p = e.split()
        if len(p) > 1 and p[1] == "FOUNDED":
            kv = dict(t.split("=") for t in p[2:] if "=" in t)
            return int(kv["nest_x"]), int(kv["ground_y"]), float(kv.get("start_j", 200))
    sys.exit(f"{out}: no FOUNDED line in events.txt (run still writing?)")


def placer(nx, gy):
    def place(x, y):
        if y <= gy:
            return "out"
        if abs(x - nx) <= 3:
            return "shaft" if y - gy <= 5 else "lane"
        return "room"
    return place


def cheb(a, b):
    return max(abs(a[0] - b[0]), abs(a[1] - b[1]))


class Share(float):
    """A share of a whole: printed as a percentage, and listed in the cache's
    `_shares` so it still is after a round trip through JSON."""


def share(a, b):
    return Share(round(a / b, 4)) if b else None


def med(xs):
    if not xs:
        return None
    xs = sorted(xs)
    return xs[len(xs) // 2]


def dig_events(out):
    """The rows of `digrows.csv.gz` this reader needs, through awk: every cut,
    every walk back the engine ended (`trip_end`), and the row each carrier
    let go of its pellet (the first row after a cut whose `hold` is not soil).
    Cached as `digplan.csv`."""
    path = f"{out}/digplan.csv"
    if not os.path.exists(path):
        with gzip.open(f"{out}/digrows.csv.gz", "rt") as fh:
            head = fh.readline().rstrip("\n").split(",")
        if "trip_end" not in head:
            sys.exit(f"{out}: digrows.csv.gz has no trip_end column (recorded before 2026-10-05)")
        c = {k: i + 1 for i, k in enumerate(head)}
        prog = (
            f'NR > 1 {{ id = ${c["id"]}; '
            f'if ((id in car) && ${c["hold"]} != "2") {{ print "let_go," $0; delete car[id] }} '
            f'if (${c["dig"]} == "cut") {{ print "cut," $0; car[id] = 1 }} '
            f'if (${c["trip_end"]} != "") print "end," $0 }}'
        )
        tmp = path + ".tmp"
        with open(tmp, "w") as fo:
            fo.write("ev," + ",".join(head) + "\n")
            fo.flush()
            z = subprocess.Popen(["zcat", f"{out}/digrows.csv.gz"], stdout=subprocess.PIPE)
            subprocess.run(["awk", "-F,", prog], stdin=z.stdout, stdout=fo, check=True)
            z.wait()
        os.replace(tmp, path)
    with open(path) as fh:
        return list(csv.DictReader(fh))


def soil_and_face(out, nx, gy, place, deaths):
    rows = dig_events(out)
    # Every cut, in order, per ant; the let-go and trip-end rows between them.
    by_id = defaultdict(list)
    for r in rows:
        by_id[r["id"]].append(r)
    m = Counter()
    carry_frames = []
    walks = Counter()
    walks_out = Counter()
    for i, evs in by_id.items():
        cut = None  # the open cut: (frame, cell, zone of cut, let-go row or None)
        for r in evs:
            f = int(r["frame"])
            if r["ev"] == "cut":
                cell = (int(r["dig_x"]), int(r["dig_y"]))
                if cut is not None:
                    pf, pcell, pnest, let = cut
                    if pnest and pf >= F0 and pf < F1:
                        key = ("cut at its face" if cheb(cell, pcell) <= 2 else "cut somewhere else")
                        walks[key] += 1
                        if let is not None and not let:
                            walks_out[key] += 1
                cut = [f, cell, cell[1] > gy, None]
            elif r["ev"] == "let_go" and cut is not None and cut[3] is None:
                cf, ccell, cnest, _ = cut
                hy = int(r["hy"])
                inside = hy > gy
                cut[3] = inside
                if F0 <= cf < F1:
                    if cnest:
                        m["nest pellets let go"] += 1
                        m["nest pellets let go in the nest"] += inside
                        m[f"nest pellets let go in the {place(int(r['hx']), hy)}"] += 1
                        m["nest pellets let go hungry (under half its start)"] += float(r["energy"]) < 0.5
                        carry_frames.append(f - cf)
                    else:
                        m["outside pellets let go"] += 1
                        m["outside pellets let go in the nest"] += inside
            elif r["ev"] == "end" and cut is not None:
                pf, pcell, pnest, let = cut
                ret = (int(r["ret_x"]), int(r["ret_y"])) if r["ret_x"] else None
                if pnest and F0 <= pf < F1 and ret == pcell:
                    walks[r["trip_end"]] += 1
                    if let is not None and not let:
                        walks_out[r["trip_end"]] += 1
                    cut = None
        if cut is not None:
            pf, pcell, pnest, let = cut
            if pnest and F0 <= pf < F1:
                key = "died" if i in deaths else "still walking at the end"
                walks[key] += 1
                if let is not None and not let:
                    walks_out[key] += 1
                if let is None:
                    m["nest pellets never let go"] += 1
                    m["nest pellets never let go, carrier died"] += i in deaths
    res = {
        "soil: nest pellets let go": m["nest pellets let go"],
        "soil: put back (let go in the nest)": share(m["nest pellets let go in the nest"], m["nest pellets let go"]),
        "soil: let go in the shaft": share(m["nest pellets let go in the shaft"], m["nest pellets let go"]),
        "soil: let go in the lane": share(m["nest pellets let go in the lane"], m["nest pellets let go"]),
        "soil: let go in the room": share(m["nest pellets let go in the room"], m["nest pellets let go"]),
        "soil: let go hungry": share(m["nest pellets let go hungry (under half its start)"], m["nest pellets let go"]),
        "soil: carried, median frames": med(carry_frames),
        "soil: carried 2,000+ frames": share(sum(c >= 2000 for c in carry_frames), len(carry_frames)),
        "soil: never let go (of nest cuts)": m["nest pellets never let go"],
        "soil: never let go, carrier died": m["nest pellets never let go, carrier died"],
        "soil: mound pellets let go in the nest": share(m["outside pellets let go in the nest"], m["outside pellets let go"]),
    }
    n = sum(walks.values())
    for k in ("cut at its face", "cut somewhere else", "arrived", "gave_up", "hungry", "food", "other", "mismatch", "died", "still walking at the end"):
        res[f"face: walk back ended {k}"] = share(walks[k], n)
    res["face: walks back"] = n
    no = sum(walks_out.values())
    res["face: soil carried out, walks back"] = no
    res["face: soil carried out, ended cut at its face"] = share(walks_out["cut at its face"], no)
    res["face: soil carried out, ended gave up"] = share(walks_out["gave_up"], no)
    return res


def cuts(out, nx, gy, place):
    seen = set()
    by_id = {}
    m = Counter()
    nxt = []
    with open(f"{out}/cuts.csv") as fh:
        for r in csv.DictReader(fh):
            f, i = int(r["frame"]), r["id"]
            cell = (int(r["x"]), int(r["y"]))
            nest = cell[1] > gy
            if F0 <= f < F1 and nest:
                m["nest cuts"] += 1
                m["nest cuts at new ground"] += cell not in seen
                m[f"nest cuts in the {place(*cell)}"] += 1
            if F0 <= f < F1 and not nest:
                m["cuts out"] += 1
            prev = by_id.get(i)
            if prev is not None and prev[2] and F0 <= prev[0] < F1:
                nxt.append(cheb(cell, prev[1]) <= 2)
            by_id[i] = (f, cell, nest)
            seen.add(cell)
    return {
        "dig: nest cuts": m["nest cuts"],
        "dig: nest cuts at new ground": m["nest cuts at new ground"],
        "dig: nest cuts that are re-digs": share(m["nest cuts"] - m["nest cuts at new ground"], m["nest cuts"]),
        "dig: cuts out of the nest (mound)": m["cuts out"],
        "face: next cut within 2 cells (of nest cuts)": share(sum(nxt), len(nxt)),
    }


def walk(out, nx, gy, place, start_j, starvers):
    """One pass over `walkrows.csv.gz`."""
    m = Counter()
    last = defaultdict(list)  # starvers in the nest: (frame, pull) over their last 3,000 frames
    with gzip.open(f"{out}/walkrows.csv.gz", "rt") as fh:
        col = {k: n for n, k in enumerate(next(fh).rstrip("\n").split(","))}
        if "nurse_w" not in col:
            sys.exit(f"{out}: walkrows.csv.gz has no nurse_w column (recorded before 2026-10-05)")
        F, I, HX, HY, E, LEG, PULL, NW = (col[k] for k in ("frame", "id", "hx", "hy", "e", "leg", "pull", "nurse_w"))
        for line in fh:
            a = line.rstrip("\n").split(",")
            f = int(a[F])
            i = a[I]
            d = starvers.get(i)
            if d is not None and d - 3000 <= f <= d:
                last[i].append(a[PULL])
            if not F0 <= f < F1:
                continue
            x, y = int(a[HX]), int(a[HY])
            pl = place(x, y)
            leg, pull = a[LEG], a[PULL]
            if pl != "out":
                if leg == "spoil":
                    m["spoil decisions in the nest"] += 1
                    if pull != "not scored":
                        m["spoil scored in the nest"] += 1
                        m[f"spoil pull {pull}"] += 1
                elif leg == "empty" and float(a[E]) < 1.0:
                    m["hungry empty decisions in the nest"] += 1
                    if pull != "not scored":
                        m["hungry empty scored in the nest"] += 1
                        m[f"hungry pull {pull}"] += 1
            if leg == "laden":
                m["laden decisions"] += 1
                m[f"laden in the {pl}"] += 1
                if pl in ("lane", "room") and pull != "not scored":
                    m["laden scored below the shaft"] += 1
                    m["laden scored below the shaft, nurse term"] += a[NW] != ""
                    m["laden scored below the shaft, no pull"] += pull == "none"
    ss, hs = m["spoil scored in the nest"], m["hungry empty scored in the nest"]
    res = {
        "soil: carrier decisions in the nest": m["spoil decisions in the nest"],
        "soil: ...pulled by the soil way out": share(m["spoil pull soil way out"], ss),
        "soil: ...pulled by the straight haul": share(m["spoil pull spoil haul"], ss),
        "soil: ...no pull": share(m["spoil pull none"], ss),
        "way: hungry empty decisions in the nest": m["hungry empty decisions in the nest"],
        "way: ...pulled out (hungry out)": share(m["hungry pull hungry out"], hs),
        "way: ...no pull": share(m["hungry pull none"], hs),
        "crop: laden decisions (nest region)": m["laden decisions"],
    }
    for pl in ("out", "shaft", "lane", "room"):
        res[f"crop: laden decisions in the {pl}"] = share(m[f"laden in the {pl}"], m["laden decisions"])
    lb = m["laden scored below the shaft"]
    res["crop: laden below the shaft, steered by larva scent"] = share(m["laden scored below the shaft, nurse term"], lb)
    res["crop: laden below the shaft, no pull"] = share(m["laden scored below the shaft, no pull"], lb)
    tail = Counter()
    for pulls in last.values():
        tail.update(pulls)
    tn = sum(v for k, v in tail.items() if k != "not scored")
    res["deaths: nest starvers' last 3,000 frames, scored decisions"] = tn
    for k in ("none", "hungry out", "laden home", "spoil haul", "soil way out", "back to the face", "nest worker leash"):
        res[f"deaths: ...{k}"] = share(tail[k], tn)
    return res


def brood(out, nx, gy, place):
    res = {}
    # Larvae present, per census (every 1,000 frames), by place.
    lv = Counter()
    frames = set()
    with open(f"{out}/brood.csv") as fh:
        for r in csv.DictReader(fh):
            f = int(r["frame"])
            if F0 <= f < F1:
                frames.add(f)
                if r["stage"] == "larva":
                    lv[place(int(r["x"]), int(r["y"]))] += 1
    n = max(len(frames), 1)
    fed = Counter()
    with open(f"{out}/feeds.csv") as fh:
        for r in csv.DictReader(fh):
            f = int(r["frame"])
            if F0 <= f < F1:
                pl = place(int(r["x"]), int(r["y"]))
                fed[(pl, r["kind"])] += float(r["gain"])
                fed[(pl, "all")] += float(r["gain"])
    span = (F1 - F0) / 1000.0
    for pl in ("shaft", "lane", "room"):
        per = lv[pl] / n
        res[f"brood: larvae in the {pl} (per census)"] = round(per, 1)
        tot = fed[(pl, "all")]
        res[f"brood: {pl} larvae fed, J per larva per 1,000 frames"] = round(tot / span / per, 1) if per else None
        for k in ("crop", "bank", "share", "ate"):
            res[f"brood: {pl} larvae fed from {k}, J per larva per 1,000 frames"] = round(fed[(pl, k)] / span / per, 1) if per else None
    res["brood: crop food to larvae, kJ (all places)"] = round(sum(v for (pl, k), v in fed.items() if k == "crop") / 1000, 1)
    res["brood: crop food to larvae below the shaft, kJ"] = round((fed[("lane", "crop")] + fed[("room", "crop")]) / 1000, 1)
    # Eggs laid and larvae starved, by where the larva lay when it died. A
    # walker lifts brood and puts it back (PUSH_PAST), so the log shows it
    # `gone` and then seen again (an egg seen again is named `laid`): an egg
    # is counted once, and a larva starved is one whose last word is `gone`
    # -- a starved larva leaves an empty cell (its bank under 1 J leaves no
    # corpse) and is never seen again. Checked against the world's own
    # `larvae_starved` in `stats.csv`.
    laid = 0
    starved = Counter()
    stage = {}
    last = {}
    end = 0
    with open(f"{out}/broodlog.csv") as fh:
        for r in csv.DictReader(fh):
            f = int(r["frame"])
            i = r["id"]
            end = f
            if r["event"] == "laid" and i not in stage and F0 <= f < F1:
                laid += 1
            if r["event"] in ("laid", "seen", "moved", "stage"):
                stage[i] = r["stage"]
            last[i] = (r["event"], f, int(r["x"]), int(r["y"]), stage.get(i))
    for ev, f, x, y, st in last.values():
        if ev == "gone" and st == "larva" and F0 <= f < F1 and f < end:
            starved[place(x, y)] += 1
    res["brood: eggs laid"] = laid
    res["brood: larvae starved per egg laid"] = share(sum(starved.values()), laid)
    for pl in ("shaft", "lane", "room"):
        res[f"brood: larvae starved in the {pl}, per larva present"] = share(starved[pl], lv[pl] / n) if lv[pl] else None
    return res


def colony(out, nx, gy, place, start_j):
    res = {}
    ants = Counter()
    worker = {}
    fed_heads = defaultdict(list)
    larvae = defaultdict(list)
    with open(f"{out}/colony.csv") as fh:
        for r in csv.DictReader(fh):
            f = int(r["frame"])
            worker[r["id"]] = r["worker"] == "1"
            if F0 <= f < F1:
                ants[f] += 1
                if float(r["energy_j"]) > start_j:
                    fed_heads[f].append((int(r["hx"]), int(r["hy"]), int(r["crop_cells"]) > 0))
    with open(f"{out}/brood.csv") as fh:
        for r in csv.DictReader(fh):
            f = int(r["frame"])
            if F0 <= f < F1 and r["stage"] == "larva":
                larvae[f].append((int(r["x"]), int(r["y"])))
    res["colony: mean adults"] = round(sum(ants.values()) / max(len(ants), 1), 1)
    near, near_c, lvn = Counter(), Counter(), Counter()
    for f, ls in larvae.items():
        heads = {(x, y) for x, y, _ in fed_heads.get(f, [])}
        laden = {(x, y) for x, y, c in fed_heads.get(f, []) if c}
        for x, y in ls:
            pl = place(x, y)
            lvn[pl] += 1
            box = [(x + dx, y + dy) for dx in range(-2, 3) for dy in range(-2, 3)]
            near[pl] += any(p in heads for p in box)
            near_c[pl] += any(p in laden for p in box)
    for pl in ("shaft", "lane", "room"):
        res[f"brood: {pl} larvae with a fed ant's head within 2 cells"] = share(near[pl], lvn[pl])
        res[f"brood: {pl} larvae with a fed crop carrier within 2 cells"] = share(near_c[pl], lvn[pl])
    return res, worker


def deaths(out, nx, gy, place, worker):
    died = {}
    rows = []
    pat = re.compile(r"^(\d+) DIED census=1 id=(\d+) age=(\d+) at=\((-?\d+),(-?\d+)\) zone=(\S+) energy=(\S+) crop_cells=(\d+) cause=(\S+)")
    for line in open(f"{out}/events.txt"):
        mt = pat.match(line)
        if not mt:
            continue
        f, i, age, x, y, zone, e, crop, cause = mt.groups()
        f = int(f)
        died[i] = f
        if F0 <= f < F1:
            rows.append((i, int(age), int(x), int(y), cause, int(crop), worker.get(i)))
    res = {"deaths: adults died": len(rows)}
    by = Counter()
    for i, age, x, y, cause, crop, wk in rows:
        by[cause] += 1
        if cause == "STARVED":
            by[f"starved in the {place(x, y)}"] += 1
            by["starved, nest workers"] += wk is True
            by["starved with crop food"] += crop > 0
    for k in ("STARVED", "OLD", "KILLED"):
        res[f"deaths: {k.lower()}"] = sum(v for c, v in by.items() if c.startswith(k))
    for pl in ("out", "shaft", "lane", "room"):
        res[f"deaths: starved in the {pl}"] = by[f"starved in the {pl}"]
    res["deaths: starved, nest workers"] = by["starved, nest workers"]
    res["deaths: starved with crop food in it"] = by["starved with crop food"]
    starvers = {i: died[i] for i, age, x, y, cause, crop, wk in rows if cause == "STARVED" and y > gy}
    return res, died, starvers


def stats(out):
    """`stats.csv`'s running totals over the span: the world's own count of
    births and deaths, the positive control for the census lines."""
    rows = list(csv.DictReader(open(f"{out}/stats.csv")))
    a = next((r for r in rows if int(r["frame"]) >= F0), None)
    b = next((r for r in reversed(rows) if int(r["frame"]) < F1), None)
    if a is None or b is None:
        return {}
    d = lambda k: float(b[k]) - float(a[k])
    res = {
        "colony: eggs laid (world count)": int(d("eggs_laid")),
        "colony: pupae": int(d("pupae")),
        "colony: adults born": int(d("births")),
        "colony: larvae starved (world count)": int(d("larvae_starved")),
        "colony: adults starved (world count)": int(d("died_starved") + d("died_starved_aloft")),
        "colony: adults died of old age (world count)": int(d("died_old_age")),
        "colony: mean brood": round(sum(int(r["brood"]) for r in rows if F0 <= int(r["frame"]) < F1) / max(1, sum(F0 <= int(r["frame"]) < F1 for r in rows)), 1),
    }
    for k in ("ate", "crop_fed", "nursed", "shared"):
        res[f"colony: larva food from {k.replace('_fed', '')}, kJ"] = round(d(f"brood_{k}_j") / 1000, 1)
    # The heap the harness keeps topped up: what it put back is what the
    # colony took, read off the run's own progress lines (`OUT.log`, if the
    # run's output was saved beside it).
    log = out.rstrip("/") + ".log"
    if os.path.exists(log):
        took = {}
        for line in open(log):
            mt = re.match(r"frame=(\d+) .*food_dropped=(\d+)", line)
            if mt:
                took[int(mt.group(1))] = int(mt.group(2))
        a = max((f for f in took if f <= F0), default=None)
        b = max((f for f in took if f < F1), default=None)
        if a is not None and b is not None and b > a:
            res["colony: food taken from the heap, cells"] = took[b] - took[a]
    return res


def summary(out):
    path = f"{out}/plan.json" if (F0, F1) == (100000, 200001) else f"{out}/plan-{F0}-{F1 - 1}.json"
    if os.path.exists(path):
        return json.load(open(path))
    nx, gy, start_j = geo(out)
    place = placer(nx, gy)
    res = {}
    col, worker = colony(out, nx, gy, place, start_j)
    d, died, starvers = deaths(out, nx, gy, place, worker)
    res.update(cuts(out, nx, gy, place))
    res.update(soil_and_face(out, nx, gy, place, died))
    res.update(walk(out, nx, gy, place, start_j, starvers))
    res.update(brood(out, nx, gy, place))
    res.update(col)
    res.update(d)
    res.update(stats(out))
    res["_shares"] = [k for k, v in res.items() if isinstance(v, Share)]
    json.dump(res, open(path, "w"), indent=1)
    return res


def fmt(v, is_share):
    if v is None:
        return "-"
    if is_share:
        return f"{100 * v:.1f}%" if 0 < v < 0.1 else f"{100 * v:.0f}%"
    if isinstance(v, float):
        return f"{v:.1f}" if abs(v) < 100 else f"{v:.0f}"
    return str(v)


def main():
    global F0, F1
    args = sys.argv[1:]
    as_json = "--json" in args
    win = next((a for a in args if a.startswith("--window=")), None)
    if win:
        a, b = win.split("=", 1)[1].split("-")
        F0, F1 = int(a), int(b) + 1
    outs = [a for a in args if not a.startswith("--")]
    if not outs:
        sys.exit(__doc__)
    runs = [(os.path.basename(o.rstrip("/")), summary(o)) for o in outs]
    if as_json:
        print(json.dumps(dict(runs), indent=1))
        return
    keys = []
    for _, r in runs:
        keys += [k for k in r if k not in keys and k != "_shares"]
    shares = {k for _, r in runs for k in r.get("_shares", [])}
    names = [n for n, _ in runs]
    w = max(len(k) for k in keys)
    print(" " * w + "  " + "  ".join(f"{n:>11}" for n in names))
    for k in keys:
        print(f"{k:<{w}}  " + "  ".join(f"{fmt(r.get(k), k in shares):>11}" for _, r in runs))


if __name__ == "__main__":
    main()
