#!/bin/bash
# SCRATCH (chambers crumbs lane): the corrected room-seed screen (slot 18 zeroed off crop put-downs under the switch):
# bs = H3+eat with PP_ROOMBESIDE=1 and no seed (its own baseline), e2 = the room seed with it. usage: queue2.sh RUNS SCRATCH
R=$1; S=$2
cd "$(dirname "$0")/../../.."
export SETX=PP_ROOMBESIDE=1
bash scratch/nest/crumbs/arm.sh $R e2 $S/poolE2 3 61 62 63 64
bash scratch/nest/crumbs/arm.sh $R bs $S/pool/poolH3eat 3 61 62 63 64
bash scratch/nest/crumbs/arm.sh $R e2 $S/poolE2 3 65 66 67 68
bash scratch/nest/crumbs/arm.sh $R bs $S/pool/poolH3eat 3 65 66 67 68
