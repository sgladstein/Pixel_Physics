#!/bin/bash
# sc founders seed
cd /home/user/Pixel_Physics; export RAYON_NUM_THREADS=1 PIXEL_PHYSICS_LAB_SCENARIOS=$S/scen
o=$S/lab/$1-$2-$3.txt
mkdir -p $(dirname $o); [ -s $o ] && exit 0
target/release/examples/labsurvive scenario=$1 founders=$2 seed=$3 after=60000 probe=10000 > $o.tmp 2>&1 && mv $o.tmp $o
