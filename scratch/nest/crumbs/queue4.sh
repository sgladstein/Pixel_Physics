#!/bin/bash
# SCRATCH (chambers crumbs lane, step 2): ss = H3+eat under PP_STORESENSE=1 and no seed (the switch's own baseline);
# then the store seed arm(s) named in ARMS (ARM=POOLDIR, ...), 4 worlds at a time. usage: queue4.sh RUNS [ARM=POOL ...]
R=$1; shift
cd "$(dirname "$0")/../../.."
export SETX=PP_STORESENSE=1
S=/tmp/claude-0/-home-user-Pixel-Physics/5839c770-56d6-594d-ba6d-0b2ec65d8626/scratchpad
for spec in "$@"; do
  a=${spec%%=*}; p=${spec#*=}
  bash scratch/nest/crumbs/arm.sh $R $a $p 3 61 62 63 64
  bash scratch/nest/crumbs/arm.sh $R $a $p 3 65 66 67 68
done
