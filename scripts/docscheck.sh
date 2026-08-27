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

# --- --selftest: put each fault back and watch its check go red -------------
# `docscheck` had no sensitivity test until 2026-08-27, and on that day it
# came back CLEAN against a tree carrying four instances of the defect class
# it is named for -- three stale `**Status:**` headers and a report claiming
# to live on a branch that had merged. Green was evidence about the checks,
# not about the documentation.
#
# This is CLAUDE.md's standing rule ("before you cite a guard's green as
# evidence, put the fault it is named for back and watch it go red") built as
# a command rather than left as a discipline, for the reason that file gives:
# as prose the same check costs 1-3k tokens a time and its own injection can
# silently match nothing, which reads as a pass. `scripts/docbench.py
# selftest` is the same device over the documentation benchmark, and found two
# blind controls on its first run.
#
# Restores every file it touches, including on failure. Add a row whenever you
# add a check -- a check with no row here has never been shown able to fire.
if [ "${1:-}" = "--selftest" ]; then
  st_ok=0
  # id | file | exact text to break | what to break it to
  while IFS='|' read -r cid sf needle repl; do
    [ -z "$cid" ] && continue
    # Back up by copy, never by `orig=$(cat ...)`: command substitution strips
    # trailing newlines, so a restore through it silently rewrites the last
    # line of every file the selftest touches.
    bak=$(mktemp); cp "$sf" "$bak"
    if ! python3 - "$sf" "$needle" "$repl" <<'INNER'
import sys, pathlib
# `\n` in a fault row is a real newline. Needed because the faults that matter
# span the hard-wrapped prose: 3c's stale header is a whole paragraph, and a
# one-line injection left the correcting sentence ("the branch is gone from
# the remote, which in this repo means merged") standing, which kept the check
# quiet. That is a blind INJECTION, not a blind check -- the same trap
# docbench.py's docstring records hitting on its own B3b row.
#
# The presence check lives HERE rather than in a `grep -qF` upstream, because
# upstream still holds the escaped form and would report every multi-line
# fault as missing.
f, needle, repl = (a.replace('\\n', '\n') for a in sys.argv[1:4])
p = pathlib.Path(f); t = p.read_text(encoding='utf-8')
if needle not in t:
    sys.exit(3)
p.write_text(t.replace(needle, repl), encoding='utf-8')
INNER
    then
      echo "docscheck: $cid INJECTION FAILED -- its needle is not in $sf"
      echo "  -> the check may be fine; the fault this test injects has moved."
      rm -f "$bak"; st_ok=1; continue
    fi
    out=$(bash "$0" 2>&1)
    cp "$bak" "$sf"; rm -f "$bak"
    if printf '%s' "$out" | grep -qF "$cid"; then
      echo "docscheck: $cid went red"
    else
      echo "docscheck: $cid STAYED GREEN -- the check is blind, not weak."
      echo "  -> widening its assertion will not help. Replace it."
      st_ok=1
    fi
  done <<'FAULTS'
plant-substrate-v2-design.md|Reports/plant-substrate-v2-design.md|**Status: implemented and merged.**|**Status:** design only. No code in this pass.
branch-angle-and-the-width-bound.md|Reports/branch-angle-and-the-width-bound.md|**Status: merged. Corrected 2026-08-27**, having stood four days after\n`plant-project-review-2026-08-23.md` §3 named it stale with the address\nattached. `branch_angle`, `straightness`, `internode` and `path_len` all\nstand in `src/sim/plant.rs` and `src/sim/organism.rs`; `branch_angle` and\n`internode` are authored in five species files; the branch is gone from the\nremote, which in this repo means merged (branch deletion returns HTTP 403,\nso nothing else prunes one). §4's width bound is closed in code too — the\nturgor gate reads `path_len`.|**Status: built, measured, working, and NOT merged.** It lives on branch\n`plant-branch-angle`.
FAULTS
  [ "$st_ok" -eq 0 ] && echo "docscheck: all faults detected -- 3b and 3c can go red"
  exit "$st_ok"
fi

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

# --- 3b/3c helpers ---------------------------------------------------------
# A report's `**Status:**` block, flattened to one whitespace-normalised line.
# Read as a BLOCK, not a line: the prose here is hard-wrapped, so
# plant-substrate-v2-design.md's "No code in this pass" sits three lines below
# the word `Status` and a line-based grep could never see it. That is the
# false-negative class `scripts/docgrep.py` exists for, in a check written
# before it -- and it is why 3b came back clean on 2026-08-27 against a tree
# carrying three instances of the defect it is named for.
status_block() {
  # Ends at a blank line OR at the next line-initial `**Label:**`. The second
  # terminator is not optional: these headers run several bolded labels
  # together with no blank line between them, and without it
  # plant-substrate-v2-design.md's block swallowed the `**Companion to:**`
  # line that follows -- which reads "(the shipped ...)" and so matched
  # $POSITIVE, silencing the check on the very report it was widened for.
  # Caught by --selftest on its first run, and by nothing else: the check was
  # green, the injection was faithful, and the block was simply too long.
  awk 'tolower($0) ~ /^\*\*status/ && !f {f=1; print; next}
       f && /^[[:space:]]*$/ {exit}
       f && /^\*\*/ {exit}
       f {print}' "$1" \
    | tr '\n' ' ' | tr -s ' '
}

NEGATIVE='not started|not built|nothing built|no code|no implementation|not implemented|not merged|unmerged|lives on branch'
# Two positive vocabularies, because BUILT and INTEGRATED are different axes
# and a single list conflates them. `branch-angle-and-the-width-bound.md` read
# "built, measured, working, and NOT merged" -- true on both counts when
# written -- and one shared list saw the word "built" and fell silent on a
# report whose whole defect was the merge status. This is CLAUDE.md's "when a
# rule must tell apart two things that can look identical, state the
# difference as data", found the expensive way by --selftest.
BUILT='implemented|merged|shipped|landed|built'
INTEGRATED='merged|landed|superseded|corrected'

# Does this status block claim nothing is built -- and NOT also say something
# is? The second half is what lets a corrected report keep its old claim.
#
# **The negatives are stripped before the positives are looked for**, and that
# ordering is the whole mechanism. Both alternatives were tried on 2026-08-27
# and both were wrong: matching positives directly silences a header that says
# "not implemented" (the substring is there), and matching only the header's
# first sentence splits on a house style that has two forms in the tree
# (`**Status: claim**` and `**Status:** claim`, the second with nothing bolded
# to read). Stripping first means "No code in this pass" cancels itself and
# "implemented and merged" in the same block still counts.
#
# The payoff is that a report can quote the claim it is retracting -- which is
# this repo's own convention, since how a record went wrong is usually worth
# more than the correction -- without the quotation re-arming the check.
_claims_negative() {
  local st="$1" positive="$2" rest
  printf '%s' "$st" | grep -qiE "$NEGATIVE" || return 1
  rest=$(printf '%s' "$st" | sed -E "s/($NEGATIVE)//gI")
  printf '%s' "$rest" | grep -qiE "$positive" && return 1
  return 0
}

# "Nothing is built here" -- 3b's question.
claims_nothing_built() { _claims_negative "$1" "$BUILT"; }

# "This has not landed" -- 3c's question, and deliberately NOT the same one.
claims_unintegrated() { _claims_negative "$1" "$INTEGRATED"; }

# --- 3b. A report's own Status must not contradict its index entry ---------
# The index is maintained; the headers drift. Measured 2026-08-26: of the
# seven reports declaring "not built"/"not started"/"nothing built", FOUR were
# contradicted by their own index entry, and the index was right every time --
# load-model-handoff ("not started" vs "superseded by landing"),
# fracture-mechanics-design, physical-trees-design-2026-08-23, felling-blockers.
#
# This is the trap class the benchmark tests for: CLAUDE.md tells you to check
# a report's standing in the index, but an agent that opens the file directly
# reads the header first and takes "not started" as a live work order.
#
# **Vocabulary widened 2026-08-27.** The original list was drawn from the four
# structural reports that prompted it, so it knew "not built" and not the three
# other ways a report in this tree says the same thing: "No code in this pass"
# (plant-substrate-v2-design, the most-cited plant report at 28 source
# citations), "No implementation" (plant-evolution-design), "NOT merged"
# (branch-angle-and-the-width-bound). Adding a phrase to $NEGATIVE is cheap and
# the failure it prevents is expensive, so prefer widening to arguing -- but a
# phrase no report uses is dead weight, and one that fires on a report whose
# index AGREES with it is noise (building-rethink and destruction-plan both say
# "not built" truthfully and must stay silent).
if [ -f Reports/README.md ]; then
  for f in Reports/*.md; do
    base=$(basename "$f")
    case "$base" in README.md) continue ;; esac
    claims_nothing_built "$(status_block "$f")" || continue
    # Scope to THIS entry only: from its "- [base]" line to the next "- [".
    # `grep -A3` was tried first and false-positived on destruction-plan.md,
    # whose own entry says "plan." while the two entries after it mention
    # landings -- the context window bled across the boundary.
    idx=$(awk -v b="[$base]" 'index($0,b){f=1} f&&/^- \[/&&!index($0,b){exit} f' \
          Reports/README.md | tr '\n' ' ')
    if printf '%s' "$idx" | grep -qiE 'landed|superseded|implemented|stage is built|has been built|shipped|merged'; then
      note "$base: its own Status says nothing is built, but the index says it landed -- an agent opening the file directly reads the header, not the index"
    fi
  done
fi

# --- 3c. A report must not claim to live on a branch that has since merged --
# The sibling of 3b, for the case 3b structurally cannot see: when BOTH the
# header and the index are wrong, comparing them finds nothing.
# `branch-angle-and-the-width-bound.md` said "built, measured, working, and NOT
# merged. It lives on branch `plant-branch-angle`" while `branch_angle`,
# `straightness`, `internode` and `path_len` all stood in `plant.rs` and
# `organism.rs` -- and its index entry said only "measured study", so 3b had
# nothing to contradict. It had also been named stale, with the address
# attached, four days earlier by `plant-project-review-2026-08-23.md` §3, and
# was still standing: a defect named in prose does not get fixed, a check does.
#
# Reports get written on branches here (branchcheck reports 12 carrying 132
# commits main lacks), so a report outliving its branch is a recurring class.
# The remote is the oracle: this repo cannot delete branches (HTTP 403,
# CLAUDE.md), so a named branch that is absent has merged and been pruned.
#
# Guarded twice: on the remote being fetched at all -- in a bare or unfetched
# clone every branch looks absent -- and on the report not already saying it
# merged, so a corrected report may quote the claim it retracted.
if [ -n "$(git branch -r 2>/dev/null)" ]; then
  for rf in Reports/*.md; do
    # Whole file, newlines flattened: the branch name wraps onto the next line
    # in the one report that does this, so a line-based grep finds the phrase
    # and an empty name. Same false-negative class as 3b, and it bit this check
    # on its own first run.
    br=$(tr '\n' ' ' < "$rf" | tr -s ' ' \
         | grep -oE 'lives on branch `[^`]+`' | head -1 | sed 's/.*`\(.*\)`/\1/')
    [ -z "$br" ] && continue
    claims_unintegrated "$(status_block "$rf")" || continue
    git branch -r 2>/dev/null | grep -qE "origin/$br\$" && continue
    note "$(basename "$rf"): says it lives on branch '$br', which is not on the remote -- this repo cannot delete branches, so it merged. Re-read the code before trusting the header"
  done
fi

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
