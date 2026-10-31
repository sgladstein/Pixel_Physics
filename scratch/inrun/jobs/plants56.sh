#!/bin/bash
# Plants lane: rungs 5-6 stage 0 (invasion into a grown stand) on played_bed with the colony.
# Main 43da91c7 or later. 16 single-core processes, ~25 min each, ~6 core-hours in all.
# Usage: bash run.sh OUTDIR   (from a checkout of main; builds plant_m0 itself)
set -euo pipefail
O=${1:?outdir}; mkdir -p "$O"
cargo build --release --example plant_m0
B=${CARGO_TARGET_DIR:-target}/release/examples/plant_m0  # teacher-lane copy: on the VM it builds into its own target dir
git rev-parse HEAD > "$O/commit.txt"
for role in tree shrub herb scrambler; do for s in 1 2 3 4; do
  arms=none,shipped,cripple; [ $s = 1 ] && arms=none,shipped,cripple,none
  RAYON_NUM_THREADS=1 $B mode=stand bed=played role=$role seed=$s \
    grow=300000 window=150000 sample=10000 sow=4 gap=16 arms=$arms > "$O/${role}_s$s.txt" 2>&1 &
done; done
wait
tar czf "$O.tgz" -C "$(dirname "$O")" "$(basename "$O")"
echo done
