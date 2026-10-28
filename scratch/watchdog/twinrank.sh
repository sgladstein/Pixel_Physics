#!/usr/bin/env bash
# SCRATCH (score-gaming watchdog, 2026-10-28): do the training lab boxes rank carries the way the game's boxes do?
# Teacher lane's question for env5's donorlab= check: lab_herbs and lab_far_mix are the twins of herb_first_colony and
# played_bed, so a carry that wins the twins should win the exam boxes. One line of JOBS per run:
#   NAME FOUNDERS SCENDIR SCENARIO SEED AFTER ENVSET
# ENVSET 'lab' = colsel's LAB_ENV (PP_BED_W=448,PP_MIND_DOOR_OPEN=drop); 'exam' = exam.py's lab() (no extra env).
# Resumable: a run whose log already has a RESULT line is skipped, so after a container restart relaunch the same
# command.   usage: twinrank.sh JOBS OUTDIR [P]
set -u
J=$1; O=$2; P=${3:-4}
mkdir -p $O
run() {
  set -- $1
  local name=$1 f=$2 dir=$3 sc=$4 seed=$5 after=$6 envset=$7
  local out=$O/$name
  grep -q '^RESULT' $out.log 2>/dev/null && return 0
  mkdir -p $out
  local at=$(python3 -c "a=$after; print(','.join(str(x) for x in range(a//6, a+1, a//6)))")
  if [ "$envset" = lab ]; then
    env PP_BED_W=448 PP_MIND_DOOR_OPEN=drop RAYON_NUM_THREADS=1 PIXEL_PHYSICS_LAB_SCENARIOS=$dir target/release/examples/labsurvive \
      founders=$f scenario=$sc seed=$seed after=$after nestmap=$out nestat=$at > $out.log 2>&1
  else
    env -u PP_BED_W -u PP_MIND_DOOR_OPEN RAYON_NUM_THREADS=1 PIXEL_PHYSICS_LAB_SCENARIOS=$dir target/release/examples/labsurvive \
      founders=$f scenario=$sc seed=$seed after=$after nestmap=$out nestat=$at > $out.log 2>&1
  fi
}
export -f run; export O
grep -v '^#' $J | grep . | xargs -P$P -I{} bash -c 'run "{}"'
