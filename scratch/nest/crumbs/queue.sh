#!/bin/bash
# SCRATCH (chambers crumbs lane): the screen's batches in turn, 4 worlds at a time. usage: queue.sh RUNS SCRATCH
R=$1; S=$2
cd "$(dirname "$0")/../../.."
bash scratch/nest/crumbs/arm.sh $R base $S/pool/poolH3eat 3 65 66 67 68
bash scratch/nest/crumbs/arm.sh $R e1 $S/poolE1 3 61 62 63 64
SETX=PP_ROOMBESIDE=1 bash scratch/nest/crumbs/arm.sh $R e2 $S/poolE2 3 61 62 63 64
bash scratch/nest/crumbs/arm.sh $R e1 $S/poolE1 3 65 66 67 68
SETX=PP_ROOMBESIDE=1 bash scratch/nest/crumbs/arm.sh $R e2 $S/poolE2 3 65 66 67 68
