# A live flag is not a live arm

**Measured 2026-09-10, on `claude/gallant-fermat-dqvtm4`.**

`scripts/deadendindex.py` tags **27 entries** `live-flag=`, meaning the entry
cites an env-var ablation switch that is still readable in `src/`. That was
offered as the cheap tier of the triage: an entry whose arm still exists can be
re-run in one command instead of re-implemented.

**The tag counts flags that exist, not arms that fire, and the difference is
the whole value of the tag.**

## The run

`GROUND_ROOT` is the most-cited live flag in the register (3 entries:
`structural:002`, `:003`, `:008`). Paired arms, same binary, same seed:

```
./target/release/examples/filmstrip scene=worldcrack strike=12 preset=rolling \
    seed=7 start=2 every=900 count=5
```

| arm | overloaded | unsupported | crumbled | rock | cells lost |
|---|---|---|---|---|---|
| default | 207 (8,286 cells) | 314 (1,619) | 114 regions / 179 cells | −884 | 491 |
| `GROUND_ROOT=flat` | 207 (8,286 cells) | 314 (1,619) | 114 regions / 179 cells | −884 | 491 |

**Byte-identical**, at 12 s per arm.

## Why that is not a result

The first attempt was worse and is worth recording: `strike=0`, 1,802 frames.
Both arms reported `failures: overloaded 0 (0 cells), unsupported 0 (0 cells)` —
**a comparison between two runs of a world in which nothing happened.** The
scene did not contain the situation under test. `strike=12` at 4,502 frames
fixes that: 207 overload failures and 884 rock lost say the structural system is
working hard.

It is *still* not a result, and the census says why:

```
SCHED_PASS=1 ... | grounded 0 (flat 0) | ...     on every frame
```

`structural.rs:463` only takes the ground root when `relaxed == u16::MAX` and
the cell rests on ground. On this scene that never happens, so the branch
`GROUND_ROOT` switches — `structural.rs:489`, which chooses whether to call
`ground_footing_distance` — is never reached. The flag is correctly wired to a
path the scene does not exercise.

**The register already knew.** `structural:007` records the sibling flag
`STRUCT_NO_GROUND_ROOT=1` as *"byte-identical to the baseline, and a new
`grounded` counter on the `[struct]` census reads 0 on every frame, so it is a
vacuous null rather than a negative result."* This run reproduces that finding
on the other flag, which is the best available evidence that the method works —
and the worst available news for the cost model that motivated it.

## What this changes

**Before re-running any archived arm, run its firing check.** The arm is only
meaningful once some counter on the far side of the switch reads non-zero. Here
that is 12 seconds and one env var (`SCHED_PASS=1`), against a whole session
spent interpreting a null that could not have moved.

This is `CLAUDE.md`'s *"pair every 'it fired' counter with an effect counter
from the far side of the call"* applied to the re-test rather than to the
original measurement, and its *"a size cap must bound work, never gate whether
something happens"* test in a new costume: exhausting the condition produced an
**answer** (byte-identical, therefore no effect) rather than **less work**.

So the triage's cheap tier is smaller than 27, and the correct pipeline for an
archived arm is three steps rather than one:

1. **Does the scene contain the situation?** (a live effect counter in the
   baseline arm — here, 207 failures rather than 0)
2. **Does the switched path execute?** (a counter on the far side of the switch
   — here, `grounded`, which reads 0 and stops the exercise)
3. only then, **do the arms differ?**

Steps 1 and 2 are seconds each. Skipping them is how a byte-identical null gets
written up as a negative result, which is the failure this register exists to
prevent.
