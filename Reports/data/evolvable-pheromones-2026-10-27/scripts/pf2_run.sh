#!/bin/bash
F=/tmp/claude-0/-home-user-Pixel-Physics/2ef7e65c-531a-5db4-866b-18b502ff6e79/scratchpad/e3_cells_aborted/h2_s31/founder.raw.txt; BIN=/tmp/claude-0/-home-user-Pixel-Physics/2ef7e65c-531a-5db4-866b-18b502ff6e79/scratchpad/nc/inrun2.bin; OUT=/tmp/claude-0/-home-user-Pixel-Physics/2ef7e65c-531a-5db4-866b-18b502ff6e79/scratchpad/pf2
job(){ s=$1; arm=$2; d=$OUT/$arm/s$s; mkdir -p $d; if [ $arm = normal ]; then X=; else X="PP_SCENT_DEAF=$arm"; fi
  env RAYON_NUM_THREADS=1 PIXEL_PHYSICS_ALARM_GROUND=0.03:0.05 PP_MIND_ALARM=train $X $BIN founders=$F:30 seed=$s out=$d sel=off frames=100000 dist=100 food=mix patches=20000:30:110 maps=0 > $d.log 2>&1; rm -f $d/ants_f* $d/events.csv; }
export -f job; export F BIN OUT
for s in $(seq 7001 7016); do for a in normal B A,B; do echo $s $a; done; done | xargs -P 4 -n 2 bash -c 'job "$0" "$1"'
