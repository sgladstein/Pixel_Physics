#!/usr/bin/env bash
# Does every generation pass still put something in the world, and is any of
# them eating another's cells?
#
# **This exists because the instrument that answers both already existed and
# nothing ran it.** `examples/pass_ablation.rs` measured `brows` deleting
# 100% of boulders on 2026-08-20 (`Reports/pass-interference-2026-08.md`,
# finding R4-1). It was written up, and the feature was still being deleted
# nine days later -- an entire wiki-documented mechanic, absent from every
# world, with every test green and every pass counter reporting exactly what
# a failed noise draw would report. That is the same shape as
# `scripts/acceptance.sh`'s own origin: `scene=capped` was recorded as
# passing while its structure had never been load-evaluated, because the
# check was a thing a session remembered rather than a thing that ran.
#
# Two claims, both from the ablation matrix and neither visible in a pass's
# own counter:
#
#   1. no pass APPEARS when another is switched off -- the R4-1 signature,
#      a feature at zero whose cells someone else had already taken;
#   2. every pass writes cells on at least one preset.
#
# Magnitudes are deliberately **not** gated. Several suppressions are the
# generator working: `soil_blanket` feeds `residuals` the socket it digs,
# `ponds` refuses a spring basin that would merge with standing water and
# make the world's pools un-level. A bar on those would be a bar on correct
# behaviour, and this file would then be the permanently-red gate that
# `acceptance.sh` has a paragraph about.
#
# **At the shipped world size, always.** `pass_ablation` builds at
# `app::WORLD_WIDTH` x `app::WORLD_HEIGHT`; at `filmstrip`'s 512x320 the
# answer is different and wrong -- `boulders`, `vaults` and `springs` cannot
# fire there at all, so a check run small would report the defect it is named
# for as normal.
#
# Not wired into CI as a blocking job by default: it is minutes, not seconds,
# because every row of the matrix is a whole world. Run it after touching
# `src/worldgen/`, the way `seedsweep.sh` is run before changing a model over
# procedural content.
#
#   bash scripts/worldgencheck.sh              # 2 seeds, every preset
#   SEEDS=6 bash scripts/worldgencheck.sh      # the order statistic the report quotes
#   bash scripts/worldgencheck.sh --selftest   # put R4-1 back and watch it go red

set -uo pipefail
cd "$(dirname "$0")/.."

SEEDS="${SEEDS:-2}"
BIN=target/release/examples/pass_ablation

# **Build first, and read cargo's status rather than the pipe's.** Every
# measurement in this repo comes out of an example, `cargo build --release`
# does not build examples, and a stale binary prints plausible numbers with a
# newer mtime than the source it disagrees with (CLAUDE.md).
if ! cargo build --release --quiet --example pass_ablation; then
  echo "worldgencheck: the harness did not build" >&2
  exit 2
fi

# --- --selftest: put the fault back and watch the check go red -------------
#
# CLAUDE.md's standing rule, as a command. Green is this check's default
# state -- it is a loose assertion over emergent behaviour on procedural
# content, which is exactly the case the rule names -- so citing its green
# proves nothing until the fault it is named for has been seen to turn it
# red. `PIXEL_PHYSICS_BROW_YIELD=0` restores the pre-fix `brows` exactly:
# lips stop yielding to boulder sockets and R4-1 comes back.
#
# One seed and one preset, because the fault is deterministic once it is
# back: `canyon` ships `brow_chance: 0.9`, so a lip hangs at almost every
# qualifying edge and every socket in the world is under one.
if [ "${1:-}" = "--selftest" ]; then
  echo "worldgencheck --selftest: R4-1 restored (PIXEL_PHYSICS_BROW_YIELD=0); the check MUST report it"
  faulted=$(PIXEL_PHYSICS_BROW_YIELD=0 "$BIN" gate=1 seeds=1 preset=canyon 2>&1)
  echo "$faulted" | grep -E '^(gate:|  FAIL|  note)'
  # **Match the R4-1 line, not merely a non-zero exit.** A single preset at a
  # single seed can trip the *other* half of the gate for an unrelated reason
  # -- `canyon` seed 1 stands no pond, so "ponds wrote nothing on any preset"
  # fires when the run is narrowed to it -- and a selftest satisfied by that
  # would report the check as sighted while never having seen the defect it
  # is named for. That is a blind INJECTION rather than a blind check, which
  # is the trap `scripts/docscheck.sh --selftest` and `docbench.py` both
  # record hitting.
  if ! echo "$faulted" | grep -q 'FAIL  canyon: without brows, boulders APPEARS'; then
    echo "worldgencheck: SELFTEST FAILED -- R4-1 was put back and the check did not name it, so it is blind"
    exit 1
  fi
  # And the clean arm must not report it, or the check is stuck on.
  clean=$("$BIN" gate=1 seeds=1 preset=canyon 2>&1)
  if echo "$clean" | grep -q 'without brows, boulders APPEARS'; then
    echo "worldgencheck: SELFTEST FAILED -- the fix is not in this binary; R4-1 is still live"
    exit 1
  fi
  echo "worldgencheck: selftest ok -- red with the defect back, quiet without it"
  exit 0
fi

echo "worldgencheck: $SEEDS seeds, every preset, at the shipped world size"
out=$("$BIN" gate=1 seeds="$SEEDS" 2>&1)
status=$?
# The matrix itself, verbatim: the gate says pass or fail and the matrix says
# what changed, and a run that prints only the verdict is a run nobody learns
# anything from.
echo "$out"
if [ "$status" -ne 0 ]; then
  echo
  echo "worldgencheck: FAILED -- a pass is deleting another pass's output, or has stopped firing."
  echo "  Read the row above: 'without X: Y APPEARS' means X took every cell Y wanted."
  exit 1
fi
echo
echo "worldgencheck: clean"
