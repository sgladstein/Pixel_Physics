#!/bin/bash
# SCRATCH (chambers lane, larder seed): the paired screen -- poolrounds.sh 4 x 200k sel=score, nest_r=0, trace rows and
# the hungry probe (the eat screen's own args, eatseed/queue.sh), PP_FOODBESIDE=1, one world per lane, 4 lanes at once.
# usage: go_screen.sh OUT ARM:POOL [ARM:POOL ...]   (WORLDS="68 73 61 64" by default)
O=$1; shift
mkdir -p $O
for a in "$@"; do
  arm=${a%%:*}; pool=${a#*:}
  for w in ${WORLDS:-68 73 61 64}; do
    PP_FOODBESIDE=${BESIDE:-1} PP_HUNGRYPROBE=1 BIN=target/release/examples/inrun bash scratch/nest/poolrounds.sh $O/${arm}_w$w $w 4 200000 $pool nest_r=0 trace=250 &
  done
  wait
done
echo SCREENDONE
