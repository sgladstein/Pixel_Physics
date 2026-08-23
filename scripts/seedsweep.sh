#!/usr/bin/env bash
# Sweep a destructive verb over presets x seeds and report order statistics.
#
# # Why this exists, and why a ninth hand-authored scene would not do
#
# `scripts/acceptance.sh` is the gate; this is the instrument. All eight of
# the original acceptance cases stayed green through a change that made one
# world seed lose 26x more material to a single dig, and they were not too
# few -- they were blind *by construction*, because `seed=` reaches only two
# scenes and every structural case builds hand-placed geometry at the
# default seed. That happened twice in one session, the second change eating
# fifty times more world than the bug it was fixing. Both were caught in one
# command, and this is that command.
#
# So: **run this before and after any change to the load, bearing or
# fracture model**, not just the acceptance suite. Green acceptance on its
# own would have shipped both.
#
# # Read the order statistic, not any single seed
#
# Outcomes here are chaotic in the seed -- seed 7 and seed 1 have differed
# 25x on identical preset parameters -- so *which* seed is worst reshuffles
# on any legitimate change, and a per-seed baseline gets rubber-stamped
# within two commits. The numbers to compare across a change are the **max**
# and **p90** printed at the end.
#
# # What it measures
#
# `cells lost since the cut`: the world's Solid+Powder census after the
# scene is built, minus the census at the end. A *failure* count is not a
# *damage* count -- a failed cell that became rubble is still standing
# there, and two digs whose event counts looked comparable removed 894 and
# 23,042 cells. See `Args::max_lost` in `examples/filmstrip.rs`.
#
# Usage:
#   scripts/seedsweep.sh                          # dig=6, the default sweep
#   scripts/seedsweep.sh strike=12                # the big blow (1c)
#   scripts/seedsweep.sh dig=4 tunnel=8           # the gnome's bore envelope
#   PRESETS="flat canyon" SEEDS="1 7" scripts/seedsweep.sh strike=20
#
# Any further arguments are passed to `filmstrip` verbatim, so `yield=0.0`
# or `relax=1` can ride along.
set -uo pipefail

PRESETS="${PRESETS:-rolling terraced wetland arid flat canyon}"
SEEDS="${SEEDS:-1 3 7 24301}"
FRAMES="${FRAMES:-start=2 every=400 count=4}"
VERB="${*:-dig=6}"
# Plain path first, `.exe` as the fallback: this repo is built on Windows
# *and* Linux, and the default used to be Windows-only.
FILM="${FILM:-target/release/examples/filmstrip}"
if [ ! -x "$FILM" ] && [ -x "$FILM.exe" ]; then
  FILM="$FILM.exe"
fi
OUT="${OUT_DIR:-target/filmstrips}"
mkdir -p "$OUT"

if [ ! -x "$FILM" ]; then
  echo "build it first: cargo build --release --example filmstrip" >&2
  exit 2
fi

echo "sweep: $VERB over [$PRESETS] x [$SEEDS]"
# `rock` is the damage and `lost` is the removal: rock turning to rubble
# moves nothing out of the world, and a run can chew a whole surface layer
# to gravel while `lost` stays near zero. Read both.
printf '%-10s %7s %9s %9s %9s %9s\n' preset seed lost rock overload largest
losses=""
rocks=""
for preset in $PRESETS; do
  for seed in $SEEDS; do
    # shellcheck disable=SC2086
    out=$("$FILM" scene=worldcrack preset="$preset" seed="$seed" $VERB $FRAMES \
          zoom=1 out="$OUT/sweep-$preset-$seed.png" 2>&1)
    census=$(echo "$out" | grep "cells lost since the cut" | tail -1)
    lost=$(echo "$census" | sed -nE 's/.*cut: (-?[0-9]+) .*/\1/p')
    rock=$(echo "$census" | sed -nE 's/.*rock (-?\+?[0-9]+),.*/\1/p' | tr -d '+')
    over=$(echo "$out" | grep "failures: overloaded" | tail -1 | sed -E 's/.*overloaded ([0-9]+).*/\1/')
    big=$(echo "$out" | grep "failing region size" | tail -1 | sed -E 's/.*largest ([0-9]+).*/\1/')
    # An empty parse is a broken parser, not a zero -- an earlier version
    # silently reported "max 0" across a whole sweep because the census
    # line had gained a parenthetical it did not match. Say so loudly.
    printf '%-10s %7s %9s %9s %9s %9s\n' "$preset" "$seed" "${lost:-PARSE?}" "${rock:-PARSE?}" "${over:-0}" "${big:-0}"
    losses="$losses ${lost:-}"
    rocks="$rocks ${rock:-}"
  done
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
  #
  # **`total` is reported because it is the only stable number here.** At
  # these sample counts p90 is an extreme order statistic wearing a
  # percentile's clothes: nearest-rank puts it at the *second worst* of 18
  # runs and the *third worst* of 24, so one chaotic seed moves it bodily.
  # Not hypothetical -- a review of the load-sharing change ran this with 3
  # seeds instead of 4 and reported an **84% regression that did not exist**
  # (p90 4,358 against a true 2,351). A sum over every run cannot do that,
  # and it is what settled the same question: +5.5%.
  #
  # Read them together. `total` says whether the change costs anything on
  # aggregate; `max` and `p90` say whether it has a new worst case; a gap
  # between them means the distribution reshuffled, which is the normal
  # state of affairs here and not a regression on its own.
  echo "$2" | tr ' ' '\n' | grep -E '^-?[0-9]+$' | sort -n | awk -v label="$1" '
    { v[NR] = $1; if ($1 > 0) total += $1 }
    END {
      if (NR == 0) { print label ": NO RESULTS PARSED -- the census line format probably changed"; exit }
      printf "%s over %d runs: total %d, max %d, p90 %d, median %d, min %d\n", label, NR, total, v[NR], v[int((NR * 9 + 9) / 10)], v[int((NR+1)/2)], v[1]
    }'
}

# Loud, because the alternative is a plausible-looking number. Anything
# short of the full grid puts p90 within one seed of the maximum, and the
# people most likely to trim the seed list are the ones in a hurry.
sample_count=$(echo "$losses" | tr ' ' '\n' | grep -cE '^-?[0-9]+$')
if [ "$sample_count" -lt 24 ]; then
  echo "!! WARNING: only $sample_count runs. p90 is within one seed of the max at this size and is"
  echo "!!          not comparable across changes -- a 3-seed sweep once reported an 84% regression"
  echo "!!          that did not exist. Read total, or restore the full PRESETS x SEEDS grid."
fi
stats "cells lost" "$losses"
stats "rock destroyed" "$(echo "$rocks" | tr ' ' '\n' | awk 'NF { print -$1 }')"
