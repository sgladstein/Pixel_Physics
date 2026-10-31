#!/bin/bash
# Plants lane: rungs 5-6 stage 1 (invasion from four seeds into a grown lab bed with the colony).
# Main a4a15fe0 or later. 4 roles x beds 1-12 = 48 single-core processes, ~31 min each on the VM (~25 core-hours).
# trace=150000 adds a DEATH line per sown seed (cause: eaten, rotted, ...): what happened to each of the four.
# Usage: bash run.sh OUTDIR   (from a checkout of main; builds plant_m0 itself; honours CARGO_TARGET_DIR)
set -euo pipefail
O=${1:?outdir}; mkdir -p "$O"
cargo build --release --example plant_m0
B=${CARGO_TARGET_DIR:-target}/release/examples/plant_m0
git rev-parse HEAD > "$O/commit.txt"
for role in tree shrub herb scrambler; do for s in $(seq 1 12); do
  arms=none,shipped,cripple; [ $s = 1 ] && arms=none,shipped,cripple,none
  RAYON_NUM_THREADS=1 $B mode=stand bed=played role=$role seed=$s \
    grow=300000 window=150000 sample=10000 sow=4 gap=16 trace=150000 arms=$arms > "$O/${role}_s$s.txt" 2>&1 &
done; done
wait
tar czf "$O.tgz" -C "$(dirname "$O")" "$(basename "$O")"
echo done
