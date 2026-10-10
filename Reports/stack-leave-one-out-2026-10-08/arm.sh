#!/bin/bash
# arm.sh NAME SEED "STORE" -- steady_income, the playtest line plus edible but NEST_STORE replaced by STORE.
name=$1; s=$2; store=$3; out=/home/user/runs/boom/$name-s$s; mkdir -p $out
sw=(PIXEL_PHYSICS_NEEDS_FIRST=on,backfill PIXEL_PHYSICS_CARRY_HOME=on PIXEL_PHYSICS_DOOR_COLUMN=on PIXEL_PHYSICS_LAY_BAR=body PIXEL_PHYSICS_NEST_STORE=$store PIXEL_PHYSICS_WAY_FOOT=on)
cd /home/user/wt-stack
env $(env | grep -o '^PIXEL_PHYSICS_[A-Z_]*' | sed 's/^/-u /') RAYON_NUM_THREADS=1 PIXEL_PHYSICS_MUTATION=off \
  PIXEL_PHYSICS_LAB_SCENARIOS=/home/user/wt-boom/assets/lab_scenarios "${sw[@]}" \
  nice -n 5 /home/user/runs/deeptrace-brakes scenario=steady_income food=0 founder=evolved ants=0 hungry=1 mapevery=10000 \
  set=ant.digest_hunger_weight=0 seed=$s frames=200000 out=$out > $out.log 2>&1
rc=$?; echo "sw=${sw[*]}" >> $out.log; echo "rc=$rc" >> $out.log
