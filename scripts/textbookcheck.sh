#!/usr/bin/env bash
# Does the textbook ant still pass the textbook tests it has passed?
#
# **One trail setting, two papers** (2026-10-16, `examples/antbridge.rs`;
# results in `emergence-sandbox/dead-branch/` in the project folder). The
# trail is laid both ways and fades over 10,000 frames; an ant that has
# searched an empty feeder for 1500 decisions with no food in sight turns
# home laying a warning mark (Robinson et al. 2005) back to the fork. At
# that one setting, over 16 runs each (seeds 1-8, both mirrors):
#
#   1. the double bridge (Goss et al. 1989): with one branch twice as
#      long, the short one carries over 80% of the last half's crossings
#      on 10 of 16 (paper 11 of 14). Gated: over 80% on at least 2 of 4
#      (seeds 1-2, both mirrors; measured 4 of 4). Scent off (`mute=1`):
#      0 of 8 -- geometry alone makes the short branch a 0.68-0.76
#      majority, so the majority bar would pass with no trail and is not
#      the one gated;
#   2. the short branch opened late (`opens=30000`, three trail
#      lifetimes in) carries under 20% of the last half on at least 3 of 4.
#      Measured 8 of 8 (0.00-0.02; paper 16 of 18 stay);
#   3. food moved to the other room (Dussutour et al. 2009, 15 of 15
#      colonies switch, in 1.0-1.9 trail lifetimes): switched within 2.5
#      lifetimes on at least 3 of 4. Measured 16 of 16, in 1.1-1.8. With
#      the trail never fading 0 of 8 switch, fading alone 1 of 8.
#
# Every run must also move: over 200 crossings in the last half, and food
# delivered, or a box where nobody walks would pass claim 2.
#
# `--selftest` runs the same grid with the trail laid and never read
# (`mute=1`) and must fail EACH claim: the positive control that every
# gate can go red on its own.
#
# Deterministic per build, so the seeds are fixed rather than a sample: a
# change that moves these numbers moved the ant. ~1 min on four cores (the
# six runs go in parallel).
set -euo pipefail
cd "$(dirname "$0")/.."

extra=""
expect=pass
if [ "${1:-}" = "--selftest" ]; then
    extra="mute=1"
    expect=fail
fi

cargo build --release --locked --example antbridge
bin=target/release/examples/antbridge
out=$(mktemp -d)
trap 'rm -rf "$out"' EXIT

base="tunnel=2 release=50 bdiff=0 lay=0.1 k=2 ants=150 seeds=2 brho=0.0012 layout=1 giveup=1500 search=24 sense=6 noentry=0.1 markfork=2 $extra"
bridge="long=40"
moved="feeders=2 room=44 gap=18 moves=30000 frames=62000 every=1000"
# shellcheck disable=SC2086
{
    "$bin" $base $bridge frames=30000 > "$out/early_lower.txt" &
    "$bin" $base $bridge frames=30000 longside=upper > "$out/early_upper.txt" &
    "$bin" $base $bridge opens=30000 frames=60000 > "$out/late_lower.txt" &
    "$bin" $base $bridge opens=30000 frames=60000 longside=upper > "$out/late_upper.txt" &
    "$bin" $base $moved > "$out/moved_upper.txt" &
    "$bin" $base $moved start=lower > "$out/moved_lower.txt" &
    wait
}

python3 - "$out" "$expect" <<'EOF'
import re, sys, pathlib
out, expect = pathlib.Path(sys.argv[1]), sys.argv[2]
runs, moved = {}, {}
for f in sorted(out.glob("*.txt")):
    text = f.read_text()
    m = re.search(r"SUMMARY (?:short|fed)-branch share, last half \(share/crossings\): \[([^\]]*)\]", text)
    d = re.search(r"delivered per seed \[([^\]]*)\]", text)
    if not m or not d:
        sys.exit(f"textbookcheck: {f.name} has no SUMMARY -- the run failed:\n{text[-2000:]}")
    pairs = [tuple(map(float, p.split("/"))) for p in m.group(1).split()]
    delivered = [int(x) for x in d.group(1).split()]
    if len(pairs) != 2 or len(delivered) != 2:
        sys.exit(f"textbookcheck: {f.name}: expected 2 seeds, parsed {len(pairs)} shares and {len(delivered)} deliveries")
    runs[f.stem] = list(zip(pairs, delivered))
    if f.stem.startswith("moved"):
        sw = re.search(r"SUMMARY moved food at \d+: switch after \[[^\]]*\] frames = \[([^\]]*)\] trail lifetimes", text)
        if not sw:
            sys.exit(f"textbookcheck: {f.name} has no switch SUMMARY")
        moved[f.stem] = [None if x == "-" else float(x) for x in sw.group(1).split()]
        if len(moved[f.stem]) != 2:
            sys.exit(f"textbookcheck: {f.name}: expected 2 switch times, parsed {moved[f.stem]}")
    sws = moved.get(f.stem)
    print(f"{f.stem:12} " + "  ".join(
        f"{'fed' if sws else 'short'} {s:.2f} of {int(c)} crossings, delivered {dl}"
        + (f", switched after {sws[i]} lifetimes" if sws else "")
        for i, ((s, c), dl) in enumerate(runs[f.stem])))

if len(runs) != 6:
    sys.exit(f"textbookcheck: expected 6 arms, got {sorted(runs)}")
dead = [(k, i) for k, v in runs.items() for i, ((_, c), dl) in enumerate(v) if c <= 200 or dl == 0]
early = [s for k in ("early_lower", "early_upper") for (s, _), _ in runs[k]]
late = [s for k in ("late_lower", "late_upper") for (s, _), _ in runs[k]]
over80 = sum(s > 0.8 for s in early)
stayed = sum(s < 0.2 for s in late)
switched = sum(t is not None and t <= 2.5 for v in moved.values() for t in v)
claims = {"bridge": over80 >= 2, "late": stayed >= 3, "moved": switched >= 3}
print(f"short branch over 80%: {over80} of 4 (bar 2); stayed on the long branch when the short opened late: {stayed} of 4 (bar 3); "
      f"switched to the moved food within 2.5 lifetimes: {switched} of 4 (bar 3); runs with no traffic or no food home: {len(dead)}")
if expect == "pass":
    if dead or not all(claims.values()):
        sys.exit(f"textbookcheck: FAIL -- {[k for k, v in claims.items() if not v] or 'a run with no traffic'}: "
                 "the textbook ant no longer passes Goss 1989 and Dussutour 2009 at one setting")
    print("textbookcheck: double bridge, late branch and moved food pass")
else:
    if any(claims.values()):
        sys.exit(f"textbookcheck --selftest: {[k for k, v in claims.items() if v]} PASSED with the trail never read -- blind")
    print("textbookcheck --selftest: every claim goes red with the trail never read, as it must")
EOF
