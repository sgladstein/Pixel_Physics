#!/usr/bin/env bash
# The single-fix arms (review: run only if all-together fails the gate). 4 arms x 4 seeds, 4 at a time.
B=/home/claude/runs/bin/deeptrace-79437e32a2c0
R=/tmp/claude-0/-home-claude-Pixel-Physics/c8162b93-2657-592f-b840-597b957aa530/scratchpad/walkrun/frun.sh
declare -A PARTS=( [P1]=only_diggers,dig_job,clear [P2]=pace,give_up,lay_home [P3]=meal [P4]=won_stall )
for arm in P1 P2 P3 P4; do
  for s in 1 2 3 4; do
    $R $arm $B $s 300000 needs=walk needsat=50000 needsparts=${PARTS[$arm]} walktrace=20 dig=0 &
  done
  wait
  echo "$arm done $(date +%T)"
done
echo ARMSDONE
