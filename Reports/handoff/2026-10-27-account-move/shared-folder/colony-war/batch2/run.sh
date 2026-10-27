#!/bin/bash
B=/tmp/claude-0/-home-user-Pixel-Physics/0849b493-0a74-5ff7-a459-f6a00eae7ae9/scratchpad/b2/rivalbed.bin
D=/tmp/claude-0/-home-user-Pixel-Physics/0849b493-0a74-5ff7-a459-f6a00eae7ae9/scratchpad/b2
for s in $(seq 1 12); do
  for arm in "both:bite=0,0" "none:bite=off,off" "west:bite=0,off" "east:bite=off,0" "soloa:solo=a" "solob:solo=b" "keep:bite=keep,keep" "agg:bite=1,1" "aggw:bite=1,0" "gap40:bite=0,0 gap=40"; do
    n=${arm%%:*}; a=${arm#*:}
    echo "$B seed=$s frames=100000 $a out=$D/$n-s$s > $D/$n-s$s.log 2>&1"
  done
done | xargs -P 4 -I{} bash -c "{}"
echo ALLDONE > $D/DONE
