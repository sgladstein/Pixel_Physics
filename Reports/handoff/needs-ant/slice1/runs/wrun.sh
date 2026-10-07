#!/usr/bin/env bash
# wrun.sh LABEL BIN SEED FRAMES ENVS ARGS -- one deeptrace run of the goal box for the slice-1 gate.
set -u
label=$1; bin=$2; seed=$3; frames=$4; envs=$5; extra=$6
out=/home/claude/runs/walk/$label-s$seed
mkdir -p /home/claude/runs/walk
rm -rf "$out"
for v in $(env | grep -o '^PIXEL_PHYSICS_[A-Z_]*'); do unset "$v"; done
cd /home/claude/Pixel_Physics || exit 1
{
  echo "env: [$envs] args: [$extra] seed=$seed frames=$frames bin=$(sha256sum $bin | cut -c1-12)"
  env RAYON_NUM_THREADS=1 $envs $bin scenario=nest_goal seed=$seed frames=$frames ants=0 mapevery=1000 hungry=1 dig=0 out=$out $extra
  echo "exit $?"
} > "$out.log" 2>&1
