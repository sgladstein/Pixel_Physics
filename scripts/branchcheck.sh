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
# 3. **Work that is finished and invisible.** A branch can be current, green
#    and complete and still be reachable by nobody, because the thing that
#    makes work visible here is a pull request -- and a session that cannot
#    reach the GitHub API cannot open one. Measured 2026-08-24: package W3
#    sat at **13 commits ahead and 37 behind**, holding 660 insertions and a
#    latent bug `main` needed, for hours. It was not stale (37 is under the
#    40 bar) so this script printed it as `ok`, and no PR existed, so the
#    integrator's board did not show it either. It was found by enumerating
#    branches by hand.
#
#    The lesson, and the reason for the UNLANDED summary below: **the PR list
#    is not the work list.** `ahead > 0` is the only reliable statement that
#    a branch holds something the trunk does not, and it is true whether or
#    not anyone has opened a PR, whether or not the branch is stale, and
#    whether or not its session still exists. It is reported rather than
#    gated, because a branch mid-work is supposed to be ahead -- the number
#    is for a human deciding what has been forgotten, not a pass/fail.
#
#    **Which of them has a PR is now answerable here (--prs), and it was the
#    missing half.** Measured 2026-08-29: 17 branches carried 174 commits
#    main lacked, and only 4 had an open PR -- so **13 branches holding 141
#    commits were invisible**, including three that were landable that
#    minute (10/20, 6/20 and 3/31 ahead/behind) and one, `worldgen-caves-r6`,
#    at BxF 48,202, which is 160x the bar CLAUDE.md says to act on. The
#    header above says to "check each of these has a PR or an owner"; nobody
#    was doing it by hand, which is the same reason the drift check became a
#    hook rather than a convention.
#
#    **A session cannot make this call itself, and that shapes the design.**
#    Measured 2026-08-29 with a healthy proxy and no relay failures: an
#    authenticated GET of `/repos/.../pulls` returns **HTTP 403** from inside
#    a session -- the same credential scope that makes branch deletion
#    impossible (CLAUDE.md records 37 attempted deletes, all 403). The MCP
#    GitHub tools work where curl does not, so `--prs-from FILE` takes a
#    listing an agent produced through those, and the API path is for CI and
#    for properly scoped tokens.
#
#    **So the one thing this must never do is answer "NO PR" when it does not
#    know.** A failed, truncated or absent lookup renders `?`. That is not
#    defensiveness: `readguard.py` shipped a confidently wrong DENY over
#    every README in the repo and fail-open never saw it, because a wrong
#    answer is not an error (`Reports/pr89-review.md`). Reporting `NO PR` on
#    a 403 would send the next session to open a duplicate of a PR that
#    already exists.
#
# Usage:
#   scripts/branchcheck.sh            # full drift report + the divergence gate
#   scripts/branchcheck.sh --gate     # divergence gate only (quiet, for CI)
#   scripts/branchcheck.sh --no-fetch # skip the fetch, use refs as they are
#   scripts/branchcheck.sh --brief    # summary only, no per-branch table
#   scripts/branchcheck.sh --prs      # ...and say which unlanded branches have a PR
#   scripts/branchcheck.sh --prs-from FILE   # read the PR listing from FILE, not the API
#   scripts/branchcheck.sh --selftest # put each fault back, watch the check go red
#
# STALE_AFTER=<n> overrides the advisory staleness bar (default 40).
# BRANCHCHECK_PRS=<file> supplies a PR listing and turns --prs on by itself,
# which is how the SessionStart hook gets the annotation at zero latency: a
# file costs no network call, so --brief can use it without a timeout risk.
#
# The FILE format, one record per line, blank lines and `#` comments ignored:
#   <number><TAB><head-ref>     e.g.  100\tclaude/world-size-resolution-perf
#   <head-ref>                  number unknown; renders as `PR yes`
#   !truncated                  the listing is INCOMPLETE -- see below
#
# `!truncated` exists because a capped listing is the one way this check can
# manufacture a wrong answer: a branch absent from a truncated page is not a
# branch without a PR. CLAUDE.md's rule is that exhausting a cap must produce
# *less work*, never an *answer* -- so a truncated listing still reports the
# PRs it found and renders every branch it did not find as `?`.

set -u
cd "$(dirname "$0")/.."

gate_only=0
do_fetch=1
brief=0
want_prs=0
selftest=0
prs_from="${BRANCHCHECK_PRS:-}"
# A file configured by env turns the annotation on by itself: it costs no
# network call, so there is no reason to make the hook pass a second flag.
[ -n "$prs_from" ] && want_prs=1
# `while`, not `for`, because --prs-from takes a value. The `=` form is
# accepted too; a flag that works one way and not the other is a footgun in a
# script whose main caller is a hook nobody re-reads.
while [ "$#" -gt 0 ]; do
  case "$1" in
    --gate) gate_only=1 ;;
    --no-fetch) do_fetch=0 ;;
    --brief) brief=1 ;;
    --prs) want_prs=1 ;;
    --selftest) selftest=1 ;;
    --prs-from)
      shift
      [ "$#" -gt 0 ] || { printf 'branchcheck: --prs-from needs a file\n' >&2; exit 2; }
      prs_from="$1"; want_prs=1 ;;
    --prs-from=*) prs_from="${1#*=}"; want_prs=1 ;;
    # Prints the header block by structure -- every comment line after the
    # shebang, stopping at the first line that is not one. A hardcoded line
    # range rots the moment the header is edited, and prints `set -u`.
    -h|--help) awk 'NR>1 && /^#/ {print; next} NR>1 {exit}' "$0"; exit 0 ;;
    *) printf 'branchcheck: unknown argument: %s\n' "$1" >&2; exit 2 ;;
  esac
  shift
done

stale_after="${STALE_AFTER:-40}"
fail=0
note() { printf 'branchcheck: %s\n' "$*"; fail=1; }

# --- PR annotation ----------------------------------------------------------
# Answers "which unlanded branches are invisible", the half the UNLANDED
# report could not do. Never sets `fail`: an open PR is not a defect and a
# missing one is a prompt, not a fault.
pr_list=""     # newline-separated "<number>\t<ref>"; "?" when the number is unknown
pr_complete=1  # 0 = the listing is NOT known to be exhaustive
pr_reason=""   # why it is incomplete, in words a session can act on

# **The whole safety property of this feature lives in the last branch of
# pr_for(): a miss against an incomplete listing is `PR ?`, never `NO PR`.**
# Everything else here is plumbing. See the header for why a wrong answer is
# worse than no answer.
load_prs() {
  [ "$want_prs" = "1" ] || return 0

  if [ -n "$prs_from" ]; then
    if [ ! -r "$prs_from" ]; then
      pr_complete=0; pr_reason="cannot read $prs_from"; return 0
    fi
    grep -qx '!truncated' "$prs_from" 2>/dev/null && {
      pr_complete=0; pr_reason="listing marked !truncated"; }
    # Strip CRs, comments, blank lines and the `!` directives; then normalise
    # both accepted shapes to "<number>\t<ref>".
    pr_list=$(sed 's/\r$//' "$prs_from" \
      | grep -v '^[[:space:]]*#' | grep -v '^[[:space:]]*!' \
      | awk 'NF' \
      | awk -F'\t' '{ if (NF >= 2) print $1 "\t" $2; else print "?\t" $1 }')
    return 0
  fi

  local tok slug tmp code n
  tok="${GH_TOKEN:-${GITHUB_TOKEN:-}}"
  if [ -z "$tok" ]; then
    pr_complete=0; pr_reason="no GH_TOKEN or GITHUB_TOKEN in the environment"; return 0
  fi
  command -v curl >/dev/null 2>&1 || {
    pr_complete=0; pr_reason="curl is not available"; return 0; }
  slug=$(git config --get remote.origin.url 2>/dev/null \
    | sed -E 's#^.*[:/]([^/]+/[^/]+?)(\.git)?$#\1#')
  if [ -z "$slug" ]; then
    pr_complete=0; pr_reason="cannot derive owner/repo from remote.origin.url"; return 0
  fi
  tmp=$(mktemp)
  code=$(curl -sS -o "$tmp" -w '%{http_code}' --max-time 15 \
    -H "Authorization: Bearer $tok" \
    -H "Accept: application/vnd.github+json" \
    -H "X-GitHub-Api-Version: 2022-11-28" \
    "https://api.github.com/repos/$slug/pulls?state=open&per_page=100" 2>/dev/null) || code=000
  if [ "$code" != "200" ]; then
    pr_complete=0
    if [ "$code" = "403" ]; then
      # The measured in-session case. Name the way out rather than the status.
      pr_reason="HTTP 403 -- the in-session credential cannot list PRs (same scope that blocks branch deletion). Use the MCP GitHub tools and pass --prs-from FILE."
    else
      pr_reason="HTTP $code from the GitHub API"
    fi
    rm -f "$tmp"; return 0
  fi
  pr_list=$(python3 -c 'import json,sys
try:
    d = json.load(open(sys.argv[1]))
except Exception:
    sys.exit(3)
for pr in d:
    print("%s\t%s" % (pr.get("number", "?"), (pr.get("head") or {}).get("ref", "")))' "$tmp" 2>/dev/null) || {
    pr_complete=0; pr_reason="could not parse the API response"; rm -f "$tmp"; return 0; }
  rm -f "$tmp"
  # A capped page is the one way this check can manufacture a wrong answer:
  # a branch absent from a truncated listing is not a branch without a PR.
  n=$(printf '%s\n' "$pr_list" | grep -c . || true)
  if [ "$n" -ge 100 ]; then
    pr_complete=0; pr_reason="listing hit the 100-PR page cap, so it is not exhaustive"
  fi
  return 0
}

# The annotation for one branch. Empty when --prs was not asked for, so the
# existing output is byte-identical unless the feature is on.
pr_for() {
  [ "$want_prs" = "1" ] || { printf ''; return 0; }
  local hit
  hit=$(printf '%s\n' "$pr_list" | awk -F'\t' -v r="$1" '$2 == r { print $1; exit }')
  if [ -n "$hit" ]; then
    if [ "$hit" = "?" ]; then printf 'PR yes'; else printf 'PR #%s' "$hit"; fi
  elif [ "$pr_complete" = "1" ]; then
    printf 'NO PR'
  else
    printf 'PR ?'
  fi
}

# A shallow clone cannot answer an ancestry question, and CI checkouts are
# shallow by default -- so say that plainly rather than reporting a clean
# run over history that is not there.
# A shallow clone cannot answer the ancestry question the *gate* asks, so the
# gate still refuses. The drift report is a different question and mostly
# survives: measured in a cloud container at depth 645, every ahead/behind
# count matched the same figures taken with full history. Refusing outright
# meant this script could not run at all in the environment where most
# sessions now start -- which is how a convention nobody can execute stays
# unexecuted. So the report runs, and says what it cannot vouch for.
shallow=0
if [ "$(git rev-parse --is-shallow-repository 2>/dev/null)" = "true" ]; then
  shallow=1
  if [ "$gate_only" = "1" ]; then
    note "shallow clone: ancestry is unknowable here. Use actions/checkout with fetch-depth: 0, or run git fetch --unshallow."
    exit 1
  fi
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

# --- --selftest: put each fault back and watch the check go red -------------
# CLAUDE.md: "before you cite a guard's green as evidence, put the fault it is
# named for back and watch it go red", built as a command rather than left as
# a discipline. The row that matters is C -- a lookup that FAILED must render
# `?`, never `NO PR` -- because that is the branch a wrong answer hides in,
# and `readguard.py` shipped exactly that failure (a confidently wrong DENY
# over every README in the repo) with fail-open never seeing it.
#
# Row F is the sensitivity control, and it is the reason this is not
# `lanecheck --selftest` before its repair: that test re-implemented its own
# predicate inline and passed with the real function gutted to `return []`.
# So F does not re-implement anything -- it MUTATES this very file, inverting
# pr_for's unknown branch to `NO PR`, and asserts row C catches the mutant.
# A green C with a green mutant means C is blind, not that the code is right.
if [ "$selftest" = "1" ]; then
  st_fail=0
  d=$(mktemp -d)
  mutant="scripts/.branchcheck-selftest-mutant.sh"
  trap 'rm -rf "$d" "$mutant"' EXIT
  st() { if [ "$1" = "0" ]; then printf '  ok   %s\n' "$2"; else printf '  FAIL %s\n' "$2"; st_fail=1; fi; }

  # Every row needs a branch that is ahead of main, or it passes vacuously --
  # and a test that cannot fail reads exactly like one that passed. Rather
  # than borrow whichever branch happens to be unlanded today (which makes
  # the test a function of repo state, and silently vacuous on a clean tree),
  # build one: an empty commit on top of main, published as a
  # remote-tracking ref because that is what the report iterates. Removed by
  # the trap; refs/remotes is local-only and `git fetch --prune` would clear
  # a stray anyway.
  # TWO probes, because one is not enough to test the interesting row: `seen`
  # appears in the listing, `unseen` never does. With a single probe, row D
  # ("hits keep their number, misses go ?") has no miss to look at and would
  # pass on a tree whose only unlanded branch is the probe itself.
  probe="branchcheck-selftest-seen"
  probe2="branchcheck-selftest-unseen"
  mk_probe() {
    GIT_AUTHOR_NAME=selftest GIT_AUTHOR_EMAIL=selftest@invalid \
    GIT_COMMITTER_NAME=selftest GIT_COMMITTER_EMAIL=selftest@invalid \
    git commit-tree 'origin/main^{tree}' -p origin/main -m "$1" 2>/dev/null
  }
  probe_sha=$(mk_probe 'branchcheck selftest probe seen') || probe_sha=""
  probe2_sha=$(mk_probe 'branchcheck selftest probe unseen') || probe2_sha=""
  if [ -z "$probe_sha" ] || [ -z "$probe2_sha" ]; then
    printf 'branchcheck --selftest: CANNOT RUN -- could not build probe commits off origin/main.\n' >&2
    exit 1
  fi
  git update-ref "refs/remotes/origin/$probe"  "$probe_sha"  || exit 1
  git update-ref "refs/remotes/origin/$probe2" "$probe2_sha" || exit 1
  trap 'rm -rf "$d" "$mutant"
        git update-ref -d refs/remotes/origin/branchcheck-selftest-seen 2>/dev/null || true
        git update-ref -d refs/remotes/origin/branchcheck-selftest-unseen 2>/dev/null || true' EXIT
  printf 'branchcheck --selftest: probes %s / %s\n' "$probe" "$probe2"

  # The annotation for ONE named branch, so every row asserts on a probe this
  # test controls rather than on "some line somewhere". Asserting `grep -c
  # 'NO PR' >= 1` over the whole report passes off any unrelated real branch,
  # which makes the row a measurement of the repo instead of the code.
  ann_of() {
    printf '%s\n' "$2" | grep -E "^[[:space:]]+$1[[:space:]]" \
      | sed -E 's/.*last [0-9]{4}-[0-9]{2}-[0-9]{2}[[:space:]]*//' | head -1
  }

  printf '4242\t%s\n' "$probe" > "$d/hit.tsv"     # `seen` has a PR; `unseen` never does
  : > "$d/empty.tsv"                               # complete listing, no PRs
  { printf '4242\t%s\n' "$probe"; printf '!truncated\n'; } > "$d/trunc.tsv"

  run() { bash scripts/branchcheck.sh --no-fetch "$@" 2>&1; }
  n_of() { printf '%s\n' "$2" | grep -c -- "$1" || true; }

  a=$(run --prs-from "$d/hit.tsv")
  [ "$(ann_of "$probe" "$a")" = "PR #4242" ] \
    && st 0 "A  a branch WITH a PR renders its number" \
    || st 1 "A  a branch WITH a PR renders its number (got '$(ann_of "$probe" "$a")')"

  b=$(run --prs-from "$d/empty.tsv")
  [ "$(ann_of "$probe2" "$b")" = "NO PR" ] \
    && st 0 "B  a complete listing can report NO PR" \
    || st 1 "B  a complete listing can report NO PR (got '$(ann_of "$probe2" "$b")')"

  c=$(run --prs-from "$d/missing.tsv")
  if [ "$(n_of 'NO PR' "$c")" = "0" ] && [ "$(ann_of "$probe2" "$c")" = "PR ?" ]; then
    st 0 "C  a FAILED lookup renders ? and never NO PR"
  else
    st 1 "C  a FAILED lookup renders ? and never NO PR (probe '$(ann_of "$probe2" "$c")', NO PR x$(n_of 'NO PR' "$c"))"
  fi

  t=$(run --prs-from "$d/trunc.tsv")
  if [ "$(n_of 'NO PR' "$t")" = "0" ] \
     && [ "$(ann_of "$probe" "$t")" = "PR #4242" ] \
     && [ "$(ann_of "$probe2" "$t")" = "PR ?" ]; then
    st 0 "D  a truncated listing keeps its hits and marks the rest ?"
  else
    st 1 "D  a truncated listing keeps its hits and marks the rest ? (seen '$(ann_of "$probe" "$t")', unseen '$(ann_of "$probe2" "$t")')"
  fi

  e=$(run)
  if [ -z "$(ann_of "$probe" "$e")" ] && [ "$(n_of 'NO PR' "$e")" = "0" ] \
     && [ "$(n_of 'PR ?' "$e")" = "0" ] && [ "$(n_of 'PR #' "$e")" = "0" ]; then
    st 0 "E  without --prs the report carries no annotation"
  else
    st 1 "E  without --prs the report carries no annotation"
  fi

  # F -- the control. Invert pr_for's unknown branch and require C to catch it.
  sed "s/printf 'PR ?'/printf 'NO PR'/" scripts/branchcheck.sh > "$mutant"
  # Exactly one F row, whatever happens. An injection that matched nothing is
  # itself the failure -- it means the control did not run, which is the
  # blind-injection trap docbench.py's own selftest records hitting.
  if cmp -s scripts/branchcheck.sh "$mutant"; then
    st 1 "F  the mutation matched nothing -- row C's control never ran"
  else
    m=$(bash "$mutant" --no-fetch --prs-from "$d/missing.tsv" 2>&1)
    if [ "$(ann_of "$probe2" "$m")" = "NO PR" ]; then
      st 0 "F  row C is NOT blind: the inverted-fallback mutant fails it"
    else
      st 1 "F  row C is BLIND: the mutant renders no NO PR either, so C proves nothing"
    fi
  fi

  [ "$st_fail" = "0" ] && printf 'branchcheck --selftest: all rows green\n' \
                       || printf 'branchcheck --selftest: FAILURES above\n' >&2
  exit "$st_fail"
fi

load_prs

# --- 2. The drift report ----------------------------------------------------
# Advisory. Nothing here sets `fail` -- a branch being behind is a fact about
# a work in progress, not a defect in the tree, and a gate on it would fire
# on every branch that is merely a few days old.
if [ "$brief" = "0" ]; then
  printf '\n%-42s %6s %7s %6s %7s  %-8s %s\n' BRANCH AHEAD BEHIND FILES 'BxF' STATE 'LAST COMMIT'
  printf -- '-------------------------------------------------------------------------------------------------\n'
fi

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
    # **The `files` half of `CLAUDE.md`'s `behind x files > 300` rule, which
    # this script claimed to print and did not.** For a year the document
    # said "prints your two numbers" while only `behind` existed here, so
    # every session that tried to apply the rule invented its own `files`
    # operand -- two readings of the very same merge scored it 132 and 198,
    # because one counted the branch's changed files and the other main's.
    #
    # **Branch-side, and three-dot.** `origin/main...$ref` is the branch's
    # own changes since the merge base, which is the operand `CLAUDE.md`'s
    # own reasoning implies: it reads a large `files` as "the branch has
    # quietly become more than one feature", a statement about this branch's
    # scope and not about main's. Two-dot would count main's files too and
    # make the term grow while the branch sat still.
    #
    # A DATA branch shares no merge base, so `...` has nothing to resolve
    # and would error; it is scored 0 and reported as DATA regardless.
    if git merge-base "$ref" origin/main >/dev/null 2>&1; then
      files=$(git diff --name-only "origin/main...$ref" | wc -l | tr -d ' ')
    else
      files=0
    fi
    product=$((behind * files))
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
    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$state" "$behind" "$short" "$ahead" "$last" "$files" "$product"
  done
)

if [ "$brief" = "0" ]; then
  printf '%s\n' "$rows" | sort -k1,1 -k2,2nr \
    | while IFS=$'\t' read -r state behind short ahead last files product; do
        [ -z "$short" ] && continue
        printf '%-42s %6s %7s %6s %7s  %-8s %s\n' "$short" "$ahead" "$behind" "$files" "$product" "$state" "$last"
      done
fi

count_state() { printf '%s\n' "$rows" | grep -c "^$1	" || true; }
merged=$(count_state MERGED); stale=$(count_state STALE)
healthy=$(count_state ok); data=$(count_state DATA)

# Unlanded work, counted across every state except DATA -- a STALE branch and
# a current one are equally capable of holding a finished package nobody can
# see, and W3 (the case in the header) was neither merged nor stale.
unlanded_rows=$(printf '%s\n' "$rows" | awk -F'\t' '$1 != "DATA" && $4 > 0')
unlanded=$(printf '%s' "$unlanded_rows" | grep -c . || true)
unlanded_commits=$(printf '%s\n' "$unlanded_rows" | awk -F'\t' '{n += $4} END {print n+0}')

# The line a session actually needs at start-up: where *this* branch stands.
# The whole point of the SessionStart hook is that CLAUDE.md's "run this when
# you pick up a branch" was a convention nobody executed -- and the drift it
# exists to prevent happened anyway, at scale.
here=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo '?')
if [ "$here" != "main" ] && [ "$here" != "?" ]; then
  h_ahead=$(git rev-list --count origin/main..HEAD 2>/dev/null || echo '?')
  h_behind=$(git rev-list --count HEAD..origin/main 2>/dev/null || echo '?')
  h_files=$(git diff --name-only origin/main...HEAD 2>/dev/null | grep -c . || true)
  printf 'branchcheck: YOU ARE ON %s -- %s ahead, %s behind main, %s file(s) changed.\n' \
    "$here" "$h_ahead" "$h_behind" "$h_files"
  # CLAUDE.md's landing rule, applied rather than quoted.
  if [ "$h_behind" != "?" ] && [ "$h_behind" -gt 0 ] && [ $((h_behind * h_files)) -gt 300 ]; then
    printf 'branchcheck: behind x files = %s, past the 300 bar where merges get expensive. Merge main in, or land what you have.\n' \
      "$((h_behind * h_files))"
  fi
fi

# Cross-lane traffic, surfaced rather than remembered. Two lanes exchanged two
# substantive corrections on 2026-08-25 and every message was carried by hand,
# because both could read each other's branches and neither had a place to
# write. `Reports/lanes/` is that place; this is how you find out somebody
# wrote. Searched across all refs, so a note on an unmerged branch still shows.
notes=$(git log --all --since='7 days ago' --name-only --format='%h' -- Reports/lanes/ 2>/dev/null \
  | grep '^Reports/lanes/' | grep -v 'README.md' | sort -u)
if [ -n "$notes" ]; then
  printf 'branchcheck: lane notes touched in the last 7 days: %s\n' \
    "$(printf '%s\n' "$notes" | sed 's|Reports/lanes/||; s|\.md$||' | tr '\n' ' ')"
  printf 'branchcheck: read one without checking out --- git show origin/<branch>:Reports/lanes/<lane>.md\n'
fi

if [ "$shallow" = "1" ]; then
  printf 'branchcheck: shallow clone (depth %s). Ahead/behind counts held against full history when this was measured, but a branch whose common ancestor is beyond the boundary is reported DATA when it is merely old. Run git fetch --unshallow before trusting a DATA verdict.\n' \
    "$(git rev-list --count origin/main 2>/dev/null || echo '?')"
fi

printf -- '---------------------------------------------------------------------------------\n'
printf 'branchcheck: %s merged (0 ahead -- carry nothing main lacks, deletable), %s stale (>%s behind), %s current, %s data.\n' \
  "$merged" "$stale" "$stale_after" "$healthy" "$data"
[ "$merged" != "0" ] && printf 'branchcheck: MERGED branches are fully contained in main. Deleting one loses no commit.\n'
[ "$stale" != "0" ] && printf 'branchcheck: a STALE branch merges against a trunk it has not seen. Pull main in before trusting a measurement taken on it.\n'
[ "$data" != "0" ] && printf 'branchcheck: DATA branches share no history with main by design. Never merge main into one.\n'

if [ "$unlanded" != "0" ]; then
  printf 'branchcheck: %s branch(es) carry %s commit(s) main does not have.\n' "$unlanded" "$unlanded_commits"
  # Counted in a subshell and captured, not incremented across the pipe --
  # the same trap the `rows` comment above records paying for once already.
  if [ "$want_prs" = "1" ]; then
    noprs=$(printf '%s\n' "$unlanded_rows" | while IFS=$'\t' read -r state behind short ahead last files product; do
      [ -z "$short" ] && continue
      [ "$(pr_for "$short")" = "NO PR" ] && printf 'x\n'
    done | grep -c . || true)
    nocommits=$(printf '%s\n' "$unlanded_rows" | while IFS=$'\t' read -r state behind short ahead last files product; do
      [ -z "$short" ] && continue
      [ "$(pr_for "$short")" = "NO PR" ] && printf '%s\n' "$ahead"
    done | awk '{n += $1} END {print n+0}')
    if [ -n "$pr_reason" ]; then
      printf 'branchcheck: PR status UNKNOWN for branches not listed below -- %s\n' "$pr_reason"
    fi
    if [ "$noprs" != "0" ]; then
      printf 'branchcheck: %s of them hold %s commit(s) and have NO OPEN PR -- invisible to anyone reading the PR list.\n' \
        "$noprs" "$nocommits"
    fi
  fi
  printf 'branchcheck: THE PR LIST IS NOT THE WORK LIST -- a finished branch with no PR is invisible, and sessions that cannot reach the GitHub API cannot open one. Check each of these has a PR or an owner before concluding a program is done:\n'
  # Brief mode shows the deepest few only. The full list is the point of the
  # full report; at session start it is a prompt to go look, not the lookup.
  if [ "$brief" = "1" ]; then show=5; else show=0; fi
  printf '%s\n' "$unlanded_rows" | sort -k4,4nr | { n=0; while IFS=$'\t' read -r state behind short ahead last files product; do
    [ -z "$short" ] && continue
    n=$((n + 1))
    if [ "$show" != "0" ] && [ "$n" -gt "$show" ]; then continue; fi
    printf '    %-40s %3s ahead  %3s behind  %3s files  BxF %5s  %-6s last %s%s\n' \
      "$short" "$ahead" "$behind" "$files" "$product" "$state" "$last" \
      "$(ann=$(pr_for "$short"); [ -n "$ann" ] && printf '  %s' "$ann")"
  done
  if [ "$show" != "0" ] && [ "$n" -gt "$show" ]; then
    printf '    ... and %s more -- run scripts/branchcheck.sh for the full list.\n' "$((n - show))"
  fi; }
fi

exit "$fail"
