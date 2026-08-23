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

# **These cases run at the shipped `chain_reach`, which is `TIGHT`.**
#
# That is deliberate and it has a trap in it. TIGHT only licenses a
# structural failure near something that reported itself disturbed
# (`World::record_disturbance`), so a scene that hand-places geometry and
# asserts *nothing fails* passes on the leash rather than on the load
# model -- vacuously, and it would stop catching the regression it exists
# for. Two scenes broke outright when TIGHT landed -- `ligament` reported 0
# overload failures on the case that exists to show a neck snapping, and
# `rockdrop` left 600 cells of slab hanging in the air -- and both now
# record the disturbance their own construction implies.
#
# `capped` was checked for the *opposite* failure and does not have it:
# run with the leash off (`chain_reach=spread`) it still measures 0
# failures, so the model is what holds that column up, not the policy. It
# records a disturbance anyway, so the case cannot quietly acquire the
# dependency later. If a new "must stand" case is added, check it the same
# way: run it at `chain_reach=spread` and confirm the verdict does not
# move.
#
# `chain_reach=NAME` on a `run` line takes the leash off for a case that
# genuinely wants the model unlimited. Nothing here needs it today.

set -uo pipefail

FILM="cargo run --release --quiet --example filmstrip --"
OUT="${OUT_DIR:-target/filmstrips}"
mkdir -p "$OUT"
fails=0

# `SKIP_CASES` / `ONLY_CASES` are space-separated case names. They exist so CI
# can gate the healthy cases while a case with a *known, recorded* failure
# still runs somewhere visible, instead of the whole suite being marked
# non-blocking and taking sixteen working guards down with it. Neither
# variable is set by a normal local run: `bash scripts/acceptance.sh` with no
# environment runs everything, exactly as before.
#
# The rule these serve, from the workflow that uses them: a gate that is
# permanently red teaches everyone to ignore it, which is the same defect as
# a gate that never runs -- and this repo has already paid for that once, when
# `capped` was recorded as passing for two commits while the structure it
# guards had never been evaluated at all.
#
# A skipped case is announced rather than silently dropped, so a reader of the
# log can never mistake a filtered run for a full one.
run() {
  local name="$1"; shift
  case " ${SKIP_CASES:-} " in *" $name "*) echo "--- $name (SKIPPED via SKIP_CASES)"; return 0 ;; esac
  if [ -n "${ONLY_CASES:-}" ]; then
    case " $ONLY_CASES " in *" $name "*) ;; *) return 0 ;; esac
  fi
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
#    its attachment bonus failed wherever it stood.
#
#    **The cut case's bar is cells, not events, and the swap was forced by a
#    measurement.** `min_overloaded` measured 56 when written and the bar was
#    5; the quench-crust work then took the identical cut from 11 overload
#    events carrying 2,398 cells to 4 carrying 2,197 -- coarser pieces, the
#    same room coming down (2,742 total failing cells against 2,713), and a
#    bar that reads "at least 5 events" calls that a regression. It was
#    measuring granularity, which is the very thing that change set out to
#    coarsen. `min_failing_cells` asks the question the case is named for --
#    does cutting the wall bring the room down -- and `CLAUDE.md` prefers a
#    sum over a count for exactly this reason. Measured 2,713; the bar is
#    1,800.
#
#    **Both bars are kept, and that is the merge, not indecision.** Two
#    branches hit this same mode shift independently and each replaced the
#    event count with a different outcome: `min_failing_cells` asks how much
#    of the room gave way, `max_cave` asks whether the roof is still up.
#    They are not the same question -- a room can shed 1,800 cells off its
#    walls with the ceiling intact, and a ceiling can come down in fewer.
#    Carrying both is strictly stricter than either and re-bars nothing;
#    `CLAUDE.md`'s "a guard test must be able to fail for the *replacement*
#    artifact" is the argument for keeping the pair rather than picking.
#
#    **`roomcut`'s bar was an event count and is now an outcome.** It read
#    `min_overloaded=5` (measured 56 when it was written). `c089aa2` reshaped
#    what a failing region is -- boundary erosion, and fragments separating
#    along fissures -- and the room's collapse merged from thirty-seven
#    separate failures into **one** paced failure of 1,903 cells. Same roof,
#    same rubble on the floor, one event. Measured side by side against
#    `origin/main`, roofed void as a percentage of what was there at the cut:
#
#        frame        2     200     400     800
#        main      100%     20%     22%     22%
#        this      100%     24%     18%     18%
#
#    The ceiling comes down on both, and slightly *further* here by frame
#    400. What changed is that it arrives as one staged collapse rather than
#    a shower of separate ones, and lands a little later -- main is down by
#    frame ~150, this by ~350. So the bar is now `max_cave=40`: set from the
#    24% measured with sixteen points of headroom, against 100% for a roof
#    that never moves. **The timing change is real and is not hidden by
#    this** -- it is recorded in `Reports/open-bugs-handoff.md` for a
#    playtest verdict, because the owner has separately complained about
#    breakage arriving late.
#
#    Second time an event-count bar here has caught a mode shift rather than
#    a behaviour change; see case 6's note on `strike`.
#
#    A bigger budget than the rest, and not a tuning fudge: the room is the
#    largest structure any scene here builds and the only one that is mostly
#    *surface*, so far more of it is structurally interesting than in a solid
#    massif. Measured 29-32 ms against the 13-22 ms the others sit at, so 90
#    keeps the same ~3x margin over the measurement that 60 gives them.
run roomstands scene=room wall=5 dig=0 start=2 every=50 count=5 crop=100,120,280,200 zoom=2 max_failures=0   repeat=2 max_frame_ms=90
run roomcut    scene=room wall=5 dig=3 start=2 every=50 count=5 crop=100,120,280,200 zoom=2 min_failing_cells=1800 max_cave=40 repeat=2 max_frame_ms=90

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
#    **The shallow case is gated on the void too, as of the grain-footing
#    change.** It was `min_overloaded=50`, which contradicted the paragraph
#    above and read *which failure mode fired* rather than whether the roof
#    fell in: overload failures went 65 (3,918 cells) to 7 (169) across that
#    change while the roofed void went 678 -> 69 before and 678 -> 64 after,
#    i.e. the roof came down slightly harder and the bar called it a
#    regression. Second time in one session an event-count bar has caught a
#    mode shift and not a behaviour change -- see `roomcut`. Measured 9-10%
#    left; the bar is 20%.
#
#    `preset=flat` specifically. On sloping terrain the scene drives a
#    horizontal bore at a fixed depth below the surface *at one x*, so on a
#    hillside it leaves the hill -- `rolling` seed 24301 starts with 124
#    cells of roofed void against flat's 678, and that is the scene, not
#    the model. Two seeds because it is procedural; measured 100% on seeds
#    1, 7 and 24301, so the bar at 90 has real headroom.
run cavedeep   scene=worldcrack preset=flat seed=7 dig=4 tunnel=35 depth=18 start=2 every=600 count=4 zoom=1 min_cave=90       repeat=2 max_frame_ms=$BUDGET_MS
run cavedeep1  scene=worldcrack preset=flat seed=1 dig=4 tunnel=35 depth=18 start=2 every=600 count=4 zoom=1 min_cave=90       repeat=2 max_frame_ms=$BUDGET_MS
run caveshallow scene=worldcrack preset=flat seed=7 dig=4 tunnel=35 depth=6 start=2 every=600 count=4 zoom=1 max_cave=20       repeat=2 max_frame_ms=$BUDGET_MS

# 10b/10c. **Rock into water, and a lava quench — the hole this suite had.**
#    Every case above is structural, terrain, coldsnap or strike. None of
#    them drops anything into a liquid, and that blindness let a 600-cell
#    slab ship sitting in mid-air 30 rows above its own pond with all
#    seventeen cases green. `CLAUDE.md`: "the scenes were not too few; they
#    were blind by construction."
#
#    Gated on **where the rock ended up**, not on how many events fired and
#    not on the model's own verdict. Both `hanging:` and `afloat:` read zero
#    through the whole bug — correctly, by their own definitions, since the
#    load model believed the slab was supported. A row and a count ask the
#    world instead. Measured 0 loose solid cells left aloft in both scenes
#    against 522 with the bug; the bars are 10 and 5.
#
#    `min_bodies` alongside, because "the rock went down" and "the rock went
#    down *as pieces*" are different claims and only the second is the
#    milestone. Measured 24 and 17 concurrent bodies; the bars are 8.
run rockdrop scene=rockdrop fall=20 start=2 every=100 count=4 crop=180,150,180,170 zoom=2 max_rock_above=195,10 min_bodies=8 repeat=2 max_frame_ms=$BUDGET_MS
run lavadrop scene=lavadrop         start=2 every=300 count=4 crop=196,216,120,104 zoom=2 max_rock_above=248,5  min_bodies=8 repeat=2 max_frame_ms=$BUDGET_MS

# 11. A pond freezes over under a cold snap and nothing gives way. The
#    acceptance bar the ice milestone was set: a floating sheet's only
#    anchor is the water under it (`MaterialDef::floats`), and before that
#    existed the *first* cell to freeze in open water was a lone solid with
#    no path to an anchor and was dismantled the frame after it appeared --
#    3,969 freezes in one storm and never ten cells of ice standing.
#
#    `max_unconfined=0`, not `max_failures=0`, and the distinction was
#    forced by a real event: this case was written (and passed) on the ice
#    branch alone, then failed the moment it merged with the lava branch --
#    which gave stone `heat_conductivity: 0.1`, so the storm's cold now
#    soaks through the basin walls, gets around `weather.rs`'s
#    WATER_CHILL_DEPTH bound, and freezes the shallow pond solid to the
#    floor late in the front. The frozen block's interior then
#    crush-fissures: 4 failures, 845 cells, largest region 319, every one
#    of them *confined* (no free face), and the pond thaws back to clean
#    open water afterwards. Isolated by a control run with the stone line
#    commented out: 0 failures, matching the pre-merge measurement
#    exactly. A pond freezing through in a hard front and cracking as it
#    does is the mechanic working, not the sheet coming apart -- so the
#    bar counts what this scene is actually about (case 6's own rule):
#    unsupported failures and overloads with a free face, i.e.
#    dismantling, which stays at zero across freeze-over, the drift, AND
#    the thaw -- count=6 runs the full arc to frame 1080 now that the bar
#    holds through it.
#
#    The bar is real rather than a formality: at ice.ron's
#    `max_unsupported_span` of 16 the same run measures 17 overloads
#    (2,210 cells) and 118 unsupported (289) -- dismantling, which this
#    gate fails on -- and at 32 it measures 6 (924). The scene is
#    deterministic -- weather is a pure function of `(seed, frame)` and
#    the scene pins both -- so this is not a sampled case the way the
#    `worldcrack` seeds are.
#
#    `repeat=3` rather than 2, and that is a measurement not a habit: this
#    is the busiest scene in the file -- a storm keeps the whole surface of
#    the world awake -- so its worst frame is the most contention-sensitive
#    here. Measured at **17.85 ms** as the minimum over three runs, with
#    single runs on a loaded machine reaching 58; two samples landed on 46
#    once, which is close enough to the 60 ms bar to flake. Three keeps the
#    same ~3x margin the other cases have.
run coldsnap scene=coldsnap start=180 every=180 count=6 crop=180,228,160,44 zoom=3 max_unconfined=0 repeat=3 max_frame_ms=$BUDGET_MS

# 15. A pond freezes over, and stops.
#
#    The case above watches the first eighteen seconds of the spell, which
#    is the busy part and the right place for the frame budget. It cannot
#    see either half of what play reported, because both are about where
#    the freeze *ends up*:
#
#    - "it never really freezes and has snow accumulate on top. The pixels
#      seem to be constantly shifting" -- a pond can hold hundreds of ice
#      cells forever as a churning slush and never close. Gated as the
#      **coverage floor**: 50 of 60 columns frozen at the surface, measured
#      at 60 and previously stuck at 3.
#    - the fix for it, left unbounded, freezes the pond solid. Gated as the
#      **ceiling on the ice itself**: 700 cells, measured at 450 with
#      `weather::SHEET_MAX_THICKNESS` and **823** with it removed, so this
#      is the case that guards that constant. It cannot be guarded in a
#      unit fixture -- the cap limits how far the sweep reaches *through*
#      what is already frozen, and it is a lying drift that spends that
#      budget, so a dry fixture never sees it. Checked: a unit test written
#      against it passed with the cap taken out.
#
#    No `max_frame_ms`: this runs 10,800 frames rather than 1,080 and its
#    worst frame spreads 27 to 43 ms run to run, which is too wide to gate
#    on. The short case above holds the frame budget for this scene.
run coldsheet scene=coldsnap start=1800 every=1800 count=6 crop=180,228,160,44 zoom=2 ice=50,700 repeat=2

# 6. A struck cliff throws pieces. Asserted as *bodies in flight*, not as
#    overload failures: the mechanism here is the blow's own fracture, and
#    an earlier bar on overload failures duly broke when an unrelated
#    change to the fragment ladder shifted how many separate events the
#    same material came away in. Measure what the scene is about.
run strike   scene=strike   start=2 every=60 count=4 crop=200,90,120,120 zoom=3 min_bodies=2 repeat=2 max_frame_ms=$BUDGET_MS

# 8. The gnome gets through a wood.
#
#    The character path had no gated case at all before this, which is how
#    a gnome who could be *walled in by a tree* went unnoticed until it was
#    played: on this very scene he travelled 0 cells and spent the run
#    BURIED, having been entombed by a crown that grew over the spot he was
#    standing on.
#
#    Distance, not a picture, for the same reason `min_overloaded` is a
#    count rather than a screenshot: a gnome stopped against a trunk and a
#    gnome standing inside one are the same few pixels at any zoom a sheet
#    is read at.
#
#    Measured 362 cells over the 600 ticks after he sets off, past six of
#    the eight trees; the bar is set below that with headroom, and the
#    failure it guards is 0. No frame-cost bar: this scene's worst frame is
#    six thousand frames of tree *growth* before he takes a step, which is
#    nothing to do with the gnome and sits close enough to the budget to
#    flake on other hardware. `repeat=1` for the same reason -- there is no
#    timing claim here to stabilise, and the run is 26 s.
run wood     scene=wood     start=6600 every=1 count=1 crop=0,140,512,180 zoom=2 min_travelled=200 repeat=1

echo
if [ "$fails" -gt 0 ]; then
  echo "acceptance: $fails case(s) FAILED -- images in $OUT"
  exit 1
fi
echo "acceptance: all cases met their expectations -- images in $OUT"
