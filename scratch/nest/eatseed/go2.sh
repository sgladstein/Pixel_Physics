#!/bin/bash
# SCRATCH (chambers lane, eat seed): arm B (Eat only at home, make.py nest=60). usage: go2.sh SCRATCHDIR ARM... (h3n, stdn)
# h3n's smoke (w68 round 0 sel=off with the mind log) runs beside its lane.
S=$1; shift
python3 scratch/nest/eatseed/make.py $S/h3tgz/poolH3 $S/pools/h3n nest=60 > /dev/null
python3 scratch/nest/eatseed/make.py scratch/inrun/genomes/std_pool_raw $S/pools/stdn nest=60 > /dev/null
for a in "$@"; do
  [ $a = h3n ] && { MINDLOG=1 bash scratch/nest/eatseed/run1.sh $S/eat/smoke/h3n_w68 68 $S/pools/h3n & }
  bash scratch/nest/eatseed/queue.sh $S/eat/r4 $a:$S/pools/$a > $S/eat/queue_$a.log 2>&1 &
done
wait
echo GO2DONE
