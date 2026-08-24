#!/usr/bin/env bash
# Mechanical checks over the documentation. Run from anywhere; checks the
# repo this script lives in. Exit 0 clean, 1 with findings.
#
# Exists because three documentation defect classes have each recurred here
# (Reports/documentation-audit.md): references that outlive their target,
# the README architecture map silently omitting new modules, and freshness
# notes duplicated by a bad edit (the bb20167 damage class). A check that
# runs catches the third instance; a convention alone did not catch the
# second. Not wired into CI as a gate on purpose -- a build that fails on a
# link typo is its own tax; run it by hand after touching documentation,
# the way `cargo fmt --check` stays informational.

set -u
cd "$(dirname "$0")/.."
fail=0
note() { printf 'docscheck: %s\n' "$*"; fail=1; }

doc_files() {
  ls README.md PLAN.md PLAN-log.md CLAUDE.md 2>/dev/null
  ls wiki/*.md Reports/*.md docs/*.md research/*.md 2>/dev/null
}

# --- 1. Relative links and paths must resolve -------------------------------
# Markdown links only. Backticked file mentions are prose, not promises, and
# line-number suffixes rot too fast to gate on.
while IFS='|' read -r file link; do
  [ -z "$link" ] && continue
  case "$link" in
    http*|mailto*|\#*) continue ;;
  esac
  target="${link%%#*}"
  [ -z "$target" ] && continue
  dir=$(dirname "$file")
  if [ ! -e "$dir/$target" ] && [ ! -e "$target" ]; then
    note "dead link in $file: ($link)"
  fi
done < <(doc_files | while read -r f; do
  grep -oE '\]\([^)]+\)' "$f" 2>/dev/null | sed -E 's/^\]\(//; s/\)$//' \
    | while read -r l; do printf '%s|%s\n' "$f" "$l"; done
done)

# --- 2. The README architecture map must cover the real module tree ---------
# One direction only: every source module must be mentioned somewhere in
# README.md. The map omitting a module is the defect that hid ~1 MB of
# simulation from the orientation document.
for src in src/*.rs src/sim/*.rs src/worldgen/*.rs; do
  base=$(basename "$src")
  case "$base" in mod.rs) continue ;; esac
  grep -q "$base" README.md || note "architecture map: $base missing from README.md"
done

# --- 3. Freshness notes: exactly one per wiki page, and dated ---------------
for f in wiki/*.md; do
  case "$f" in wiki/README.md) continue ;; esac
  n=$(grep -c 'urrent as of' "$f")
  if [ "$n" -gt 1 ]; then
    note "doubled freshness note in $f ($n occurrences) -- the bb20167 damage class"
  fi
  if grep -q 'Current as of: this build' "$f"; then
    note "undated freshness note in $f -- 'this build' cannot go stale; use a date"
  fi
done

# --- 4. Every report must be indexed, and in-flight entries must promote ----
if [ -f Reports/README.md ]; then
  for f in Reports/*.md; do
    base=$(basename "$f")
    case "$base" in README.md) continue ;; esac
    grep -q "$base" Reports/README.md || note "Reports/README.md: $base not indexed"
  done
  # An "In flight" entry whose file now exists in Reports/ has merged and
  # should move up into a real section. (Process substitution, not a pipe:
  # a piped while runs in a subshell and its note would not stick.)
  while read -r base; do
    [ -f "Reports/$base" ] && note "Reports/README.md: $base merged but still listed as in flight"
  done < <(sed -n '/^## In flight/,$p' Reports/README.md \
    | grep -oE '`[a-z0-9-]+\.md`' | tr -d '`' | sort -u)
else
  note "Reports/README.md missing"
fi

# --- 5. Every examples/ binary must have a row in the instruments index -----
# `Reports/instruments.md` exists because measurement harnesses were being
# rebuilt: a lane needs a number, cannot tell from the names that a harness
# already answers it, and writes a second one. An index only prevents that
# while it is complete, and an index nobody is forced to update decays into
# the exact state it was written to fix -- so this is a check rather than a
# convention. Same shape as the report-index check above.
if [ -f Reports/instruments.md ]; then
  for f in examples/*.rs; do
    [ -e "$f" ] || continue
    base=$(basename "$f" .rs)
    grep -q "\`$base\`" Reports/instruments.md \
      || note "Reports/instruments.md: examples/$base.rs has no row -- say what it answers, and what it can answer BEYOND the question you built it for"
  done
  # The inverse: a row for a binary that no longer exists sends the next
  # session looking for a harness that was deleted.
  while read -r name; do
    [ -f "examples/$name.rs" ] \
      || note "Reports/instruments.md: lists \`$name\` but examples/$name.rs does not exist"
  done < <(grep -oE '^\| `[a-z0-9_]+`' Reports/instruments.md | tr -d '|` ' | sort -u)
else
  note "Reports/instruments.md missing -- the instruments index is referenced by CLAUDE.md"
fi

# --- result -----------------------------------------------------------------
if [ "$fail" -eq 0 ]; then
  echo "docscheck: clean"
else
  exit 1
fi
