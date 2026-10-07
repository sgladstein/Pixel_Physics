#!/bin/bash
# runfood.sh SEED -- the modes arm again with Deep trace's store-food census (probe-v4-foodcensus patch, measuring only):
# hungry=1 (ledger, hungry.csv.gz), dig=1 with the per-decision record kept (for the starvers' last rows), foodevery=500.
# Output in /dev/shm (RAM) because the disk is nearly full. The per-decision record is a FIFO read by awk, which keeps
# only each ant's last row (lastrows.csv: frame,id,hx,hy); the full record would be ~2 GB a run.
# Otherwise exactly run.sh's modes arm.
S=/tmp/claude-0/-home-claude-Pixel-Physics/bb3bd74c-356c-5bc0-9757-39369f5a8b59/scratchpad
SEED=$1
BIN=$S/modes/bin/deeptrace-modes-census
OUT=/dev/shm/nb/modes-food-s$SEED
[ -f "$OUT/DONE" ] && exit 0
mkdir -p "$OUT"
cd $S/wt-modes || exit 1
mkfifo "$OUT/digrows.csv.gz"
zcat < "$OUT/digrows.csv.gz" 2>"$OUT/zcat.err" | awk -F, 'NR==1{next} {l[$2]=$1","$2","$5","$6} END{print "frame,id,hx,hy"; for(i in l) print l[i]}' > "$OUT/lastrows.csv" &
READER=$!
echo "$(date -u +%FT%TZ) arm=modes+census seed=$SEED bin=$(sha256sum $BIN | cut -c1-16)" > "$OUT/RUN.txt"
env RAYON_NUM_THREADS=1 PIXEL_PHYSICS_NEEDS_FIRST=on,backfill PIXEL_PHYSICS_CARRY_HOME=on PIXEL_PHYSICS_DOOR_COLUMN=on \
  PIXEL_PHYSICS_LAY_BAR=body PIXEL_PHYSICS_NEST_STORE=on,pick=20,jaws,sky,meal,smell=10 PIXEL_PHYSICS_WAY_FOOT=on \
  PIXEL_PHYSICS_DIG_MODES=on PIXEL_PHYSICS_DIG_TIP=on PIXEL_PHYSICS_MODES_WHERE=below \
  $BIN scenario=nest_goal seed=$SEED frames=300000 founder=evolved ants=0 hungry=1 dig=1 mapevery=1000 foodgap=90 foodevery=500 out=$OUT \
  > "$OUT/log.txt" 2>&1
echo "exit $?" >> "$OUT/RUN.txt"
wait $READER; echo "reader $?" >> "$OUT/RUN.txt"
touch "$OUT/DONE"
