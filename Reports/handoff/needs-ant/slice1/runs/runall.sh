#!/usr/bin/env bash
d=$(dirname "$0")
while read -r label bin seed frames envs extra; do
  [ "$envs" = "-" ] && envs=""; [ "$extra" = "-" ] && extra=""
  echo "$label|$bin|$seed|$frames|${envs//|/ }|${extra//|/ }"
done < "$d/jobs.txt" | xargs -P 4 -I{} bash -c 'IFS="|" read -r l b s f e x <<< "{}"; '"$d"'/wrun.sh "$l" "$b" "$s" "$f" "$e" "$x"'
echo ALLDONE
