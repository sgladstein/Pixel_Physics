#!/usr/bin/env bash
# Branch topology checks. Run from anywhere; checks the repo this script
# lives in. Exit 0 clean, 1 with findings.
#
# Two separate defects, both of which have already cost real work here, and
# neither of which any existing check could see:
#
# 1. **A second name for the trunk.** `main` was the GitHub default while the
#    project lived on `master`, so `main` held a 15-byte stub and everything
#    real was elsewhere. That was fixed by copying `master` to `main` -- but
#    the copy left `master` standing as a writable, un-CI'd mirror, and
#    nothing anywhere asserted the two stayed in step. They did not need to
#    diverge to do damage: `3d53351` records a branch that merged `master`
#    while `main` was 10 commits ahead and silently missed the CLAUDE.md
#    restructure, the map-scroll feature and the play-button fix. The
#    session found out by reading a diff that made no sense.
#
#    The invariant gated below is deliberately *not* "the two are equal".
#    `master` lagging is harmless -- it is a mirror, it is allowed to be
#    stale. `master` holding a commit that `main` does not is the state that
#    manufactures a lost feature, because it means someone's work is
#    reachable only from the name CI does not gate. Lag: fine. Divergence:
#    fail. Once `master` is deleted this check passes vacuously and stays
#    passing, which is the intended end state, not a hole.
#
# 2. **Branches drifting off the trunk.** The parallel-worktree procedure in
#    CLAUDE.md says how to avoid two sessions sharing a `target/`; it says
#    nothing about staying current, so nothing ever pulled a branch forward
#    and the drift compounded silently. Measured 2026-08-22 across 27 remote
#    branches: two sat at 0-1 commits behind `main`, one at 33, and then a
#    plateau of twelve at exactly 125 with a tail out to 233. A branch does
#    not notice it is 125 behind; the merge does, and by then the conflict
#    surface is the whole session. The report below is advisory and prints
#    the number, because the number is the thing a session needs to see
#    before it decides whether to trust what it is sitting on.
#
# Usage:
#   scripts/branchcheck.sh            # full drift report + the divergence gate
#   scripts/branchcheck.sh --gate     # divergence gate only (quiet, for CI)
#   scripts/branchcheck.sh --no-fetch # skip the fetch, use refs as they are
#
# STALE_AFTER=<n> overrides the advisory staleness bar (default 40).

set -u
cd "$(dirname "$0")/.."

gate_only=0
do_fetch=1
for arg in "$@"; do
  case "$arg" in
    --gate) gate_only=1 ;;
    --no-fetch) do_fetch=0 ;;
    # Prints the header block by structure -- every comment line after the
    # shebang, stopping at the first line that is not one. A hardcoded line
    # range rots the moment the header is edited, and prints `set -u`.
    -h|--help) awk 'NR>1 && /^#/ {print; next} NR>1 {exit}' "$0"; exit 0 ;;
    *) printf 'branchcheck: unknown argument: %s\n' "$arg" >&2; exit 2 ;;
  esac
done

stale_after="${STALE_AFTER:-40}"
fail=0
note() { printf 'branchcheck: %s\n' "$*"; fail=1; }

# A shallow clone cannot answer an ancestry question, and CI checkouts are
# shallow by default -- so say that plainly rather than reporting a clean
# run over history that is not there.
if [ "$(git rev-parse --is-shallow-repository 2>/dev/null)" = "true" ]; then
  note "shallow clone: ancestry is unknowable here. Use actions/checkout with fetch-depth: 0, or run git fetch --unshallow."
  exit 1
fi

if [ "$do_fetch" = "1" ]; then
  if ! git fetch --quiet --prune --no-tags origin 2>/dev/null; then
    printf 'branchcheck: fetch failed; continuing against local refs (may be stale)\n' >&2
  fi
fi

if ! git rev-parse --verify --quiet origin/main >/dev/null; then
  note "no origin/main -- this check assumes main is the trunk (it is the GitHub default and the branch CI gates)"
  exit 1
fi

# --- 1. The divergence gate -------------------------------------------------
# Only asks whether some other name for the trunk holds work that `main`
# cannot reach. Add names here if another mirror is ever created; do not
# relax it into an equality check (see the header).
for mirror in master trunk; do
  git rev-parse --verify --quiet "origin/$mirror" >/dev/null || continue
  only=$(git rev-list --count "origin/$mirror" "^origin/main")
  behind=$(git rev-list --count "origin/main" "^origin/$mirror")
  if [ "$only" != "0" ]; then
    note "origin/$mirror has $only commit(s) unreachable from origin/main -- work is stranded on a branch CI does not gate:"
    git log --format='    %h %ad %s' --date=short "origin/$mirror" "^origin/main" | head -20
  elif [ "$gate_only" = "0" ]; then
    if [ "$behind" = "0" ]; then
      printf 'branchcheck: origin/%s is an exact mirror of origin/main (identical). Nothing stranded.\n' "$mirror"
    else
      printf 'branchcheck: origin/%s is a lagging mirror, %s commit(s) behind origin/main, nothing stranded. Lag is fine; it is only a name.\n' "$mirror" "$behind"
    fi
  fi
done

[ "$gate_only" = "1" ] && exit "$fail"

# --- 2. The drift report ----------------------------------------------------
# Advisory. Nothing here sets `fail` -- a branch being behind is a fact about
# a work in progress, not a defect in the tree, and a gate on it would fire
# on every branch that is merely a few days old.
printf '\n%-48s %6s %7s  %-8s %s\n' BRANCH AHEAD BEHIND STATE 'LAST COMMIT'
printf -- '---------------------------------------------------------------------------------\n'

# Collected into a variable rather than piped straight to `sort`, because a
# `while ... done | sort` runs the loop in a subshell and every counter
# incremented inside it is discarded at the pipe -- the summary line then
# reports three zeroes over a table full of findings.
rows=$(
  git for-each-ref --format='%(refname:short)' refs/remotes/origin | while read -r ref; do
    short="${ref#origin/}"
    case "$short" in main|master|trunk|HEAD) continue ;; esac
    ahead=$(git rev-list --count "origin/main..$ref")
    behind=$(git rev-list --count "$ref..origin/main")
    last=$(git log -1 --format=%ad --date=short "$ref")
    # A branch sharing no ancestor with main is not a feature branch that has
    # fallen behind -- it is a deliberate orphan carrying data rather than
    # source, and its "behind" count is just the size of main. `review-queue`
    # is the live instance: `review_lib.py` creates it with `checkout
    # --orphan` as SYNC_BRANCH and pushes card/media files to it. Telling a
    # session to "pull main in before trusting it" would merge the whole
    # engine into the review queue's storage. Detected by structure rather
    # than by name so a second data branch is classified correctly too.
    if ! git merge-base "$ref" origin/main >/dev/null 2>&1; then state=DATA
    elif [ "$ahead" = "0" ]; then state=MERGED
    elif [ "$behind" -gt "$stale_after" ]; then state=STALE
    else state=ok
    fi
    printf '%s\t%s\t%s\t%s\t%s\n' "$state" "$behind" "$short" "$ahead" "$last"
  done
)

printf '%s\n' "$rows" | sort -k1,1 -k2,2nr \
  | while IFS=$'\t' read -r state behind short ahead last; do
      [ -z "$short" ] && continue
      printf '%-48s %6s %7s  %-8s %s\n' "$short" "$ahead" "$behind" "$state" "$last"
    done

count_state() { printf '%s\n' "$rows" | grep -c "^$1	" || true; }
merged=$(count_state MERGED); stale=$(count_state STALE)
healthy=$(count_state ok); data=$(count_state DATA)

printf -- '---------------------------------------------------------------------------------\n'
printf 'branchcheck: %s merged (0 ahead -- carry nothing main lacks, deletable), %s stale (>%s behind), %s current, %s data.\n' \
  "$merged" "$stale" "$stale_after" "$healthy" "$data"
[ "$merged" != "0" ] && printf 'branchcheck: MERGED branches are fully contained in main. Deleting one loses no commit.\n'
[ "$stale" != "0" ] && printf 'branchcheck: a STALE branch merges against a trunk it has not seen. Pull main in before trusting a measurement taken on it.\n'
[ "$data" != "0" ] && printf 'branchcheck: DATA branches share no history with main by design. Never merge main into one.\n'

exit "$fail"
