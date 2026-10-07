#!/bin/bash
# runedible.sh ARM SEED -- the dig-modes pair rerun on Nest race's crumb fix (NEST_STORE part `edible`, b5922852 merged
# under the modes: branch claude/nest-building-dig-modes-below 5126352f) plus Deep trace's measuring-only store census
# (foodevery=500). hungry=1; the per-decision record is a FIFO read by awk keeping each ant's last row (lastrows.csv).
# Output in /dev/shm (RAM): the disk is nearly full. ARM base|modes.
S=/tmp/claude-0/-home-claude-Pixel-Physics/bb3bd74c-356c-5bc0-9757-39369f5a8b59/scratchpad
ARM=$1; SEED=$2
BIN=$S/modes/bin/deeptrace-edible-census
OUT=/dev/shm/nb/ed-$ARM-s$SEED
[ -f "$OUT/DONE" ] && exit 0
mkdir -p "$OUT"
cd $S/wt-modes || exit 1
mkfifo "$OUT/digrows.csv.gz"
zcat < "$OUT/digrows.csv.gz" 2>"$OUT/zcat.err" | awk -F, 'NR==1{next} {l[$2]=$1","$2","$5","$6} END{print "frame,id,hx,hy"; for(i in l) print l[i]}' > "$OUT/lastrows.csv" &
READER=$!
EXTRA=()
[ "$ARM" = modes ] && EXTRA=(PIXEL_PHYSICS_DIG_MODES=on PIXEL_PHYSICS_DIG_TIP=on PIXEL_PHYSICS_MODES_WHERE=below)
echo "$(date -u +%FT%TZ) arm=$ARM+edible seed=$SEED ${EXTRA[*]} bin=$(sha256sum $BIN | cut -c1-16)" > "$OUT/RUN.txt"
env RAYON_NUM_THREADS=1 PIXEL_PHYSICS_NEEDS_FIRST=on,backfill PIXEL_PHYSICS_CARRY_HOME=on PIXEL_PHYSICS_DOOR_COLUMN=on \
  PIXEL_PHYSICS_LAY_BAR=body PIXEL_PHYSICS_NEST_STORE=on,pick=20,jaws,sky,meal,smell=10,edible PIXEL_PHYSICS_WAY_FOOT=on "${EXTRA[@]}" \
  $BIN scenario=nest_goal seed=$SEED frames=300000 founder=evolved ants=0 hungry=1 dig=1 mapevery=1000 foodgap=90 foodevery=500 out=$OUT \
  > "$OUT/log.txt" 2>&1
echo "exit $?" >> "$OUT/RUN.txt"
wait $READER; echo "reader $?" >> "$OUT/RUN.txt"
touch "$OUT/DONE"
