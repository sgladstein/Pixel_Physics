# usage: batch.sh TREE ARM GENOME "ENV" [seeds] -- 32 seeds (or given), 4 at a time
T=$1; A=$2; G=$3; E="$4"; SEEDS=${5:-$(seq 1 32)}
echo $SEEDS | tr ' ' '\n' | xargs -P 4 -I{} sh -c "/home/user/hvs/run.sh $T $A $G {} $E"
