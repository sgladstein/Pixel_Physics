#!/bin/bash
# SCRATCH (chambersense lane, 2026-10-30): paired garden runs, H3+eat against an arm, N-wide.
# usage: sweep.sh RUNROOT ROUNDS FRAMES WIDTH ARM:POOLDIR[:ENV] ... -- SEED...
#   ENV is K=V[+K=V] exported for that arm's runs (e.g. PP_DIGROOM=1).
R=$1; ROUNDS=$2; F=$3; WIDTH=$4; shift 4
arms=(); while [ "$1" != "--" ]; do arms+=("$1"); shift; done; shift
jobs=()
for s in "$@"; do for a in "${arms[@]}"; do jobs+=("$a|$s"); done; done
run() { IFS='|' read -r a s <<< "$1"; IFS=':' read -r name pool env <<< "$a"
  out=$R/$name/w$s; mkdir -p $R/$name
  [ -f $out/r$((ROUNDS-1))/.done ] || [ -f $out/EXTINCT ] && return
  ( [ -n "$env" ] && export ${env//+/ }; BIN=target/release/examples/inrun bash scratch/nest/poolrounds.sh $out $s $ROUNDS $F $pool nest_r=0 ) }
export -f run; export R ROUNDS F
printf '%s\n' "${jobs[@]}" | xargs -P $WIDTH -I{} bash -c 'run "$@"' _ {}
