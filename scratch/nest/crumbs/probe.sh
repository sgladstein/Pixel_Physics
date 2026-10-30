#!/bin/bash
# SCRATCH (chambers crumbs lane, step 2): two mind-logged round-0 runs under PP_STORESENSE=1, H3+eat and the
# units-only store seed, FRAMES (default 40k) frames on world W. usage: probe.sh OUT SCRATCH W
O=$1; S=$2; W=$3
cd "$(dirname "$0")/../../.."
export PP_STORESENSE=1 FRAMES=${FRAMES:-40000}
bash scratch/nest/trace.sh $O/plain $W $S/pool/poolH3eat nest_r=0 &
bash scratch/nest/trace.sh $O/units $W $S/poolSu nest_r=0 &
wait
for a in plain units; do echo "== $a"; python3 scratch/nest/crumbs/probe.py $O/$a/mind.log.gz; done
