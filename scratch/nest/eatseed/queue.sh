#!/bin/bash
# SCRATCH (chambers lane, eat seed): the paired screen -- poolrounds.sh 4 x 200k sel=score on worlds 68 73 61 64, one
# lane per arm, with the trace rows and hungry probe the chambers trace used (read-only), so the H3 and std_pool
# arms already in OUT/../r4 (same commit, same args) pair with these. usage: queue.sh OUT ARM:POOL [ARM:POOL ...]
O=$1; shift
mkdir -p $O
lane() { local arm=${1%%:*} pool=${1#*:}; for w in ${WORLDS:-68 73 61 64}; do
  PP_HUNGRYPROBE=1 BIN=target/release/examples/inrun bash scratch/nest/poolrounds.sh $O/${arm}_w$w $w 4 200000 $pool nest_r=0 trace=250; done; }
for a in "$@"; do lane $a & done
wait
echo QUEUEDONE
