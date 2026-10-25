#!/bin/bash
# SCRATCH (old-ant thread, 2026-10-25): which of the hand-built ant's rules carry it on far food?
# usage: P=4 ablate.sh OUT SCENARIO "SEEDS" ARMSFILE [extra labsurvive args]
OUT=$1; SC=$2; SEEDS=$3; ARMS=$4; shift 4
mkdir -p "$OUT"
D=$(dirname "$0")
while read -r a e; do for s in $SEEDS; do echo "$a $e $s"; done; done < "$ARMS" | \
  xargs -P "${P:-4}" -L 1 bash -c '"$0" "$1" "$2" "$3" "$4" "$5" '"$*" "$D/one.sh" "$OUT" "$SC"
