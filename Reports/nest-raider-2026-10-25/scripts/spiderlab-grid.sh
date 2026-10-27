#!/bin/bash
G=/tmp/claude-0/-home-user-Pixel-Physics/df1dc8d5-6938-5008-bf40-a54eb4cee899/scratchpad/grid4
read name smell pool seed <<< "$1"
cd /home/user/Pixel_Physics
PP_KILLLOG=1 PIXEL_PHYSICS_NEST_SMELL=$smell PIXEL_PHYSICS_LAB_SCENARIOS=scratch/inrun/labscen $G/spiderlab_bin scenario=lab_mid_fruit pool=$pool species=centipede spiders=2 frames=300000 seed=$seed every=10000 delay=30000 > $G/${name}_${seed}.txt 2> $G/${name}_${seed}.err
