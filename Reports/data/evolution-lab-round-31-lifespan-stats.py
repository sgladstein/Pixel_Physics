#!/usr/bin/env python3
"""Order statistics over the round-31 played_bed sweep digest.

Reads Reports/data/evolution-lab-round-31-lifespan-sweep.txt -- the committed
digest, not the logs -- so this reproduces from what is on the branch.
"""
import re, sys, math, itertools
from collections import defaultdict

PATH = "/home/user/Pixel_Physics/Reports/data/evolution-lab-round-31-lifespan-sweep.txt"
ARMS = {"A": "shipped   life 40000, gate on",
        "B": "immortal  life 0,     gate on",
        "C": "no gate   life 40000, gate off",
        "D": "half      life 20000, gate on",
        "E": "double    life 80000, gate on"}
WANT_LIFE = {"A": 40000, "B": 0, "C": 40000, "D": 20000, "E": 80000}
WANT_GATE = {"A": "true", "B": "true", "C": "false", "D": "true", "E": "true"}

runs = {}          # (arm, seed) -> {frame -> {field: value}}
params = {}        # (arm, seed) -> dict of echoed parameters
cur = None
for line in open(PATH):
    if line.startswith("===== bed run "):
        # The digest also carries the shipped arm on the other five beds
        # (question 3). Those blocks are NOT sweep arms, and leaving `cur`
        # pointing at the previous arm let their echoed `life_half_life =
        # 40000` overwrite arm E's 80000 -- which is what the arm-consistency
        # check below caught. Clear the cursor rather than widening the check.
        cur = None
        continue
    m = re.match(r"===== run ([A-E])-s(\d+) =====", line)
    if m:
        cur = (m.group(1), int(m.group(2)))
        runs[cur] = {}
        params[cur] = {}
        continue
    if cur is None:
        continue
    if line.startswith("  ant life_half_life"):
        params[cur]["life"] = int(line.split("=")[1].split()[0])
    elif line.startswith("  room_gate"):
        params[cur]["gate"] = line.split("=")[1].split()[0]
    elif line.startswith("STOP "):
        kv = dict(p.split("=", 1) for p in line.split()[1:])
        f = int(kv["frame"])
        runs[cur][f] = {k: float(v) for k, v in kv.items()}

# --- the echoed parameters must match the arm the filename claims -----------
bad = []
for (arm, seed), p in params.items():
    if p.get("life") != WANT_LIFE[arm] or p.get("gate") != WANT_GATE[arm]:
        bad.append(f"{arm}-s{seed}: log says life={p.get('life')} gate={p.get('gate')}")
if bad:
    print("ARM MISMATCH -- the log does not name the arm the filename claims:")
    print("\n".join(bad)); sys.exit(2)

seeds = sorted({s for (_, s) in runs})
ARMS_PRESENT = "".join(a for a in "ABCDE" if any((a, s) in runs for s in seeds))
complete = [s for s in seeds if all((a, s) in runs and 200000 in runs[(a, s)] for a in ARMS_PRESENT)]
print(f"arms present: {ARMS_PRESENT}")
print(f"seeds with every arm complete at 200,000 frames: {len(complete)} -> {complete}")
print(f"every run's echoed life_half_life and room_gate match its arm ({len(runs)} runs checked)\n")

def q(vals, p):
    v = sorted(vals)
    if not v: return float("nan")
    return v[int(round((len(v) - 1) * p))]

PAIRED = [("ants","ants alive"),("plants","plants standing"),("bank","seed bank"),("starved","starved deaths"),("oldage","old-age deaths"),("digs","digs"),("edible","edible cells")]
FIELDS = [("ants", "ants alive"), ("plants", "plants standing"), ("bank", "seed bank"),
          ("starved", "starved deaths"), ("oldage", "old-age deaths"),
          ("born", "births"), ("digs", "digs"), ("edible", "edible cells")]

for frame in (120000, 200000):
    print(f"### order statistics at {frame:,} frames, n={len(complete)} seeds")
    print(f"{'quantity':<18}{'arm':<34}{'p10':>8}{'median':>9}{'p90':>8}{'min':>8}{'max':>8}")
    for key, label in FIELDS:
        for arm in ARMS_PRESENT:
            vals = [runs[(arm, s)][frame][key] for s in complete]
            print(f"{label if arm=='A' else '':<18}{ARMS[arm]:<34}"
                  f"{q(vals,0.10):>8.0f}{q(vals,0.50):>9.0f}{q(vals,0.90):>8.0f}"
                  f"{min(vals):>8.0f}{max(vals):>8.0f}")
        print()
    print()

def sign_test(pairs):
    """Two-sided exact sign test over per-seed (x, y); ties dropped."""
    d = [y - x for x, y in pairs if y != x]
    n, k = len(d), sum(1 for v in d if v > 0)
    if n == 0: return 0, 0, 1.0
    tail = lambda k: sum(math.comb(n, i) for i in range(k, n + 1)) / 2 ** n
    p = min(1.0, 2 * min(tail(k), 1 - tail(k + 1) + math.comb(n, k) / 2 ** n))
    return k, n, p

print("### paired, per seed -- the arm that cancels everything the rule is not about")
for frame in (120000, 200000):
    for base, other, name in (("B", "A", "lifespan 40000 against immortal"),
                              ("C", "A", "dig gate on against off"),
                              ("D", "A", "lifespan 40000 against 20000"),
                              ("E", "A", "lifespan 40000 against 80000")):
        if base not in ARMS_PRESENT or other not in ARMS_PRESENT:
            continue
        print(f"\n-- {name}, at {frame:,} frames -- each cell reads "
              f"[{ARMS[base].split()[0]} -> {ARMS[other].split()[0]}]")
        print(f"{'seed':>5}", end="")
        for key, _ in PAIRED: print(f"{key:>22}", end="")
        print()
        for s in complete:
            print(f"{s:>5}", end="")
            for key, _ in PAIRED:
                x = runs[(base, s)][frame][key]; y = runs[(other, s)][frame][key]
                print(f"{x:>10.0f} ->{y:>9.0f}", end="")
            print()
        for key, label in PAIRED:
            pairs = [(runs[(base, s)][frame][key], runs[(other, s)][frame][key]) for s in complete]
            k, n, p = sign_test(pairs)
            med = q([y - x for x, y in pairs], 0.50)
            print(f"   {label:<18} {other} higher on {k} of {n} untied seeds, "
                  f"median delta {med:+.0f}, sign-test p={p:.3f}")

# --- alive at all: the bar Z6 is written as ---------------------------------
print("\n### colonies alive at each stop (of the seeds complete), by arm")
print(f"{'arm':<34}" + "".join(f"{f//1000:>7}k" for f in range(0, 200001, 20000)))
for arm in ARMS_PRESENT:
    row = []
    for f in range(0, 200001, 20000):
        row.append(sum(1 for s in complete if runs[(arm, s)][f]["ants"] > 0))
    print(f"{ARMS[arm]:<34}" + "".join(f"{v:>8}" for v in row))
print(f"\n(of {len(complete)} seeds)")
