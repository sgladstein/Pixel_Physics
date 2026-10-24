#!/bin/bash
# run.sh NAME SEED FRAMES [extra inrun args]  -- common-garden settings, hcs founder x30, sel=off
S=/tmp/claude-0/-home-user-Pixel-Physics/b300ed13-cc9c-5e26-8b14-2acd8b3d007f/scratchpad/g
R=/home/user/Pixel_Physics
n=$1; seed=$2; fr=$3; shift 3
A="min_age=8000 cooldown=3000 w_deliver=7 w_field=0.05 w_depth=2 rim=15 fresh_kid=1 mutation=neutral kevents=10 w_home_eat=2 kid_cap=4 retire=1 hall=30000 fprice=tank w_hungry=10 progress=1 turn=30000 kid_grants=3"
E="setenv=${XENV:+$XENV,}PP_HOMEVEC=40:30,PP_DIGHEAD=$R/scratch/inrun/genomes/dh7.head,PP_DIGHEAD_GATE=1,PP_DIGHEAD_EVOLVE=1,PP_DIGLIGHT=1"
rm -rf $S/out/$n; mkdir -p $S/out/$n
cd $R && target/release/examples/inrun founders=$R/scratch/inrun/genomes/hcs.raw.txt:30 seed=$seed frames=$fr grants=3 window=4000 cut_join=32 "$@" sel=off $A $E out=$S/out/$n > $S/out/$n.log 2>&1; echo "wall $((SECONDS)) s" >> $S/out/$n.log
