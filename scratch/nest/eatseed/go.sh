#!/bin/bash
# SCRATCH (chambers lane, eat seed): the whole screen as run on 2026-10-29. usage: go.sh SCRATCHDIR (from the repo root)
# SCRATCHDIR holds h3tgz/poolH3 (the H3 founders) and gets pools/, eat/smoke/, eat/r4/.
S=$1
python3 scratch/nest/eatseed/make.py $S/h3tgz/poolH3 $S/pools/h3e > /dev/null
python3 scratch/nest/eatseed/make.py scratch/inrun/genomes/std_pool_raw $S/pools/stde > /dev/null
mkdir -p $S/eat/smoke
MINDLOG=1 bash scratch/nest/eatseed/run1.sh $S/eat/smoke/h3_w68 68 $S/h3tgz/poolH3 &
MINDLOG=1 bash scratch/nest/eatseed/run1.sh $S/eat/smoke/h3e_w68 68 $S/pools/h3e &
bash scratch/nest/eatseed/queue.sh $S/eat/r4 h3e:$S/pools/h3e stde:$S/pools/stde > $S/eat/queue.log 2>&1 &
wait
echo GODONE
