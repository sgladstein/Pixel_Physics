#!/bin/bash
# SCRATCH (chambers crumbs lane): the tracer binary (OLDBIN) against this build with PP_ROOMBESIDE=1 and no seed,
# one round 0 of 30k frames each; nest.csv and stock.csv must match. usage: noop2.sh OUT POOLDIR WORLD OLDBIN
O=$1; P=$2; W=$3; OB=$4
cd "$(dirname "$0")/../../.."
BIN=$OB bash scratch/nest/poolrounds.sh $O/old $W 1 30000 $P nest_r=0 &
SETX=PP_ROOMBESIDE=1 bash scratch/nest/poolrounds.sh $O/room $W 1 30000 $P nest_r=0 &
wait
cmp $O/old/r0/nest.csv $O/room/r0/nest.csv && cmp $O/old/r0/stock.csv $O/room/r0/stock.csv && echo IDENTICAL
