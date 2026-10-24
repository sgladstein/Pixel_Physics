#!/bin/bash
# sweep4.sh MAPLIST OUTROOT : frame-0 runs for inherit_link=N
cd /home/user/wt
while read m; do
  tag=$(echo $m | awk -F/ '{print $(NF-2)"_"$(NF-1)}')
  for N in $NS; do
    mkdir -p $2/$tag/N$N
    ./target/release/examples/inrun nomind=1 founders=scratch/inrun/genomes/c1.raw.txt:1 frames=0 sel=off dist=25 inherit=$m inherit_nest=1 inherit_link=$N out=$2/$tag/N$N/f0 > $2/$tag/N$N/log 2>&1
  done
done < $1
