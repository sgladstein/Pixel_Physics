#!/usr/bin/env bash
# The shared performance benchmark: every scene, two builds, alternating.
#
# Built 2026-10-21 for the four performance threads (lab world, creatures,
# plants, training) so a claimed speedup is measured the same way by all of
# them. A speedup must not change results -- training depends on bit-for-bit
# reproduction -- so each scene prints its timing AND a hash line, and this
# script says, per scene, whether the two builds' hash lines are identical.
# A timing beside hashes that moved is not a speedup, it is a different
# simulation.
#
#   bash scripts/perfbench.sh BASE_DIR NEW_DIR [rounds] [scene,scene,...]
#
# BASE_DIR and NEW_DIR each hold release example binaries (perfbench,
# needscost, plantcost, train) copied out of target/release/examples after a
# build at each commit -- copy them, because the next build overwrites
# target/. A scene whose binary is missing from either dir is skipped and
# says so. Rounds alternate base/new (default 2) and the table quotes the
# minimum, the only wall-time shape that survives a loaded machine; for a
# number that does not move with load, run a scene's binary under callgrind
# with cg=1 (see examples/perfbench.rs). Everything runs single-threaded
# (RAYON_NUM_THREADS=1), as training does.
#
# Scenes (owner thread in brackets):
#   box      the shipped lab box, perfbench at=20000 window=1200   [lab world]
#   colony   needscost nest_goal seed 1 at=100000 window=1200      [creatures]
#   plants   plantcost scene=community at=60000 window=1200        [plants]
#   rung     plantcost scene=rung layouts=4 (whole run timed)      [plants]
#   fetch, trail, soil   the brain lane's judge arenas, 8 test worlds each,
#            whole run timed; the hash is the per-world NOISE rows   [training]
set -uo pipefail

base=${1:?usage: perfbench.sh BASE_DIR NEW_DIR [rounds] [scenes]}
new=${2:?usage: perfbench.sh BASE_DIR NEW_DIR [rounds] [scenes]}
rounds=${3:-2}
scenes=${4:-box,colony,plants,rung,fetch,trail,soil}
export RAYON_NUM_THREADS=1 PIXEL_PHYSICS_MUTATION=off

G=Reports/brain-training/stage2/dig/4b4/dig-4b4-s161-g340-hold-best.txt
S=training/starts/s171-8-g79.txt

# scene -> "binary|arguments"
spec() {
  case $1 in
    box)    echo "perfbench|at=20000 window=1200" ;;
    colony) echo "needscost|seed=1 at=100000 window=1200 reps=1" ;;
    plants) echo "plantcost|scene=community at=60000 window=1200 reps=1" ;;
    rung)   echo "plantcost|scene=rung layouts=4" ;;
    fetch)  echo "train|brain=mind challenge=fetch dist=10,30 budget=1200 test=8 start=$G score=$S threads=1 noise=1" ;;
    trail)  echo "train|brain=mind challenge=trail test=8 start=$G score=$S threads=1 noise=1" ;;
    soil)   echo "train|brain=mind challenge=soil step=4 shaft=8 foot=1 budget=2400 frames=12000 test=8 start=$G score=$S threads=1 noise=1" ;;
    *) return 1 ;;
  esac
}

# The lines that must not move, with every timing token stripped.
hashes() {
  grep -E 'RESULT|hash|^NOISE seed' "$1" |
    sed -E 's/(min_ms_per_frame|median|ms_per_frame|s_per_bed|ms|pre_s)=[0-9.]+//g; s/[0-9]+\.[0-9]+ (s|ms\/frame),//g'
}

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
printf '%-8s %12s %12s %8s  %s\n' scene base new change hashes
IFS=, read -ra list <<< "$scenes"
for sc in "${list[@]}"; do
  if ! s=$(spec "$sc"); then echo "$sc: unknown scene"; continue; fi
  bin=${s%%|*}; args=${s#*|}
  if [[ ! -x $base/$bin || ! -x $new/$bin ]]; then
    printf '%-8s skipped: %s missing from one of the two dirs\n' "$sc" "$bin"; continue
  fi
  best_b=; best_n=
  for ((r = 0; r < rounds; r++)); do
    for side in b n; do
      dir=$base; [[ $side == n ]] && dir=$new
      out=$tmp/$sc.$side.$r
      t0=$(date +%s.%N)
      # shellcheck disable=SC2086
      "$dir/$bin" $args > "$out" 2>&1
      t=$(echo "$(date +%s.%N) - $t0" | bc)
      if [[ $side == b ]]; then
        [[ -z $best_b ]] || (( $(echo "$t < $best_b" | bc) )) && best_b=$t
      else
        [[ -z $best_n ]] || (( $(echo "$t < $best_n" | bc) )) && best_n=$t
      fi
    done
  done
  if diff <(hashes "$tmp/$sc.b.0") <(hashes "$tmp/$sc.n.0") > /dev/null; then
    same="identical ($(hashes "$tmp/$sc.b.0" | wc -l) lines)"
  else
    same="DIFFER"
  fi
  [[ $(hashes "$tmp/$sc.b.0" | wc -l) -eq 0 ]] && same="NO HASH LINES -- nothing compared"
  printf '%-8s %11.2fs %11.2fs %+7.1f%%  %s\n' "$sc" "$best_b" "$best_n" "$(echo "($best_n - $best_b) * 100 / $best_b" | bc -l)" "$same"
done
