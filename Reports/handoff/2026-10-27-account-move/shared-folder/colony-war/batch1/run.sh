#!/bin/bash
# batch 1: 12 seeds x 6 arms, 100k frames
B=/tmp/claude-0/-home-user-Pixel-Physics/0849b493-0a74-5ff7-a459-f6a00eae7ae9/scratchpad/b1/rivalbed.bin
D=$(dirname $0)
jobs=()
for s in $(seq 1 12); do
  for arm in "both:bite=0,0" "none:bite=off,off" "west:bite=0,off" "east:bite=off,0" "soloa:solo=a" "solob:solo=b"; do
    n=${arm%%:*}; a=${arm#*:}
    echo "$B seed=$s frames=100000 $a out=$D/$n-s$s > $D/$n-s$s.log 2>&1"
  done
done | xargs -P 4 -I{} bash -c "{}"
echo ALLDONE > $D/DONE
