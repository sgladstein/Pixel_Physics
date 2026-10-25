#!/bin/bash
# arm seed  (off | old | new)
cd /home/claude/shade
arm=$1; s=$2
case $arm in
  off) unset PIXEL_PHYSICS_PLANT_POOL ;;
  old) export PIXEL_PHYSICS_PLANT_POOL=1 ;;
  new) export PIXEL_PHYSICS_PLANT_POOL=/tmp/claude-0/sh/played/pool.ron ;;
esac
RAYON_NUM_THREADS=1 /tmp/claude-0/sh/latecensus_m seed=$s frames=500000 sample=50000 mister=1 > /tmp/claude-0/sh/judge3/${arm}_s$s.txt 2>&1
