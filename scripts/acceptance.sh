#!/usr/bin/env bash
# The structural acceptance cases, run rather than remembered.
#
# `Reports/load-model-handoff.md` section 7 lists five cases that decide
# whether the fracture model is behaving. They were checked by rendering a
# contact sheet and looking at it -- which is right, and must stay, because
# an image shows everything including what nobody thought to measure. But
# nothing *ran* them, and that is how `scene=capped` came to be recorded as
# passing while its entire 15,840-cell structure was frozen and had never
# been load-evaluated. "It still stands" was true and meant nothing.
#
# So each case asserts the **mechanism** as well as the outcome:
#   - a scene that should collapse must show the criterion firing
#     (`min_overloaded`), not merely that material moved
#   - a scene that should stand must show nothing fired (`max_failures`),
#     which is only meaningful because the same binary demonstrates on the
#     other scenes that it can fire at all
#
# Bars are set below what is currently measured, with headroom, per
# `CLAUDE.md` -- they are regression guards, not targets, and a bar sitting
# on the measured value would flake. Measured at the time of writing:
# worked 8, undercut 3, ligament 1, strike 10, snap 1.
#
# Deliberately not a `cargo test`: the scenes live in `examples/filmstrip.rs`
# and moving them into the library to satisfy the test harness would be a
# real refactor for no gain -- CI can run this directly. Images are written
# so a failure can be looked at, not just read about.
#
# **Frame-cost bars guard catastrophes, not tuning, and that is
# deliberate.** They are checked against the *minimum* of several runs
# (`repeat=`), because contention can only make a frame slower and a bar
# checked against one sample would be permanently flaky -- this machine
# has produced 18.0 ms twice running on a scene that schedules no
# structural work at all. They are also set far above what is measured
# here (3-14 ms) because CI hardware is not this hardware, and a bar tuned
# to a developer laptop trains everyone to ignore a red build.
#
# What they are sized to catch is the class of regression this project has
# actually shipped: a single change took `scene=capped` to **6,556 ms** and
# `scene=strike` to 4,456 ms per frame. A 60 ms bar catches that with room
# to spare, and will not fire on a slow runner having a bad minute. Tighten
# only alongside a measurement taken on the runner itself.
BUDGET_MS=60

set -uo pipefail

FILM="cargo run --release --quiet --example filmstrip --"
OUT="${OUT_DIR:-target/filmstrips}"
mkdir -p "$OUT"
fails=0

run() {
  local name="$1"; shift
  echo "--- $name"
  # shellcheck disable=SC2086
  if ! $FILM "$@" out="$OUT/acceptance-$name.png"; then
    echo "    ^^ $name FAILED"
    fails=$((fails + 1))
  fi
}

# 1. A worked root gives way. The whole point of the load model: six blows
#    at the join of a 160-cell shelf used to leave it standing.
run worked   scene=worked   start=2 every=50 count=6 crop=40,120,220,170 zoom=3 min_overloaded=3 repeat=2 max_frame_ms=$BUDGET_MS

# 2. The thick column still stands. The regression the change most easily
#    causes, and the case that was silently vacuous for two commits.
run capped   scene=capped   start=2 every=90 count=4 crop=150,70,220,190 zoom=3 max_failures=0 repeat=2 max_frame_ms=$BUDGET_MS

# 3. An undercut shelf still spalls.
run undercut scene=undercut start=1 every=45 count=6 crop=0,120,240,190 zoom=2 min_overloaded=1 repeat=2 max_frame_ms=$BUDGET_MS

# 4. A big overhang on a thin ligament snaps at the neck. The owner's
#    original case, and the one reach could not get right in principle.
run ligament scene=ligament start=2 every=70 count=6 crop=60,110,180,120 zoom=3 min_overloaded=1 repeat=2 max_frame_ms=$BUDGET_MS

# 5. Generated terrain does not move on its own. Not in the handoff's five,
#    added because it is the failure that would be worst and quietest: the
#    world eating itself with nobody having touched it.
run terrain  scene=terrain  start=2 every=90 count=4 crop=0,0,512,320 zoom=1 max_failures=0 repeat=2 max_frame_ms=$BUDGET_MS

# 6. A struck cliff throws pieces. Asserted as *bodies in flight*, not as
#    overload failures: the mechanism here is the blow's own fracture, and
#    an earlier bar on overload failures duly broke when an unrelated
#    change to the fragment ladder shifted how many separate events the
#    same material came away in. Measure what the scene is about.
run strike   scene=strike   start=2 every=60 count=4 crop=200,90,120,120 zoom=3 min_bodies=2 repeat=2 max_frame_ms=$BUDGET_MS

echo
if [ "$fails" -gt 0 ]; then
  echo "acceptance: $fails case(s) FAILED -- images in $OUT"
  exit 1
fi
echo "acceptance: all cases met their expectations -- images in $OUT"
