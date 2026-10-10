#!/bin/bash
# Deep trace 2026-10-07: the same runs as runres.sh plus the store food census (foodevery=500, storefood.csv; measuring only).
# Usage: runfood.sh <on|off> <seed> <frames>
SP=/tmp/claude-0/-home-claude-Pixel-Physics/7984c59b-615f-5ce8-a05f-c056f80a3af9/scratchpad
BIN=$SP/store2/dtwffood
ARM=$1; S=$2; FR=$3
cd /home/claude/wt-wf || exit 1
D=/dev/shm/wf/food-$ARM-s$S; mkdir -p $D
env RAYON_NUM_THREADS=1 PIXEL_PHYSICS_NEEDS_FIRST=on,backfill PIXEL_PHYSICS_CARRY_HOME=on PIXEL_PHYSICS_DOOR_COLUMN=on \
    PIXEL_PHYSICS_NEST_STORE=on,pick=20,jaws,sky,meal,smell=10 PIXEL_PHYSICS_LAY_BAR=body PIXEL_PHYSICS_WAY_FOOT=$ARM \
    $BIN scenario=nest_goal seed=$S frames=$FR founder=evolved ants=0 mapevery=1000 hungry=1 dig=1 foodgap=90 foodevery=500 out=$D > $D.log 2>&1
echo "rc=$? $(date +%H:%M:%S)" >> $D.log
