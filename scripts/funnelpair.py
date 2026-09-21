#!/usr/bin/env python3
"""Pair `trailfollow`'s FUNNEL blocks across two arms, within seed.

The `funnel` skill says to read the funnel before quoting any ratio, and to
pair within seed rather than pool -- but nothing in `scripts/` could do it.
`trailpair.py` pairs the OUTCOME row (what the colony did) and `tracepair.py`
the TRACE block (what the ant read); the stages *between* those two, which is
where the loop is actually lost, had to be read by eye across two logs.

    python3 scripts/funnelpair.py a.log b.log --a-label nest --b-label founders
    python3 scripts/funnelpair.py --selftest

**Why paired and never pooled.** The arms found colonies of different sizes --
measured on the sensor arms, 1.0 against 11.0 animals at the median over the
same 36 seeds -- so a pooled share is weighted by whichever arm founded more
ants, and the weights are the thing under test. Every stage here is compared
within one seed and then sign-tested over seeds, which cancels seed, supply,
gap and crowding together.

**It keys on (gap, seed) and prints the cardinality**, because `trailfollow`
sweeps commute distances as well as seeds and a parse keyed on the seed alone
pools them silently, last write wins (`CLAUDE.md`).

Both percentage columns are reported for the same reason the harness prints
both: *of prev* says where ants are lost, *of all* says whether the colony is
doing anything at all. A stage that doubles `of prev` can still be four ants.
"""

import argparse
import re
import sys
from pathlib import Path

# `   90    1  hand   10/10   ...` -- gap, seed, arm.
ROW = re.compile(r"^\s*(\d+)\s+(\d+)\s+(\w+)\s")
HEAD = re.compile(r"THE LOOP, ANT BY ANT")
STAGE = re.compile(r"^\s*(\d)\.\s+(.+?)\s{2,}(\d+)\s+([\d.]+)% of prev\s+([\d.]+)% of all")


def parse(path):
    """{(gap, seed): {arm, stages: [(name, count)]}} -- one block per seed."""
    out, cur, collecting = {}, None, None
    for line in Path(path).read_text(errors="replace").splitlines():
        m = ROW.match(line)
        if m and "of prev" not in line:
            cur = (int(m.group(1)), int(m.group(2)), m.group(3))
            continue
        if HEAD.search(line):
            if cur is None:
                continue
            collecting = []
            out[(cur[0], cur[1])] = {"arm": cur[2], "stages": collecting}
            continue
        if collecting is not None:
            m = STAGE.match(line)
            if m:
                collecting.append((m.group(2).strip(), int(m.group(3))))
            elif line.strip() and not line.startswith(" " * 16):
                collecting = None
    return {k: v for k, v in out.items() if v["stages"]}


def sign_test(pairs):
    """(up, down, tied). Deliberately not a p-value: these are counts of seeds
    and the useful statement is '19 up, 5 down', which a reader can weigh."""
    up = sum(1 for a, b in pairs if b > a)
    down = sum(1 for a, b in pairs if b < a)
    return up, down, len(pairs) - up - down


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("a", nargs="?")
    ap.add_argument("b", nargs="?")
    ap.add_argument("--a-label", default="A")
    ap.add_argument("--b-label", default="B")
    ap.add_argument("--selftest", action="store_true")
    o = ap.parse_args()
    if o.selftest:
        return selftest()

    A, B = parse(o.a), parse(o.b)
    ka, kb = set(A), set(B)
    shared = sorted(ka & kb)
    print(f"funnelpair: {o.a_label} {len(ka)} (gap,seed) keys | {o.b_label} {len(kb)} | shared {len(shared)}")
    gaps = sorted({g for g, _ in shared})
    print(f"            gaps {gaps}  seeds {sorted({s for _, s in shared})}")
    if ka != kb:
        print(f"            ** keys differ: {sorted(ka - kb)} only in {o.a_label}, "
              f"{sorted(kb - ka)} only in {o.b_label} -- those seeds are dropped, not paired across.")
    if not shared:
        print("funnelpair: nothing to pair.")
        return 1

    n_stages = min(len(A[k]["stages"]) for k in shared)
    print(f"\n{'stage':<42} {o.a_label:>9} {o.b_label:>9} {'delta':>8}   seeds b>a/b<a/tie")
    print("-" * 92)
    for i in range(n_stages):
        name = A[shared[0]]["stages"][i][0]
        pairs = [(A[k]["stages"][i][1], B[k]["stages"][i][1]) for k in shared]
        sa, sb = sum(p[0] for p in pairs), sum(p[1] for p in pairs)
        up, down, tie = sign_test(pairs)
        # Totals are printed because the owner asks "how many ants", and the
        # sign test beside them is what says whether the total is one seed.
        print(f"{name:<42} {sa:>9} {sb:>9} {sb - sa:>+8}   {up:>3}/{down:>3}/{tie:>3}")

    print("\n  'of all' -- the share of every ant that ever lived, per arm:")
    for i in range(n_stages):
        name = A[shared[0]]["stages"][i][0]
        fa = sum(A[k]["stages"][i][1] for k in shared) / max(1, sum(A[k]["stages"][0][1] for k in shared))
        fb = sum(B[k]["stages"][i][1] for k in shared) / max(1, sum(B[k]["stages"][0][1] for k in shared))
        print(f"    {name:<42} {100 * fa:>5.1f}%  ->{100 * fb:>6.1f}%")
    return 0


def selftest():
    """Positive control: a constructed pair where pooled and paired disagree.

    The failure this guards is the one `tracepair.py`'s own selftest carries --
    an arm that looks better pooled because it founded more ants. Here arm B
    wins the total on stage 1 while losing 1-up-against-4-down within seed, so
    a script that pooled would print the wrong direction and this asserts the
    sign test does not.
    """
    import tempfile

    def log(rows):
        out = []
        for gap, seed, arm, counts in rows:
            out.append(f"  {gap}   {seed}  {arm}   1/1   0   0   0   0   0   1/1   1   0 0 0 0 0")
            out.append(" " * 16 + "THE LOOP, ANT BY ANT:")
            for i, (nm, c) in enumerate(counts):
                out.append(" " * 18 + f"{i}. {nm:<38} {c:>5}   100.0% of prev   100.0% of all")
        return "\n".join(out)

    st = ["lived", "reached the food"]
    a = [(90, s, "hand", [(st[0], 10), (st[1], 5)]) for s in range(1, 6)]
    # B: one seed with a huge colony carries the total, the other four lose.
    b = [(90, 1, "hand", [(st[0], 100), (st[1], 60)])] + [
        (90, s, "hand", [(st[0], 10), (st[1], 4)]) for s in range(2, 6)
    ]
    d = Path(tempfile.mkdtemp())
    (d / "a.log").write_text(log(a))
    (d / "b.log").write_text(log(b))
    A, B = parse(d / "a.log"), parse(d / "b.log")
    assert len(A) == 5 and len(B) == 5, f"parsed {len(A)}/{len(B)} of 5 -- the block reader is not finding the seeds"
    assert A[(90, 1)]["stages"] == [("lived", 10), ("reached the food", 5)], A[(90, 1)]["stages"]
    pairs = [(A[k]["stages"][1][1], B[k]["stages"][1][1]) for k in sorted(set(A) & set(B))]
    up, down, tie = sign_test(pairs)
    pooled_a = sum(p[0] for p in pairs)
    pooled_b = sum(p[1] for p in pairs)
    assert pooled_b > pooled_a, "the control is not constructed: B must win the POOLED total"
    assert down > up, f"...and lose the PAIRED sign test, but got {up} up / {down} down"
    # And the parse must key on the gap, not just the seed.
    two = log(a + [(200, s, "hand", [(st[0], 1), (st[1], 1)]) for s in range(1, 6)])
    (d / "c.log").write_text(two)
    C = parse(d / "c.log")
    assert len(C) == 10, f"keyed on the seed alone: 10 blocks over two gaps collapsed to {len(C)}"
    print(f"funnelpair selftest: ok -- pooled says B wins ({pooled_a} -> {pooled_b}), "
          f"paired says {up} up / {down} down; two gaps stay {len(C)} keys, not 5")
    return 0


if __name__ == "__main__":
    sys.exit(main())
