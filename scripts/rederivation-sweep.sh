#!/usr/bin/env bash
# Overnight re-derivation sweep: the constants the costs pass left uncalibrated.
#
# Each arm edits ONE constant, rebuilds (they are Rust consts and .ron is
# include_str!'d, so a rebuild per arm is mandatory), and runs the competitive
# bed at 8 world seeds. Reads the binding counters and the who-won Gini, which
# are the quantities a stand total cannot show.
#
# Restores every edited file via trap on ANY exit, including a kill.
set -uo pipefail
cd /home/user/Pixel_Physics
OUT=Reports/data/rederivation-sweep-2026-08-28
SEEDS="1 2 3 4 5 6 7 8"
FRAMES=28800          # 8 whole day/night periods -- phase pinned
BED="trees=24 width=512"
cp src/sim/plant.rs "$OUT/.plant.rs.orig"
cp assets/species/tree.ron "$OUT/.tree.ron.orig"
trap 'cp "$OUT/.plant.rs.orig" src/sim/plant.rs; cp "$OUT/.tree.ron.orig" assets/species/tree.ron' EXIT

arm_done() {                      # $1 = arm label; complete iff 8 substantial logs
  local arm="$1" n=0
  for s in $SEEDS; do
    [ -s "$OUT/${arm}_s${s}.log" ] && [ "$(stat -c%s "$OUT/${arm}_s${s}.log")" -gt 2000 ] && n=$((n+1))
  done
  [ "$n" -eq 8 ]
}

run_arm() {                       # $1 = arm label
  local arm="$1"
  # **Resumable.** This sweep has already been killed once mid-arm by a
  # container restart, losing ten hours. An arm with eight substantial logs
  # is complete and is skipped, so re-running the script continues rather
  # than starting over. Header-only logs (a probe that died at launch) are
  # under the size floor and get redone.
  if arm_done "$arm"; then echo "skip $arm (already complete)" >> "$OUT/PROGRESS"; return 0; fi
  rm -f "$OUT/${arm}_s"*.log
  cargo build --release --example plant_probe > "$OUT/$arm.build" 2>&1
  if [ "${PIPESTATUS[0]}" -ne 0 ]; then echo "BUILD FAILED: $arm" >> "$OUT/PROGRESS"; return 1; fi
  for s in $SEEDS; do
    ./target/release/examples/plant_probe frames=$FRAMES $BED worldseed=$s > "$OUT/${arm}_s${s}.log" 2>&1 &
    if [ $((s % 2)) -eq 0 ]; then wait; fi
  done
  wait
  echo "done $arm  $(date -u +%H:%M)" >> "$OUT/PROGRESS"
}

sweep_const() {                   # $1 = const name, $2.. = values
  local name="$1"; shift
  local base; base=$(grep -oP "^const $name: f32 = \K[0-9.]+" "$OUT/.plant.rs.orig")
  for v in "$@"; do
    cp "$OUT/.plant.rs.orig" src/sim/plant.rs
    sed -i "s/^const $name: f32 = $base;/const $name: f32 = $v;/" src/sim/plant.rs
    if [ "$(grep -c "^const $name: f32 = $v;" src/sim/plant.rs)" -ne 1 ]; then
      echo "EDIT FAILED: $name=$v" >> "$OUT/PROGRESS"; continue
    fi
    run_arm "${name}_${v}"
  done
  cp "$OUT/.plant.rs.orig" src/sim/plant.rs
}

echo "sweep started $(date -u)" > "$OUT/PROGRESS"
run_arm BASELINE
sweep_const LEAF_CONSTRUCTION_MULTIPLE 0.6 0.9 1.5 2.0
sweep_const WOOD_CONSTRUCTION_MULTIPLE 0.4 1.2 1.6
sweep_const MAINTENANCE_PER_NODE 1.0e-5 4.0e-5
sweep_const TRANSPIRATION_PER_RATE 0.05 0.2
# reproductive_allocation is per-species data, not a const -- swept on tree.ron
for v in 0.05 0.20 0.30; do
  cp "$OUT/.tree.ron.orig" assets/species/tree.ron
  sed -i "s/reproductive_allocation: 0.10,/reproductive_allocation: $v,/" assets/species/tree.ron
  [ "$(grep -c "reproductive_allocation: $v," assets/species/tree.ron)" -eq 1 ] && run_arm "repro_alloc_${v}"
done
cp "$OUT/.tree.ron.orig" assets/species/tree.ron
echo "SWEEP COMPLETE $(date -u)" >> "$OUT/PROGRESS"
