# usage: hsumm.sh ARM... -- 8 seeds: loads put down within 8 columns of the hole (honest deliveries), all "deliver" events, starved
S=/tmp/claude-0/-home-user-Pixel-Physics/e58654de-422f-584a-a4c1-44fcef22179d/scratchpad/fl
for a in "$@"; do cat $S/l/${a}[1-8]/events.csv | awk -F, -v a=$a '$3=="deliver"{d=$4-128; if(d<0)d=-d; n++; if(d<=8)h++} $3=="death"{s++} END{printf "%-8s home deliveries %5d  (all deliver events %5d)  died %3d of 192\n", a,h,n,s}'; done
