#!/bin/bash
# M2 role re-fit runner. Run from the root of a Pixel_Physics clone:
#
#   bash /mnt/project-files/plant-community/runners/run.sh NAME SPECIES STATIONS SEED
#
# It pins the tree to e45cd5a17, applies m2-trainer.patch, builds plantrain,
# trains SPECIES on rung 2 twice (score selection, then random selection as
# the control), checks both champions over rung 2 and a whole life in the
# bright and dim stations, and copies every result to
# /mnt/project-files/plant-community/runners/NAME/. It takes about an hour on
# four cores. The last line it prints is RUNNER DONE or RUNNER FAILED.
set -o pipefail
NAME=$1; SP=$2; ST=$3; SEED=$4
if [ -z "$SEED" ]; then echo "usage: run.sh NAME SPECIES STATIONS SEED"; echo RUNNER FAILED; exit 2; fi
HERE=/mnt/project-files/plant-community/runners
OUT=$HERE/$NAME
W=$(mktemp -d /tmp/m2-$NAME-XXXX)
mkdir -p "$OUT"
fail() { echo "$1"; echo "$1" >> "$OUT/status.txt"; cp "$W"/*.log "$OUT/" 2>/dev/null; echo RUNNER FAILED; exit 1; }
echo "started $(date -u +%FT%TZ) name=$NAME species=$SP stations=$ST seed=$SEED host=$(hostname) cores=$(nproc)" > "$OUT/status.txt"

git fetch -q origin main || git fetch -q --unshallow origin || true
git cat-file -e e45cd5a17^{commit} 2>/dev/null || git fetch -q --unshallow origin 2>/dev/null || true
git checkout -q -B m2-runner-$NAME e45cd5a17 || fail "checkout of e45cd5a17 failed"
git apply "$HERE/m2-trainer.patch" || fail "patch did not apply"
cargo build --release --example plantrain > "$W/build.log" 2>&1 || fail "build failed (see build.log)"
cp target/release/examples/plantrain "$W/plantrain"
echo "built $(date -u +%FT%TZ)" >> "$OUT/status.txt"

# Same bed count per child whatever the station count: 6 beds.
NST=$(echo "$ST" | tr ',' '\n' | wc -l)
LAY=$((6 / NST))
COMMON="control=$SP rung=grow stations=$ST pop=16 gens=16 layouts=$LAY holdout=6 final=8 genes=8 sigma=0.9 seed=$SEED"

"$W/plantrain" $COMMON out="$W/score" > "$W/score.log" 2>&1 || fail "score training failed"
echo "score run done $(date -u +%FT%TZ)" >> "$OUT/status.txt"
"$W/plantrain" $COMMON select=random out="$W/random" > "$W/random.log" 2>&1 || fail "random training failed"
echo "random run done $(date -u +%FT%TZ)" >> "$OUT/status.txt"

cp "$W/score/best.ron" "$W/score_best.ron"; cp "$W/random/best.ron" "$W/random_best.ron"
"$W/plantrain" mode=rungs species="$SP,$W/score_best.ron,$W/random_best.ron" rungs=grow,life stations=bright,dim layouts=12 base=7000000 > "$W/check.log" 2>&1 || fail "whole-life check failed"

for r in score random; do
  mkdir -p "$OUT/$r"
  cp "$W/$r"/{best.ron,final.txt,holdout.csv,progress.csv,settings.txt} "$OUT/$r/" 2>/dev/null
done
cp "$W"/*.log "$OUT/"
echo "finished $(date -u +%FT%TZ)" >> "$OUT/status.txt"
echo "== $NAME: $SP on $ST, seed $SEED"
for r in score random; do echo "-- $r"; sed -n '/^final set/,/^champion -/p' "$W/$r.log"; done
echo "-- check (rung 2 and a whole life, 12 layouts each)"
grep -v '^plantrain' "$W/check.log"
echo RUNNER DONE
