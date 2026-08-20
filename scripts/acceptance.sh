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

# 7/8. A building survives being chipped, and still comes apart when a wall
#    is actually cut through. The pair is the point: either alone is passable
#    by cheating in one direction, and this project has shipped both cheats.
#    "Nothing fails" passes case 7 by making rock invincible, which is how
#    four earlier support models died; "everything fails" passes case 8.
#
#    What they guard is the regression that prompted them: a radius-1 chisel
#    cut into a 17-cell-thick wall took down 2,516 cells of a room that was
#    standing at 15% of capacity, because a column carried its roof's full
#    bending moment all the way to the floor and any one cell of it that lost
#    its attachment bonus failed wherever it stood. `min_overloaded` on case
#    8 measured 56; the bar is 5.
#
#    A bigger budget than the rest, and not a tuning fudge: the room is the
#    largest structure any scene here builds and the only one that is mostly
#    *surface*, so far more of it is structurally interesting than in a solid
#    massif. Measured 29-32 ms against the 13-22 ms the others sit at, so 90
#    keeps the same ~3x margin over the measurement that 60 gives them.
run roomstands scene=room wall=5 dig=0 start=2 every=50 count=5 crop=100,120,280,200 zoom=2 max_failures=0   repeat=2 max_frame_ms=90
run roomcut    scene=room wall=5 dig=3 start=2 every=50 count=5 crop=100,120,280,200 zoom=2 min_overloaded=5 repeat=2 max_frame_ms=90

# 6. One dig into a *generated* world, on more than one seed. The case that
#    the other seven are structurally blind to: every scene above builds
#    hand-placed geometry at the default seed, so none of them can see a
#    change that only misbehaves on procedural terrain. Two changes to the
#    load model went green on all seven while eating tens of thousands of
#    cells here -- the second of them fifty times more world than the bug it
#    was fixing.
#
#    Two seeds, because outcomes are chaotic in the seed:  seed 7 and
#    seed 1 differed 25x on identical preset parameters. Two is not a sweep;
#    it is the cheapest thing that is not blind. The real instrument is a
#    seeds= sweep over order statistics, and it is the next thing to build.
#
#     is the control -- an untouched generated world must not move at
#    all -- and the bars come from measurement: the cut cases sat at 27,409
#    and 23,042 cells before the bearing model and at 0 after, so 40 leaves
#    room for legitimate rubble without admitting a cascade.
run crackflat0  scene=worldcrack preset=flat   seed=7 dig=0 start=2 every=250 count=4 zoom=1 max_failures=0  repeat=2 max_frame_ms=$BUDGET_MS
run crackcan0   scene=worldcrack preset=canyon seed=7 dig=0 start=2 every=250 count=4 zoom=1 max_failures=0  repeat=2 max_frame_ms=$BUDGET_MS
run crackflat   scene=worldcrack preset=flat   seed=7 dig=6 start=2 every=250 count=4 zoom=1 max_failures=40 repeat=2 max_frame_ms=$BUDGET_MS
run crackflat1  scene=worldcrack preset=flat   seed=1 dig=6 start=2 every=250 count=4 zoom=1 max_failures=40 repeat=2 max_frame_ms=$BUDGET_MS
run crackcanyon scene=worldcrack preset=canyon seed=7 dig=6 start=2 every=250 count=4 zoom=1 max_failures=40 repeat=2 max_frame_ms=$BUDGET_MS

# 9/10. A cave can be dug and it does not collapse -- and a shallow one
#    does. The owner's own statement of what this milestone has to do:
#    "just want to make sure a cave can be dug and not collapse."
#
#    Gated on **roofed void** (empty cells with rock above them), not on
#    cells destroyed, because cells-destroyed ranks these backwards: a
#    2-cell roof and an 8-cell roof both come down completely and the thin
#    one contains less rock, so the worse outcome reads as the smaller
#    number. Measured at depth 6/12/18 the void keeps 10% / 41% / 100%,
#    which is the ordering anyone looking at it would give.
#
#    The pair is the point, as with the room cases: "nothing collapses"
#    passes the first by making rock invincible, and that is how four
#    earlier support models died.
#
#    `preset=flat` specifically. On sloping terrain the scene drives a
#    horizontal bore at a fixed depth below the surface *at one x*, so on a
#    hillside it leaves the hill -- `rolling` seed 24301 starts with 124
#    cells of roofed void against flat's 678, and that is the scene, not
#    the model. Two seeds because it is procedural; measured 100% on seeds
#    1, 7 and 24301, so the bar at 90 has real headroom.
run cavedeep   scene=worldcrack preset=flat seed=7 dig=4 tunnel=35 depth=18 start=2 every=600 count=4 zoom=1 min_cave=90       repeat=2 max_frame_ms=$BUDGET_MS
run cavedeep1  scene=worldcrack preset=flat seed=1 dig=4 tunnel=35 depth=18 start=2 every=600 count=4 zoom=1 min_cave=90       repeat=2 max_frame_ms=$BUDGET_MS
run caveshallow scene=worldcrack preset=flat seed=7 dig=4 tunnel=35 depth=6 start=2 every=600 count=4 zoom=1 min_overloaded=50 repeat=2 max_frame_ms=$BUDGET_MS

# 11. A pond freezes over under a cold snap and nothing gives way. The
#    acceptance bar the ice milestone was set: a floating sheet's only
#    anchor is the water under it (`MaterialDef::floats`), and before that
#    existed the *first* cell to freeze in open water was a lone solid with
#    no path to an anchor and was dismantled the frame after it appeared --
#    3,969 freezes in one storm and never ten cells of ice standing.
#
#    `max_failures=0` covers freeze-over and the deepest part of the drift
#    on the ice -- **not the thaw**, which an earlier draft of this comment
#    claimed. The storm lifts at frame 700 and this run ends at 720; the
#    sheet's own thaw breakup lands just past that window (measured on the
#    merged branch: 4 overloads, 845 cells, between frames 720 and 900,
#    deterministic). That breakup is the mechanic -- a thinning sheet under
#    a snow load gives way, the debris is snow that floats and melts, and
#    the end state is clean open water -- so it is deliberately outside
#    this bar rather than silently inside it. If a bar over the thaw is
#    ever wanted, set it from paired runs of the full arc (count=6 reaches
#    frame 1260), not from the single measurement quoted here.
#    The bar is real rather than a formality: at ice.ron's
#    `max_unsupported_span` of 16 the same run measures 17 overloads
#    (2,210 cells) and 118 unsupported (289), and at 32 it measures 6
#    (924). The scene is deterministic -- weather is a pure function of
#    `(seed, frame)` and the scene pins both -- so this is not a sampled
#    case the way the `worldcrack` seeds are.
#
#    `repeat=3` rather than 2, and that is a measurement not a habit: this
#    is the busiest scene in the file -- a storm keeps the whole surface of
#    the world awake -- so its worst frame is the most contention-sensitive
#    here. Measured at **17.85 ms** as the minimum over three runs, with
#    single runs on a loaded machine reaching 58; two samples landed on 46
#    once, which is close enough to the 60 ms bar to flake. Three keeps the
#    same ~3x margin the other cases have.
run coldsnap scene=coldsnap start=180 every=180 count=4 crop=180,228,160,44 zoom=3 max_failures=0 repeat=3 max_frame_ms=$BUDGET_MS

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
