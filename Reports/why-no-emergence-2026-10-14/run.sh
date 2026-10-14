#!/bin/bash
# The 16 runs behind ../why-no-emergence-2026-10-14.md §5 (main bb8a2151).
#   bash Reports/why-no-emergence-2026-10-14/run.sh ARM SEED OUTDIR   (from the repo root)   ARM: h30 | h90 | nurse | rest
# Build first:  set -o pipefail; cargo build --release --example deeptrace
# Each run: ~17-31 min single-threaded, ~0.5 GB. Run four at once (RAYON_NUM_THREADS=1).
# Readers (in this directory):  python3 margin.py OUT...    (reserve, burn, trips, deaths; window LO-HI env, default 100k-300k)
#           LO=20000 python3 starvers.py OUT...   (each starver: stuck / held / free)
#           python3 stuckway.py OUT...  (nest starvers: depth, nearest approach to the door)
#           python3 scripts/deeptrace_tools/starvewhere.py OUT...  (sealed / pocket / encased / open)
set -e
arm=$1; seed=$2; out=$3
extra=""
case $arm in
  h30) ;;
  h90) extra="foodgap=90" ;;
  nurse) export PIXEL_PHYSICS_NURSE_STAY=on ;;
  rest) export PIXEL_PHYSICS_NEST_REST=workers ;;
  *) echo "arm: h30 h90 nurse rest"; exit 1 ;;
esac
mkdir -p "$out"
RAYON_NUM_THREADS=1 target/release/examples/deeptrace seed=$seed frames=300000 founder=evolved ants=60 hungry=1 mapevery=1000 out="$out" $extra > "$out/stdout.txt"
