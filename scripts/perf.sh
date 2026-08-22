#!/usr/bin/env bash
# Run a timing harness without measuring the rest of the machine.
#
# Several agents work this box at once, each in its own worktree with its own
# `target/`, and it has **four logical cores** against a simulation that runs
# `parallel::step` across all of them. One other session's `cargo build
# --release` saturates it.
#
# What that costs, measured rather than assumed: two runs of a byte-identical
# `examples/ascii` binary, doing bit-identical deterministic work, reported
#
#     water round a pillar     0.373 ms  ->  0.904 ms   (2.42x)
#     stress, parallel       196.801 ms  -> 122.412 ms  (0.62x)
#     stress + field, para   102.089 ms  -> 146.729 ms  (1.44x)
#     ants, *mean*             3.939 ms  ->   4.152 ms  (1.05x)
#
# Nothing in the simulation changed between them. The first run had the
# parallel stress scene *slower* than the serial one (196 vs 121), which is
# backwards from M5's entire purpose, and the second run reversed it. A
# worst-frame figure taken off a contended box cannot support a claim in
# either direction below about 2.5x.
#
# So this script exists to do the two things `cargo run --example ascii`
# cannot:
#
#   1. **Build outside the lock, run inside it.** `cargo run` fuses the two,
#      which would mean holding a machine-wide lock across a compile -- long
#      enough that everyone would route around it. Compilation is contention
#      like any other, but it is contention nobody has to *measure* through.
#   2. **Run the binary directly**, so cargo's own dependency check does not
#      land inside the measured window.
#
# Usage:
#     scripts/perf.sh                                   # the whole suite: the counter gates
#     scripts/perf.sh ascii scene="sand and water"      # one scene: the timing measurement
#     scripts/perf.sh filmstrip scene=strike count=4
#
# **Prefer a single scene when the question is milliseconds.** `quiet_probe`
# measured this box at 8% quiet with a longest quiet spell of 40 s. A scene is
# 7-11 s and fits; the 143 s suite does not, which is why a full-suite run
# reliably ends UNTRUSTED however long it waits. The suite is for the
# counters, and counters do not care what else is running.
#
# The lock itself lives in `src/perf.rs` and is taken by the *binary*, not by
# this script -- so a plain `cargo run --example ascii` still serialises
# correctly, it just holds the lock across its own build. Set
# PIXEL_PHYSICS_NO_PERF_LOCK=1 to bypass entirely, which is right in CI (the
# runner is alone on its box) and right when the output is being read for
# behaviour rather than for timing.
set -uo pipefail

EXAMPLE="${1:-ascii}"
shift || true

# Windows puts the suffix on; git-bash reports MINGW/MSYS in $OSTYPE.
SUFFIX=""
case "${OSTYPE:-}" in
  msys* | cygwin* | win32) SUFFIX=".exe" ;;
esac
BIN="target/release/examples/${EXAMPLE}${SUFFIX}"

echo "=== building $EXAMPLE (outside the lock -- this is the part that does not need to be quiet)"
if ! cargo build --release --example "$EXAMPLE" --locked; then
  echo "perf: build failed; not running" >&2
  exit 1
fi

if [ ! -x "$BIN" ]; then
  echo "perf: expected $BIN after a successful build, but it is not there" >&2
  exit 1
fi

echo
echo "=== running $EXAMPLE (the binary takes the machine-wide timing lock itself)"
echo "=== if this waits, another session is measuring; that is the feature"
echo
exec "$BIN" "$@"
