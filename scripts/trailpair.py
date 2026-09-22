#!/usr/bin/env python3
"""Pair `examples/trailfollow`'s per-seed OUTCOME rows across arms, and sign-test.

    python3 scripts/trailpair.py <dir-of-arm-logs> <baseline-arm-name>

**The sibling of `scripts/tracepair.py`, and the division is the point.**
That one pairs the `TRACE` block -- what the ant *read* and how often it faced
up-gradient, i.e. the sensor. This one pairs the seed summary rows -- what the
colony *did*: `DELIVERED`, `carry->nest`, the laden leg, `born`, `alive`,
`starved`. A change can move either without the other, and
`pheromone-trail-direction-2026-09-16.md` 7.49 is the case where exactly that
happened, so neither script answers for the other.

**Both halves of the counter rule, in one table.** `tumbles (homeward N, X%)`
is the "it fired" counter and `DELIVERED` is the effect counter from the far
side of the call -- CLAUDE.md asks for the pair, because a null looks the same
whether the mechanism is quiet or the probe never reached it. On the home_bias
sweep the first column read 0.00 on the baseline and rose 36/0/0 with the
lever, which is what made the second column's 0 -> 28 readable as an effect
rather than as noise.

Paired within seed, never pooled: the arms found colonies of very different
sizes, and a total over ticks from an 11x colony is a measurement of the
colony (7.49 again).

Arms are paired by seed number read off the row, not by position, so a run
missing a seed drops that pair rather than shifting every later one.
"""
import re, sys, os, glob, signal

# Prints a wide table that will be read through `head`; without this Python
# turns that into a traceback, which in a log reads as the script failing.
signal.signal(signal.SIGPIPE, signal.SIG_DFL)
import statistics as st

SEED = re.compile(r"^\s+(\d+)\s+(\d+)\s+(\w+)\s+(\d+)/(\d+)\s+(\d+)")
TUMB = re.compile(r"tumbles\s+(\d+) \(homeward\s+(\d+),\s+([\d.]+)%\)")
DELI = re.compile(r"DELIVERED\s+(\d+)")
CARRY = re.compile(r"carry->nest\s+(\d+)")
BORN = re.compile(r"born\s+(\d+) died\s+(\d+) \(starved\s+(\d+)\)")
LEG = re.compile(r"leg home n\s+(\d+) med\s+(\d+) p90\s+(\d+) \(laden n\s+(\d+) med\s+(\d+)")


def per_seed(path, want_arm="hand"):
    rows, cur = {}, None
    for line in open(path):
        m = SEED.match(line)
        if m:
            cur = dict(seed=int(m[2]), arm=m[3], alive=int(m[4]), ate=int(m[6])) if m[3] == want_arm else None
            if cur:
                rows[cur["seed"]] = cur
            continue
        if cur is None:
            continue
        for rx, keys in ((TUMB, ("tumbles", "homeward", "homeward_pct")),
                         (DELI, ("delivered",)), (CARRY, ("carry_nest",)),
                         (BORN, ("born", "died", "starved")),
                         (LEG, ("leg_n", "leg_med", "leg_p90", "laden_n", "laden_med"))):
            m = rx.search(line)
            if m:
                for i, k in enumerate(keys):
                    cur[k] = float(m[i + 1])
    return rows


def sign(base, arm, key):
    b = w = t = 0
    d = []
    for s in sorted(set(base) & set(arm)):
        if key not in base[s] or key not in arm[s]:
            continue
        x, y = base[s][key], arm[s][key]
        d.append(y - x)
        if y > x: b += 1
        elif y < x: w += 1
        else: t += 1
    return b, w, t, (st.median(d) if d else 0.0)


if __name__ == "__main__":
    d = sys.argv[1]
    base_name = sys.argv[2]
    arms = {}
    for p in sorted(glob.glob(os.path.join(d, "*.log"))):
        n = os.path.basename(p)[:-4]
        r = per_seed(p)
        if r:
            arms[n] = r
    base = arms[base_name]
    print(f"baseline: {base_name}   {len(base)} seeds, `hand` arm, paired within seed\n")
    cols = [("homeward_pct", "homeward tumble %"), ("delivered", "DELIVERED"),
            ("carry_nest", "carry->nest"), ("laden_med", "laden leg med"),
            ("alive", "alive"), ("born", "born"), ("starved", "starved")]
    for key, label in cols:
        print(f"  {label:<18}", end="")
        for name in sorted(arms):
            vals = [r[key] for r in arms[name].values() if key in r]
            if not vals:
                print(f"  {name}: --        ", end=""); continue
            med = st.median(vals)
            if name == base_name:
                print(f"  {name}: {med:8.2f} (base)  ", end="")
            else:
                b, w, t, md = sign(base, arms[name], key)
                print(f"  {name}: {med:8.2f} {b:2d}/{w:2d}/{t:2d}  ", end="")
        print()
