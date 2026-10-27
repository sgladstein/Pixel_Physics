#!/bin/bash
# Rung-4 runner. Run from the root of a Pixel_Physics clone:
#
#   bash /mnt/project-files/plant-community/runners/hold_floor.sh NAME SPECIES SEED START_RON
#
# Pins the tree to 4d63b9028 (branch claude/project-thread-uodokv) plus floor.patch (the neighbour floor in the rung-4 score), builds
# plantrain, trains SPECIES on rung 4 (a whole life beside each other role)
# from START_RON (the rung-2 re-fit) twice -- score selection, then random selection as the control -- checks
# both champions on fresh rung-4 beds, and copies every result to
# /mnt/project-files/plant-community/runners/NAME/. About an hour on four
# cores. The last line it prints is RUNNER DONE or RUNNER FAILED.
set -o pipefail
NAME=$1; SP=$2; SEED=$3; START=$4
if [ -z "$START" ]; then echo "usage: hold_floor.sh NAME SPECIES SEED START_RON"; echo RUNNER FAILED; exit 2; fi
SHA=4d63b9028174341e18b9e4e742a3f6bc9eaa2409
HERE=/mnt/project-files/plant-community/runners
OUT=$HERE/$NAME
W=$(mktemp -d /tmp/m2-$NAME-XXXX)
mkdir -p "$OUT"
fail() { echo "$1"; echo "$1" >> "$OUT/status.txt"; cp "$W"/*.log "$OUT/" 2>/dev/null; echo RUNNER FAILED; exit 1; }
echo "started $(date -u +%FT%TZ) name=$NAME species=$SP seed=$SEED host=$(hostname) cores=$(nproc)" > "$OUT/status.txt"

git fetch -q origin claude/project-thread-uodokv || git fetch -q --unshallow origin || true
git cat-file -e $SHA^{commit} 2>/dev/null || git fetch -q origin $SHA || fail "cannot fetch $SHA"
git reset -q --hard; git checkout -q -B m2-hold-$NAME $SHA || fail "checkout of $SHA failed"
git apply "$HERE/floor.patch" || fail "floor.patch did not apply"
cp "$START" "$W/start.ron" || fail "no start file $START"
cargo build --release --example plantrain > "$W/build.log" 2>&1 || fail "build failed (see build.log)"
cp target/release/examples/plantrain "$W/plantrain"
echo "built $(date -u +%FT%TZ)" >> "$OUT/status.txt"

COMMON="start=$W/start.ron control=$SP rung=hold stations=bright pop=16 gens=12 layouts=2 holdout=4 final=6 genes=8 sigma=0.9 seed=$SEED"
"$W/plantrain" $COMMON out="$W/score" > "$W/score.log" 2>&1 || fail "score training failed"
echo "score run done $(date -u +%FT%TZ)" >> "$OUT/status.txt"
"$W/plantrain" $COMMON select=random out="$W/random" > "$W/random.log" 2>&1 || fail "random training failed"
echo "random run done $(date -u +%FT%TZ)" >> "$OUT/status.txt"

cp "$W/score/best.ron" "$W/score_best.ron"; cp "$W/random/best.ron" "$W/random_best.ron"
"$W/plantrain" mode=rungs species="$SP,$W/start.ron,$W/score_best.ron,$W/random_best.ron" rungs=hold stations=bright layouts=8 base=7000000 > "$W/check.log" 2>&1 || fail "check failed"

for r in score random; do
  mkdir -p "$OUT/$r"
  cp "$W/$r"/{best.ron,final.txt,holdout.csv,progress.csv,settings.txt} "$OUT/$r/" 2>/dev/null
done
cp "$W"/*.log "$OUT/"
echo "finished $(date -u +%FT%TZ)" >> "$OUT/status.txt"
echo "== $NAME: $SP on rung 4, seed $SEED"
for r in score random; do echo "-- $r"; sed -n '/^final set/,/^champion -/p' "$W/$r.log"; done
echo "-- check (rung 4, 8 layouts, beside each other role)"
grep -v '^plantrain' "$W/check.log" | sed 's/| rgr.*//'
echo RUNNER DONE
