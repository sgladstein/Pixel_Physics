#!/bin/bash
# SCRATCH (nest lane, 2026-10-27): one held-out garden world (exam.py's settings) with every mind decision logged
# (PP_MIND_LOG, gzipped through a fifo) and the hungry probe (PP_HUNGRYPROBE). For tracing soil haulers, crumb drops and
# hungry ants at home, ant by ant. usage: trace.sh OUTDIR SEED [POOLDIR (a baseline.py ARM/pool)] [extra inrun args...]
set -e
O=$1; SD=$2; POOL=${3:-}; shift 3 || shift $#
mkdir -p $O
F=$(for p in $POOL/*.raw.txt; do n=$(basename $p .raw.txt); printf '%s@%s:1,' $p $n; done)
F=${F%,}
rm -f $O/mind.fifo; mkfifo $O/mind.fifo
gzip -1 < $O/mind.fifo > $O/mind.log.gz &
RAYON_NUM_THREADS=1 PP_HUNGRYPROBE=1 PP_MIND_LOG=$O/mind.fifo target/release/examples/inrun founders=$F seed=$SD \
  frames=${FRAMES:-200000} dist=25 spread=10 grants=3 window=4000 sel=off food=mix \
  lesson=scratch/inrun/genomes/c1.raw.txt:0.1 patches=20000:40:120 setenv=PP_BED_W=448,PP_MIND_DOOR_OPEN=drop \
  out=$O/w$SD "$@" > $O/run.log 2>&1
wait
rm -f $O/mind.fifo
