#!/bin/bash
# usage: dlogT.sh GENOME SEED -- the genome acts; the recruitment-trail teacher labels its states (PP_TRAIL_LABEL)
G=$1; SD=$2; N=$(basename $G .txt)
L=/tmp/claude-0/tt/logs
cd /tmp/claude-0/cloud
CTL="min_age=8000 cooldown=3000 w_deliver=7 w_field=0.05 w_depth=2 rim=15 cap=60 fresh_kid=1 drip=20 mutation=neutral kevents=10 w_home_eat=2 kid_cap=4 retire=1 hall=30000 fprice=tank w_hungry=10 progress=1 turn=30000 kid_grants=3"
env RAYON_NUM_THREADS=1 PP_PACK=1 PP_TRAIL_LABEL=1:0.2 PP_TRAIL_OUT=1 PP_COPY_LOG=$L/O$N-$SD.log PP_COPY_SAMPLE=30:10 /tmp/claude-0/cloudtarget/release/examples/inrun founders=$G:1 mutants=30:0.03:1.0:7 seed=$SD frames=200000 dist=25 spread=10 grants=3 window=4000 sel=off $CTL out=$L/O$N-$SD > $L/O$N-$SD.out 2>&1
rm -rf $L/O$N-$SD
