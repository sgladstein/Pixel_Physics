#!/bin/bash
cd /home/user/runs
while pgrep -x partq.sh >/dev/null; do sleep 60; done
for s in 1 2 3 4; do grep -q '^rc=0' nostore90-s$s.log 2>/dev/null && continue; rm -rf nostore90-s$s nostore90-s$s.log; echo $s; done | xargs -P4 -L1 ./nostore90.sh
echo "NOSTORE90 DONE $(date -u +%H:%M)" >> /home/user/runs/boom/sweep.out
