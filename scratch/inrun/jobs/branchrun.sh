#!/bin/bash
# SCRATCH (teacher lane, 2026-10-31): run another lane's script at THAT lane's branch head, as an inrun_cloud `sh` job,
# when the batch itself builds a different ref. Usage (job line): `NAME sh scratch/inrun/jobs/branchrun.sh SHA SCRIPT`
# -> bash branchrun.sh OUTDIR SHA SCRIPT. SHA must be a full 40-hex commit (fetched --depth 1 by id, as the VM fetches
# its own REF). SCRIPT is a path in this checkout, run as `bash SCRIPT OUTDIR` from a worktree at SHA with its own
# target dir, so it builds and runs that lane's code. First user: the Plants lane's rich-earth test (richearth.sh).
set -euo pipefail
O=$1; SHA=$2; SCRIPT=$(realpath "$3")
D=$(mktemp -d)/tree
git fetch -q --depth 1 origin "$SHA"
git worktree add -q --detach "$D" "$SHA"
cd "$D"
CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target}-${SHA:0:8}" bash "$SCRIPT" "$O"
