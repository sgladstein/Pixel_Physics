#!/usr/bin/env bash
# The genetic-variability megastudy, owner-authorized for a multi-hour run.
#
# Three species x eight world seeds x 16 plants x 45,000 frames. Genotypes
# key on (world seed, germination coordinate), so each world seed is a
# fresh population of individuals; 16 plants per run at the standard
# 57-cell spacing (width auto-scales) gives 128 individuals per species.
# 45,000 frames is past the measured single-tree plateau (~50k for one
# individual, stands saturate earlier), so the numbers are mature forms,
# not growth snapshots.
#
# Resumable: a run whose log already exists is skipped, so the study can
# be re-launched after an interruption and only does what is missing.
# Conifer is INCLUDED despite the known uniform-lean bug -- its per-seed
# data is evidence for the fix (does the lean side vary by seed? by
# individual?), which is exactly the question the bug is stuck on.
set -u
cd "$(dirname "$0")/.."
OUT=target/megastudy
mkdir -p "$OUT"
echo "megastudy started $(date -Iseconds)" >> "$OUT/MANIFEST.txt"
for species in tree shrub conifer; do
  for seed in 11 22 33 44 55 66 77 88; do
    log="$OUT/$species-seed$seed.log"
    if [ -s "$log" ]; then
      echo "skip $log (exists)" >> "$OUT/MANIFEST.txt"
      continue
    fi
    echo "run $species seed $seed $(date -Iseconds)" >> "$OUT/MANIFEST.txt"
    ./target/release/examples/plant_probe trees=16 frames=45000 species=$species worldseed=$seed > "$log" 2>&1
  done
  # One sheet per species from the first seed's world, for the eye.
  ./target/release/examples/filmstrip scene=grove species=$species start=15000 every=15000 count=3 cols=3 zoom=1 crop=0,40,512,206 \
    out="$OUT/$species-sheet.png" > /dev/null 2>&1
done
echo "megastudy complete $(date -Iseconds)" >> "$OUT/MANIFEST.txt"
