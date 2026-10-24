#!/bin/bash
arm=$1; seed=$2; S=/tmp/claude-0/-home-user-Pixel-Physics/9303af08-6d06-50c1-8ee3-75b50ebc3033/scratchpad
nb=0; [ "$arm" = nut ] && nb=1
PIXEL_PHYSICS_NUTRIENT_BY_ROOT=$nb PIXEL_PHYSICS_LEAF_DEPTH=1.0 $S/plant_m0n2 mode=community mix=roles loadfail=0 rain=steady spells=20000,40000 ground=patchy seed=$seed frames=450000 sample=50000 shots=150000,450000 below=150 rootcensus=1 out=$S/nbatch/${arm}_s$seed > $S/nbatch/${arm}_s$seed.txt 2>&1
