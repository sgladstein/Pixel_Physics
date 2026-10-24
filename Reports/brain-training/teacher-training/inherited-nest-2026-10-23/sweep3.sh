#!/bin/bash
cd /home/user/wt
while read m; do
  tag=$(echo $m | awk -F/ '{print $(NF-2)"_"$(NF-1)}')
  for L in 0 1; do for f in 0 $2; do
    ./target/release/examples/inrun nomind=1 founders=scratch/inrun/genomes/c1.raw.txt:1 frames=$f sel=off dist=25 inherit=$m inherit_nest=1 inherit_link=$L out=$3/$tag/L$L/f$f > /dev/null 2>&1
  done; done
done < $1
