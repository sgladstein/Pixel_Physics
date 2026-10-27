#!/bin/bash
S=/tmp/claude-0/-home-user-Pixel-Physics/df1dc8d5-6938-5008-bf40-a54eb4cee899/scratchpad/np
read name np far seed <<< "$1"
cd /home/user/Pixel_Physics
$S/predtrain_bin out=$S/$name-$seed species=centipede pool=/mnt/project-files/predator/nest-raider/pool-hunter2-r7 sel=score rounds=16 frames=100000 warm=60000 far=$far nestpay=$np seed=$seed > $S/$name-$seed.log 2>&1
