#!/bin/bash
# Mutation-load test for the in-colony training review (2026-10-22). hA1 founder, 12 seeds, 6 rounds x 200k frames.
# Arms: scoreM = standard score selection with the standard mutation; randM = random parents, standard mutation;
#       score0 = score selection, NO mutation (children are exact copies); rand0 = random parents, no mutation.
set -u
W=/home/user/review-inrun
S=/tmp/claude-0/-home-user-Pixel-Physics/44bcf220-1a62-5c29-9989-f15bd0e562ae/scratchpad/mutload
export BIN=$W/target/release/examples/inrun
G=$W/scratch/inrun/genomes/hA1.raw.txt
COMMON="min_age=8000 cooldown=3000 w_deliver=7 w_field=0.05 w_depth=2 rim=15 cap=60 fresh_kid=1 drip=20 mutation=neutral w_home_eat=2 kid_cap=4 retire=1 hall=30000 fprice=tank w_hungry=10 progress=1 turn=30000 kid_grants=3"
: > $S/jobs.txt
for s in $(seq 1 12); do
  echo "scoreM_$s $s 6 200000 score $G $COMMON kevents=10" >> $S/jobs.txt
  echo "randM_$s $s 6 200000 random $G $COMMON kevents=10" >> $S/jobs.txt
  echo "score0_$s $s 6 200000 score $G $COMMON kevents=0 kpm=0" >> $S/jobs.txt
  echo "rand0_$s $s 6 200000 random $G $COMMON kevents=0 kpm=0" >> $S/jobs.txt
done
cd $W
run_one() {
  o=$1; shift
  bash scratch/inrun/rounds.sh $S/jobs/$o "$@" > $S/jobs/$o.log 2>&1
  # keep the disk small: the event logs of the training rounds are not needed, only final/ and births.csv
  rm -f $S/jobs/$o/r*/events.csv $S/jobs/$o/r*/map_f* $S/jobs/$o/r*/ants_f0[01]*
  echo "$o done $(date -u +%T)" >> $S/progress.txt
}
export -f run_one; export S
echo "start $(date -u +%T)" > $S/progress.txt
xargs -P 4 -L 1 bash -c 'run_one "$@"' _ < $S/jobs.txt
echo "TRAINING DONE $(date -u +%T)" >> $S/progress.txt
