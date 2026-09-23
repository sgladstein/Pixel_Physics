#!/usr/bin/env python3
"""Read `trailfollow decisioncsv` files: where each leg spends its decisions,
where it stalls, and why the homeward re-roll did or did not fire.

The rows are `creature::DecisionRow`, written by the engine at the moment it
decided, so unlike `ladencsv` nothing here is inferred from positions: the
outcome column says which branch the decision took, and `homeward` says which
gate of `home_weighted_pick_why` refused a re-roll. The harness has already
asserted that the rows agree with the engine's census and its per-verb
counters (`trailfollow.rs`, "The run checks its own trace"); this script is
the reading, not the check.

    python3 scripts/decisioncensus.py DIR [DIR|FILE ...] [--gap 90]
    python3 scripts/decisioncensus.py --selftest

**It prints the key's cardinality first** -- files, distinct (seed, gap, arm,
tag) keys, rows -- and refuses a second file carrying a key already read,
because `trailfollow` sweeps gaps and arms as well as seeds and a parse keyed
on fewer dimensions pools them, last write wins (`CLAUDE.md`).

**Shares are printed two ways**: pooled over every decision, and the median
over runs of each run's own share. They differ when runs differ in size, and
a pooled share is then weighted by whichever runs had more decisions -- the
per-run median is the one to compare across arms.

Sections:

1. **Where decisions are spent** -- per leg, the share in each setting
   (pocket / corridor / junction / open, by usable-heading count), and in
   each setting the outcome mix: stepped, fell, tumbled after a failed roll,
   idle, blocked.
2. **Stalls** -- a stall is a maximal run of one ant's consecutive decisions
   in which the head did not relocate. Per leg: how much of the leg's time is
   spent in stalls of 10+ decisions, where those long stalls start (setting,
   distance from the nest), and what they are made of (share of decisions at
   `p_move` exactly 0, idle against tumbled).
3. **The homeward re-roll** -- per leg, every tumble's `homeward` reason, and
   for the ones that fired, the sign of the cosine between the chosen heading
   and home. A re-roll that fires but can only choose a heading pointing away
   is what `ant-movement-plan-2026-09-22.md` §2a predicts for the long tail.
4. **What sets `p_move`** -- `Move`'s pre-squash sum by term.
5. **The drop (C2)** -- per leg, every drop roll by outcome; how many won rolls
   found no room; and for the rolls that found none and the ones that placed,
   what the eight neighbours were: the ant's own body, another organism, or
   the material by name.
6. **The cone (C3)** -- per leg and setting, which of the three forward
   candidates each step took, how often a side candidate was open at all,
   whether the side steps went up, level or down (left and right are
   relative to the heading, so they mix the two), and how many `Turn`
   requests there were and how many were discarded.

Sections 5 and 6 need the columns step 2 added; older files skip them.
"""
import argparse
import csv
import statistics
import sys
import tempfile
from collections import Counter, defaultdict
from pathlib import Path

LEGS = ["empty", "laden", "spoil"]
SETTINGS = ["pocket", "corridor", "junction", "open"]
OUTCOME_GROUPS = [
    ("stepped", {"stepped"}),
    ("fell", {"fell"}),
    ("tumbled", {"roll_failed_tumbled"}),
    ("idle", {"roll_failed_idle"}),
    ("blocked", {"blocked_tumbled", "reversed", "crossing", "swapped"}),
    ("other", {"launched", "no_body"}),
]
RELOCATING = {"stepped", "fell", "swapped", "reversed"}
LONG_STALL = 10
DIST_BANDS = [(0, 9), (10, 29), (30, 59), (60, 10**9)]


def band(d):
    for lo, hi in DIST_BANDS:
        if lo <= d <= hi:
            return f"{lo}-{hi}" if hi < 10**9 else f"{lo}+"
    return "?"


def squash(x):
    return x / (1.0 + abs(x))


def invsquash(y):
    y = max(min(y, 0.999999), -0.999999)
    return y / (1.0 - abs(y))


# **`Move`'s pre-squash sum, term by term, at the founder genome's weights**
# (`how-the-ant-works.md` §4). The rows carry the raw `Move` output, so the
# true sum is recovered exactly by inverting `squash`; whatever these terms do
# not account for -- mutated weights in descendants, or an input the ant wires
# that this table does not name -- lands in `residual`, which is printed so a
# decomposition that has stopped describing the ant says so.
DIRECT = [("bias", None, 2.0), ("energy", "energy", -1.75), ("home_aligned", "home_aligned", 3.0),
          ("stillness", "stillness", 1.5), ("kin_need", "kin_need", 1.25), ("food_adjacent", "food_adjacent", -1.16),
          ("crowding", "crowding", -0.3)]
# `(Alarm, Move, -1.0)` is wired too, but the rows do not carry `Alarm`, so it
# lands in `residual` -- which is therefore not zero on a bed with a fight.


def move_terms(r):
    cf = 1.0 if r["leg"] == "laden" else 0.0
    out = {}
    for name, col, w in DIRECT:
        if name == "bias":
            out[name] = w
        elif col:
            out[name] = w * float(r[col])
    a, b = float(r["along_a"]), float(r["along_b"])
    out["trail_A (units 0-1)"] = 2.5 * (squash(-45 + 45.5 * cf + 6 * a) - squash(-45 + 45.5 * cf - 6 * a))
    out["trail_B (units 2-3)"] = 2.5 * (squash(0.5 - 45.5 * cf + 6 * b) - squash(0.5 - 45.5 * cf - 6 * b))
    total = invsquash(float(r["move_out"]))
    out["residual"] = total - sum(out.values())
    out["= Move sum"] = total
    return out


def files_of(paths):
    out = []
    for p in paths:
        p = Path(p)
        if p.is_dir():
            out.extend(sorted(p.glob("trailfollow-decisions-*.csv")))
        else:
            out.append(p)
    return out


class Census:
    def __init__(self):
        self.keys = set()
        self.rows = 0
        # (key) -> leg -> setting -> Counter(outcome group)
        self.cells = defaultdict(lambda: defaultdict(lambda: defaultdict(Counter)))
        self.stall_decisions = defaultdict(Counter)  # key -> leg -> decisions in long stalls
        self.leg_decisions = defaultdict(Counter)  # key -> leg -> decisions
        self.long_stalls = []  # (leg, setting at start, dist band, length, zero_pmove, idle, tumbled)
        self.stall_lengths = defaultdict(list)  # leg -> lengths
        self.homeward = defaultdict(Counter)  # leg -> reason
        self.fired_cos = defaultdict(Counter)  # leg -> sign
        # leg -> "zero"/"live" -> term -> [sum, n]
        self.terms = defaultdict(lambda: defaultdict(lambda: defaultdict(lambda: [0.0, 0])))
        self.has_step2 = False
        self.drops = defaultdict(Counter)  # leg -> drop outcome
        self.drop_free8 = defaultdict(Counter)  # outcome -> free8
        self.drop_nbr = defaultdict(Counter)  # outcome -> neighbour kind/material -> cells
        self.drop_where = defaultdict(Counter)  # outcome -> "on anchor"/"off anchor"
        self.picks = defaultdict(lambda: defaultdict(Counter))  # leg -> setting -> pick
        self.side_open = defaultdict(lambda: defaultdict(Counter))  # leg -> setting -> number of open sides (0/1/2)
        self.turns = defaultdict(Counter)  # leg -> "requests"/"discarded"
        self.side_dir = defaultdict(lambda: defaultdict(Counter))  # leg -> setting -> up/level/down, side picks only

    def add_file(self, path):
        with open(path, newline="") as fh:
            rows = list(csv.DictReader(fh))
        if not rows:
            return
        keys = {(r["seed"], r["gap"], r["arm"], r["tag"]) for r in rows}
        if len(keys) != 1:
            sys.exit(f"decisioncensus: {path} carries {len(keys)} keys; one file is one run")
        key = keys.pop()
        if key in self.keys:
            sys.exit(f"decisioncensus: key {key} already read from another file -- two runs would pool")
        self.keys.add(key)
        self.rows += len(rows)
        by_ant = defaultdict(list)
        for r in rows:
            leg, setting, outcome = r["leg"], r["setting"], r["outcome"]
            group = next(g for g, members in OUTCOME_GROUPS if outcome in members)
            self.cells[key][leg][setting][group] += 1
            self.leg_decisions[key][leg] += 1
            by_ant[r["id"]].append(r)
            bucket = "zero" if float(r["p_move"]) == 0.0 else "live"
            for name, v in move_terms(r).items():
                acc = self.terms[leg][bucket][name]
                acc[0] += v
                acc[1] += 1
            if "drop" in r:
                self.has_step2 = True
                self.add_step2(r)
            if outcome in ("roll_failed_tumbled", "blocked_tumbled"):
                self.homeward[leg][r["homeward"]] += 1
                if r["homeward"] in ("fired", "fired_haul"):
                    c = float(r["home_cos"])
                    self.fired_cos[leg]["toward (>0.01)" if c > 0.01 else "away (<-0.01)" if c < -0.01 else "perpendicular"] += 1
        for rs in by_ant.values():
            rs.sort(key=lambda r: int(r["frame"]))
            run = []
            for r in rs + [None]:
                # A stall ends at a relocation, and also when the leg changes:
                # an ant that picks up food while standing still is two
                # stalls, one per leg, or the laden leg is charged for time
                # it spent empty.
                relocated = r is None or r["outcome"] in RELOCATING
                if not relocated and (not run or run[-1]["leg"] == r["leg"]):
                    run.append(r)
                    continue
                if run:
                    leg = run[0]["leg"]
                    self.stall_lengths[leg].append(len(run))
                    if len(run) >= LONG_STALL:
                        self.stall_decisions[key][leg] += len(run)
                        dist = abs(int(run[0]["x"]) - int(run[0]["nest_x"]))
                        zero = sum(1 for x in run if float(x["p_move"]) == 0.0)
                        idle = sum(1 for x in run if x["outcome"] == "roll_failed_idle")
                        tumbled = sum(1 for x in run if x["outcome"] in ("roll_failed_tumbled", "blocked_tumbled"))
                        self.long_stalls.append((leg, run[0]["setting"], band(dist), len(run), zero, idle, tumbled))
                    run = []
                if r is not None and r["outcome"] not in RELOCATING:
                    run.append(r)

    def add_step2(self, r):
        leg, why = r["leg"], r["drop"]
        if why != "not_asked":
            self.drops[leg][why] += 1
            self.drop_free8[why][int(r["free8"])] += 1
            on_anchor = (r["x"], r["y"]) == (r["ax"], r["ay"])
            self.drop_where[why]["on the anchor" if on_anchor else "off the anchor"] += 1
            mine, other = int(r["nbr_self"]), int(r["nbr_other"])
            for i, col in enumerate(NBR_COLS):
                if mine >> i & 1:
                    kind = "(own body)"
                elif other >> i & 1:
                    kind = f"(organism) {r[col]}"
                else:
                    kind = r[col]
                self.drop_nbr[why][kind] += 1
        if r["pick"] != "-":
            setting = r["setting"]
            self.picks[leg][setting][r["pick"]] += 1
            sides = sum(1 for c in ("cone_l", "cone_r") if float(r[c]) > 0.0)
            self.side_open[leg][setting][sides] += 1
            if r["pick"] != "straight":
                # Left and right are relative to the heading; in side view
                # the question is whether the step went up or down.
                dy = DIRS[int(r["heading2"])][1]
                self.side_dir[leg][setting]["up" if dy < 0 else "down" if dy > 0 else "level"] += 1
            t = float(r["turn"])
            if t != 0.0:
                self.turns[leg]["requests"] += 1
                if float(r["cone_l" if t > 0 else "cone_r"]) == 0.0:
                    self.turns[leg]["discarded"] += 1

    def report(self, out=sys.stdout):
        p = lambda *a: print(*a, file=out)
        p(f"decisioncensus: {len(self.keys)} runs (distinct seed/gap/arm/tag keys), {self.rows} decisions")
        gaps = sorted({k[1] for k in self.keys}, key=int)
        arms = sorted({(k[2], k[3]) for k in self.keys})
        seeds = sorted({k[0] for k in self.keys}, key=int)
        p(f"  gaps {gaps}  arms {arms}  seeds {len(seeds)}")
        if len(self.keys) != len(gaps) * len(arms) * len(seeds):
            p("  WARNING: keys are not a full grid of gap x arm x seed -- some runs are missing")

        p("\n1. WHERE DECISIONS ARE SPENT (share of the leg's decisions; outcome mix within the setting)")
        p("   pooled over all decisions  |  [median over runs]")
        for leg in LEGS:
            total = sum(sum(self.cells[k][leg][s].values()) for k in self.keys for s in SETTINGS)
            if total == 0:
                continue
            p(f"\n   {leg}: {total} decisions")
            p("     {:<9} {:>16}   ".format("setting", "share") + "".join(f"{g:>17}" for g, _ in OUTCOME_GROUPS))
            for s in SETTINGS:
                n = sum(sum(self.cells[k][leg][s].values()) for k in self.keys)
                if n == 0:
                    continue
                per_run_share = []
                for k in self.keys:
                    lt = sum(sum(self.cells[k][leg][x].values()) for x in SETTINGS)
                    if lt:
                        per_run_share.append(sum(self.cells[k][leg][s].values()) / lt)
                cols = []
                for g, _ in OUTCOME_GROUPS:
                    pooled = sum(self.cells[k][leg][s][g] for k in self.keys) / n
                    runs = [self.cells[k][leg][s][g] / sum(self.cells[k][leg][s].values()) for k in self.keys if sum(self.cells[k][leg][s].values())]
                    cols.append(f"{100*pooled:>6.1f}% [{100*statistics.median(runs):>5.1f}%]")
                p(f"     {s:<9} {100*n/total:>6.1f}% [{100*statistics.median(per_run_share):>5.1f}%]   " + "".join(f"{c:>17}" for c in cols))

        p(f"\n2. STALLS (a run of an ant's consecutive decisions with no relocation; long = {LONG_STALL}+ decisions)")
        for leg in LEGS:
            lengths = self.stall_lengths[leg]
            if not lengths:
                continue
            leg_total = sum(self.leg_decisions[k][leg] for k in self.keys)
            in_long = sum(self.stall_decisions[k][leg] for k in self.keys)
            per_run = [self.stall_decisions[k][leg] / self.leg_decisions[k][leg] for k in self.keys if self.leg_decisions[k][leg]]
            lengths_sorted = sorted(lengths)
            p(f"\n   {leg}: {len(lengths)} stalls, median {statistics.median(lengths_sorted)} decisions, p90 {lengths_sorted[int(0.9*(len(lengths_sorted)-1))]}, max {lengths_sorted[-1]}")
            p(f"     time in long stalls: {100*in_long/leg_total:.1f}% of the leg's decisions pooled [{100*statistics.median(per_run):.1f}% median over runs]")
            longs = [x for x in self.long_stalls if x[0] == leg]
            if not longs:
                continue
            by_where = Counter()
            for _, s, b, n, *_ in longs:
                by_where[(s, b)] += n
            p("     where the long-stall decisions start (setting, cells from nest):")
            for (s, b), n in by_where.most_common(8):
                p(f"       {s:<9} {b:>6}   {100*n/in_long:5.1f}%")
            zero = sum(x[4] for x in longs)
            idle = sum(x[5] for x in longs)
            tumbled = sum(x[6] for x in longs)
            p(f"     inside long stalls: p_move exactly 0 on {100*zero/in_long:.1f}%, idle {100*idle/in_long:.1f}%, tumbled {100*tumbled/in_long:.1f}%")

        p("\n4. WHAT SETS p_move: Move's pre-squash sum by term, mean over decisions at p_move exactly 0 against the rest")
        for leg in LEGS:
            z, l = self.terms[leg]["zero"], self.terms[leg]["live"]
            nz = next(iter(z.values()), [0, 0])[1]
            nl = next(iter(l.values()), [0, 0])[1]
            if nz + nl == 0:
                continue
            p(f"\n   {leg}: p_move exactly 0 on {nz} of {nz + nl} decisions ({100*nz/(nz+nl):.1f}%)")
            p(f"     {'term':<22} {'at p_move 0':>12} {'moving':>10} {'difference':>11}")
            names = list(z.keys() or l.keys())
            for name in names:
                mz = z[name][0] / z[name][1] if z[name][1] else float("nan")
                ml = l[name][0] / l[name][1] if l[name][1] else float("nan")
                p(f"     {name:<22} {mz:>+12.3f} {ml:>+10.3f} {mz-ml:>+11.3f}")

        p("\n3. THE HOMEWARD RE-ROLL (every tumble, by leg: which gate decided)")
        for leg in LEGS:
            h = self.homeward[leg]
            n = sum(h.values())
            if n == 0:
                continue
            p(f"\n   {leg}: {n} tumbles")
            for reason, c in h.most_common():
                p(f"     {reason:<12} {c:>9}  {100*c/n:5.1f}%")
            f = self.fired_cos[leg]
            nf = sum(f.values())
            if nf:
                p("     of those that fired, the chosen heading points: " + ", ".join(f"{k} {100*v/nf:.1f}%" for k, v in f.most_common()))

        if not self.has_step2:
            p("\n5-6. (no drop or cone columns: these files predate step 2)")
            return
        p("\n5. THE DROP (C2: every drop roll, by leg; then the neighbours when it was rolled)")
        for leg in LEGS:
            d = self.drops[leg]
            n = sum(d.values())
            if n == 0:
                continue
            won = d["placed"] + d["delivered"] + d["no_room"]
            p(f"\n   {leg}: {n} drop rolls, {won} won")
            for why, c in d.most_common():
                p(f"     {why:<10} {c:>9}  {100*c/n:5.1f}%")
            if won:
                p(f"     no room on {100*d['no_room']/won:.1f}% of won rolls")
        for why in ("no_room", "placed", "delivered", "roll_lost"):
            nb = self.drop_nbr[why]
            cells = sum(nb.values())
            if cells == 0:
                continue
            rolls = cells // 8
            f8 = self.drop_free8[why]
            where = self.drop_where[why]
            p(f"\n   {why}: {rolls} rolls; " + ", ".join(f"{k} {100*v/rolls:.1f}%" for k, v in where.most_common()))
            p("     free neighbours: " + ", ".join(f"{k}: {100*v/rolls:.1f}%" for k, v in sorted(f8.items())))
            p("     the eight neighbours: " + ", ".join(f"{k} {100*v/cells:.1f}%" for k, v in nb.most_common(8)))

        p("\n6. THE CONE (C3: which forward candidate each step took; how many sides were open)")
        for leg in LEGS:
            if not self.picks[leg]:
                continue
            t = self.turns[leg]
            p(f"\n   {leg}: Turn nonzero on {t['requests']} steps, discarded on {t['discarded']}")
            p("     {:<9} {:>8} {:>9} {:>9} {:>9}   {:<24} {}".format("setting", "steps", "left", "straight", "right", "sides open 0 / 1 / 2", "side steps up / level / down"))
            for s_ in SETTINGS:
                pk = self.picks[leg][s_]
                n = sum(pk.values())
                if n == 0:
                    continue
                so = self.side_open[leg][s_]
                sd = self.side_dir[leg][s_]
                ns = sum(sd.values())
                p("     {:<9} {:>8} {:>8.1f}% {:>8.1f}% {:>8.1f}%   {:<24} {}".format(
                    s_, n, 100*pk["left"]/n, 100*pk["straight"]/n, 100*pk["right"]/n,
                    " / ".join(f"{100*so[k]/n:.1f}%" for k in (0, 1, 2)),
                    " / ".join(f"{sd[k]}" for k in ("up", "level", "down")) if ns else "-"))


# `creature::DIRS`: E, NE, N, NW, W, SW, S, SE, with y growing downward.
DIRS = [(1, 0), (1, -1), (0, -1), (-1, -1), (-1, 0), (-1, 1), (0, 1), (1, 1)]
NBR_COLS = ["n_nw", "n_n", "n_ne", "n_w", "n_e", "n_sw", "n_s", "n_se"]
HEADER = ("seed,gap,arm,tag,frame,id,leg,fill,x,y,x2,y2,heading,heading2,usable,setting,ax,ay,energy,home_aligned,at_nest,crowding,"
          "stillness,along_a,along_b,food_adjacent,kin_need,nest_x,move_out,p_move,turn,roll_move,roll_tumble,outcome,homeward,home_cos,moved,"
          "drop,drop_roll,drop_p,free8," + ",".join(NBR_COLS) + ",nbr_self,nbr_other,cone_l,cone_s,cone_r,pick")


def selftest():
    """Positive controls: a synthetic run whose every answer is known."""
    def row(frame, aid, leg, setting, outcome, p_move=0.5, homeward="not_asked", cos="nan", x=10, **extra):
        vals = dict.fromkeys(HEADER.split(","), "0")
        vals.update(seed="1", gap="90", arm="hand", tag="", frame=str(frame), id=str(aid), leg=leg, setting=setting,
                    outcome=outcome, p_move=str(p_move), homeward=homeward, home_cos=cos, x=str(x), nest_x="0",
                    drop="not_asked", free8="-", pick="stepped" == outcome and "straight" or "-", cone_s="1.6")
        vals.update({c: "-" for c in NBR_COLS})
        vals.update(extra)
        return ",".join(vals[k] for k in HEADER.split(","))
    lines = [HEADER]
    # Ant 1, laden: 12 idle decisions at p_move 0 in a corridor 10 cells out (a long stall), then a step.
    for f in range(12):
        lines.append(row(f, 1, "laden", "corridor", "roll_failed_idle", p_move=0.0, x=10))
    lines.append(row(12, 1, "laden", "corridor", "stepped", x=10))
    # Ant 2, empty: alternating step / tumble in a junction; tumbles report no_crop.
    for f in range(0, 20, 2):
        lines.append(row(f, 2, "empty", "junction", "stepped", x=40))
        lines.append(row(f + 1, 2, "empty", "junction", "roll_failed_tumbled", homeward="no_crop", x=40))
    # Ant 1 again later: a laden tumble that fired and chose a heading pointing away.
    lines.append(row(30, 1, "laden", "pocket", "roll_failed_tumbled", homeward="fired", cos="-0.7", x=10))
    # Ant 3, laden, on its anchor: two drop rolls with no room (own tail west,
    # seven packedsoil) and one placed with the east cell free.
    walled = {c: "packedsoil" for c in NBR_COLS}
    for f in (40, 41):
        lines.append(row(f, 3, "laden", "pocket", "roll_failed_idle", x=5, drop="no_room", free8="0", nbr_self=str(1 << 3), **walled))
    lines.append(row(42, 3, "laden", "pocket", "roll_failed_idle", x=5, drop="placed", free8="1", **dict(walled, n_e="empty")))
    # Ant 4, empty, a junction step to the left with Turn nonzero, both sides open,
    # and one to the right with Turn asking left and left zeroed (discarded).
    # Facing west (4): left is SW (down), right is NW (up).
    lines.append(row(50, 4, "empty", "junction", "stepped", x=60, pick="left", cone_l="0.6", cone_r="0.6", turn="0.2", heading="4", heading2="5"))
    lines.append(row(51, 4, "empty", "junction", "stepped", x=61, pick="right", cone_l="0", cone_r="0.6", turn="0.2", heading="4", heading2="3"))
    with tempfile.TemporaryDirectory() as d:
        path = Path(d) / "trailfollow-decisions-seed1-gap90-hand.csv"
        path.write_text("\n".join(lines) + "\n")
        c = Census()
        c.add_file(path)
        ok = True
        def check(cond, what):
            nonlocal ok
            print(("  ok    " if cond else "  FAIL  ") + what)
            ok &= cond
        check(c.rows == 39, f"39 rows read (got {c.rows})")
        check(c.drops["laden"]["no_room"] == 2 and c.drops["laden"]["placed"] == 1, "two no-room rolls and one placement")
        check(c.drop_nbr["no_room"]["(own body)"] == 2 and c.drop_nbr["no_room"]["packedsoil"] == 14, "the no-room neighbours: own tail twice, packedsoil 14")
        check(c.drop_nbr["placed"]["empty"] == 1 and c.drop_where["no_room"]["on the anchor"] == 0, "the placement's free cell is seen; x=5 is not the anchor (0,0)")
        check(c.picks["empty"]["junction"]["left"] == 1 and c.picks["empty"]["junction"]["right"] == 1, "one left and one right pick")
        check(c.turns["empty"]["requests"] == 2 and c.turns["empty"]["discarded"] == 1, "two Turn requests, one discarded")
        check(c.side_open["empty"]["junction"][2] == 1 and c.side_open["empty"]["junction"][1] == 1, "sides open: 2 on the first step, 1 on the second")
        check(c.side_dir["empty"]["junction"]["down"] == 1 and c.side_dir["empty"]["junction"]["up"] == 1, "facing west, the left step went down and the right one up")
        check(c.leg_decisions[("1", "90", "hand", "")]["laden"] == 17, "17 laden decisions (ant 1: 14, ant 3: 3)")
        check(any(x[0] == "laden" and x[3] == 12 and x[4] == 12 for x in c.long_stalls), "one laden long stall of 12, all at p_move 0")
        check(not any(x[0] == "empty" for x in c.long_stalls), "no empty long stall (it relocates every other decision)")
        check(c.homeward["empty"]["no_crop"] == 10, "10 empty tumbles refused for no_crop")
        check(c.fired_cos["laden"]["away (<-0.01)"] == 1, "the fired laden tumble is classified as pointing away")
        # A second file with the same key must be refused, not pooled.
        dup = Path(d) / "copy.csv"
        dup.write_text(path.read_text())
        try:
            c.add_file(dup)
            check(False, "a duplicate key is refused")
        except SystemExit:
            check(True, "a duplicate key is refused")
        # The decomposition: a fed empty ant with nothing else reads bias 2.0 +
        # energy -1.75 = 0.25, so a raw Move of squash(0.25) = 0.2 must leave a
        # residual of zero, and a trail-B reading of +0.1 must add +1.537.
        r = dict.fromkeys(HEADER.split(","), "0")
        r.update(leg="empty", energy="1", move_out=str(squash(0.25)))
        terms = move_terms(r)
        check(abs(terms["residual"]) < 1e-6 and abs(terms["= Move sum"] - 0.25) < 1e-6, f"a known Move sum decomposes with no residual (got {terms['residual']:+.6f})")
        r.update(along_b="0.1", move_out=str(squash(0.25 + 2.5 * (squash(1.1) - squash(-0.1)))))
        terms = move_terms(r)
        check(abs(terms["trail_B (units 2-3)"] - 1.537) < 1e-3 and abs(terms["residual"]) < 1e-6, f"trail B at +0.1 reads +1.537 (got {terms['trail_B (units 2-3)']:+.3f})")
    print("decisioncensus selftest:", "PASS" if ok else "FAIL")
    return 0 if ok else 1


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("paths", nargs="*")
    ap.add_argument("--gap", help="only files for this gap")
    ap.add_argument("--selftest", action="store_true")
    a = ap.parse_args()
    if a.selftest:
        return selftest()
    if not a.paths:
        ap.error("give a directory or files, or --selftest")
    c = Census()
    for f in files_of(a.paths):
        if a.gap and f"-gap{a.gap}-" not in f.name:
            continue
        c.add_file(f)
    c.report()
    return 0


if __name__ == "__main__":
    sys.exit(main())
