#!/usr/bin/env bash
# Census every worldgen pass over presets x seeds, and gate order statistics.
#
# # Why this exists
#
# `scripts/seedsweep.sh` is the same instrument pointed at the destruction
# model; this one is pointed at worldgen, for the reason CLAUDE.md gives
# directly: **build the sweep before changing a model that governs
# procedural content**, and gate an order statistic rather than a seed.
# Worldgen is procedural content, so every task after this one lands behind
# this command.
#
# The specific blindness it exists to remove is already on the record. The
# August 2026 world review found `brows` writing 34 cells and `talus` 148 in
# a ~1.3M-cell world -- passes that had been silently near-dead for however
# long, while every render still looked like a plausible world and every
# test stayed green. Before that, `talus` wrote *nothing at all* in any world
# for a while, and what caught it was a per-pass counter, not a picture. A
# pass can go to zero, or explode, without a single test noticing.
#
# # Read the order statistic, not any single seed
#
# Outcomes here are chaotic in the seed, so *which* seed is worst reshuffles
# on any legitimate change and a per-seed baseline gets rubber-stamped
# within two commits. The numbers that travel across a change are the **p90
# and max per preset**, which is what the baseline file holds.
#
# # Flagged means look, not fail
#
# `compare` flags any per-preset p90 that moves more than +/-30%. That is
# deliberately not an exit-code gate: a task that reworks a pass *should*
# move its p90, and the point of the flag is that the move is visible and
# attributed rather than silent. What a flag asks is "did you mean this pass,
# and only this pass". Two flags are special-cased and always worth reading
# twice, because they are the failure this file was written for:
#
#   ZEROED   a pass that used to write cells now writes none -- the
#            brows/talus failure, exactly
#   WOKE     a preset that used to arrive asleep now has awake chunks, which
#            defeats `field::step`'s early-out for the whole world
#
# # Usage
#
#   scripts/worldgen_sweep.sh                 # run and print the table
#   scripts/worldgen_sweep.sh baseline        # (re)write the committed TSV
#   scripts/worldgen_sweep.sh compare         # re-run and diff against it
#   PRESETS="canyon arid" SEEDS="1 2 3" scripts/worldgen_sweep.sh
#
# Sixteen seeds x six presets is 96 runs of a **512x320** generation, about
# 25 seconds. Do not sweep at the shipped 8192x2560: it is 128x the work
# (generation scales with area, and 8192x2560 is 16x the width and 8x the
# height of 512x320) for the same answer, because what is being compared is
# a ratio against the same size on both sides.
set -uo pipefail

MODE="${1:-run}"
PRESETS="${PRESETS:-rolling terraced canyon wetland arid flat}"
SEEDS="${SEEDS:-1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16}"
BASELINE="${BASELINE:-scripts/worldgen_sweep_baseline.tsv}"
OUT="${OUT_DIR:-target/worldgen-sweep}"

# `.exe` on Windows, bare on everything else. Checked rather than assumed:
# `seedsweep.sh` hardcodes the Windows name and silently tells you to build
# something you have already built if you are not on it.
FILM=""
for candidate in "${FILM_OVERRIDE:-}" target/release/examples/filmstrip target/release/examples/filmstrip.exe; do
  [ -n "$candidate" ] && [ -x "$candidate" ] && { FILM="$candidate"; break; }
done
if [ -z "$FILM" ]; then
  echo "build it first: cargo build --release --example filmstrip" >&2
  exit 2
fi
mkdir -p "$OUT"

# One build, then binary runs. `cargo run` per seed re-checks the whole
# dependency graph 96 times and, worse, would hide the `include_str!` trap:
# an asset edit mid-sweep would take effect for some runs and not others.
# Running the binary directly means every point of a sweep is the same
# build, which is the property that makes the numbers comparable.

# --- collect ---------------------------------------------------------------
# Raw rows, one per (preset, seed, counter): preset \t counter \t value.
# `awake` rides along as a pseudo-counter so the arrives-asleep guarantee is
# in the same table as the cell counts -- it is the one line here whose
# *correct* value is zero, and keeping it elsewhere is how it stops being
# looked at.
collect() {
  local raw="$1"
  : > "$raw"
  for preset in $PRESETS; do
    for seed in $SEEDS; do
      local out
      out=$("$FILM" scene=worldgen preset="$preset" seed="$seed" count=1 \
            out="$OUT/$preset-$seed.png" 2>&1)
      # The pass table: two-space-indented `name  N cells`.
      local passes
      passes=$(echo "$out" | sed -nE 's/^  ([a-z_]+) +([0-9]+) cells$/\1\t\2/p')
      # An empty parse is a broken parser, not a zero. `seedsweep.sh` reported
      # "max 0" across a whole sweep once because the census line had gained a
      # parenthetical it did not match, and a sweep that silently measures
      # nothing is worse than no sweep. Say so and stop.
      if [ -z "$passes" ]; then
        echo "PARSE FAILURE: no pass table in $preset seed $seed -- the output format changed" >&2
        echo "$out" >&2
        exit 3
      fi
      echo "$passes" | while IFS=$'\t' read -r name cells; do
        printf '%s\t%s\t%s\n' "$preset" "$name" "$cells" >> "$raw"
      done
      local awake
      awake=$(echo "$out" | sed -nE 's/.*awake ([0-9]+)\/[0-9]+.*/\1/p' | tail -1)
      if [ -z "$awake" ]; then
        echo "PARSE FAILURE: no awake line in $preset seed $seed" >&2
        exit 3
      fi
      printf '%s\t%s\t%s\n' "$preset" "awake_chunks" "$awake" >> "$raw"
    done
  done
}

# --- aggregate -------------------------------------------------------------
# Per (preset, counter): p90 by nearest rank and max, the same convention
# `seedsweep.sh` uses so the two sweeps are read the same way. Sorted keys
# throughout -- an awk array iterated in hash order would make the file's
# line order depend on the awk build, and this file is committed and diffed.
aggregate() {
  sort -k1,1 -k2,2 "$1" | awk -F'\t' '
    { key = $1 "\t" $2; n[key]++; v[key, n[key]] = $3 + 0 }
    END {
      for (key in n) keys[++k] = key
      # Insertion sort on the key strings: tiny (six presets x eleven
      # counters) and it removes awk-implementation order from a committed
      # artifact, which `gawk` and `mawk` do not otherwise agree on.
      for (i = 2; i <= k; i++) {
        t = keys[i]
        for (j = i - 1; j >= 1 && keys[j] > t; j--) keys[j + 1] = keys[j]
        keys[j + 1] = t
      }
      for (i = 1; i <= k; i++) {
        key = keys[i]; c = n[key]
        for (a = 1; a <= c; a++) s[a] = v[key, a]
        for (a = 2; a <= c; a++) {
          t = s[a]
          for (b = a - 1; b >= 1 && s[b] > t; b--) s[b + 1] = s[b]
          s[b + 1] = t
        }
        p90 = s[int((c * 9 + 9) / 10)]
        printf "%s\t%d\t%d\t%d\n", key, p90, s[c], c
      }
    }'
}

HEADER=$'#preset\tcounter\tp90\tmax\truns'

case "$MODE" in
  run|baseline)
    RAW="$OUT/raw.tsv"
    collect "$RAW"
    { echo "$HEADER"; aggregate "$RAW"; } > "$OUT/summary.tsv"
    column -t -s$'\t' "$OUT/summary.tsv" 2>/dev/null || cat "$OUT/summary.tsv"
    if [ "$MODE" = baseline ]; then
      cp "$OUT/summary.tsv" "$BASELINE"
      echo
      echo "baseline written: $BASELINE"
    fi
    ;;
  compare)
    if [ ! -f "$BASELINE" ]; then
      echo "no baseline at $BASELINE -- run: scripts/worldgen_sweep.sh baseline" >&2
      exit 2
    fi
    RAW="$OUT/raw-compare.tsv"
    collect "$RAW"
    { echo "$HEADER"; aggregate "$RAW"; } > "$OUT/summary-compare.tsv"
    echo "compare against $BASELINE  (+/-30% on p90 is flagged; flagged means LOOK, not fail)"
    awk -F'\t' -v thresh=30 '
      FNR == NR {
        if ($1 ~ /^#/) next
        base[$1 "\t" $2] = $3; basemax[$1 "\t" $2] = $4; next
      }
      $1 ~ /^#/ { next }
      {
        key = $1 "\t" $2
        if (!(key in base)) { printf "%-10s %-15s %8s -> %-8d NEW COUNTER\n", $1, $2, "-", $3; next }
        b = base[key]; c = $3
        flag = ""
        if ($2 == "awake_chunks") {
          # **Its floor is not zero, and that is measured, not assumed.**
          # A pristine `flat` world -- bare level rock, no water, no life,
          # nothing to move -- reads 3..8 of 40 chunks awake at frame 100 on
          # this baseline, with active sites climbing to one per column. So
          # "awake > 0" is not a failure signal here and a rule written as if
          # it were would flag every run forever. See the task file Findings
          # for the measurement and for why chasing it is out of this track.
          # What this row is for is the *move*: a change that adds three
          # chunks of permanent wakefulness pays the ~7 ms/frame that
          # field::step early-outs on a quiet world, and that shows up here.
          #
          # Small integer counters need an absolute floor as well as a
          # percentage: 6 -> 8 is +33% and is noise at this scale.
          d = (b > 0) ? (c - b) * 100.0 / b : 0
          if ((c - b >= 3 || b - c >= 3) && (b == 0 || d > thresh || d < -thresh))
            flag = sprintf("%+d chunks -- check the field::step early-out", c - b)
        } else if (b == 0 && c > 0) {
          flag = "was 0, now writes"
        } else if (b > 0 && c == 0) {
          flag = "ZEROED -- the pass stopped firing"
        } else if (b > 0) {
          d = (c - b) * 100.0 / b
          if (d > thresh || d < -thresh) flag = sprintf("%+.0f%%", d)
        }
        moved = (c != b || $4 != basemax[key])
        if (flag != "" || moved)
          printf "%-10s %-15s p90 %7d -> %-7d  max %7d -> %-7d  %s\n", $1, $2, b, c, basemax[key], $4, flag
        if (flag != "") flags++
      }
      END { printf "\n%d counter(s) moved past +/-%d%% or changed state\n", flags + 0, thresh }
    ' "$BASELINE" "$OUT/summary-compare.tsv"
    ;;
  *)
    echo "usage: $0 [run|baseline|compare]" >&2
    exit 2
    ;;
esac
