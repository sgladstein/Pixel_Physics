#!/bin/bash
# PF4 positive control: A and B deaf in both arms; free hears the third plane. usage: run.sh ARM
cd /home/user/Pixel_Physics
export BIN=/tmp/claude-0/-home-user-Pixel-Physics/2ef7e65c-531a-5db4-866b-18b502ff6e79/scratchpad/pf5/inrun_main.bin PP_GATE=off PP_REACH=off PIXEL_PHYSICS_ALARM_GROUND=0.03:0.05
exec python3 scratch/pheromones/emerge.py /tmp/claude-0/-home-user-Pixel-Physics/2ef7e65c-531a-5db4-866b-18b502ff6e79/scratchpad/pf4/$1 $1 16 40 100000 assets/minds/std_pool.txt P=2 SEED=41 EVALS=2 SCORE=home MUT=0.013:0.25:1 CHANMUT=0.013:1.0 DEAFALSO=A,B ZERO=2410-2444 FILL=0.05 CHAN=alarm ALARM=train EVERY=10 LISTEN=4 HARNESS=2 >> /tmp/claude-0/-home-user-Pixel-Physics/2ef7e65c-531a-5db4-866b-18b502ff6e79/scratchpad/pf4/$1.log 2>&1
