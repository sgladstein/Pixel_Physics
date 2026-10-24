#!/bin/bash
# arm.sh ARM GROUP SEED : ARM = ctl | v3 | v6 | v3s
R=/tmp/claude-0/-home-user-Pixel-Physics/8a2e7e11-3afa-57e4-8c63-69bb9c615c96/scratchpad/review
a=$1; g=$2; k=$3; n=$(echo $g|tr : _)
case $a in ctl) ;; v3) export PP_RIMVETO=3;; v6) export PP_RIMVETO=6;; v3s) export PP_RIMVETO=3 PP_SPOILVETO=10;; roof|rooms|both) export PP_BODYRULES=$a BIN_INRUN=$R/build/target/release/examples/inrun;; esac
out=$R/d/${a}-${n}_$k
$R/run.sh $g $k 200000 $out window=200000
rm -f $out/map_f* $out/ants_f* $out/trace.csv $out/scent.csv
grep -v ",eat,\|,homeeat,\|,field," $out/events.csv > $out/ev.tmp && mv $out/ev.tmp $out/events.csv
