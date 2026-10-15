#!/usr/bin/env bash
# Does the textbook ant still pass the textbook tests it has passed?
#
# **The double bridge (Goss et al. 1989), on the textbook ant.** Passed
# 2026-10-15 over 16 seeds (`examples/antbridge.rs`, report
# `emergence-sandbox/e3-bridge/goss-r2-2026-10-15.md` in the project folder):
# with one branch twice as long the colony settles on the short one, and
# when the short branch opens late the colony stays on the long one it
# already marked. The textbook-tests plan says a test that passes becomes a
# quick check; this is that check, at two seeds a mirror.
#
# Two claims, each the one the trail makes and geometry alone does not:
#
#   1. the short branch carries over 80% of crossings in the last half, on
#      at least 2 of 4 runs (seeds 1-2, both mirrors). Measured 4 of 4 on
#      these seeds (5 of 8 over seeds 1-4). Scent off (`mute=1`): 0 of 8 --
#      the short branch is a 0.68-0.76 majority by geometry alone, so the
#      majority bar would pass with no trail and is not the one gated;
#   2. opened late (`opens=10000`), the short branch carries under 20% of
#      the last half on at least 3 of 4 runs. Measured 8 of 8 (0.00-0.01);
#      scent off 0 of 8 (0.69-0.75).
#
# Every run must also move: over 200 crossings in the last half, and food
# delivered, or a box where nobody walks would pass claim 2.
#
# `--selftest` runs the same grid with the trail laid and never read
# (`mute=1`) and must FAIL: the positive control that the gate can go red.
#
# Deterministic per build, so the seeds are fixed rather than a sample: a
# change that moves these numbers moved the ant. ~4 min serial on one core,
# ~1 min on four (the runs go in parallel).
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

base="tunnel=2 release=50 brho=0.0002 bdiff=0 lay=0.1 k=2 long=40 ants=150 frames=30000 seeds=2 $extra"
# shellcheck disable=SC2086
{
    "$bin" $base > "$out/early_lower.txt" &
    "$bin" $base longside=upper > "$out/early_upper.txt" &
    "$bin" $base opens=10000 > "$out/late_lower.txt" &
    "$bin" $base opens=10000 longside=upper > "$out/late_upper.txt" &
    wait
}

python3 - "$out" "$expect" <<'EOF'
import re, sys, pathlib
out, expect = pathlib.Path(sys.argv[1]), sys.argv[2]
runs = {}
for f in sorted(out.glob("*.txt")):
    text = f.read_text()
    m = re.search(r"SUMMARY short-branch share, last half \(share/crossings\): \[([^\]]*)\]", text)
    d = re.search(r"delivered per seed \[([^\]]*)\]", text)
    if not m or not d:
        sys.exit(f"textbookcheck: {f.name} has no SUMMARY -- the run failed:\n{text[-2000:]}")
    pairs = [tuple(map(float, p.split("/"))) for p in m.group(1).split()]
    delivered = [int(x) for x in d.group(1).split()]
    if len(pairs) != 2 or len(delivered) != 2:
        sys.exit(f"textbookcheck: {f.name}: expected 2 seeds, parsed {len(pairs)} shares and {len(delivered)} deliveries")
    runs[f.stem] = list(zip(pairs, delivered))
    print(f"{f.stem:12} " + "  ".join(f"short {s:.2f} of {int(c)} crossings, delivered {dl}" for (s, c), dl in runs[f.stem]))

if len(runs) != 4:
    sys.exit(f"textbookcheck: expected 4 arms, got {sorted(runs)}")
dead = [(k, i) for k, v in runs.items() for i, ((_, c), dl) in enumerate(v) if c <= 200 or dl == 0]
early = [s for k in ("early_lower", "early_upper") for (s, _), _ in runs[k]]
late = [s for k in ("late_lower", "late_upper") for (s, _), _ in runs[k]]
over80 = sum(s > 0.8 for s in early)
stayed = sum(s < 0.2 for s in late)
ok = not dead and over80 >= 2 and stayed >= 3
print(f"short branch over 80%: {over80} of 4 (bar 2); stayed on the long branch when the short opened late: {stayed} of 4 (bar 3); runs with no traffic or no food home: {len(dead)}")
if expect == "pass":
    if not ok:
        sys.exit("textbookcheck: FAIL -- the textbook ant no longer passes the double bridge (Goss 1989)")
    print("textbookcheck: double bridge passes")
else:
    if ok:
        sys.exit("textbookcheck --selftest: the gate PASSED with the trail never read -- it is blind")
    print("textbookcheck --selftest: the gate goes red with the trail never read, as it must")
EOF
