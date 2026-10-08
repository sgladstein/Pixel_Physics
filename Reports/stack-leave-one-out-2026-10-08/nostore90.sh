#!/bin/bash
# nostore90.sh SEED -- the default-flip comparison's stack arm (heap 90, 300k) with NEST_STORE unset.
s=$1; out=/home/user/runs/nostore90-s$s; mkdir -p $out
sw=(PIXEL_PHYSICS_NEEDS_FIRST=on,backfill PIXEL_PHYSICS_CARRY_HOME=on PIXEL_PHYSICS_DOOR_COLUMN=on PIXEL_PHYSICS_LAY_BAR=body PIXEL_PHYSICS_WAY_FOOT=on)
cd /home/user/wt-stack
env $(env | grep -o '^PIXEL_PHYSICS_[A-Z_]*' | sed 's/^/-u /') RAYON_NUM_THREADS=1 "${sw[@]}" \
  nice -n 5 /home/user/runs/deeptrace-f55b33b8 scenario=nest_goal seed=$s foodgap=90 frames=120000 founder=evolved ants=0 mapevery=25000 hungry=1 out=$out > $out.log 2>&1
rc=$?; echo "sw=${sw[*]}" >> $out.log; echo "rc=$rc" >> $out.log
