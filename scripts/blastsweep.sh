#!/usr/bin/env bash
# Nine charges into a *generated rolling world*, over N seeds, with order
# statistics -- the standing acceptance artifact for the explosion work.
#
# # Why this exists, and why the eight acceptance scenes could not do it
#
# Every explosion scene this branch has (`boom_stone`, `sandbed`, `cavern`,
# `worked`, `capped`, `ligament`, `room`, `strike`) is **hand-placed geometry
# at one seed**. The owner plays a generated rolling world. That gap is how
# nine sessions of work came to read correctly on every contact sheet and
# read as "a graphic, not physics" in the hand:
#
#   > nine explosions produced nine near-identical small black holes
#   > surrounded by thin criss-cross wiggly crack patterns. Cracks that
#   > fully surround chunks of rock do not break them off -- no pieces move,
#   > ever.
#
# So the bar this script exists to answer is a **picture** one: the panels
# sheet must read as nine *different*, satisfying explosions with visible
# breakage, and the whole-world sheet must go quiet afterwards. The numbers
# below are the "did it fire at all" counters that stand next to the picture
# -- `CLAUDE.md` is emphatic that two very different mechanisms look
# identical at the zoom a contact sheet is read at, and that only a counter
# tells them apart.
#
# # Why `blast=` and not `explode=`
#
# A fixed `y` is a different situation on every seed -- open sky on one,
# bedrock on the next -- so a sweep over absolute coordinates measures the
# terrain rather than the change. `blast=x,depth,...` is depth below the
# local solid surface, resolved at the frame it fires, so one charge list is
# valid on every seed and a later charge fired into an earlier one's crater
# sees the crater. `CLAUDE.md`: a guard over a procedural system has to
# sweep the procedure.
#
# # Read the order statistic, not any single seed
#
# Outcomes here are chaotic in the seed -- seed 7 and seed 1 have differed
# 25x on identical preset parameters -- so *which* seed is worst reshuffles
# on any legitimate change and a per-seed baseline gets rubber-stamped
# within two commits. The numbers to compare across a change are the **max**
# and **p90** printed at the end. `stats` below is copied verbatim from
# `scripts/seedsweep.sh` rather than re-derived, so the two sweeps cannot
# quietly disagree about what p90 means.
#
# # Two filmstrip runs per seed, and why
#
# The whole-world sheet wants `zoom=1` (512x320 seven times over, read at a
# glance) and the panels sheet wants `zoom=2` (a 150x130 crop is unreadable
# at 1:1). `zoom` is one argument and `panels=` deliberately does not add a
# second one -- a sheet with its own private zoom is a sheet that drifts out
# of step with the one beside it. So: run A is the whole-world sheet and is
# the run every number below is parsed from; run B exists only to draw the
# panels at 2x. Both fire the identical charge list, so they are the same
# world twice.
#
# # This is not in `scripts/acceptance.sh`, deliberately
#
# It is a long run (two 5,000-frame passes per seed) and acceptance is a
# gate. Wiring it in is a separate decision, once the bars are known.
#
# Usage:
#   scripts/blastsweep.sh                       # the default four seeds
#   SEEDS="1" scripts/blastsweep.sh             # one seed, for a quick look
#   PRESET=terraced scripts/blastsweep.sh
#   CHAIN=chain_reach=48 scripts/blastsweep.sh
#
# Any further arguments are passed to `filmstrip` verbatim, so `yield=0.0`
# or `relax=1` can ride along.
set -uo pipefail

SEEDS="${SEEDS:-1 3 7 24301}"
PRESET="${PRESET:-rolling}"
CHAIN="${CHAIN:-}"
# `seedsweep.sh` defaulted this to a `.exe` path; this repo is built on
# Windows *and* Linux. Plain path first, `.exe` as the fallback.
FILM="${FILM:-target/release/examples/filmstrip}"
if [ ! -x "$FILM" ] && [ -x "$FILM.exe" ]; then
  FILM="$FILM.exe"
fi
OUT="${OUT_DIR:-target/filmstrips}"
EXTRA="$*"
mkdir -p "$OUT"

if [ ! -x "$FILM" ]; then
  echo "build it first: cargo build --release --example filmstrip" >&2
  exit 2
fi

# The nine charges, one list for every seed, in fire order. Frames are 400
# apart so each blast's aftermath is over before the next fires -- the whole
# point is nine *separable* outcomes, not one continuous mess.
#
#   #  x    depth  r   strength  frame   what it is for
#   1  470  -8     20  180       200     airburst over the ridge
#   2  430  1      14  180       600     surface bite, small charge
#   3  390  6      20  180       1000    shallow crater
#   4  350  14     20  180       1400    shallow-buried
#   5  300  30     20  180       1800    buried
#   6  250  60     20  180       2200    deep
#   7  210  10     28  240       2600    big charge, shallow, on the slope
#   8  120  8      20  180       3000    seabed, water overhead
#   9  60   25     20  180       3400    deep under the sea
CHARGES="blast=470,-8,20,180,200 \
blast=430,1,14,180,600 \
blast=390,6,20,180,1000 \
blast=350,14,20,180,1400 \
blast=300,30,20,180,1800 \
blast=250,60,20,180,2200 \
blast=210,10,28,240,2600 \
blast=120,8,20,180,3000 \
blast=60,25,20,180,3400"

# The run continues to frame 5000 so the last blast has 1,600 frames to go
# quiet in. "Goes quiet" is half the bar and it is a *trajectory*: a world
# still eating itself and one that has stopped look identical in a single
# tile, which is why `awake` and `sites` are read at the last tile and not
# at the bang.
FRAMES="start=200 every=800 count=7"

parse_fail=0
# An empty parse is a broken parser, not a zero -- `seedsweep.sh` records a
# whole sweep that silently reported "max 0" because a census line had
# gained a parenthetical it did not match. Say so loudly, and fail.
#
# **Checked here in the main shell rather than inside the `$(...)` that
# prints the value**, and that is not a style preference: a command
# substitution is a subshell, so a `parse_fail=1` set inside one is
# discarded when it exits. The first version of this script did exactly
# that and printed a page of `PARSE?` while exiting 0 -- a silent-zero
# sweep wearing a warning label, which is worse than either.
check() {
  # check <name> <value...>
  local name="$1"
  shift
  for v in "$@"; do
    if [ -z "$v" ]; then
      echo "  PARSE? -- could not read '$name' out of the run" >&2
      parse_fail=1
      return
    fi
  done
}

losses=""
rocks=""
promoted=""
reaches=""
damages=""
confineds=""
honests=""
sites_all=""
awakes=""
frames_ms=""

for seed in $SEEDS; do
  echo "=============================================================="
  echo "seed $seed, preset $PRESET"
  sheet="$OUT/blast-$PRESET-$seed.png"
  panels="$OUT/blast-$PRESET-$seed-z2.png"

  # Run A -- the whole-world sheet, and the run every number below comes
  # from. One run, not several: a number stitched together out of two runs
  # of a chaotic system is a number about neither of them.
  # shellcheck disable=SC2086
  out=$("$FILM" scene=worldgen preset="$PRESET" seed="$seed" $CHARGES $CHAIN $FRAMES \
        zoom=1 out="$sheet" $EXTRA 2>&1)

  # Run B -- the panels sheet at 2x. Its own main sheet is the final frame
  # of the world at 2x, which is cheap and worth having.
  # shellcheck disable=SC2086
  outp=$("$FILM" scene=worldgen preset="$PRESET" seed="$seed" $CHARGES $CHAIN \
         start=5000 every=1 count=1 cols=1 crop=0,0,512,320 zoom=2 \
         panels=150,130,60,900 out="$panels" $EXTRA 2>&1)

  # Verbatim, both of them. `boom:` says where each charge actually landed
  # once `blast=`'s depth was resolved against the terrain, and the
  # per-site `blast report (x, y)` lines are the reason `Blasts` grew a
  # queue: `last_blast_report` is one slot, so eight of these nine were
  # being silently overwritten.
  echo "$out" | grep -E '^  boom:'
  echo "$out" | grep -E '^  blast report \('

  last_tile=$(echo "$out" | grep -E '^  tile ' | tail -1)
  awake=$(echo "$last_tile" | sed -nE 's/.*awake ([0-9]+)\/([0-9]+).*/\1/p')
  awake_of=$(echo "$last_tile" | sed -nE 's/.*awake ([0-9]+)\/([0-9]+).*/\2/p')
  sites=$(echo "$last_tile" | sed -nE 's/.*sites ([0-9]+).*/\1/p')
  fail_line=$(echo "$out" | grep -E '^    failures: overloaded ' | tail -1)
  over=$(echo "$fail_line" | sed -nE 's/.*overloaded ([0-9]+) .*/\1/p')
  unsup=$(echo "$fail_line" | sed -nE 's/.*unsupported ([0-9]+) .*/\1/p')
  # **The containment column, and it changed under this parser on purpose.**
  # The line this used to read ("furthest a failure landed from its
  # trigger") was mislabelled: it holds `max_chain_reach`, the Manhattan
  # distance from a checked cell to its failing ancestor, bounded to
  # `ROOTWARD_CHECK_STEPS` hops -- it read **1 cell** on a rolling-world
  # blast that was tearing the hillside apart. `max_damage_reach` is the
  # real one: Chebyshev distance from the nearest *live* disturbance to a
  # cell that was actually destroyed, in the same units as the `F9` setting,
  # so it can be read straight against `CHAIN` above. The old statistic is
  # kept as a second row rather than deleted -- it answers a different, real
  # question (how far rootward the chain walk finds its failures).
  damage_line=$(echo "$out" | grep -E 'furthest damage landed from a live disturbance' | tail -1)
  damage=$(echo "$damage_line" | sed -nE 's/.*disturbance: ([0-9]+) cells.*/\1/p')
  chainmode=$(echo "$damage_line" | sed -nE 's/.*chain_reach = ([^)]+)\).*/\1/p')
  reach=$(echo "$out" | grep -E "furthest a failure.s root was" | tail -1 | sed -nE 's/.*checked: ([0-9]+) cells.*/\1/p')
  paced=$(echo "$out" | grep -E 'paced across ticks' | tail -1 | sed -nE 's/.*ticks: ([0-9]+) slice.*/\1/p')
  # The only parsed field that is a *displacement* rather than a judgement.
  # `over`/`unsup` above are recorded before the free-face test, the
  # erosion, the slicing and the fracture, so they can both be in the
  # hundreds on a run where nothing whatsoever moved -- which is exactly
  # the shape of "no pieces move, ever" against this sweep's own numbers.
  # All three parts are kept: bodies and cells promoted are the blocks,
  # cells shattered is the grit, and the sweep is here to find out whether
  # either of them is zero.
  moved_line=$(echo "$out" | grep -E 'of those, actually moved' | tail -1)
  moved_bodies=$(echo "$moved_line" | sed -nE 's/.*moved: ([0-9]+) bodies.*/\1/p')
  moved_cells=$(echo "$moved_line" | sed -nE 's/.*\(([0-9]+) cells promoted\).*/\1/p')
  grit=$(echo "$moved_line" | sed -nE 's/.*, ([0-9]+) cells shattered.*/\1/p')
  census=$(echo "$out" | grep -E 'cells lost since the cut' | tail -1)
  lost=$(echo "$census" | sed -nE 's/.*cut: (-?[0-9]+) .*/\1/p')
  rock=$(echo "$census" | sed -nE 's/.*rock (-?\+?[0-9]+),.*/\1/p' | tr -d '+')
  ms=$(echo "$out" | grep -E 'worst frame so far' | tail -1 | sed -nE 's/.*so far: ([0-9.]+) ms.*/\1/p')
  bodies=$(echo "$out" | grep -E 'peak chunk bodies in flight' | tail -1 | sed -nE 's/.*at once: ([0-9]+).*/\1/p')
  cracked=$(echo "$out" | grep -E 'cracked cells in the world' | tail -1 | sed -nE 's/.*world: ([0-9]+).*/\1/p')
  # **The counter `structural.rs` names and this script has never printed.**
  # Its confined branch says outright: "The number to watch is
  # `FailureCounts::confined` in `scripts/blastsweep.sh`: it climbing without
  # bound is the treadmill." It was never in the parse set -- `filmstrip`
  # printed it and this script discarded the tile it was on. It is also the
  # counter the licence clip moves *most*: clipping a region only removes
  # cells, and removed cells are solid rock, so it can only cost free-face
  # witnesses and `confined` can only move up.
  confined=$(echo "$out" | grep -E 'of those, confined \(no free face' | tail -1 | sed -nE 's/.*anywhere\): ([0-9]+) .*/\1/p')
  # The containment measure that is **not** the gate restated -- see
  # `filmstrip::damage_radius`. `max_damage_reach` is recorded only at sites
  # downstream of the licence clip, so it is `<= chain_reach` by arithmetic
  # and cannot report a containment failure at all. This one compares two
  # material grids and measures a distance, so it can.
  honest=$(echo "$out" | grep -E 'furthest cell this run actually changed' | tail -1 | sed -nE 's/.*made it: (-?[0-9]+) cells.*/\1/p')

  panels_line=$(echo "$outp" | grep -E '^panels sheet' | sed -E 's/.*: //')

  check "failures" "$over" "$unsup"
  check "furthest damage landed from a live disturbance" "$damage" "$chainmode"
  check "furthest a failure's root was from the cell that was checked" "$reach"
  check "paced across ticks" "$paced"
  check "of those, actually moved" "$moved_bodies" "$moved_cells" "$grit"
  check "cells lost since the cut" "$lost" "$rock"
  check "cracked cells in the world" "$cracked"
  check "of those, confined (no free face" "$confined"
  check "furthest cell this run actually changed" "$honest"
  check "awake / sites at the last tile" "$awake" "$awake_of" "$sites"
  check "worst frame" "$ms"
  check "peak chunk bodies" "$bodies"
  check "panels sheet" "$panels_line"

  echo "  at the last tile:"
  echo "    failures: overloaded ${over:-PARSE?} / unsupported ${unsup:-PARSE?}"
  echo "    furthest damage landed from a live disturbance: ${damage:-PARSE?} cells (chain_reach = ${chainmode:-PARSE?})"
  echo "    furthest a failure's root was from the cell checked: ${reach:-PARSE?} cells"
  echo "    paced across ticks: ${paced:-PARSE?} slice(s)"
  echo "    actually moved: ${moved_bodies:-PARSE?} bodies (${moved_cells:-PARSE?} cells promoted), ${grit:-PARSE?} cells shattered"
  echo "    cells lost since the cut: ${lost:-PARSE?} (rock ${rock:-PARSE?})"
  echo "    cracked cells in the world: ${cracked:-PARSE?}"
  echo "    confined failures (no free face anywhere): ${confined:-PARSE?}"
  echo "    furthest rock this run actually lost, from its charge: ${honest:-PARSE?} cells"
  echo "    awake ${awake:-PARSE?}/${awake_of:-PARSE?}, sites ${sites:-PARSE?}"
  echo "    worst frame: ${ms:-PARSE?} ms, peak chunk bodies ${bodies:-PARSE?}"
  echo "  sheets: $sheet"
  echo "          ${panels_line:-PARSE?}"

  losses="$losses ${lost:-}"
  rocks="$rocks ${rock:-}"
  promoted="$promoted ${moved_cells:-}"
  reaches="$reaches ${reach:-}"
  damages="$damages ${damage:-}"
  confineds="$confineds ${confined:-}"
  honests="$honests ${honest:-}"
  sites_all="$sites_all ${sites:-}"
  awakes="$awakes ${awake:-}"
  # `stats` takes integers (see its own note); the worst frame is the only
  # float here, so it is rounded on the way in. The unrounded value is on
  # the per-seed line above. Nothing is appended when it did not parse --
  # a `0` here would be a silent zero in the order statistic, which is the
  # exact failure the `check`es above exist to prevent.
  if [ -n "$ms" ]; then
    frames_ms="$frames_ms $(printf '%.0f' "$ms")"
  fi
done

# Order statistics over the whole sweep. p90 by nearest-rank on the sorted
# list, which for a sweep this size is "the second worst" -- deliberately
# not a mean: one seed cascading while the rest are clean is exactly the
# regression this is for, and a mean hides it.
echo
stats() {
  # `sort -n` then nearest-rank, and the sign convention differs between
  # the two columns: `lost` is worst when most positive, `rock` when most
  # negative (rock only ever goes away). So `rock` is negated on the way in
  # and reported back as a magnitude.
  echo "$2" | tr ' ' '\n' | grep -E '^-?[0-9]+$' | sort -n | awk -v label="$1" '
    { v[NR] = $1 }
    END {
      if (NR == 0) { print label ": NO RESULTS PARSED -- the census line format probably changed"; exit }
      printf "%s over %d runs: max %d, p90 %d, median %d, min %d\n", label, NR, v[NR], v[int((NR * 9 + 9) / 10)], v[int((NR+1)/2)], v[1]
    }'
}
stats "cells lost" "$losses"
stats "rock destroyed" "$(echo "$rocks" | tr ' ' '\n' | awk 'NF { print -$1 }')"
# Read this one from the *bottom*. Every other statistic here is worst at
# its max -- more rock eaten, a failure landing further out, a slower
# frame. This one is worst at its **min**: a seed that promoted nothing is
# a seed where nine charges produced no moving rock at all, and the max
# cannot see it. `stats` prints min alongside max for exactly this reason,
# and the sign is not flipped on the way in because both ends are worth
# reading -- an enormous max is its own kind of report.
stats "promoted cells" "$promoted"
# **The containment statistic.** How far past the nearest live disturbance
# damage was still landing, in the units `chain_reach` is set in -- so at
# `CHAIN=chain_reach=48` a max of 200 here says the leash is not holding,
# and there is no other number in this sweep that can say it.
# **Read this one knowing what it cannot say.** It is `max_damage_reach`,
# recorded only downstream of `clip_region_to_licence`, and for any cell that
# clip retains `within_disturbance` guarantees a live disturbance within
# `chain_reach + extent` while `distance_to_live_disturbance` takes the *min*
# over disturbances of `distance - extent`. So it is `<= chain_reach` by
# arithmetic at every site: a run reading exactly the leash is a saturated
# ceiling, not a measurement. Kept beside the honest one so the pair reads.
stats "furthest damage landed from a live disturbance (CEILING)" "$damages"
# The containment statistic that can actually go wrong: rock that stopped
# being rock, measured from the charge that made it, reading none of the
# licence machinery. This is the one to compare against CHAIN.
stats "furthest rock lost, from its charge" "$honests"
# The treadmill reading. Climbing without bound across a change is the
# re-crush treadmill coming back.
stats "confined failures" "$confineds"
# Kept, demoted: this is `max_chain_reach`, how far rootward the chain walk
# had to go to find the cell that gave way. A real question, and not the
# containment one -- it is bounded to `ROOTWARD_CHECK_STEPS` by construction
# and read 1 cell on a blast that was eating the hillside.
stats "furthest a failure's root was from the cell checked" "$reaches"
# The two quiet statistics. Both are read at the last tile, 1,600 frames
# after the final charge -- a world that has stopped eating itself and one
# that is still going look identical in any single picture.
stats "sites at the final tile" "$sites_all"
stats "awake chunks at the final tile" "$awakes"
stats "worst frame ms" "$frames_ms"

if [ "$parse_fail" -ne 0 ]; then
  echo
  echo "blastsweep: at least one field failed to parse (PARSE? above) -- a sweep that silently reports zeros is worse than no sweep" >&2
  exit 1
fi
