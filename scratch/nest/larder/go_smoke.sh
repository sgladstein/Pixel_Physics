#!/bin/bash
# SCRATCH (chambers lane, larder seed): the switch control (H3+eat with PP_FOODBESIDE=1 must reproduce H3+eat's round 0
# on w68: poolrounds.sh, sel=score, the eat screen's args) beside three smokes (w68 round 0 sel=off, smoke.sh).
# usage: go_smoke.sh SCRATCH   (pools in SCRATCH/pools/{h3n,lt,lh}; results in SCRATCH/lard)
S=$1; L=$S/lard; P=$S/pools
mkdir -p $L/ctl $L/smoke
PP_FOODBESIDE=1 PP_HUNGRYPROBE=1 BIN=target/release/examples/inrun bash scratch/nest/poolrounds.sh $L/ctl/h3n_w68 68 1 200000 $P/h3n nest_r=0 trace=250 &
for a in ${ARMS:-h3n lt lh}; do bash scratch/nest/larder/smoke.sh $L/smoke/${a}_w68 68 $P/$a & done
wait
echo SMOKEDONE
