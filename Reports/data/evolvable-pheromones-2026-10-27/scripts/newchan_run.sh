#!/bin/bash
# usage: run.sh ARM SEED  -- new-channel emergence run (resumes)
cd /home/user/Pixel_Physics
export BIN=/tmp/claude-0/-home-user-Pixel-Physics/2ef7e65c-531a-5db4-866b-18b502ff6e79/scratchpad/nc/inrun.bin PIXEL_PHYSICS_ALARM_GROUND=0.03:0.05
exec python3 scratch/pheromones/emerge.py /tmp/claude-0/-home-user-Pixel-Physics/2ef7e65c-531a-5db4-866b-18b502ff6e79/scratchpad/nc/$1_s$2 $1 16 150 100000 assets/minds/std_pool.txt P=2 SEED=$2 EVALS=2 SCORE=stored ZERO=2410-2444 FILL=0.05 CHAN=alarm ALARM=train >> /tmp/claude-0/-home-user-Pixel-Physics/2ef7e65c-531a-5db4-866b-18b502ff6e79/scratchpad/nc/$1_s$2.log 2>&1
