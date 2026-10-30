#!/bin/bash
# the Nest lane's 4-round sel=score runs (poolrounds.sh), with trace rows and the hungry probe, one lane per arm
S=/tmp/claude-0/-home-user-Pixel-Physics/5839c770-56d6-594d-ba6d-0b2ec65d8626/scratchpad
W=/home/user/Pixel_Physics/.claude/worktrees/agent-a66af3a6cb52f6ee7
cd $W
mkdir -p $S/r4
lane() { local arm=$1 pool=$2; for w in ${WORLDS:-68 73 61 64}; do
  PP_HUNGRYPROBE=1 BIN=target/release/examples/inrun bash scratch/nest/poolrounds.sh $S/r4/${arm}_w$w $w 4 200000 $pool nest_r=0 trace=250; done; }
lane std $W/scratch/inrun/genomes/std_pool_raw &
lane h3 $S/h3tgz/poolH3 &
wait
echo ALLDONE4
