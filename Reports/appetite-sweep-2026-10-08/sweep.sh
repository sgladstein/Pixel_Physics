#!/bin/bash
# Appetite sweep queue: skips runs whose log ends rc=0; deletes partial dirs first. Restart-safe.
cd /home/user/runs/boom
jobs() {
  for env in play main; do for s in 1 2 3 4; do for w in 0 0.25 0.5 0.75; do echo "$env $w steady_income $s 200000"; done; done; done
  for env in play main; do for s in 1 2 3 4; do for w in 0 0.5; do echo "$env $w nest_goal $s 200000"; done; done; done
}
jobs | while read env w scen s fr; do
  d=$scen-$env-w$w-s$s; grep -q '^rc=0' $d.log 2>/dev/null && continue; rm -rf $d $d.log; echo "$env $w $scen $s $fr"
done | xargs -P4 -L1 ./run4.sh
echo "SWEEP DONE $(date -u +%H:%M)" >> sweep.out
