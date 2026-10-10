#!/bin/bash
cd /home/user/runs
for s in 1 2 3 4; do for a in main bundle nostore; do grep -q '^rc=0' h30-$a-s$s.log 2>/dev/null && continue; rm -rf h30-$a-s$s h30-$a-s$s.log; echo "$a $s"; done; done | xargs -P4 -L1 ./flip30.sh
echo "FLIP30 DONE $(date -u +%H:%M)" >> /home/user/runs/boom/sweep.out
