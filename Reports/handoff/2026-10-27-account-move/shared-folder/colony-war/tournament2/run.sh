#!/bin/bash
cd /home/user/Pixel_Physics
T=/tmp/claude-0/-home-user-Pixel-Physics/0849b493-0a74-5ff7-a459-f6a00eae7ae9/scratchpad/tour3; B=/tmp/claude-0/-home-user-Pixel-Physics/0849b493-0a74-5ff7-a459-f6a00eae7ae9/scratchpad/rivalbed-v3.bin
COMMON="gap=40 food=split mutate=0.013:1:4 bite=keep,keep"
SC="score=lived:1,nest_deep:0.5,delivered:0.5"
python3 scratch/inrun/rivaltour.py $T/fight ncol=12 rounds=16 frames=60000 P=2 bin=$B $SC -- $COMMON > $T/fight.log 2>&1 &
PP_MIND_BITE_SCALE=0.25 python3 scratch/inrun/rivaltour.py $T/graded ncol=12 rounds=16 frames=60000 P=2 bin=$B $SC -- $COMMON > $T/graded.log 2>&1 &
wait
echo ALLDONE
