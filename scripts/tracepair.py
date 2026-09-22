#!/usr/bin/env python3
"""Pair `examples/trailfollow`'s per-seed TRACE block across arms, and sign-test.

    python3 scripts/tracepair.py <dir-of-arm-logs> <baseline-arm-name>
    python3 scripts/tracepair.py --selftest

**Why this exists.** `trailfollow` prints its TRACE block once per seed AND a
pooled footer, and the footer is the one that catches the eye.  It cannot be
compared between arms: the three nose arms ran **55,322 / 69,875 / 87,369**
laden ticks over the same 36 seeds, because their colonies were **1.0 / 2.5 /
11.0** ants at the median.  A share over pooled ticks is therefore weighted by
whichever arm founded a colony -- a measurement of the colony, not of the
thing under test.  Read pooled, the readability test looked like it doubled
the share of ants facing up-gradient; paired, it is 19/17 and moves nothing.
`Reports/pheromone-trail-direction-2026-09-16.md` §7.49 is the account.

**The tell, for next time:** the arms' `n` is printed on the same line as the
mean, and it differed by 58%.  Before quoting any per-run aggregate across
arms, read its `n` across arms first.

Arms are paired by position, because every arm runs the same seed list from the
same `seed0` -- so the script **refuses to run** when the block counts differ
rather than silently pairing seed 7 against seed 9.
"""
import re, sys, os, signal
import statistics as st

# This prints a wide table that will be read through `head`, and Python's
# default SIGPIPE handling turns that into a traceback on stderr -- which in a
# log reads exactly like the script failing.
signal.signal(signal.SIGPIPE, signal.SIG_DFL)

# One TRACE block per seed.  Seeds appear in run order and every arm runs the
# same seed list from the same seed0, so index is a valid pairing key -- but
# assert the counts match rather than assuming it.
BLOCK = re.compile(r"TRACE laden ants: n (\d+).*?mean P\(move\) ([\d.]+)"
                   r".*?net cells homeward (-?\d+) \(([-+][\d.]+)/tick\)", re.S)
# **An empty bucket prints as a bare `n 0` with no columns after it**, so a
# pattern that demands the columns drops exactly the seeds where the mechanism
# never fired -- the two most informative rows in the file.  Parse the count
# first and the columns only if they are there.
BUCKET = re.compile(r"(UP-gradient|DOWN-gradient|no readable gradient)\s+n\s+(\d+)"
                    r"(?:\s+P\(move\) ([\d.]+).*?cells homeward\s+-?\d+ \(([-+][\d.]+)/tick\))?")


def per_seed(path):
    txt = open(path).read()
    # Split on the block header so a seed's SPLIT cannot be read from the next.
    chunks = txt.split("TRACE laden ants:")[1:]
    rows = []
    for c in chunks:
        c = "TRACE laden ants:" + c
        b = BLOCK.search(c)
        # The gate-open repeat prints the same two bucket lines again, so take
        # the FIRST occurrence of each name and no more.
        seen = {}
        for m in BUCKET.finditer(c):
            seen.setdefault(m[1], (int(m[2]), float(m[4]) if m[4] else 0.0))
        if not b or len(seen) < 3:
            rows.append(None)          # keep the slot so pairing stays aligned
            continue
        n = int(b[1])
        up, up_d = seen["UP-gradient"]
        down, _ = seen["DOWN-gradient"]
        none_, _ = seen["no readable gradient"]
        rows.append(dict(
            n=n,
            p_move=float(b[2]),
            drift=float(b[4]),                 # net cells homeward per laden tick
            up_share=up / n if n else 0.0,
            down_share=down / n if n else 0.0,
            none_share=none_ / n if n else 0.0,
            up_drift=up_d,
            degenerate=(up == 0 and down == 0),
        ))
    return rows


def sign(base, arm, key):
    """Paired within seed.  A seed is dropped only if it is missing on EITHER
    side -- never on one, which would compare different seed sets."""
    b = w = t = 0
    d = []
    for x, y in zip(base, arm):
        if x is None or y is None:
            continue
        a, c = x[key], y[key]
        d.append(c - a)
        if c > a: b += 1
        elif c < a: w += 1
        else: t += 1
    return b, w, t, st.median(d) if d else 0.0


BLOCK_TEMPLATE = """\
    TRACE laden ants: n {n}  mean PheroAAlong -0.2  MEAN |PheroAAlong| 0.2  \
mean P(move) {pm:.4f}  mean trail term (h0+h1 -> Move) -2.5  \
net cells homeward 99 ({drift:+.6f}/tick)
    TRACE split by the sign of `along` -- the run-and-tumble test:
      facing UP-gradient     n {up:8d}   P(move) 0.6   not resting  100.0%   \
P(move)|>0 0.6   cells homeward       96 (+0.05/tick)
      facing DOWN-gradient   n {down:8d}   P(move) 0.02   not resting    9.4%   \
P(move)|>0 0.25   cells homeward       65 (+0.001/tick)
      no readable gradient   n {none_:8d}   P(move) 0.2   not resting   65.8%   \
P(move)|>0 0.34   cells homeward       -8 (-0.0018/tick)
"""


def selftest():
    """**The positive control, and the two negative ones.**

    `CLAUDE.md`: a checker whose green is cited has to be watched going red.
    The faults put back are the three this script exists to catch -- an empty
    bucket printed as a bare `n 0` (which the first version of the parser
    dropped, silently taking the two most informative seeds out of the sample),
    a seed-count mismatch between arms, and the pooled-vs-paired inversion
    itself, constructed so the two reductions disagree in sign.
    """
    import tempfile, subprocess
    ok = True
    with tempfile.TemporaryDirectory() as d:
        # Ten seeds.  The BASE arm has one huge seed whose shares are extreme;
        # the ARM is better than base on 9 of 10 seeds and far worse on the
        # huge one.  Pooled, the huge seed decides and the arm looks worse;
        # paired, the arm is 9/1 better.  If this script ever reports the
        # pooled sign, this case says so.
        def write(path, rows):
            with open(path, "w") as f:
                for n, up, down, none_ in rows:
                    f.write(BLOCK_TEMPLATE.format(n=n, up=up, down=down, none_=none_,
                                                  pm=0.1, drift=0.004))
        base = [(1000, 100, 800, 100)] * 9 + [(900000, 90000, 720000, 90000)]
        arm = [(1000, 150, 750, 100)] * 9 + [(900000, 9000, 801000, 90000)]
        write(os.path.join(d, "X_base.log"), base)
        write(os.path.join(d, "X_arm.log"), arm)
        pooled_base = sum(r[1] for r in base) / sum(r[0] for r in base)
        pooled_arm = sum(r[1] for r in arm) / sum(r[0] for r in arm)
        b, w, t, _ = sign(per_seed(os.path.join(d, "X_base.log")),
                          per_seed(os.path.join(d, "X_arm.log")), "up_share")
        print(f"  pooled up-grad share: base {pooled_base:.4f} arm {pooled_arm:.4f} "
              f"-> arm looks {'WORSE' if pooled_arm < pooled_base else 'better'}")
        print(f"  paired sign test:     {b}/{w}/{t} -> arm is {'BETTER' if b > w else 'worse'}")
        if not (pooled_arm < pooled_base and b > w):
            print("  FAIL: the constructed inversion did not invert"); ok = False
        else:
            print("  ok: the two reductions disagree, and this script reports the paired one")

        # Negative control 1: an empty bucket must parse, not vanish.
        p = os.path.join(d, "X_zero.log")
        with open(p, "w") as f:
            f.write(BLOCK_TEMPLATE.format(n=3354, up=0, down=0, none_=3354, pm=0.2875, drift=-0.001789)
                    .replace("n        0", "n 0"))
        rows = per_seed(p)
        if len(rows) == 1 and rows[0] and rows[0]["degenerate"]:
            print("  ok: a bare `n 0` bucket parses, and is flagged degenerate rather than dropped")
        else:
            print(f"  FAIL: bare `n 0` bucket gave {rows}"); ok = False

        # Negative control 2: mismatched seed counts must refuse, not pair.
        write(os.path.join(d, "Y_base.log"), base)
        write(os.path.join(d, "Y_short.log"), arm[:5])
        r = subprocess.run([sys.executable, __file__, d, "Y_base"], capture_output=True, text=True)
        if r.returncode != 0 and "not pairable" in r.stderr:
            print("  ok: mismatched seed counts refuse to pair")
        else:
            print(f"  FAIL: mismatched counts ran anyway (rc={r.returncode})"); ok = False
    print("tracepair: selftest " + ("clean" if ok else "FAILED"))
    return 0 if ok else 1


if __name__ == "__main__":
    if "--selftest" in sys.argv:
        sys.exit(selftest())
    d = sys.argv[1]
    base_name = sys.argv[2]
    arms = {os.path.basename(p)[:-4]: per_seed(os.path.join(d, p))
            for p in sorted(os.listdir(d)) if p.endswith(".log")}
    arms = {k: v for k, v in arms.items() if v and k.startswith(base_name[0])}
    base = arms[base_name]
    for k, v in arms.items():
        assert len(v) == len(base), f"{k} has {len(v)} slots, {base_name} has {len(base)} -- not pairable"
    print(f"baseline: {base_name}   {len(base)} seeds, paired within seed")
    for name in sorted(arms):
        ok = [r for r in arms[name] if r]
        deg = sum(1 for r in ok if r["degenerate"])
        print(f"  {name:<10} {len(ok)}/{len(arms[name])} blocks parsed, "
              f"{deg} with NO up- or down-gradient tick at all (the plane never lit)")
    print()
    keys = [("up_share", "up-grad %", 100), ("down_share", "down-grad %", 100),
            ("none_share", "silent %", 100), ("drift", "cells/tick", 1),
            ("up_drift", "up cells/tk", 1), ("p_move", "P(move)", 1)]
    for key, label, mul in keys:
        print(f"  {label:<12}", end="")
        for name in sorted(arms):
            med = st.median(r[key] for r in arms[name] if r) * mul
            if name == base_name:
                print(f"  {name}: {med:8.4f} (base)     ", end="")
            else:
                b, w, t, md = sign(base, arms[name], key)
                print(f"  {name}: {med:8.4f} {b:2d}/{w:2d}/{t:2d} d{md*mul:+8.4f}   ", end="")
        print()
