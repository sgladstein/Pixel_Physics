#!/usr/bin/env bash
# One leg of one batch runner (`.github/workflows/batch-leg.yml`): run this runner's job lines in parallel until they
# finish or the leg's DEADLINE, then write <entry>.status and pack every job's folder for the next leg.
#
#   batch_run.sh ENTRY OUTROOT PUB < lines      (cwd: the code tree; env BIN, DEADLINE, HARD (seconds left to kill at))
#
# A line runs as the VM's run_one does (scratch/cloud/inrun_cloud.py), with the runner's cores for colsel's P and
# KEY=VALUE tokens after OUT as that line's environment. OUTROOT is a FIXED path (/tmp/batch/out): colsel's
# state.json stores absolute paths to its round folders, so a resume must find them where the last leg left them.
#
# A line's state: done (exit 0), paused (colsel's DEADLINE exit 75, or killed at HARD: the next leg carries it on),
# failed (anything else, or a paused line that made no progress in a whole leg -- one round longer than a leg would
# otherwise loop until the chain of legs ran out).
set -uo pipefail
entry=$1 root=$2 pub=$3
mkdir -p "$root" "$pub"
mapfile -t LINES
here=$(cd "$(dirname "$0")" && pwd)
export NPROC=$(nproc)
progress_of() {  # a number that grows as a job advances: colsel's round, emerge's generations, rounds.sh's rounds
  local d=$1
  if [ -f "$d/state.json" ]; then python3 -I -c "import json,sys;print(json.load(open(sys.argv[1]))['r'])" "$d/state.json"
  elif [ -f "$d/gens.csv" ]; then wc -l < "$d/gens.csv"
  else find "$d" -maxdepth 2 -name .done 2>/dev/null | wc -l; fi
}
export -f progress_of
run_line() {
  local line=$1 o env=() rest
  read -r -a rest <<< "$line"; o=${rest[0]}; rest=("${rest[@]:1}")
  while [ ${#rest[@]} -gt 0 ] && [[ "${rest[0]}" =~ ^[A-Z][A-Z0-9_]*= ]]; do env+=("${rest[0]}"); rest=("${rest[@]:1}"); done
  local before; before=$(progress_of "$root/$o" 2>/dev/null || echo 0)
  local left=$(( HARD - $(date +%s) )); [ $left -lt 60 ] && left=60
  local rc
  case "${rest[0]}" in
    colsel)  # OUT colsel P MODE NCOL ROUNDS FRAMES GENOME args: P is the runner's
      env "${env[@]}" P=$NPROC timeout -k 60 $left python3 -I scratch/inrun/colsel.py "$root/$o" "${rest[@]:2}" >> "$root/$o.log" 2>&1; rc=$?
      mkdir -p "$root/$o" && cp "$root/$o.log" "$root/$o/driver.log" ;;
    emerge)  # OUT emerge ARM N GENS FRAMES FOUNDER opts
      env "${env[@]}" timeout -k 60 $left python3 -I scratch/pheromones/emerge.py "$root/$o" "${rest[@]:1}" >> "$root/$o.log" 2>&1; rc=$?
      mkdir -p "$root/$o" && cp "$root/$o.log" "$root/$o/driver.log" ;;
    *)       # OUT SEED ROUNDS FRAMES SEL GENOME args
      env "${env[@]}" timeout -k 60 $left bash scratch/inrun/rounds.sh "$root/$o" "${rest[@]}" >> "$root/$o.log" 2>&1; rc=$? ;;
  esac
  local after; after=$(progress_of "$root/$o" 2>/dev/null || echo 0)
  # rounds.sh's exit code is its last test's (1 whenever ROUNDS <= 10), so its own markers say whether it finished
  if [ "${rest[0]}" != colsel ] && [ "${rest[0]}" != emerge ] && [ "$rc" != 124 ] && [ "$rc" != 137 ]; then
    if [ -f "$root/$o/r$(( ${rest[1]} - 1 ))/.done" ] || [ -f "$root/$o/EXTINCT" ]; then rc=0; elif [ "$rc" = 0 ]; then rc=1; fi
    # armsum.py counts food and starved from the event logs, which a carried-on line no longer has for earlier legs
    if [ "$rc" = 0 ] && [ "$before" = 0 ]; then
      local t; t=$(mktemp -d); ln -s "$root/$o" "$t/$o"; python3 -I scratch/inrun/armsum.py "$t" > "$root/$o.armsum" 2>&1; rm -rf "$t"
    fi
  fi
  local st note
  case $rc in
    0) st=done; note="exit 0" ;;
    75|124|137) if [ "$after" = "$before" ]; then st=failed; note="no progress in a whole leg (exit $rc, at $after): one step is longer than a leg"
                else st=paused; note="exit $rc at $after (was $before)"; fi ;;
    *) st=failed; note="exit $rc at $after; see $o.log" ;;
  esac
  echo "$o $st $note" > "$root/$o.state"
  echo "$(date -u +%T) $o: $st, $note"
}
export -f run_line
export root
[ ${#LINES[@]} -gt 0 ] || { echo "nothing left to run on $entry"; exit 0; }
printf '%s\n' "${LINES[@]}" | xargs -d '\n' -P "${#LINES[@]}" -I{} bash -c 'run_line "$1"' _ {}
# status: this leg's lines, then the earlier legs' lines this leg did not run (kept by the workflow in $pub)
: > "$pub/$entry.status.new"
for l in "${LINES[@]}"; do o=${l%% *}; cat "$root/$o.state" >> "$pub/$entry.status.new" 2>/dev/null || echo "$o failed no state written" >> "$pub/$entry.status.new"; done
if [ -f "$pub/$entry.status" ]; then
  while read -r o rest; do grep -q "^$o " "$pub/$entry.status.new" || echo "$o $rest" >> "$pub/$entry.status.new"; done < "$pub/$entry.status"
fi
mv "$pub/$entry.status.new" "$pub/$entry.status"
# the carry, and what a lane reads: every job folder but event logs and the every-4k maps and ant lists (the VM's
# small copy). colsel deletes each round's event log as it goes; rounds.sh only gzips them.
tar -C "$root" -czf "$pub/$entry.tgz" --exclude='events.csv' --exclude='events.csv.gz' --exclude='map_f0[01]*' \
  --exclude='ants_f0[01]*' --exclude='*.state' --exclude='*.armsum' . || echo "tar warned (exit $?)"
# readable without unpacking: each job's summary csvs and log
for l in "${LINES[@]}"; do
  o=${l%% *}; mkdir -p "$pub/$o"
  for f in colsel.csv cg.csv donor600.csv gens.csv garden.json EXTINCT; do [ -f "$root/$o/$f" ] && cp "$root/$o/$f" "$pub/$o/"; done
  [ -f "$root/$o.log" ] && tail -n 200 "$root/$o.log" > "$pub/$o/log-tail.txt"
  [ -f "$root/$o.armsum" ] && cp "$root/$o.armsum" "$pub/$o/armsum.txt"
done
ls -la "$pub/$entry.tgz"
cat "$pub/$entry.status"
