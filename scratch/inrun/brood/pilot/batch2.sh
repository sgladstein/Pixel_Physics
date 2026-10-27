#!/bin/bash
cd $(dirname $0)
jobs=()
for s in 1 2 3; do
  jobs+=("capdeep$s $s brood=lay lay_cap=48:8 @PIXEL_PHYSICS_EGG_DOOR=deep")
  jobs+=("capspread$s $s brood=lay lay_cap=48:8 @PIXEL_PHYSICS_BROOD_SPREAD=on")
  jobs+=("capsoft$s $s brood=lay lay_cap=36:16")
  jobs+=("capff$s $s brood=lay lay_cap=48:8 @PIXEL_PHYSICS_FEED_FIRST=on")
done
printf '%s\n' "${jobs[@]}" | xargs -P 4 -I{} bash -c 'set -- {}; n=$1; s=$2; shift 2; x=""; a=(); for v in "$@"; do if [[ $v == @* ]]; then x=${v#@}; else a+=("$v"); fi; done; SETX=$x ./run.sh b/$n $s 200000 "${a[@]}"'
