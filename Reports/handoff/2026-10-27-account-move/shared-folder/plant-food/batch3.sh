S=/tmp/claude-0/-home-user-Pixel-Physics/b300ed13-cc9c-5e26-8b14-2acd8b3d007f/scratchpad/g; cd $S
export L="lesson=/home/user/Pixel_Physics/scratch/inrun/genomes/c1.raw.txt:0.1"
{ echo "pilechk_s1 1"; for s in 1 2 3 4; do echo "wean_s$s $s"; echo "weannw_s$s $s"; echo "gweed_s$s $s"; echo "swean_s$s $s"; done; } | xargs -P 4 -L 1 bash -c '
n=$0; s=$1; G="plants=grass,herb,shrub,scrambler"; P="dist=25 spread=10 drip=20 cap=60"; W="cap_to=0:150000 fjoule=1"; export XENV=PP_MIND_GRAZE=1; case $n in
 pilechk_*) XENV= ./run.sh $n $s 200000 $P;;
 wean_*) ./run.sh $n $s 200000 $G $P $W weed=30 film=10000 film_hw=128;;
 weannw_*) ./run.sh $n $s 200000 $G $P $W film=10000 film_hw=128;;
 gweed_*) ./run.sh $n $s 200000 $G cap=0 fjoule=1 weed=30 film=10000 film_hw=128;;
 swean_*) ./run.sh $n $s 200000 $G $P $W weed=30 sel=score $L film=10000 film_hw=128;;
esac'
echo BATCH3-DONE
