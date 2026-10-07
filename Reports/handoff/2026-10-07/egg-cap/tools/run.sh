#!/bin/bash
# Laying lane: what limits egg laying on the Nest race stack (claude/nest-race-way-foot b59228529 + laying census in colony.csv). $1=seed $2=frames $3=tag $4.. extra env
L=/tmp/claude-0/-home-claude-Pixel-Physics/9bfa6614-6457-5816-98a1-971a1bf2db4f/scratchpad/laycap
seed=$1; frames=${2:-200000}; tag=${3:-base}; shift 3
out=$L/runs/$tag-s$seed
mkdir -p "$out"; ln -sf /dev/null "$out/digrows.csv.gz"
cd /home/claude/wt-wayfoot
ENVS="PIXEL_PHYSICS_NEEDS_FIRST=on,backfill PIXEL_PHYSICS_CARRY_HOME=on PIXEL_PHYSICS_DOOR_COLUMN=on PIXEL_PHYSICS_LAY_BAR=body PIXEL_PHYSICS_NEST_STORE=on,pick=20,jaws,sky,meal,smell=10 PIXEL_PHYSICS_WAY_FOOT=on PIXEL_PHYSICS_MUTATION=off $*"
echo "env: $ENVS seed=$seed frames=$frames bin=$(md5sum $L/bin/deeptrace | cut -c1-12) head=$(git rev-parse --short=9 HEAD)" > $out.log
env $(env | grep -o '^PIXEL_PHYSICS_[A-Z_]*' | sed 's/^/-u /') RAYON_NUM_THREADS=1 $ENVS $L/bin/deeptrace scenario=nest_goal seed=$seed frames=$frames founder=evolved ants=0 mapevery=5000 nestevery=5000 dig=1 foodgap=90 out=$out >> $out.log 2>&1
echo "rc=$?" >> $out.log
