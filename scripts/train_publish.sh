#!/usr/bin/env bash
# Push a small snapshot of a training run's out folder to the branch
# `train-results/<name>` as ONE orphan commit, force-pushed each time, so the
# branch always holds the latest snapshot and no history. A lane reads it with
#   git fetch origin train-results/<name> && git checkout FETCH_HEAD -- .
# Keeps the whole out folder; the run's artifact has it too (14 days). Needs GH_TOKEN and GITHUB_REPOSITORY, which
# Actions provides. Usage: scripts/train_publish.sh <out dir> <name>
set -euo pipefail
out=$1 name=$2
# absolute, taken before the cd below ($0 may be a relative path)
here=$(cd "$(dirname "$0")" && pwd)
remote=${TRAIN_PUBLISH_REMOTE:-https://x-access-token:${GH_TOKEN}@github.com/${GITHUB_REPOSITORY}.git}
[ -d "$out" ] || { echo "no out dir $out, nothing to publish"; exit 0; }
branch="train-results/$name"
work=$(mktemp -d)
cd "$work"
git init -q . && git checkout -q -b "$branch"
git config user.name "train-run" && git config user.email "train-run@users.noreply.github.com"
cp -r "$out"/. .
# Every picture is kept (about 350 KB each, one per 5 generations): the brain lane reads
# them from this branch, and the branch is one force-pushed commit, so no history grows.
# curves.png beside the svg the trainer redraws every generation (owner,
# 2026-10-17: curves for every generation from gen 0). Redrawn here with the
# job name as title (the trainer's title is the folder name, "out"). Best
# effort: needs `pip install cairosvg`, which the workflow does.
python3 "$here/traincurves.py" . --out curves.svg --title "$name" >/dev/null 2>&1 || true
python3 -c "import cairosvg; cairosvg.svg2png(url='curves.svg', write_to='curves.png', scale=1.5)" 2>/dev/null || true
git add -A && git commit -q -m "train snapshot $name $(date -u +%FT%TZ)"
git push -q -f "$remote" "$branch"
echo "published $branch"
