#!/usr/bin/env bash
# Push a small snapshot of a training run's out folder to the branch
# `train-results/<name>` as ONE orphan commit, force-pushed each time, so the
# branch always holds the latest snapshot and no history. A lane reads it with
#   git fetch origin train-results/<name> && git checkout FETCH_HEAD -- .
# Keeps text, csv, curves and the last two pictures; the full folder (every
# picture) is the run's artifact. Needs GH_TOKEN and GITHUB_REPOSITORY, which
# Actions provides. Usage: scripts/train_publish.sh <out dir> <name>
set -euo pipefail
out=$1 name=$2
[ -d "$out" ] || { echo "no out dir $out, nothing to publish"; exit 0; }
branch="train-results/$name"
work=$(mktemp -d)
cd "$work"
git init -q . && git checkout -q -b "$branch"
git config user.name "train-run" && git config user.email "train-run@users.noreply.github.com"
cp -r "$out"/. .
# every picture but the last two. `find`, not `ls`: pictures are written every
# 5th generation, so the first 15-minute snapshot of a slow run has none, `ls`
# failed under pipefail, and the script exited silently -- the snapshot at
# 03:50 of the 2026-10-18 threads=1 probe never happened.
find . -maxdepth 1 -name 'pic-gen-*.png' | sort | head -n -2 | xargs -r rm -f
# curves.png beside the svg the trainer redraws every generation (owner,
# 2026-10-17: curves for every generation from gen 0). Redrawn here with the
# job name as title (the trainer's title is the folder name, "out"). Best
# effort: needs `pip install cairosvg`, which the workflow does.
here=$(cd "$(dirname "$0")" && pwd)
python3 "$here/traincurves.py" . --out curves.svg --title "$name" >/dev/null 2>&1 || true
python3 -c "import cairosvg; cairosvg.svg2png(url='curves.svg', write_to='curves.png', scale=1.5)" 2>/dev/null || true
git add -A && git commit -q -m "train snapshot $name $(date -u +%FT%TZ)"
git push -q -f "https://x-access-token:${GH_TOKEN}@github.com/${GITHUB_REPOSITORY}.git" "$branch"
echo "published $branch"
