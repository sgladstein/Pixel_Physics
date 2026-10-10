#!/bin/bash
# pair12.sh ARM HEAP SEED OUTROOT -- one 300k run of the default-flip comparison.
#   ARM main : no PIXEL_PHYSICS_* set (the stack all off = main's game)
#   ARM stack: the tested stack, every switch on as below
# Build once from the repo root: cargo build --release --example deeptrace
# Run from the repo root (deeptrace reads assets/ relative to it). One run takes ~20 min on one core.
# hungry.csv is cut on the fly to the five columns one12.py reads (hred.csv.gz); broodlog/digrows/genomes are dropped.
set -u; arm=$1; heap=$2; seed=$3; root=$4; out=$root/$arm$heap-s$seed
mkdir -p "$out"; rm -f "$out/hungry.csv.gz"; mkfifo "$out/hungry.csv.gz"
ln -sfn /dev/null "$out/broodlog.csv"; ln -sfn /dev/null "$out/digrows.csv.gz"; ln -sfn /dev/null "$out/genomes.txt"
(gunzip -c < "$out/hungry.csv.gz" | awk -F, 'NR==1{for(i=1;i<=NF;i++)c[$i]=i; print "frame,id,hy,zone,outcome"; next}{print $c["frame"]","$c["id"]","$c["hy"]","$c["zone"]","$c["outcome"]}' | gzip -4 > "$out/hred.csv.gz") &
sw=()
[ "$arm" = stack ] && sw=(PIXEL_PHYSICS_NEEDS_FIRST=on,backfill PIXEL_PHYSICS_CARRY_HOME=on PIXEL_PHYSICS_DOOR_COLUMN=on PIXEL_PHYSICS_LAY_BAR=body PIXEL_PHYSICS_NEST_STORE=on,pick=20,jaws,sky,meal,smell=10,edible PIXEL_PHYSICS_WAY_FOOT=on)
env $(env | grep -o '^PIXEL_PHYSICS_[A-Z_]*' | sed 's/^/-u /') RAYON_NUM_THREADS=1 "${sw[@]}" \
  target/release/examples/deeptrace scenario=nest_goal seed=$seed foodgap=$heap frames=300000 founder=evolved ants=0 mapevery=25000 shots=1 hungry=1 dig=1 out=$out > $out.log 2>&1
echo "rc=$?" >> $out.log
wait
