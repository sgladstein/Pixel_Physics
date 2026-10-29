#!/bin/bash
# SCRATCH (chambers lane, larder seed): one round-0 world, sel=off, poolrounds.sh's bed settings with PP_FOODBESIDE=1,
# keeping only the mind log's food put-downs (M rows whose pick is Drop, verb 5, with the crop holding food and no
# pellet) gzipped to OUT.drops.gz -- the picked option's slot 18 is the share of its 8 neighbours holding food.
# usage: smoke.sh OUT SEED POOLDIR      (run from the repo root)
set -o pipefail
O=$1; S=$2; P=$3
mkdir -p $O
G=$(for p in $P/*.raw.txt; do n=$(basename $p .raw.txt); printf '%s@%s:1,' $p $n; done); G=${G%,}
rm -f $O.fifo; mkfifo $O.fifo
awk -F, '$1=="M" && $7=="5" {split($11,c," "); if (c[2]>0 && c[3]==0) print}' < $O.fifo | gzip -1 > $O.drops.gz &
t0=$(date +%s)
env RAYON_NUM_THREADS=1 PP_HUNGRYPROBE=1 PP_FOODBESIDE=1 PP_MIND_LOG=$O.fifo target/release/examples/inrun founders=$G seed=$S \
  frames=${FRAMES:-200000} dist=25 spread=10 grants=3 window=4000 sel=off food=mix \
  lesson=scratch/inrun/genomes/c1.raw.txt:0.1 patches=20000:40:120 nest_r=0 trace=250 \
  setenv=PP_BED_W=448,PP_MIND_DOOR_OPEN=drop out=$O > $O.log 2>&1
echo "exit $? secs $(( $(date +%s) - t0 ))" >> $O.log
wait
rm -f $O.fifo
