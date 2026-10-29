#!/bin/bash
# one round-0 world, sel=off (or SEL=score), poolrounds.sh's bed settings + trace rows + hungry probe.
# usage: run1.sh OUT SEED POOLDIR
set -o pipefail
W=/home/user/Pixel_Physics/.claude/worktrees/agent-a66af3a6cb52f6ee7
O=$1; S=$2; P=$3; shift 3
mkdir -p $O
G=$(for p in $P/*.raw.txt; do n=$(basename $p .raw.txt); printf '%s@%s:1,' $p $n; done); G=${G%,}
cd $W
t0=$(date +%s)
env RAYON_NUM_THREADS=1 PP_HUNGRYPROBE=1 ${MINDLOG:+PP_MIND_LOG=$MINDLOG} target/release/examples/inrun founders=$G seed=$S \
  frames=${FRAMES:-200000} dist=25 spread=10 grants=3 window=4000 sel=${SEL:-off} food=mix \
  lesson=scratch/inrun/genomes/c1.raw.txt:0.1 patches=20000:40:120 nest_r=0 trace=250 \
  setenv=PP_BED_W=448,PP_MIND_DOOR_OPEN=drop "$@" out=$O > $O.log 2>&1
echo "exit $? secs $(( $(date +%s) - t0 ))" >> $O.log
