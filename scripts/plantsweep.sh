#!/usr/bin/env bash
# Sweep the plant economy over world seeds and report order statistics.
#
# # Why this exists
#
# `scripts/seedsweep.sh` is the destruction instrument: it sweeps
# `filmstrip scene=worldcrack` and reads a rock census. Nothing did the same
# job for the *plant* economy, and the plant economy needs it more, not
# less: `CLAUDE.md` records twelve identical trees from one genome spanning
# 31 to 153 cells, so a single `plant_probe` run is a draw from a very wide
# distribution and a per-seed baseline gets rubber-stamped within two
# commits.
#
# Genotypes are drawn from `(world seed, germination coordinate)`, so
# varying `worldseed=` re-rolls the whole population — which is what makes
# this an ensemble rather than one stand measured eight times.
#
# **Run it before changing the economy, not after.** The rule is
# `CLAUDE.md`'s: build the sweep first, gate an order statistic (p90 or max)
# rather than any single seed.
#
# # What it reads, and why these columns
#
# | column | what it answers |
# |---|---|
# | `cells` | ensemble median plant size — the headline mass |
# | `wood` | median non-leaf cells; `wood/leaf` is C1's crown-recession trajectory |
# | `leaf%` | foliage share of the plant, the other half of that trajectory |
# | `stem` | median stem thickness **above the base** — the real trunk, never `rows >1 cell wide` |
# | `roots` | median root cells per plant |
# | `root%` | root share of the plant |
# | `estab` | plants that reached >= 20 cells — the establishment rate |
# | `born`/`died` | `World::organism_turnover`, the selection-throughput gate |
# | `inhrt` | established plants carrying an *inherited* genome. At 0 every claim from the run is about founders (`plant-evolution-design.md` §5). |
# | `senes` | organisms marked senescent — the mortality tell |
# | `top` | canopy top row. **0 means the run hit the ceiling and every shape number in it is void** (`common::PlantScene`). |
#
# Usage:
#   scripts/plantsweep.sh                         # 8 seeds, 28,800 frames
#   FRAMES=45000 scripts/plantsweep.sh            # long enough for turnover
#   SEEDS="1 2" TREES=4 scripts/plantsweep.sh
#   LABEL=after scripts/plantsweep.sh             # names the run in the output
#
# Any further arguments are passed to `plant_probe` verbatim, so
# `moisture=310` or `soil=100` can ride along.
set -uo pipefail

SEEDS="${SEEDS:-1 2 3 4 5 6 7 8}"
FRAMES="${FRAMES:-28800}"
TREES="${TREES:-8}"
LABEL="${LABEL:-}"
JOBS="${JOBS:-2}"
PROBE="${PROBE:-target/release/examples/plant_probe}"
if [ ! -x "$PROBE" ] && [ -x "$PROBE.exe" ]; then
  PROBE="$PROBE.exe"
fi
OUT="${OUT_DIR:-target/plantsweep}"
mkdir -p "$OUT"

if [ ! -x "$PROBE" ]; then
  echo "build it first: cargo build --release --examples" >&2
  exit 2
fi

echo "plantsweep${LABEL:+ [$LABEL]}: trees=$TREES frames=$FRAMES seeds=[$SEEDS] extra=[$*]"

# Run the seeds, at most $JOBS at once. `plant_probe` is itself rayon-
# parallel over chunks and a sparse stand leaves most cores idle, so a small
# amount of outer parallelism is close to free; a large amount just thrashes.
pids=""
for seed in $SEEDS; do
  # shellcheck disable=SC2086
  "$PROBE" trees="$TREES" frames="$FRAMES" worldseed="$seed" "$@" > "$OUT/seed-$seed${LABEL:+-$LABEL}.log" 2>&1 &
  pids="$pids $!"
  while [ "$(jobs -rp | wc -l)" -ge "$JOBS" ]; do wait -n 2>/dev/null || break; done
done
wait

printf '%6s %7s %7s %7s %6s %6s %7s %6s %6s %6s %6s %6s %6s\n' \
  seed cells wood leaf% stem roots root% estab born died inhrt senes top
rows=""
for seed in $SEEDS; do
  log="$OUT/seed-$seed${LABEL:+-$LABEL}.log"
  # An empty parse is a broken parser, not a zero -- `seedsweep.sh` learned
  # this the expensive way when a census line gained a parenthetical and a
  # whole sweep reported "max 0".
  cells=$(sed -nE 's/^  cells .*median +([0-9]+) .*/\1/p' "$log" | head -1)
  leaves=$(sed -nE 's/^  leaves .*median +([0-9]+) .*/\1/p' "$log" | head -1)
  leafpc=$(sed -nE 's/^  foliage share.*: ([0-9]+)$/\1/p' "$log" | head -1)
  stem=$(sed -nE 's/^  stem thick .*median +([0-9]+) .*/\1/p' "$log" | head -1)
  roots=$(sed -nE 's/^  root cells .*median +([0-9]+) .*/\1/p' "$log" | head -1)
  rootpc=$(sed -nE 's/^  root share of plant .*median +([0-9]+)%.*/\1/p' "$log" | head -1)
  estab=$(sed -nE 's/^population: [0-9]+ organisms -- ([0-9]+) established.*/\1/p' "$log" | head -1)
  born=$(sed -nE 's/^lineage turnover over [0-9]+ frames: ([0-9]+) born.*/\1/p' "$log" | head -1)
  died=$(sed -nE 's/^lineage turnover over [0-9]+ frames: [0-9]+ born, ([0-9]+) died.*/\1/p' "$log" | head -1)
  inhrt=$(sed -nE 's/^  established plants carrying an inherited genome: ([0-9]+) of.*/\1/p' "$log" | head -1)
  senes=$(sed -nE 's/.*; ([0-9]+) organisms senescent.*/\1/p' "$log" | head -1)
  top=$(sed -nE 's/^  canopy top +row (-?[0-9]+).*/\1/p' "$log" | head -1)
  wood=""
  if [ -n "${cells:-}" ] && [ -n "${leaves:-}" ]; then wood=$((cells - leaves)); fi
  printf '%6s %7s %7s %7s %6s %6s %7s %6s %6s %6s %6s %6s %6s\n' \
    "$seed" "${cells:-PARSE?}" "${wood:-PARSE?}" "${leafpc:-PARSE?}" "${stem:-PARSE?}" \
    "${roots:-PARSE?}" "${rootpc:-PARSE?}" "${estab:-PARSE?}" "${born:-PARSE?}" \
    "${died:-PARSE?}" "${inhrt:-PARSE?}" "${senes:-PARSE?}" "${top:-PARSE?}"
  rows="$rows$seed,${cells:-},${wood:-},${leafpc:-},${stem:-},${roots:-},${rootpc:-},${estab:-},${born:-},${died:-},${inhrt:-},${senes:-},${top:-}\n"
done

# Order statistics per column. `total` is reported alongside `max`/`p90` for
# the reason `seedsweep.sh` records: at eight samples p90 is the second
# worst and one chaotic seed moves it bodily, so a sum is the only stable
# number and a gap between them means the distribution reshuffled.
echo
printf '%-8s %8s %8s %8s %8s %8s %8s\n' column total max p90 median min n
col_stats() {
  printf "$rows" | awk -F, -v c="$2" -v label="$1" '
    $c ~ /^-?[0-9]+$/ { v[++n] = $c + 0; total += $c }
    END {
      if (n == 0) { printf "%-8s   NO RESULTS PARSED -- the probe output format probably changed\n", label; exit }
      for (i = 1; i <= n; i++) for (j = i+1; j <= n; j++) if (v[j] < v[i]) { t = v[i]; v[i] = v[j]; v[j] = t }
      printf "%-8s %8d %8d %8d %8d %8d %8d\n", label, total, v[n], v[int((n*9+9)/10)], v[int((n+1)/2)], v[1], n
    }'
}
col_stats cells 2
col_stats wood 3
col_stats "leaf%" 4
col_stats stem 5
col_stats roots 6
col_stats "root%" 7
col_stats estab 8
col_stats born 9
col_stats died 10
col_stats inhrt 11
col_stats senes 12
col_stats top 13

# Loud, for the same reason `seedsweep.sh` is loud: the people most likely
# to trim the seed list are the ones in a hurry, and at fewer than eight
# seeds p90 sits within one seed of the max.
n=$(printf "$rows" | grep -c '^[0-9]')
if [ "$n" -lt 8 ]; then
  echo
  echo "!! WARNING: only $n seeds. p90 is within one seed of the max at this size."
  echo "!!          Read total and median, or restore the full seed list."
fi
# The ceiling voids shape conclusions rather than merely biasing them, so it
# gets its own line instead of being left to whoever reads the table.
if printf "$rows" | awk -F, '$13 ~ /^-?[0-9]+$/ && $13 <= 0 { found = 1 } END { exit !found }'; then
  echo
  echo "!! CEILING HIT on at least one seed (top <= 0). Shape numbers from those runs are void --"
  echo "!!          see common::PlantScene. Raise ground= or discard the seed; do not interpret it."
fi
