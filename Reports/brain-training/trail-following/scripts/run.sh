#!/bin/bash
# usage: run.sh NAME GENOME SEED [extra env...]
N=$1; G=$2; SD=$3; shift 3
O=/tmp/claude-0/tr/$N-$SD
cd /tmp/claude-0/cloud
CTL="min_age=8000 cooldown=3000 w_deliver=7 w_field=0.05 w_depth=2 rim=15 cap=60 fresh_kid=1 drip=20 mutation=neutral kevents=10 w_home_eat=2 kid_cap=4 retire=1 hall=30000 fprice=tank w_hungry=10 progress=1 turn=30000 kid_grants=3"
rm -rf $O; mkdir -p $O
env RAYON_NUM_THREADS=1 PP_DLOG=$O/dlog.csv PP_DLOG_MOD=5 PP_PHERMAP=1 "$@" /tmp/claude-0/cloudtarget/release/examples/inrun founders=$G:1 mutants=30:0.03:1.0:7 seed=$SD frames=200000 dist=25 spread=10 grants=3 window=4000 sel=off $CTL trace=200 out=$O > $O.log 2>&1
gzip -f $O/dlog.csv
