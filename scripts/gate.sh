#!/usr/bin/env bash
# The pre-push gates, run loudly and reported quietly.
#
# WHY THIS EXISTS. `cargo test` prints one line per test, and this repo has
# **1,081 test functions** (`grep -rE '^\s*#\[test\]' src/ tests/`). A green
# run is therefore ~1,100 lines of `... ok` that nobody reads, and an agent
# that runs the gates before every push pays for those lines in context on
# that turn and on every turn after it. Measured 2026-08-28: the digest below
# is what this script prints instead. That is the whole point -- it is a
# token saving, not a convenience.
#
# **Green prints the digest; red prints everything.** The saving only exists
# on green, which is the common case, and a failure is exactly when the full
# output is worth its tokens. There is no "quiet" mode that can hide a
# failure, deliberately.
#
# WHAT THE DIGEST MUST KEEP, and why it is not just the failures. `CLAUDE.md`
# records that a red `cargo test` stops at the first failing test binary, so
# **the absence of `Running tests/worldgen.rs` from the output is the tell**
# that later binaries never ran -- and it "reads as a pass rather than an
# error". A failures-only view destroys exactly that signal. So the digest
# keeps every `Running <binary>` line and every `test result:` tally, which
# is what makes "did each binary run, and what did each one conclude"
# answerable at a glance. Roughly a dozen lines instead of eleven hundred.
#
# NO PIPES AROUND CARGO. `CLAUDE.md`: piping cargo into `tail`/`grep` throws
# away its exit code, and a background build once reported success while it
# had actually failed. Every gate here redirects to a file and reads `$?`
# directly, so there is no pipeline whose status could be misread. That is
# also why the digest is produced by filtering the *file* afterwards rather
# than by piping the command.
#
# MODES.
#   quick (default)  docscheck, clippy, cargo test --lib
#   full             ...plus the integration tests, ascii, and acceptance
#
# `quick` is the graded gate `Reports/merge-strategy-2026-08-28.md` item 5
# recommends after a clean back-merge. **Clippy is in it and must stay**:
# `cargo test --lib` does not build `examples/`, and clippy
# (`--all-targets`) is the only gate that does. `examples/filmstrip.rs` is
# the third-most-conflicted file in the repo, so a gate that cannot compile
# it is blind to the case item 5 is about. It costs 40 s in CI.
#
# `quick` is NOT a substitute for CI before a landing -- it is what to run
# while working, and after a back-merge that produced no conflicts. Run
# `full`, or push and let CI do it, before you land.

set -uo pipefail

MODE="${1:-quick}"
case "$MODE" in
  quick|full) ;;
  -h|--help) sed -n '2,45p' "$0" | sed 's/^# \?//'; exit 0 ;;
  *) echo "gate: unknown mode '$MODE' (expected: quick | full)" >&2; exit 2 ;;
esac

LOGDIR="${TMPDIR:-/tmp}/pixel-physics-gate.$$"
mkdir -p "$LOGDIR"

FAILED=()
TOTAL_RAW=0
TOTAL_SHOWN=0

# Print the lines that answer "did it run, and what did it conclude".
# Anything a failure needs is handled by the caller dumping the whole log.
digest() {
  grep -aE '^ *(Running|Doc-tests|Finished)|^test result:|^error|^warning:|^failures:' "$1" || true
}

run_gate() {
  local label="$1"; shift
  printf '\n\033[1m%s\033[0m  %s\n' "$label" "$*"
  local log="$LOGDIR/${label//[^a-zA-Z0-9]/_}.log"
  "$@" >"$log" 2>&1
  local rc=$?
  local raw shown
  raw=$(wc -l <"$log")
  TOTAL_RAW=$((TOTAL_RAW + raw))
  if [ "$rc" -eq 0 ]; then
    digest "$log" | sed 's/^/  /'
    shown=$(digest "$log" | wc -l)
    TOTAL_SHOWN=$((TOTAL_SHOWN + shown))
    printf '  \033[32mok\033[0m  (%s lines of output, %s shown)\n' "$raw" "$shown"
  else
    # Red: everything. A failure is worth its tokens.
    sed 's/^/  /' "$log"
    TOTAL_SHOWN=$((TOTAL_SHOWN + raw))
    printf '  \033[31mFAILED\033[0m  exit %s\n' "$rc"
    FAILED+=("$label")
  fi
  return $rc
}

run_gate docscheck bash scripts/docscheck.sh
run_gate clippy    cargo clippy --all-targets --release --locked -- -D warnings
run_gate test-lib  cargo test --lib

if [ "$MODE" = full ]; then
  run_gate test-integration cargo test --locked
  run_gate ascii            cargo run --release --example ascii --locked
  run_gate acceptance       bash scripts/acceptance.sh
fi

echo
if [ ${#FAILED[@]} -eq 0 ]; then
  printf '\033[32mgate: all %s gates green\033[0m — %s lines of output, %s shown' \
    "$MODE" "$TOTAL_RAW" "$TOTAL_SHOWN"
  # One decimal, via awk: integer division reported 3 of 1,022 lines as
  # "100% suppressed", and a tidy 100% that is not 100% is the shape of
  # number CLAUDE.md says to distrust.
  [ "$TOTAL_RAW" -gt 0 ] && printf ' (%s%% suppressed)' \
    "$(awk -v s="$TOTAL_SHOWN" -v r="$TOTAL_RAW" 'BEGIN{printf "%.1f", 100-(s*100/r)}')"
  printf '\nfull logs: %s\n' "$LOGDIR"
  exit 0
else
  printf '\033[31mgate: FAILED\033[0m — %s\nfull logs: %s\n' "${FAILED[*]}" "$LOGDIR"
  exit 1
fi
