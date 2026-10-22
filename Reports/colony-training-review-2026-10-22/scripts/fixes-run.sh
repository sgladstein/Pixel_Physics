#!/bin/bash
# Fix test for the in-colony training review (2026-10-22). hA1 founder, 12 seeds, same frames per arm (1.2M).
# ctl  = standard score selection, 6 x 200k (should reproduce the mutload test's scoreM byte-for-byte: switches off)
# band = headcount band grows 24..36 -> 72 by 15k-frame steps (grow_band=15000:72)
# cross= half of quota children are crossovers of two eligible parents (cross=0.5)
# lex  = parents picked by epsilon-lexicase over five per-ant rates (pick=lexicase)
# prot = lesson slots (hA1 vs c1) mutate at a tenth of the rate, no structural events on them (lesson=c1.raw.txt:0.1)
set -u
W=/home/user/review-fix
S=/tmp/claude-0/-home-user-Pixel-Physics/44bcf220-1a62-5c29-9989-f15bd0e562ae/scratchpad/fixes
export BIN=$W/target/release/examples/inrun
G=$W/scratch/inrun/genomes/hA1.raw.txt
COMMON="min_age=8000 cooldown=3000 w_deliver=7 w_field=0.05 w_depth=2 rim=15 cap=60 fresh_kid=1 drip=20 mutation=neutral kevents=10 w_home_eat=2 kid_cap=4 retire=1 hall=30000 fprice=tank w_hungry=10 progress=1 turn=30000 kid_grants=3"
: > $S/jobs.txt
for s in $(seq 1 12); do
  echo "band_$s $s 6 200000 score $G $COMMON grow_band=15000:72" >> $S/jobs.txt
  echo "cross_$s $s 6 200000 score $G $COMMON cross=0.5" >> $S/jobs.txt
  echo "lex_$s $s 6 200000 score $G $COMMON pick=lexicase" >> $S/jobs.txt
  echo "prot_$s $s 6 200000 score $G $COMMON lesson=$W/scratch/inrun/genomes/c1.raw.txt:0.1" >> $S/jobs.txt
done
cd $W
run_one() {
  o=$1; shift
  bash scratch/inrun/rounds.sh $S/jobs/$o "$@" > $S/jobs/$o.log 2>&1
  # keep nest.csv/stock.csv/births.csv/final; drop the big logs
  rm -f $S/jobs/$o/r*/events.csv $S/jobs/$o/r*/map_f* $S/jobs/$o/r*/ants_f0[01]*
  echo "$o done $(date -u +%T)" >> $S/progress.txt
}
export -f run_one; export S
echo "start $(date -u +%T)" > $S/progress.txt
xargs -P 4 -L 1 bash -c 'run_one "$@"' _ < $S/jobs.txt
echo "TRAINING DONE $(date -u +%T)" >> $S/progress.txt
