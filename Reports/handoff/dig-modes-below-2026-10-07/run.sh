#!/bin/bash
# run.sh ARM SEED -- ARM base|modes; the brief's stack, heap 90, 300k; hungry=0 and digfrom past the end (both logging only).
S=/tmp/claude-0/-home-claude-Pixel-Physics/bb3bd74c-356c-5bc0-9757-39369f5a8b59/scratchpad
ARM=$1; SEED=$2
BIN=$S/modes/bin/deeptrace-modes
OUT=$S/modes/runs/$ARM-s$SEED
[ -f "$OUT/DONE" ] && exit 0
mkdir -p "$OUT"
# This base has no digfrom=: send the per-decision dig record (~250 MB a run) to /dev/null. Logging only.
ln -sf /dev/null "$OUT/digrows.csv.gz"
cd $S/wt-modes || exit 1
EXTRA=()
[ "$ARM" = modes ] && EXTRA=(PIXEL_PHYSICS_DIG_MODES=on PIXEL_PHYSICS_DIG_TIP=on PIXEL_PHYSICS_MODES_WHERE=below)
echo "$(date -u +%FT%TZ) arm=$ARM seed=$SEED ${EXTRA[*]} bin=$(sha256sum $BIN | cut -c1-16)" > "$OUT/RUN.txt"
env RAYON_NUM_THREADS=1 PIXEL_PHYSICS_NEEDS_FIRST=on,backfill PIXEL_PHYSICS_CARRY_HOME=on PIXEL_PHYSICS_DOOR_COLUMN=on \
  PIXEL_PHYSICS_LAY_BAR=body PIXEL_PHYSICS_NEST_STORE=on,pick=20,jaws,sky,meal,smell=10 PIXEL_PHYSICS_WAY_FOOT=on "${EXTRA[@]}" \
  $BIN scenario=nest_goal seed=$SEED frames=300000 founder=evolved ants=0 hungry=0 dig=1 mapevery=1000 foodgap=90 out=$OUT \
  > "$OUT/log.txt" 2>&1
echo "exit $?" >> "$OUT/RUN.txt"
touch "$OUT/DONE"
