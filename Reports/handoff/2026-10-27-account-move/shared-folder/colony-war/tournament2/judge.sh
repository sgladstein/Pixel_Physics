#!/bin/bash
cd /home/user/Pixel_Physics
B=/tmp/claude-0/-home-user-Pixel-Physics/0849b493-0a74-5ff7-a459-f6a00eae7ae9/scratchpad/rivalbed-v3.bin; J=/tmp/claude-0/-home-user-Pixel-Physics/0849b493-0a74-5ff7-a459-f6a00eae7ae9/scratchpad/tour3/judge; C="gap=40 food=split bite=keep,keep"; SC=score=lived:1,nest_deep:0.5,delivered:0.5
python3 scratch/inrun/rivaljudge.py $J/fight std $(cat /tmp/claude-0/-home-user-Pixel-Physics/0849b493-0a74-5ff7-a459-f6a00eae7ae9/scratchpad/tour3/fight.champs) bin=$B $SC -- $C > $J-fight.txt 2>&1
PP_MIND_BITE_SCALE=0.25 python3 scratch/inrun/rivaljudge.py $J/graded std $(cat /tmp/claude-0/-home-user-Pixel-Physics/0849b493-0a74-5ff7-a459-f6a00eae7ae9/scratchpad/tour3/graded.champs) bin=$B $SC -- $C > $J-graded.txt 2>&1
echo ALLDONE
