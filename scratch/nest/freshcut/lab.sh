#!/bin/bash
# lab.sh OUTDIR POOLDIR SEED AFTER EVERY [ENV=V ...]: one lab-box run of a pool (one founder per *.raw.txt), nest maps
# at EVERY frames. LABBIN picks the binary, SCEN the box (played_bed), EXTRA adds labsurvive args (laycap=48:8 nestrows=88).
o=$1; p=$2; sd=$3; af=$4; ev=$5; shift 5
F=$(ls $p/*.raw.txt | awk '{printf "%s%s@p%02d:1", (NR>1?",":""), $0, NR-1}')
mkdir -p $o
env "$@" RAYON_NUM_THREADS=1 PIXEL_PHYSICS_LAB_SCENARIOS=assets/lab_scenarios ${LABBIN:-target/release/examples/labsurvive} \
  founders=$F scenario=${SCEN:-played_bed} seed=$sd after=$af nestmap=$o nestat=$ev ${EXTRA:-} > $o/log.txt 2>&1
touch $o/done
