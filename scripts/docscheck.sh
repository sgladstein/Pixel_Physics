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
  # ...and the same disease in the BODY, which this check did not look at until
  # 2026-08-26. Six instances were standing across four pages -- "New this
  # build", "until this build", "as of this build" -- each of which reads as
  # current for ever. One of them had already gone wrong in the way that is
  # hardest to notice: weather.md's "until this build" meant 2026-08-23, and
  # re-dating the page for an UNRELATED edit silently repointed it at 2026-08-26.
  # A phrase whose meaning depends on the freshness note is broken by any later
  # edit to that note, so the anchor has to be written in.
  # Deliberately narrow: only "this build". "currently" and "recently" have
  # legitimate uses in this prose ("ground where nothing is currently growing"),
  # and a check that fires on correct content stops being read.
  if grep -qi 'this build' "$f"; then
    note "$f: 'this build' in body prose -- name the date instead; it cannot go stale"
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

# --- 6. The bug register: status index current, identifiers unique ----------
# `CLAUDE.md` tells every session to read the register before touching a listed
# area. It is ~86k tokens across 93 entries, append-only, and a bug's verdict is
# written into its own heading rather than by moving the entry -- so a large
# share of what sits under `## Open` is closed, and a reader cannot tell the
# live half from the archive without reading all of it.
#
# `scripts/bugindex.py` generates the status table at the top of the file from
# those headings, so the question costs a few hundred tokens instead of eighty
# thousand. This check keeps it from decaying into one more index nobody
# updates -- the same reason the instruments index above is a check rather than
# a convention. It also reports duplicate identifiers: references are textual
# ("see §Z"), so a repeated letter resolves to whichever heading the reader
# reaches first. CLAUDE.md records that happening once (two bugs filed as §Q);
# it has happened three more times since, which is the argument for a check.
if [ -f scripts/bugindex.py ]; then
  while read -r line; do
    [ -z "$line" ] && continue
    note "${line#bugindex: }"
  done < <(python3 scripts/bugindex.py --check 2>&1 | grep -v 'index current')
else
  note "scripts/bugindex.py missing -- the bug register's status index and its duplicate-identifier guard are both unenforced"
fi

# --- 7. Every user-facing key must appear in the README Controls table ------
# Same shape as the architecture-map check: the Controls table is the only
# index of what the app can do, and a key missing from it is a feature no
# session can discover -- `Y`, the entry point to the whole ant colony, was
# once missing this way.
#
# **Driven off `App::help_columns`, not off `KeyCode::` arms in `main.rs`.**
# The first version scanned match arms through a hand-written variant list and
# was wrong twice over: the list omitted `Backquote`, `Backslash` and `Quote`
# -- three keys bound today, on the app's own help page, and absent from the
# README -- and scanning arms also sweeps in movement and modifier keys the
# table documents as groups. `help_columns` is the curated user-facing list:
# what the player is shown is exactly what the README owes a row.
#
# The table is delimited by the next `## ` heading, never by naming the
# section that follows it. Anchoring on `## Materials` made the check vacuous
# the moment that heading moved: `sed` printed to EOF, so any key mentioned
# anywhere in README's 2,600 lines counted as documented.
if [ -f src/app.rs ] && [ -f README.md ]; then
  ctl=$(awk '/^## Controls/{f=1; next} f && /^## /{exit} f' README.md)
  if [ -z "$ctl" ]; then
    note "README.md: no '## Controls' section found -- check 8 cannot run"
  else
    for tok in $(sed -n '/fn help_columns/,/fn draw_help/p' src/app.rs \
        | grep -oE 'Key\("[^"]+"' | sed 's/Key("//; s/"$//' | tr ' ' '\n' | sort -u); do
      # Rust source escapes the backslash key as "\\"; the README documents the
      # character itself, so unescape before comparing.
      tok="${tok//\\\\/\\}"
      case "$tok" in
        # Mouse and spelled-out companions to a punctuation key, plus the
        # digit range and movement/modifier groups the table documents as one
        # row rather than per key.
        LMB|RMB|LMB/RMB|TICK|QUOTE|1-9|A|D|W|S) continue ;;
        ESC)   disp="Esc" ;;
        TAB)   disp="Tab" ;;
        SPACE) disp="Space" ;;
        SHIFT) disp="Shift" ;;
        # Rust source escapes the backslash key as "\\"; the README documents
        # the character itself.
        '\\\\') disp='\\' ;;
        *)     disp="$tok" ;;
      esac
      # A literal backtick cannot sit inside single backticks; markdown spells
      # it `` ` ``, so that is what the table is checked for.
      if [ "$disp" = '`' ]; then pat='`` ` ``'; else pat="\`$disp\`"; fi
      printf '%s' "$ctl" | grep -qF -- "$pat" \
        || note "README Controls table: \`$disp\` is on the app's help page (App::help_columns) but has no row"
    done
  fi
else
  note "src/app.rs or README.md missing -- check 8 (keys vs Controls table) did not run"
fi

# --- 8. README's table of contents must be current ------------------------
# README is ~2,600 lines with 33 sections, thirteen of them milestone status
# write-ups in the order they were written rather than numeric order. A
# wholesale reorder was approved and then reversed (documentation-overhaul-plan
# item 11: agents navigate by grep, the reorder is a huge diff on a contested
# file, "a TOC buys the same navigation for 3% of the churn"). The TOC is that
# substitute, and it is only worth having while it is true -- same argument as
# the instruments index and the bug register's status table above.
if [ -f scripts/readmetoc.py ]; then
  while read -r line; do
    [ -z "$line" ] && continue
    note "${line#readmetoc: }"
  done < <(python3 scripts/readmetoc.py --check 2>&1 | grep -v 'contents current')
else
  note "scripts/readmetoc.py missing -- README's table of contents is unenforced"
fi

# 9. Every cross-document address in `Reports/dead-ends.md` still resolves.
#
# The register addresses its entries by document and heading/paragraph name
# rather than by line number -- 266 quoted fragments, 79 of them pointing into
# `README.md` and `PLAN.md`. That makes those headings a load-bearing address
# space for the one document whose whole job is stopping an agent re-walking a
# dead end, and until 2026-08-25 nothing noticed when one was renamed: the
# entry goes on reading correctly and simply stops pointing anywhere.
#
# Safe to gate on because its errors are false *passes* -- it asks whether the
# text is still somewhere in the document, not whether it is still a heading.
# See the script's own docstring for what that misses, and for the two related
# checks that were designed and rejected for failing the opposite way.
if [ -f scripts/addrcheck.py ]; then
  while read -r line; do
    [ -z "$line" ] && continue
    note "${line#addrcheck: }"
  done < <(python3 scripts/addrcheck.py --check 2>&1 | grep -v 'addresses resolve')
else
  note "scripts/addrcheck.py missing -- dead-ends.md's cross-document addresses are unenforced"
fi

# --- result -----------------------------------------------------------------
if [ "$fail" -eq 0 ]; then
  echo "docscheck: clean"
else
  exit 1
fi
