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
# every picture but the last two
ls pic-gen-*.png 2>/dev/null | sort | head -n -2 | xargs -r rm -f
git add -A && git commit -q -m "train snapshot $name $(date -u +%FT%TZ)"
git push -q -f "https://x-access-token:${GH_TOKEN}@github.com/${GITHUB_REPOSITORY}.git" "$branch"
echo "published $branch"
