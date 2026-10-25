#!/bin/bash
# arm seed
cd /home/claude/shade
arm=$1; s=$2
if [ "$arm" = off ]; then export PIXEL_PHYSICS_PLANT_POOL=0; fi
RAYON_NUM_THREADS=1 /tmp/claude-0/sh/latecensus_pool2 seed=$s frames=500000 sample=50000 > /tmp/claude-0/sh/labpool2/${arm}_s$s.txt 2>&1
