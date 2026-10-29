#!/bin/bash
# re-run round 3 of world 68's 4-round sel=score chain (rounds 0-2 kept from r4/) with every mind decision logged
S=/tmp/claude-0/-home-user-Pixel-Physics/5839c770-56d6-594d-ba6d-0b2ec65d8626/scratchpad
W=/home/user/Pixel_Physics/.claude/worktrees/agent-a66af3a6cb52f6ee7
cd $W
one() { local arm=$1 pool=$2; local O=$S/deep/${arm}_w68
  mkdir -p $O/r0 $O/r1; touch $O/r0/.done $O/r1/.done; cp -r $S/r4/${arm}_w68/r2 $O/
  rm -f $O.fifo; mkfifo $O.fifo; gzip -1 < $O.fifo > $O.mind.gz &
  env PP_MIND_LOG=$O.fifo PP_HUNGRYPROBE=1 BIN=target/release/examples/inrun bash scratch/nest/poolrounds.sh $O 68 4 200000 $pool nest_r=0 trace=250
  wait; rm -f $O.fifo; }
mkdir -p $S/deep
one std $W/scratch/inrun/genomes/std_pool_raw &
one h3 $S/h3tgz/poolH3 &
wait
echo DEEPDONE
