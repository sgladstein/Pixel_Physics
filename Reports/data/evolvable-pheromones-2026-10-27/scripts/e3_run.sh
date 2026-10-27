#!/bin/bash
# usage: run.sh H SEED -- E3: deaf arm, 40 gens, harness H (1 old, 2 fixed); resumes
cd /home/user/Pixel_Physics
export BIN=/tmp/claude-0/-home-user-Pixel-Physics/2ef7e65c-531a-5db4-866b-18b502ff6e79/scratchpad/nc/inrun2.bin PIXEL_PHYSICS_ALARM_GROUND=0.03:0.05
exec python3 scratch/pheromones/emerge.py /tmp/claude-0/-home-user-Pixel-Physics/2ef7e65c-531a-5db4-866b-18b502ff6e79/scratchpad/e3/h$1_s$2 deaf 16 40 100000 assets/minds/std_pool.txt P=1 SEED=$2 EVALS=2 SCORE=stored ZERO=2410-2444 FILL=0.05 CHAN=alarm ALARM=train EVERY=0 HARNESS=$1 >> /tmp/claude-0/-home-user-Pixel-Physics/2ef7e65c-531a-5db4-866b-18b502ff6e79/scratchpad/e3/h$1_s$2.log 2>&1
