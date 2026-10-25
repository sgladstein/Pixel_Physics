#!/bin/bash
# SCRATCH (old-ant thread): one ablation run. args: OUT SCENARIO ARM ENV SEED [extra labsurvive args]
OUT=$1; SC=$2; ARM=$3; ENVS=$4; SEED=$5; shift 5
f=$OUT/$ARM.$SC.s$SEED.txt
[ -s "$f" ] && grep -q RESULT "$f" && exit 0
export PIXEL_PHYSICS_LAB_SCENARIOS=/home/user/Pixel_Physics/scratch/oldant/scen
E=(); [ "$ENVS" != "-" ] && IFS=';' read -ra E <<< "$ENVS"
env "${E[@]}" /home/user/Pixel_Physics/target/release/examples/labsurvive founders=${FOUNDERS:-old} scenario=$SC seed=$SEED "$@" > "$f.tmp" 2>&1 && mv "$f.tmp" "$f"
