#!/bin/bash
# Mixed-bed replicate runner. From the root of a Pixel_Physics clone:
#   bash /mnt/project-files/plant-community/runners/mixed450.sh NAME SEEDS
# e.g. SEEDS=7,8,9,10. Builds plant_m0 at origin/main, runs the 450,000-frame
# four-role bed for each seed twice -- shipped species, and the seed-trained
# shrub/herb/creeper (m2/hold/seeds/) with the shipped tree -- 4 at a time,
# and copies the logs to runners/NAME/. Prints a table and RUNNER DONE.
set -o pipefail
NAME=$1; SEEDS=$2
[ -z "$SEEDS" ] && { echo "usage: mixed450.sh NAME SEEDS"; echo RUNNER FAILED; exit 2; }
OUT=/mnt/project-files/plant-community/runners/$NAME; mkdir -p "$OUT"
W=$(mktemp -d /tmp/mixed-$NAME-XXXX)
git fetch -q origin main || { echo "fetch failed"; echo RUNNER FAILED; exit 1; }
git reset -q --hard; git checkout -q -B mixed-$NAME origin/main || { echo RUNNER FAILED; exit 1; }
cargo build --release --example plant_m0 > "$W/build.log" 2>&1 || { cp "$W/build.log" "$OUT/"; echo "build failed"; echo RUNNER FAILED; exit 1; }
cp target/release/examples/plant_m0 "$W/plant_m0"
M=$PWD/Reports/plant-community-2026-10-20/m2/hold/seeds
S3="shrub:$M/shrub/score_best.ron,herb:$M/herb/score_best.ron,creeper:$M/creeper/score_best.ron"
for s in ${SEEDS//,/ }; do echo "shipped $s"; echo "seeds3 $s $S3"; done > "$W/jobs.txt"
run() { a=$1; s=$2; sw=$3; "$W/plant_m0" mode=community mix=roles loadfail=0 rain=steady spells=20000,40000 ground=patchy seed=$s frames=450000 sample=50000 ${sw:+swap=$sw} > "$OUT/${a}_s$s.txt" 2>&1; }
export -f run; export W OUT
xargs -P4 -L1 bash -c 'run "$@"' _ < "$W/jobs.txt"
for f in "$OUT"/*_s*.txt; do printf "%s " "$(basename $f .txt)"; grep "^f=450000" "$f" | grep -oE "[a-z]+ plants=[0-9]+" | paste -sd' '; done
echo RUNNER DONE
