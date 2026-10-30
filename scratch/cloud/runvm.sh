# runvm.sh NAME JOBS REF NJOBS DEST (teacher lane): start a batch VM, wait, and on preemption relaunch it under NAME-rK (up to 4 times);
# when every job is uploaded, delete the VM and pull into DEST. Prints one line per event.
# env HOURS (VM lifetime, default 3), RESUME_FROM (an earlier batch whose partial/ to resume). Run it in the background; it
# polls every 2 min. **Pulls are big**: a 192-round batch filled a session disk; delete colony folders right after.
# inrun_cloud prefixes VM names with pixel-train-, so pass the bare NAME.
cd "$(dirname "$0")/../.."
name=$1; jobs=$2; ref=$3; n=$4; dest=$5; prev=${RESUME_FROM:-}
# a short SHA cannot be fetched on the VM (git fetch --depth 1 needs the full 40 characters; g1base-1027 died on it)
full=$(git rev-parse --verify -q "$ref^{commit}" 2>/dev/null) && [ "${#ref}" -lt 40 ] && echo "$ref" | grep -qE "^[0-9a-f]+$" && ref=$full
H=scratch/cloud
for k in 0 1 2 3 4; do
  nm=$name; [ $k -gt 0 ] && nm=$name-r$k
  res=""; [ -n "$prev" ] && res="--resume $prev"
  # ATTACH=1: the first VM is already running (the session that launched it was restarted and its runvm died with it,
  # 2026-10-28 env6): skip its start and only wait, pull and relaunch
  for i in $(seq 1 100); do
    [ $k -eq 0 ] && [ "${ATTACH:-0}" = 1 ] && { echo "$(date -u +%H:%M) attached to $nm"; break; }
    out=$(python3 scratch/cloud/inrun_cloud.py start $nm jobs=$jobs --ref $ref --type c4d-highcpu-64 --disk 200 --hours ${HOURS:-3} $res 2>&1 | tail -1)
    echo "$(date -u +%H:%M) $out" | cut -c1-160
    echo "$out" | grep -q "^started" && break
    sleep 180
  done
  [ $k -eq 0 ] && [ "${ATTACH:-0}" = 1 ] || sleep 240
  while true; do
    o=$(timeout 60 python3 scratch/cloud/inrun_cloud.py status $nm 2>&1 | tail -1)
    if echo "$o" | grep -q '@@DONE'; then
      c=$(python3 $H/countjobs.py $nm '.' | tail -1)
      echo "$(date -u +%H:%M) done, uploaded $c"
      timeout 300 python3 scratch/cloud/inrun_cloud.py delete $nm 2>&1 | tail -1
      timeout 1800 python3 scratch/cloud/inrun_cloud.py pull $nm $dest 2>&1 | tail -2
      exit 0
    fi
    if echo "$o" | grep -q 'gone'; then
      c=$(python3 $H/countjobs.py $nm '.' | tail -1)
      if [ "$c" -ge "$n" ]; then
        timeout 1800 python3 scratch/cloud/inrun_cloud.py pull $nm $dest 2>&1 | tail -2; exit 0
      fi
      echo "$(date -u +%H:%M) $nm gone with $c of $n jobs: relaunching with --resume $nm"
      prev=${prev:+$prev,}$nm  # the whole chain: a finished job line in any earlier launch is skipped (inrun_cloud)
      break
    fi
    sleep 120
  done
done
echo "gave up after 5 launches"
