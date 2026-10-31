#!/bin/bash
# Plants lane: rich earth paired test (branch claude/project-thread-v4hhc9, head 38a4bb71 or later).
# 3 arms (off, refill, bonus) x played_bed seeds 1-12, 450k frames, mister on, colony on.
# 36 single-core processes, ~15-25 min each on one core (~12 core-hours).
# Usage: bash run.sh OUTDIR   (from a checkout of the branch; builds latecensus itself; honours CARGO_TARGET_DIR)
set -euo pipefail
O=${1:?outdir}; mkdir -p "$O"
cargo build --release --example latecensus
B=${CARGO_TARGET_DIR:-target}/release/examples/latecensus
git rev-parse HEAD > "$O/commit.txt"
for arm in off refill bonus; do for s in $(seq 1 12); do
  PIXEL_PHYSICS_RICH_EARTH=$arm RAYON_NUM_THREADS=1 $B scenario=played_bed mister=1 seed=$s \
    frames=450000 sample=50000 > "$O/${arm}_s$s.txt" 2>&1 &
done; done
wait
tar czf "$O.tgz" -C "$(dirname "$O")" "$(basename "$O")"
echo done
