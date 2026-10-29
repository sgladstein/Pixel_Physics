#!/bin/bash
# keep only M rows of the ids that starved (any place) in deep/ARM_w68/r3, from the gzipped mind log
S=/tmp/claude-0/-home-user-Pixel-Physics/5839c770-56d6-594d-ba6d-0b2ec65d8626/scratchpad
cd $S
for a in std h3; do
  python3 -c "
import csv
ids={r[1] for r in csv.reader(open('deep/${a}_w68/r3/events.csv')) if r[0].isdigit() and r[2]=='death' and int(float(r[5]))==1}
open('deep/${a}.ids','w').write('\n'.join(sorted(ids))+'\n')"
  zcat deep/${a}_w68.mind.gz | awk -F, 'NR==FNR{k[$1]=1;next} $1=="M" && ($4 in k)' deep/${a}.ids - > deep/${a}.starved.log &
done
wait
wc -l deep/*.starved.log
echo FILTDONE
