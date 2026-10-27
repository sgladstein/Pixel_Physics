S=/tmp/claude-0/-home-user-Pixel-Physics/b300ed13-cc9c-5e26-8b14-2acd8b3d007f/scratchpad/g; cd $S
{ for s in 1 2 3 4; do for a in c30 c15 bc15 bc15cap bg0cap; do echo "${a}_s$s $s"; done; done; } | xargs -P 4 -L 1 bash -c '
n=$0; s=$1; G="plants=grass,herb,shrub,scrambler weed=30 fjoule=1"; P="dist=25 spread=10 drip=20"; export XENV=PP_MIND_GRAZE=1; case $n in
 c30_*) ./run.sh $n $s 200000 $G $P cap=30 film=10000 film_hw=128;;
 c15_*) ./run.sh $n $s 200000 $G $P cap=15 film=10000 film_hw=128;;
 bc15_*) ./run.sh $n $s 200000 $G $P cap=15 brood=lay film=10000 film_hw=128;;
 bc15cap_*) ./run.sh $n $s 200000 $G $P cap=15 brood=lay lay_cap=48:8 film=10000 film_hw=128;;
 bg0cap_*) ./run.sh $n $s 200000 $G cap=0 brood=lay lay_cap=48:8 film=10000 film_hw=128;;
esac'
echo BATCH4-DONE
