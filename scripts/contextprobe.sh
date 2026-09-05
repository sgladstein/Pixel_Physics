#!/usr/bin/env bash
# What actually loads into a session's context, measured rather than inferred.
#
# `scripts/contextbudget.py` counts the bytes of CLAUDE.md and calls that the
# always-loaded cost. That is an *inference*: it assumes every instruction file
# in the tree is paid at session start. This script asks the runtime instead,
# via the `InstructionsLoaded` hook, whose payload names each loaded file and a
# `load_reason` of `session_start`, `nested_traversal`, `path_glob_match`,
# `include` or `compact`.
#
# `Reports/two-games-one-repo-2026-08-30.md` named this instrument as the thing
# it lacked, and decided against `paths:`-scoping on two upstream bug reports it
# could not check locally -- its own probe was void, because an
# `isolation: worktree` agent's worktree was cut from a base predating the rule
# under test, so the file was never on disk. Running the probe from a *shell*
# rather than from an agent worktree is what makes it settleable: the tree on
# disk is the tree being measured.
#
# Measured 2026-09-05 on CLI 2.1.261, in this repo: neither bug reproduces.
# A `paths:`-scoped rule loads on `path_glob_match`, not at session start; a
# nested CLAUDE.md loads on `nested_traversal`; a session that reads a
# non-matching file loads neither; and the worktree case -- the disqualifying
# one, issue #23569 -- behaves the same as the main checkout.
#
#   bash scripts/contextprobe.sh              # what a session pays before it acts
#   bash scripts/contextprobe.sh src/lab/time.rs   # ...and what reading that file adds
#   bash scripts/contextprobe.sh --selftest   # prove the probe can see a lazy file
#
# The selftest is the positive control CLAUDE.md demands before citing a green:
# it plants a scoped rule and a nested CLAUDE.md, checks the probe reports both,
# and removes them. Without it, "nothing extra loaded" is unfalsifiable -- it
# reads identically whether the scoping works or the hook never fired.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

command -v claude >/dev/null 2>&1 || { echo "contextprobe: no 'claude' CLI on PATH"; exit 2; }

LOG="$WORK/instr.log"
cat > "$WORK/hooks.json" <<EOF
{"hooks":{"InstructionsLoaded":[{"hooks":[{"type":"command","command":"cat >> $LOG; echo >> $LOG"}]}]}}
EOF

# One probe session. $1 is the prompt; the session is told not to wander,
# because anything it reads on its own initiative pulls in instruction files
# and the point of the run is to attribute each load to a known cause.
probe() {
  : > "$LOG"
  ( cd "$ROOT" && timeout 300 claude -p "$1" --settings "$WORK/hooks.json" </dev/null >/dev/null 2>&1 )
  python3 - "$LOG" "$ROOT" <<'PY'
import json, os, sys
seen = []
try:
    lines = open(sys.argv[1]).read().splitlines()
except FileNotFoundError:
    lines = []
for line in lines:
    line = line.strip()
    if not line:
        continue
    try:
        d = json.loads(line)
    except ValueError:
        continue
    key = (os.path.relpath(d.get("file_path", "?"), sys.argv[2]), str(d.get("load_reason")))
    if key not in seen:
        seen.append(key)
if not seen:
    print("  (nothing -- hook never fired; the probe is blind, not the tree empty)")
for path, reason in seen:
    print("  %-40s %s" % (path, reason))
PY
}

if [ "${1:-}" = "--selftest" ]; then
  # Plant the two lazy forms, prove the probe sees them, then take them out.
  mkdir -p "$ROOT/.claude/rules"
  RULE="$ROOT/.claude/rules/zz-contextprobe-selftest.md"
  NEST="$ROOT/src/lab/CLAUDE.md"
  [ -e "$NEST" ] && { echo "contextprobe: $NEST already exists; refusing to overwrite"; exit 2; }
  printf -- '---\npaths:\n  - "src/lab/**"\n---\ncontextprobe selftest rule\n' > "$RULE"
  printf -- 'contextprobe selftest nested memory\n' > "$NEST"

  echo "contextprobe --selftest: reading src/lab/time.rs with both lazy forms planted"
  OUT="$(probe "Read the first 3 lines of src/lab/time.rs. Nothing else.")"
  echo "$OUT"
  rm -f "$RULE" "$NEST"
  rmdir "$ROOT/.claude/rules" 2>/dev/null

  fail=0
  grep -q "path_glob_match"  <<<"$OUT" || { echo "contextprobe: FAIL -- scoped rule never reported"; fail=1; }
  grep -q "nested_traversal" <<<"$OUT" || { echo "contextprobe: FAIL -- nested CLAUDE.md never reported"; fail=1; }
  [ "$fail" = 0 ] && echo "contextprobe: selftest PASS -- both lazy forms observed"
  exit "$fail"
fi

echo "contextprobe: what loads before the session acts"
probe "Reply with exactly: OK. Do not read, list or search any files."

if [ -n "${1:-}" ]; then
  echo
  echo "contextprobe: ...and what reading $1 adds"
  probe "Read the first 3 lines of $1. Nothing else."
fi
