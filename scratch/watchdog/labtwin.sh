#!/usr/bin/env bash
# SCRATCH (score-gaming watchdog, 2026-10-28): the colsel lab case's twin. Reruns one colony-round's carry in the lab
# case's own boxes and seeds (colsel seed_of('lab', c, r, k)), past the case's 15,000 frames, with NESTAT lines at
# fixed frames after landing. The 15k NESTAT/lived prefix must equal the voted lab score (positive control).
# usage: labtwin.sh ARM_DIR ROUND COLONY OUTDIR AFTER [SCENS]    env as colsel's LAB_ENV (setenv= pairs)
set -u
A=$1; R=$2; C=$3; O=$4; AFTER=$5; SCENS=${6:-lab_far_mix,lab_far200_fruit}
D=$A/c$C/r$R
F=$(python3 -c "
import csv
r=list(csv.DictReader(open('$D/final/index.csv'))); r.sort(key=lambda x:int(x['id'])); r.sort(key=lambda x:(-float(x['score']),int(x['id']))); r=r[:30]; n=len(r)
print(','.join(f\"$D/final/{x['id']}.raw.txt@{x['family']}.{x['id']}:{30//n+(1 if i<30%n else 0)}\" for i,x in enumerate(r)))")
k=0
for sc in ${SCENS//,/ }; do
  seed=$((C * 1000 + R * 10 + k))
  out=$O/r${R}c${C}_$sc
  mkdir -p $out
  RAYON_NUM_THREADS=1 PIXEL_PHYSICS_LAB_SCENARIOS=scratch/inrun/labscen target/release/examples/labsurvive founders=$F scenario=$sc seed=$seed \
    after=$AFTER nestmap=$out nestat=15000,50000,100000,150000,200000,250000,300000 > $out.log 2>&1
  k=$((k + 1))
done
