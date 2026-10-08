#!/bin/bash
# flip30.sh ARM SEED -- nest_goal heap 30, 120k, the comparison's binary. ARM: main | bundle | nostore
arm=$1; s=$2; out=/home/user/runs/h30-$arm-s$s; mkdir -p $out
sw=()
[ "$arm" = main ] || sw=(PIXEL_PHYSICS_NEEDS_FIRST=on,backfill PIXEL_PHYSICS_CARRY_HOME=on PIXEL_PHYSICS_DOOR_COLUMN=on PIXEL_PHYSICS_LAY_BAR=body PIXEL_PHYSICS_WAY_FOOT=on)
[ "$arm" = bundle ] && sw+=(PIXEL_PHYSICS_NEST_STORE=on,pick=20,jaws,sky,meal,smell=10,edible)
cd /home/user/wt-stack
env $(env | grep -o '^PIXEL_PHYSICS_[A-Z_]*' | sed 's/^/-u /') RAYON_NUM_THREADS=1 "${sw[@]}" \
  nice -n 5 /home/user/runs/deeptrace-f55b33b8 scenario=nest_goal seed=$s foodgap=30 frames=120000 founder=evolved ants=0 mapevery=25000 hungry=1 out=$out > $out.log 2>&1
rc=$?; echo "sw=${sw[*]}" >> $out.log; echo "rc=$rc" >> $out.log
