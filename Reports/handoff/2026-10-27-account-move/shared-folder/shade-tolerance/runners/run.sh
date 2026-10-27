#!/bin/bash
# A5 shade-tolerance training runner. Run from the root of a Pixel_Physics clone:
#
#   bash /mnt/project-files/shade-tolerance/runners/run.sh NAME SPECIES STATIONS SCALD SEED
#
# SCALD is 1 (sun scald on, the price) or 0 (the control). It pins the tree to
# 147506bc (branch claude/project-thread-fprel1), builds plantrain, trains
# SPECIES on rung 4 (whole life beside the shipped tree) in STATIONS, checks
# the champion beside the tree in the bright and shade stations, and copies every result to
# /mnt/project-files/shade-tolerance/runners/NAME/. About 30-45 minutes on four
# cores. The last line it prints is RUNNER DONE or RUNNER FAILED.
set -o pipefail
NAME=$1; SP=$2; ST=$3; SCALD=$4; SEED=$5
if [ -z "$SEED" ]; then echo "usage: run.sh NAME SPECIES STATIONS SCALD SEED"; echo RUNNER FAILED; exit 2; fi
HERE=/mnt/project-files/shade-tolerance/runners
OUT=$HERE/$NAME
W=$(mktemp -d /tmp/a5-$NAME-XXXX)
mkdir -p "$OUT"
fail() { echo "$1"; echo "$1" >> "$OUT/status.txt"; cp "$W"/*.log "$OUT/" 2>/dev/null; echo RUNNER FAILED; exit 1; }
echo "started $(date -u +%FT%TZ) name=$NAME species=$SP stations=$ST scald=$SCALD seed=$SEED host=$(hostname) cores=$(nproc)" > "$OUT/status.txt"
git fetch -q origin claude/project-thread-fprel1 || fail "fetch failed"
git checkout -q -B a5-runner-$NAME 147506bc || fail "checkout of 147506bc failed"
cargo build --release --example plantrain > "$W/build.log" 2>&1 || fail "build failed (see build.log)"
cp target/release/examples/plantrain "$W/plantrain"
echo "built $(date -u +%FT%TZ)" >> "$OUT/status.txt"
export PIXEL_PHYSICS_SHADE_SCALD=$SCALD
NST=$(echo "$ST" | tr ',' '\n' | wc -l)
LAY=$((6 / NST))
"$W/plantrain" control=$SP rung=hold neighbours=tree stations=$ST pop=16 gens=16 layouts=$LAY holdout=6 final=8 genes=8 sigma=0.9 seed=$SEED out="$W/score" > "$W/score.log" 2>&1 || fail "training failed"
echo "training done $(date -u +%FT%TZ)" >> "$OUT/status.txt"
cp "$W/score/best.ron" "$W/score_best.ron"
"$W/plantrain" mode=rungs species="$SP,$W/score_best.ron" rungs=life,hold neighbours=tree stations=bright,shade layouts=12 base=7000000 > "$W/check.log" 2>&1 || fail "check failed"
mkdir -p "$OUT/score"
cp "$W/score"/{best.ron,final.txt,holdout.csv,progress.csv,settings.txt} "$OUT/score/" 2>/dev/null
cp "$W"/*.log "$OUT/"
echo "finished $(date -u +%FT%TZ)" >> "$OUT/status.txt"
echo "== $NAME: $SP on $ST, scald $SCALD, seed $SEED"
sed -n '/^final set/,/^champion -/p' "$W/score.log"
grep -n "shade_tolerate\|shade_avoid" "$W/score_best.ron" | head -2
grep -v '^plantrain' "$W/check.log"
echo RUNNER DONE
