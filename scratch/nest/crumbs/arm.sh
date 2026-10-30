#!/bin/bash
# SCRATCH (chambers crumbs lane, 2026-10-30): one arm, several garden worlds at once, crumb log on.
# usage: arm.sh RUNS ARM POOLDIR ROUNDS WORLD... (extra env, e.g. SETX, passes through)
R=$1; A=$2; P=$3; N=$4; shift 4
cd "$(dirname "$0")/../../.."
for w in "$@"; do CRUMBLOG=1 bash scratch/nest/poolrounds.sh $R/${A}_w$w $w $N 200000 $P nest_r=0 & done
wait
