#!/bin/bash
# run4.sh ENV W SCEN SEED [FRAMES] [EXTRA...]
#   ENV play : the owner's 2026-10-07 playtest switch line (no edible)
#   ENV playedible : the same plus NEST_STORE edible
#   ENV main : main's shipped defaults (no PIXEL_PHYSICS_* at all)
#   W        : ant.digest_hunger_weight
#   SCEN     : steady_income | boom_bust | nest_goal (nest_goal adds foodgap=90 and keeps the harness top-up)
envname=$1; w=$2; scen=$3; s=$4; frames=${5:-200000}; shift 5 2>/dev/null; extra="$*"
out=/home/user/runs/boom/$scen-$envname-w$w-s$s; mkdir -p $out
sw=(); [ "$envname" = play ] || [ "$envname" = playedible ] && sw=(PIXEL_PHYSICS_NEEDS_FIRST=on,backfill PIXEL_PHYSICS_CARRY_HOME=on PIXEL_PHYSICS_DOOR_COLUMN=on PIXEL_PHYSICS_LAY_BAR=body PIXEL_PHYSICS_NEST_STORE=on,pick=20,jaws,sky,meal,smell=10 PIXEL_PHYSICS_WAY_FOOT=on)
[ "$envname" = playedible ] && sw=("${sw[@]/PIXEL_PHYSICS_NEST_STORE=on,pick=20,jaws,sky,meal,smell=10/PIXEL_PHYSICS_NEST_STORE=on,pick=20,jaws,sky,meal,smell=10,edible}")
food="food=0"; [ "$scen" = nest_goal ] && food="foodgap=90"
cd /home/user/wt-stack
env $(env | grep -o '^PIXEL_PHYSICS_[A-Z_]*' | sed 's/^/-u /') RAYON_NUM_THREADS=1 PIXEL_PHYSICS_MUTATION=off \
  PIXEL_PHYSICS_LAB_SCENARIOS=/home/user/wt-boom/assets/lab_scenarios "${sw[@]}" \
  nice -n 5 /home/user/runs/deeptrace-brakes scenario=$scen $food founder=evolved ants=0 hungry=1 mapevery=10000 \
  set=ant.digest_hunger_weight=$w seed=$s frames=$frames $extra out=$out > $out.log 2>&1
echo "rc=$?" >> $out.log
