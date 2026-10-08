#!/bin/bash
# Restart-safe: the store without smell (and with a wider smell), after the mound queue.
cd /home/user/runs/boom
while pgrep -x moundq.sh >/dev/null; do sleep 60; done
for s in 1 2 3 4; do
  for a in "nosmell on,pick=20,jaws,sky,meal,edible" "smell40 on,pick=20,jaws,sky,meal,smell=40,edible"; do
    set -- $a; grep -q '^rc=0' $1-s$s.log 2>/dev/null && continue; rm -rf $1-s$s $1-s$s.log; echo "$1 $s $2"
  done; done | xargs -P4 -L1 ./arm.sh
echo "ARM DONE $(date -u +%H:%M)" >> sweep.out
