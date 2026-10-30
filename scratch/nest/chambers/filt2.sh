#!/bin/bash
# keep every M row of a hungry ant (tank < 0.5 full grants, i.e. the squashed energy sense < 1/3), both arms
S=/tmp/claude-0/-home-user-Pixel-Physics/5839c770-56d6-594d-ba6d-0b2ec65d8626/scratchpad
cd $S
for a in std h3; do
  zcat deep/${a}_w68.mind.gz | awk -F, '$1=="M" { split($11, c, " "); if (c[1] < 0.3333) print }' > deep/${a}.hungry.log &
done
wait
wc -l deep/*.hungry.log
echo FILT2DONE
