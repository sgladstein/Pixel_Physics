#!/usr/bin/env bash
# frun.sh LABEL BIN SEED FRAMES ARGS... -- one deeptrace run of the goal box for the slice-1 fix round.
set -u
label=$1; bin=$2; seed=$3; frames=$4; shift 4
out=/home/claude/runs/fix/$label-s$seed
mkdir -p /home/claude/runs/fix
rm -rf "$out"
for v in $(env | grep -o '^PIXEL_PHYSICS_[A-Z_]*'); do unset "$v"; done
cd /home/claude/Pixel_Physics || exit 1
{
  echo "args: [$*] seed=$seed frames=$frames bin=$(sha256sum $bin | cut -c1-12)"
  env RAYON_NUM_THREADS=1 $bin scenario=nest_goal seed=$seed frames=$frames ants=0 mapevery=1000 hungry=1 out=$out "$@"
  echo "exit $?"
} > "$out.log" 2>&1
