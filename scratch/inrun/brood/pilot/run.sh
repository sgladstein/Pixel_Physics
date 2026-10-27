#!/bin/bash
# usage: run.sh NAME SEED FRAMES [extra args...]; setenv extras via SETX
cd /home/user/brood-wt
N=$1; S=$2; F=$3; shift 3
O=/tmp/claude-0/-home-user-Pixel-Physics/c7f51f81-78a1-5e63-8d5d-37e2280ac4f0/scratchpad/pilot1/$N
mkdir -p $O
SE="setenv=PP_HOMEVEC=40:30,PP_BODYRULES=both,PP_DIGHEAD=scratch/inrun/genomes/dh7.head,PP_DIGHEAD_GATE=1,PP_DIGHEAD_EVOLVE=1,PP_DIGLIGHT=1,PP_BODYRULES_P=1${SETX:+,$SETX}"
SECONDS=0; target/release/examples/inrun founders=scratch/inrun/genomes/hcs.raw.txt:1 mutants=30:0.03:1.0:7 seed=$S frames=$F dist=25 spread=10 grants=3 window=4000 sel=score min_age=8000 cooldown=3000 w_deliver=7 w_field=0.05 w_depth=2 rim=15 cap=60 fresh_kid=1 drip=20 mutation=neutral kevents=10 w_home_eat=2 kid_cap=4 retire=1 hall=30000 fprice=tank w_hungry=10 progress=1 turn=30000 kid_grants=3 lesson=scratch/inrun/genomes/c1.raw.txt:0.1 "$SE" cut_join=32 "$@" out=$O > $O.log 2>&1
echo "wall $SECONDS s" >> $O.log
