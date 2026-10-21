#!/bin/bash
# wait until every check run on commit $1 has completed, then print them; nothing else
for i in $(seq 1 60); do
  n=$(gh api repos/sgladstein/pixel_physics/commits/$1/check-runs --jq '.total_count' 2>/dev/null)
  left=$(gh api repos/sgladstein/pixel_physics/commits/$1/check-runs --jq '[.check_runs[]|select(.status!="completed")]|length' 2>/dev/null)
  if [ -n "$n" ] && [ "$n" -gt 0 ] && [ "$left" = "0" ]; then gh api repos/sgladstein/pixel_physics/commits/$1/check-runs --jq '.check_runs[]|[.name,.conclusion]|@tsv'; exit 0; fi
  sleep 60
done; echo "timed out"
