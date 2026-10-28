#!/bin/bash
# SCRATCH (pheromones lane, 2026-10-28): PF4 on far food (§9.13). A and B deaf in both arms; free hears the third plane.
# The wide bed (PP_BED_W=448, the exam garden's) with the heap 100-250 columns out, scored on deliveries whose trip
# began at the heap (SCORE=heaphome), which the door loop cannot farm. usage: pf4w_run.sh OUTDIR ARM [BIN]
# Pass bar, fixed before launch: garden.py stage B (SCORE=heaphome, same bed) free >= 1.2x deaf AND the free best
# meets criteria 2 and 3. Fail: close the third channel.
OUT=$1; ARM=$2
export BIN=${3:-target/release/examples/inrun} PP_GATE=off PP_REACH=off PIXEL_PHYSICS_ALARM_GROUND=0.03:0.05 PP_BED_W=448
exec python3 scratch/pheromones/emerge.py $OUT/$ARM $ARM 16 40 100000 assets/minds/std_pool.txt P=2 SEED=42 EVALS=2 \
  SCORE=heaphome PATCHES=20000:100:250 MUT=0.013:0.25:1 CHANMUT=0.013:1.0 DEAFALSO=A,B ZERO=2410-2444 FILL=0.05 \
  CHAN=alarm ALARM=train EVERY=10 LISTEN=4 HARNESS=2 >> $OUT/$ARM.log 2>&1
