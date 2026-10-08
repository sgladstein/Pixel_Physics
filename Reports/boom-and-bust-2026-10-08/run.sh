#!/bin/bash
# Step 2: the owner's playtest env (74a2f08a's switch line, no edible) on boom_bust, food=0 (no top-up).
# ARM: play (as played) | edible (same plus NEST_STORE edible). Binary: the stack at f55b33b8.
arm=$1; s=$2; out=/home/user/runs/boom/$arm-s$s; mkdir -p $out
store="on,pick=20,jaws,sky,meal,smell=10"; [ "$arm" = edible ] && store="$store,edible"
cd /home/user/wt-stack
env $(env | grep -o '^PIXEL_PHYSICS_[A-Z_]*' | sed 's/^/-u /') RAYON_NUM_THREADS=1 \
  PIXEL_PHYSICS_LAB_SCENARIOS=/home/user/wt-boom/assets/lab_scenarios \
  PIXEL_PHYSICS_NEEDS_FIRST=on,backfill PIXEL_PHYSICS_CARRY_HOME=on PIXEL_PHYSICS_DOOR_COLUMN=on \
  PIXEL_PHYSICS_LAY_BAR=body PIXEL_PHYSICS_NEST_STORE=$store PIXEL_PHYSICS_WAY_FOOT=on PIXEL_PHYSICS_MUTATION=off \
  nice -n 5 /home/user/runs/deeptrace-f55b33b8 scenario=boom_bust food=0 founder=evolved ants=0 hungry=1 dig=1 \
  mapevery=10000 seed=$s frames=${FRAMES:-300000} out=$out > $out.log 2>&1
echo "rc=$?" >> $out.log
