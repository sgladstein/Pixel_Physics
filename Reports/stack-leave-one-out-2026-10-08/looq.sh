#!/bin/bash
# Restart-safe leave-one-out queue; waits for the appetite queue to finish first.
cd /home/user/runs/boom
while pgrep -x sweep.sh >/dev/null || pgrep -x chain2.sh >/dev/null; do sleep 60; done
for s in 1 2 3 4; do for d in NEEDS_FIRST CARRY_HOME DOOR_COLUMN LAY_BAR NEST_STORE WAY_FOOT; do
  grep -q '^rc=0' loo-$d-s$s.log 2>/dev/null && continue; rm -rf loo-$d-s$s loo-$d-s$s.log; echo "$d $s"; done; done | xargs -P4 -L1 ./loo.sh
echo "LOO DONE $(date -u +%H:%M)" >> sweep.out
