#!/bin/bash
S=/tmp/claude-0/-home-user-Pixel-Physics/df1dc8d5-6938-5008-bf40-a54eb4cee899/scratchpad/np
read name pool far seed <<< "$1"
cd /home/user/Pixel_Physics
$S/predtrain_bin out=$S/judge/o-$name-$far-$seed species=centipede pool=$pool sel=off preds=4 rounds=1 frames=30000 warm=60000 far=$far seed=$seed 2>&1 | grep "^round" | sed "s|^|$name far$far s$seed |" > $S/judge/$name-$far-$seed.txt
