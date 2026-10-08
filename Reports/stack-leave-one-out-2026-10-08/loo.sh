#!/bin/bash
# loo.sh DROP SEED -- steady_income, the owner's playtest line plus edible, with one switch left out (unset = off).
# DROP: NEEDS_FIRST | CARRY_HOME | DOOR_COLUMN | LAY_BAR | NEST_STORE | WAY_FOOT
drop=$1; s=$2; out=/home/user/runs/boom/loo-$drop-s$s; mkdir -p $out
all=(NEEDS_FIRST=on,backfill CARRY_HOME=on DOOR_COLUMN=on LAY_BAR=body NEST_STORE=on,pick=20,jaws,sky,meal,smell=10,edible WAY_FOOT=on)
sw=(); for a in "${all[@]}"; do [ "${a%%=*}" = "$drop" ] || sw+=(PIXEL_PHYSICS_$a); done
cd /home/user/wt-stack
env $(env | grep -o '^PIXEL_PHYSICS_[A-Z_]*' | sed 's/^/-u /') RAYON_NUM_THREADS=1 PIXEL_PHYSICS_MUTATION=off \
  PIXEL_PHYSICS_LAB_SCENARIOS=/home/user/wt-boom/assets/lab_scenarios "${sw[@]}" \
  nice -n 5 /home/user/runs/deeptrace-brakes scenario=steady_income food=0 founder=evolved ants=0 hungry=1 mapevery=10000 \
  set=ant.digest_hunger_weight=0 seed=$s frames=200000 out=$out > $out.log 2>&1
rc=$?; echo "sw=${sw[*]}" >> $out.log
echo "rc=$rc" >> $out.log
