# usage: ev.sh NAME GENOME [seeds...]
N=$1; G=$2; shift 2; cd /tmp/claude-0/-home-user-Pixel-Physics/c9ced244-18ea-5f03-bb65-45cf1b722ce9/scratchpad/eval
for t in $@; do echo "/home/user/tp/target/release/examples/inrun founders=$G:30 seed=$t frames=200000 dist=25 spread=10 cap=200 grants=3 window=2000 maps=0 out=/tmp/claude-0/-home-user-Pixel-Physics/c9ced244-18ea-5f03-bb65-45cf1b722ce9/scratchpad/eval/${N}_$t > ${N}_$t.out 2>&1"; done | xargs -P 4 -I{} bash -c "{}"
echo "$N: $(for t in $@; do grep 'frame  200000' ${N}_$t.out | awk '{print $4}'; done | tr '\n' ' ') | deliver $(for t in $@; do grep -c ',deliver,' /tmp/claude-0/-home-user-Pixel-Physics/c9ced244-18ea-5f03-bb65-45cf1b722ce9/scratchpad/eval/${N}_$t/events.csv; done | tr '\n' ' ')"
