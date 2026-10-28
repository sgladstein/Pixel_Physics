# run.sh ARM WORLD: one 200k sel=off run of std_pool (or the hand-built ant) on the garden bed with a food arm
cd /home/user/Pixel_Physics
S=/tmp/claude-0/-home-user-Pixel-Physics/074507f3-3bc6-5b2b-9fd4-4417d230c2c7/scratchpad/wild
arm=$1; w=$2; F=$(cat $S/../smoke/fstd/founders.txt); extra=""
case $arm in
  mix) fa="food=mix";; all) fa="food=all";; w25) fa="food=mix foodwild=0.25";; w50) fa="food=mix foodwild=0.5";;
  oldall) fa="food=all"; extra="nomind=1";; oldw25) fa="food=mix foodwild=0.25"; extra="nomind=1";; oldw50) fa="food=mix foodwild=0.5"; extra="nomind=1";; oldmix) fa="food=mix"; extra="nomind=1";;
esac
o=$S/$arm-$w
RAYON_NUM_THREADS=1 target/release/examples/inrun founders=$F seed=$w frames=200000 dist=25 spread=10 grants=3 window=4000 sel=off $fa $extra lesson=scratch/inrun/genomes/c1.raw.txt:0.1 patches=20000:40:120 setenv=PP_BED_W=448,PP_MIND_DOOR_OPEN=drop out=$o > $o.log 2>&1
rm -f $o/ants_f* $o/map_f00* $o/map_f01[0-8]*
gzip -f $o/events.csv
echo "$arm $w $(tail -1 $o/nest.csv | cut -d, -f1,2) starved=$(zcat $o/events.csv.gz | awk -F, '/,death,/ && $6==1' | wc -l) deliv=$(zcat $o/events.csv.gz | grep -c ',deliver,')"
