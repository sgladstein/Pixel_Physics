#!/bin/bash
# SCRATCH (chambers crumbs lane): is the crumb log read-only? one round 0, 30k frames, with and without it; nest.csv must match.
# usage: noop.sh OUT POOLDIR WORLD
O=$1; P=$2; W=$3
cd "$(dirname "$0")/../../.."
bash scratch/nest/poolrounds.sh $O/off $W 1 30000 $P nest_r=0 &
CRUMBLOG=1 bash scratch/nest/poolrounds.sh $O/on $W 1 30000 $P nest_r=0 &
wait
cmp $O/off/r0/nest.csv $O/on/r0/nest.csv && cmp $O/off/r0/stock.csv $O/on/r0/stock.csv && echo IDENTICAL
