#!/bin/bash
# runs after the picktest scoring: depth/roofed ablation, contacts-per-ant trail sweep, crowd-waiting trace
S=/tmp/claude-0/-home-user-pixel-physics/77dd5784-4086-59b8-8d0c-591a89f55d57/scratchpad
cd /home/user/pixel_physics
while [ "$(ls $S/mut/picktest 2>/dev/null | wc -l)" -lt 36 ] || pgrep -x train > /dev/null; do sleep 20; done
G=Reports/brain-training/stage2/dig/4b4/dig-4b4-s161-g340-hold-best.txt
H=Reports/brain-training/stage2/dig/8/dig-8-s171-hold-best.txt
X="PIXEL_PHYSICS_MUTATION=off"
for b in $H:intact $S/ablate/s171-no-depth.txt:no-depth $S/ablate/s171-no-roofed.txt:no-roofed; do f=${b%%:*}; n=${b##*:}
  env $X target/release/examples/train brain=mind challenge=soil step=4 shaft=8 foot=1 budget=2400 frames=12000 test=96 start=$G score=$f threads=4 noise=1 > $S/ablate/$n-soil.txt 2>&1
  env $X target/release/examples/train brain=mind challenge=fetch dist=10,30 budget=1200 test=96 start=$G score=$f threads=4 noise=1 > $S/ablate/$n-fetch.txt 2>&1
done
echo "ablation done $(date -u +%H:%M)"
mkdir -p $S/crowd
for a in 3 6 12 24; do env $X target/release/examples/train brain=mind challenge=trail clones=1 ants=$a test=48 start=$G score=$H threads=4 noise=1 > $S/crowd/trail-$a.txt 2>&1; echo "trail ants=$a done $(date -u +%H:%M)"; done
cd $S/gifwt
PP_MIND_LOG=$S/crowd/soil12.log env $X target/release/examples/train brain=mind challenge=soil step=4 shaft=8 foot=1 budget=2400 frames=12000 ants=12 holdout=8 start=$G score=$H threads=1 noise=1 > $S/crowd/soil12-run.txt 2>&1
PP_MIND_LOG=$S/crowd/soil1.log env $X target/release/examples/train brain=mind challenge=soil step=4 shaft=8 foot=1 budget=2400 frames=12000 ants=1 holdout=8 start=$G score=$H threads=1 noise=1 > $S/crowd/soil1-run.txt 2>&1
echo "crowd trace done $(date -u +%H:%M)"
