#!/usr/bin/env python3
"""broodstep.py RUN... : do ants step into the brood under the door as often as into open ground?

Reads `deeptrace walk=1` runs (2026-10-06 recorder, deeptrace-walk-cells.patch): walkrows.csv.gz's
`nb`/`nbn` columns say what stood in each of the 8 cells round the head (DIRS order E NE N NW W SW S SE,
then the head's own cell) before the tick, and how many ants stood in each: b brood, B an ant standing
on brood, . empty, a an ant on open ground, c/C crumbs, s/# ground, f/x food/corpse. `opts` is the
bitmask of steps the chooser offered (enterable and footed), `chose` the one it picked, s0..s7 its
scores and `k` its looseness; the pick is choose_weighted, P = (k + max(s,0))^2 / sum over offers,
so every take rate is printed beside what the scores predict.

Prints markdown: ants under the ground line / beside brood / standing on brood in the door column /
at the top of the pile (standing on open ground with brood in one of the 3 cells below); what the
top ants do; per offer by direction and cell (all, door column x 253-260, off it); brood-below vs
open-below within one decision; cells below not offered and why; and the pull acting at the top.
Energy bands: full = at or over the start grant, under grant = half to full, lean = under half.

  RAYON_NUM_THREADS=1 deeptrace scenario=nest_goal seed=S frames=130000 founder=evolved ants=0 \
      mapevery=1000 hungry=1 dig=1 walk=1 digfrom=100000 out=RUN
  python3 broodstep.py RUN1 RUN2 ...
"""
import gzip, sys, os, collections, statistics

DIRS = [(1, 0), (1, -1), (0, -1), (-1, -1), (-1, 0), (-1, 1), (0, 1), (1, 1)]
GROUP = {0: "side", 4: "side", 1: "up-diag", 3: "up-diag", 2: "up", 5: "down-diag", 7: "down-diag", 6: "down"}
GROUPS = ["down", "down-diag", "side", "up-diag", "up"]
DOOR = (253, 260)
LEAN = 0.5


def cls(ch):
    return {"b": "brood", "B": "brood+ant", ".": "open", "a": "open+ant", "c": "crumbs", "C": "crumbs"}.get(ch, "wall")


def two(ch):
    c = cls(ch)
    return "brood" if c.startswith("brood") else ("open" if c.startswith("open") else c)


def founded(run):
    for line in open(os.path.join(run, "events.txt")):
        if "FOUNDED" in line:
            kv = dict(t.split("=") for t in line.split() if "=" in t)
            return int(kv["nest_x"]), int(kv["ground_y"])


def alive(run, a, b):
    per = collections.Counter()
    with open(os.path.join(run, "colony.csv")) as f:
        f.readline()
        for line in f:
            fr = int(line.split(",", 1)[0])
            if a <= fr < b:
                per[fr] += 1
    return statistics.mean(per.values()) if per else float("nan")


def energy_band(e):
    return "full" if e >= 1.0 else ("under grant" if e >= LEAN else "lean")


def score(run):
    nx, gy = founded(run)
    S = collections.Counter()  # scalar counts
    off = collections.defaultdict(lambda: [0, 0, 0.0])  # (subset, group, class) -> offered, taken, expected
    pair = collections.defaultdict(lambda: [0, 0, 0, 0.0, 0.0])  # subset -> decisions, took brood, took open, exp brood, exp open
    refused = collections.Counter()  # (class, ants) not offered, below the head, top of the pile
    offered_below = collections.Counter()
    steps = collections.Counter()  # (subset, group of the step taken)
    pulls = collections.Counter()  # (pull, band) at the top, every decision
    pull_steps = collections.Counter()  # (pull, group) of chooser picks at the top
    gaps = []
    last = {}
    frames = [None, None]
    depth = collections.Counter()  # rows below the ground line of decisions on brood in the door column
    with gzip.open(os.path.join(run, "walkrows.csv.gz"), "rt") as f:
        head = f.readline().rstrip("\n").split(",")
        ix = {h: i for i, h in enumerate(head)}
        for line in f:
            r = line.rstrip("\n").split(",")
            fr = int(r[0])
            frames[0] = fr if frames[0] is None else min(frames[0], fr)
            frames[1] = fr if frames[1] is None else max(frames[1], fr)
            hx, hy = int(r[ix["hx"]]), int(r[ix["hy"]])
            idn = r[ix["id"]]
            if idn in last and len(gaps) < 200000:
                gaps.append(fr - last[idn])
            last[idn] = fr
            if hy < gy:
                continue
            S["nest"] += 1
            nb = r[ix["nb"]]
            nbn = r[ix["nbn"]]
            indoor = DOOR[0] <= hx <= DOOR[1]
            on = two(nb[8])
            if on == "brood":
                S["on brood"] += 1
                if indoor:
                    S["on brood, door"] += 1
                    depth[hy - gy] += 1
            br = [two(nb[d]) == "brood" for d in range(8)]
            if not any(br):
                continue
            S["beside"] += 1
            S["beside, door"] += indoor
            top = on != "brood" and (br[5] or br[6] or br[7])
            if not top:
                continue
            S["top"] += 1
            S["top, door"] += indoor
            outcome = r[ix["outcome"]]
            S["top:" + ("stood" if outcome == "roll_failed_idle" else ("stepped" if outcome == "stepped" else "other"))] += 1
            e = float(r[ix["e"]]) if r[ix["e"]] else 1.0
            band = energy_band(e)
            pull = r[ix["pull"]]
            pulls[(pull, band)] += 1
            chose = r[ix["chose"]]
            if pull == "not scored" or chose == "":
                continue
            S["top chooser"] += 1
            chose = int(chose)
            opts = int(r[ix["opts"]])
            k = float(r[ix["k"]])
            sc = {}
            for d in range(8):
                if opts >> d & 1:
                    v = r[ix[f"s{d}"]]
                    sc[d] = float(v) if v not in ("", "NaN", "nan") else 0.0
            w = {d: (k + max(s, 0.0)) ** 2 for d, s in sc.items()}
            tot = sum(w.values()) or 1.0
            p = {d: w[d] / tot for d in w}
            subs = ["all", "door" if indoor else "off-door", band]
            for sub in subs:
                steps[(sub, GROUP[chose])] += 1
                if GROUP[chose] in ("down", "down-diag"):
                    steps[(sub, "below:" + two(nb[chose]))] += 1
            pull_steps[(pull, GROUP[chose])] += 1
            for d in range(8):
                c = cls(nb[d])
                if c in ("wall", "crumbs"):
                    continue
                if opts >> d & 1:
                    for sub in subs:
                        a = off[(sub, GROUP[d], c)]
                        a[0] += 1
                        a[1] += chose == d
                        a[2] += p[d]
                    if d in (5, 6, 7):
                        offered_below[c] += 1
                elif d in (5, 6, 7):
                    refused[(c, nbn[d])] += 1
            downs = [d for d in (5, 6, 7) if opts >> d & 1]
            bd = [d for d in downs if two(nb[d]) == "brood"]
            od = [d for d in downs if two(nb[d]) == "open"]
            if bd and od:
                for sub in subs + ["pull:" + pull]:
                    q = pair[sub]
                    q[0] += 1
                    q[1] += chose in bd
                    q[2] += chose in od
                    q[3] += sum(p[d] for d in bd)
                    q[4] += sum(p[d] for d in od)
    span = frames[1] - frames[0] + 1
    gap = statistics.median(gaps) if gaps else 5
    return dict(run=run, S=S, off=off, pair=pair, refused=refused, offered_below=offered_below, steps=steps, pulls=pulls,
                pull_steps=pull_steps, span=span, gap=gap, frames=frames, alive=alive(run, frames[0], frames[1] + 1), depth=depth)


def at_a_time(n, R):
    return n * R["gap"] / R["span"]


def main():
    runs = sys.argv[1:]
    Rs = [score(r) for r in runs]
    names = [os.path.basename(r.rstrip("/")) for r in runs]
    out = []
    P = out.append
    R0 = Rs[0]
    P(f"Window: frames {R0['frames'][0]:,}-{R0['frames'][1]:,}; one decision per ant every {R0['gap']} frames (median gap), so ants at a time = decisions x {R0['gap']} / {R0['span']:,} frames.")
    P("")
    P("| seed | ants alive | under the ground line | beside brood | standing on brood (door column) | at the top of the pile |")
    P("|---|---|---|---|---|---|")
    for n, R in zip(names, Rs):
        S = R["S"]
        P(f"| {n} | {R['alive']:.0f} | {at_a_time(S['nest'], R):.1f} | {at_a_time(S['beside'], R):.1f} | {at_a_time(S['on brood, door'], R):.1f} | {at_a_time(S['top'], R):.1f} |")
    P("")
    P("Rows below the ground line of ants standing on brood in the door column (share of their decisions):")
    for n, R in zip(names, Rs):
        d = R["depth"]
        t = sum(d.values()) or 1
        bands = [(0, 2), (3, 5), (6, 10), (11, 99)]
        P(f"- {n}: " + ", ".join(f"rows {a}-{b if b < 99 else 'deeper'} {sum(v for k_, v in d.items() if a <= k_ <= b)/t:.0%}" for a, b in bands))
    P("")
    P("What an ant at the top of the pile does with a decision (share of its decisions):")
    P("")
    P("| seed | stands still | steps | of the steps: down | down-diagonal | sideways | up-diagonal | up | steps below into brood / into open |")
    P("|---|---|---|---|---|---|---|---|---|")
    for n, R in zip(names, Rs):
        S, st = R["S"], R["steps"]
        t = S["top"] or 1
        ch = sum(st[("all", g)] for g in GROUPS) or 1
        P(f"| {n} | {S['top:stood']/t:.0%} | {S['top:stepped']/t:.0%} | " + " | ".join(f"{st[('all', g)]/ch:.0%}" for g in GROUPS)
          + f" | {st[('all', 'below:brood')]:,} / {st[('all', 'below:open')]:,} |")
    P("")
    for sub, title in (("all", "all"), ("door", "door column only"), ("off-door", "off the door column")):
        P(f"Per offer at the top of the pile ({title}): taken per offer, with what the chooser's own scores predict in brackets; offers in the last column.")
        P("")
        P("| step | into | " + " | ".join(names) + " | offers (all seeds) |")
        P("|---|---|" + "---|" * len(names) + "---|")
        for g in ("down", "down-diag", "side", "up-diag", "up"):
            for c, label in (("brood", "brood"), ("brood+ant", "brood with an ant on it"), ("open", "open, empty"), ("open+ant", "open, an ant in it")):
                cells = []
                tot = 0
                for R in Rs:
                    a = R["off"].get((sub, g, c))
                    if not a or a[0] < 30:
                        cells.append("-")
                    else:
                        cells.append(f"{a[1]/a[0]:.0%} ({a[2]/a[0]:.0%})")
                    tot += a[0] if a else 0
                P(f"| {g} | {label} | " + " | ".join(cells) + f" | {tot:,} |")
        P("")
    P("Within one decision, when a brood cell below and an open cell below were both offered: took brood below / open below / something else (predicted from the scores); and brood's share of the cells below offered.")
    P("")
    for sub in ("all", "door", "off-door", "full", "under grant", "lean", "pull:hungry out", "pull:soil way out", "pull:none", "pull:back to the face"):
        parts = []
        for n, R in zip(names, Rs):
            q = R["pair"].get(sub)
            if not q or q[0] < 40:
                parts.append(f"{n} -")
                continue
            parts.append(f"{n} {q[1]/q[0]:.0%} / {q[2]/q[0]:.0%} / {1-(q[1]+q[2])/q[0]:.0%} ({q[3]/q[0]:.0%} / {q[4]/q[0]:.0%}; n {q[0]:,})")
        P(f"- {sub}: " + "; ".join(parts))
    P("")
    P("Cells below an ant at the top of the pile that the chooser did NOT offer, by what was in them (all seeds):")
    ref = collections.Counter()
    offb = collections.Counter()
    for R in Rs:
        ref.update(R["refused"])
        offb.update(R["offered_below"])
    for c, label in (("brood", "brood"), ("brood+ant", "brood with ants on it"), ("open", "open, empty"), ("open+ant", "open, ants in it")):
        nref = sum(v for (cc, _), v in ref.items() if cc == c)
        by = ", ".join(f"{k_[1]} ants {v:,}" for k_, v in sorted(ref.items()) if k_[0] == c)
        P(f"- {label}: offered {offb[c]:,}, not offered {nref:,} ({nref/max(offb[c]+nref,1):.0%}){'; by ants in the cell: ' + by if by else ''}")
    P("")
    P("Which pull was acting at the top of the pile (share of all its decisions, all seeds), and where the chooser's steps under it went (down+down-diagonal / sideways / up+up-diagonal):")
    pl = collections.Counter()
    ps = collections.Counter()
    for R in Rs:
        pl.update(R["pulls"])
        ps.update(R["pull_steps"])
    tot = sum(pl.values())
    bypull = collections.Counter()
    for (pull, band), v in pl.items():
        bypull[pull] += v
    for pull, v in bypull.most_common():
        if v / tot < 0.005:
            continue
        bands = ", ".join(f"{b} {pl[(pull, b)]/v:.0%}" for b in ("full", "under grant", "lean") if pl[(pull, b)])
        n_ = sum(ps[(pull, g)] for g in GROUPS)
        if n_:
            dn = (ps[(pull, "down")] + ps[(pull, "down-diag")]) / n_
            sd = ps[(pull, "side")] / n_
            up = (ps[(pull, "up")] + ps[(pull, "up-diag")]) / n_
            where = f"; steps {dn:.0%} / {sd:.0%} / {up:.0%} of {n_:,}"
        else:
            where = " (no chooser: the ant stood still)" if pull == "not scored" else ""
        P(f"- {pull}: {v/tot:.0%} ({bands}){where}")
    print("\n".join(out))


if __name__ == "__main__":
    main()
