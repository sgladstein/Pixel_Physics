#!/bin/bash
# SCRATCH (chambers crumbs lane): the room seed without its hold (e2t: target preference only), under the switch,
# against queue2's bs. Waits for queue2's last batch. usage: queue3.sh RUNS SCRATCH
R=$1; S=$2
cd "$(dirname "$0")/../../.."
until [ -f $R/bs_w68/r2/.done ] || [ -f $R/bs_w68/EXTINCT ]; do sleep 10; done
export SETX=PP_ROOMBESIDE=1
bash scratch/nest/crumbs/arm.sh $R e2t $S/poolE2t 3 61 62 63 64
bash scratch/nest/crumbs/arm.sh $R e2t $S/poolE2t 3 65 66 67 68
