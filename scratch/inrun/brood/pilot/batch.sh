#!/bin/bash
cd $(dirname $0)
jobs=()
for s in 1 2 3; do
  jobs+=("ctl$s $s")
  jobs+=("lay$s $s brood=lay")
  jobs+=("cap$s $s brood=lay lay_cap=48:8")
  jobs+=("sb$s $s brood=lay @PIXEL_PHYSICS_STORE_BRAKE=on")
  jobs+=("fb$s $s brood=lay @PIXEL_PHYSICS_FOOD_BRAKE=on")
done
printf '%s\n' "${jobs[@]}" | xargs -P 4 -I{} bash -c 'set -- {}; n=$1; s=$2; shift 2; x=""; a=(); for v in "$@"; do if [[ $v == @* ]]; then x=${v#@}; else a+=("$v"); fi; done; SETX=$x ./run.sh b/$n $s 200000 "${a[@]}"'
