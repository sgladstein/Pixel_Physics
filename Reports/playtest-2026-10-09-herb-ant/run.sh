#!/bin/bash
# Every arm of playtest-2026-10-09-herb-ant/README.md, as it was run.
#
#   bash run.sh OUT_DIR ARM SEED [FRAMES]
#
# ARM: alone ants null noleaf noseed noleafseed leaf20 leaf160 doortree mut_on mut_on_alone rain_light rain_light_alone
#      alone_long rain_light_alone_long   (no ants, 1,500,000 ticks by default: does the stand die on its own?)
# Output lands in OUT_DIR/s<SEED>_<ARM> with its log beside it; tables.py reads that layout.
#
# The switch bundle is the playtest's own (the chronicle header's SWITCHES line) minus the GIT_ pair, which only
# labels the chronicle. Mutation is OFF except in the mut_on arms, as in the playtest.
#
# Build first, and rebuild after any edit: `cargo build --release` does not rebuild examples.
#   set -o pipefail; cargo build --release --examples
# `RAYON_NUM_THREADS=1` is how four arms ran at once at a load-independent result; the thread count does not change
# a result, only the wall clock.
set -euo pipefail
OUT=${1:?OUT_DIR}; ARM=${2:?ARM}; SEED=${3:?SEED}
case $ARM in *_long) FRAMES=${4:-1500000} ;; *) FRAMES=${4:-450000} ;; esac
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$HERE/../..
BIN=${BIN:-$ROOT/target/release/examples/replay}

export PIXEL_PHYSICS_BROOD_SPREAD=on PIXEL_PHYSICS_BUD_SITE=nest PIXEL_PHYSICS_B_DIFFUSE=0.05 PIXEL_PHYSICS_B_RHO=0.005
export PIXEL_PHYSICS_CARRY_HOME=on PIXEL_PHYSICS_DOOR_COLUMN=on PIXEL_PHYSICS_FACE_TRIP=on PIXEL_PHYSICS_HUNGRY_OUT=on
export PIXEL_PHYSICS_LAY_BAR=body PIXEL_PHYSICS_LAY_BRAKE=on PIXEL_PHYSICS_MUTATION=off
export PIXEL_PHYSICS_NEEDS_FIRST=on,backfill
export PIXEL_PHYSICS_NEST_STORE=on,pick=20,jaws,sky,meal,smell=10,edible
export PIXEL_PHYSICS_RECRUIT=on PIXEL_PHYSICS_SOIL_WAY=on PIXEL_PHYSICS_WAY_FOOT=on PIXEL_PHYSICS_WAY_GAPS=on

# An ablation is a directory of copies of material files with one number changed, handed to `assets=` (which is
# `materials.reload`: ids are stable, so a running bed takes it). Shipped values: leaf 40, grassblade 40, moss 480,
# seed 480, pip 40 (`food_energy`).
M=$ROOT/assets/materials
ablation() { # name  then pairs  file:shipped:new
  local d=$OUT/ablate/$1; shift; mkdir -p "$d"
  cp "$M/leaf.ron" "$M/grassblade.ron" "$M/moss.ron" "$M/seed.ron" "$M/pip.ron" "$d/"
  for p in "$@"; do IFS=: read -r f was now <<<"$p"
    sed -i -E "s/food_energy: $was\.0,/food_energy: $now,/" "$d/$f.ron"
    grep -q "food_energy: $now," "$d/$f.ron" || { echo "ablation $f did not take" >&2; exit 3; }
  done
  echo "$d"
}

D=$OUT/s${SEED}_$ARM; mkdir -p "$D"
args=(frames="$FRAMES" seed="$SEED" rain=off pace=half out="$D" probe=5000 snap=25000 antsnap=2000)
export RAYON_NUM_THREADS=1
{
case $ARM in
  alone)            "$BIN" "${args[@]}" ants=0 ;;
  ants)             "$BIN" "${args[@]}" ants=1 ;;
  null)             "$BIN" "${args[@]}" ants=1 assets="$(ablation null)" ;;   # the reload with nothing changed: must equal `ants`
  noleaf)           "$BIN" "${args[@]}" ants=1 assets="$(ablation noleaf leaf:40:0.0 grassblade:40:0.0 moss:480:0.0)" ;;
  noseed)           "$BIN" "${args[@]}" ants=1 assets="$(ablation noseed seed:480:0.0 pip:40:0.0)" ;;
  noleafseed)       "$BIN" "${args[@]}" ants=1 assets="$(ablation noleafseed leaf:40:0.0 grassblade:40:0.0 moss:480:0.0 seed:480:0.0 pip:40:0.0)" ;;
  leaf20)           "$BIN" "${args[@]}" ants=1 assets="$(ablation leaf20 leaf:40:20.0 grassblade:40:20.0)" ;;
  leaf160)          "$BIN" "${args[@]}" ants=1 assets="$(ablation leaf160 leaf:40:160.0 grassblade:40:160.0)" ;;
  doortree)         "$BIN" "${args[@]}" ants=1 doortree=1 ;;
  mut_on)           PIXEL_PHYSICS_MUTATION=on "$BIN" "${args[@]}" ants=1 ;;
  mut_on_alone)     PIXEL_PHYSICS_MUTATION=on "$BIN" "${args[@]}" ants=0 ;;
  rain_light)       "$BIN" frames="$FRAMES" seed="$SEED" rain=light pace=half out="$D" probe=5000 snap=25000 antsnap=2000 ants=1 ;;
  rain_light_alone) "$BIN" frames="$FRAMES" seed="$SEED" rain=light pace=half out="$D" probe=5000 snap=25000 antsnap=2000 ants=0 ;;
  alone_long)       "$BIN" frames="$FRAMES" seed="$SEED" rain=off pace=half out="$D" probe=25000 snap=250000 antsnap=100000000 ants=0 ;;
  rain_light_alone_long) "$BIN" frames="$FRAMES" seed="$SEED" rain=light pace=half out="$D" probe=25000 snap=250000 antsnap=100000000 ants=0 ;;
  *) echo "unknown arm $ARM" >&2; exit 2 ;;
esac
} > "$D.log" 2>&1
tail -n 1 "$D.log"
