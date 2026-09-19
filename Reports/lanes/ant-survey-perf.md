# Lane P — the field's response to a walking colony

*Ant-survey follow-up round, coordinator `session_01HTNLNphUPgpg5GqCwCQvmW`.
Branch `claude/ant-survey-perf`. Four items, all done; the full account with
every table is [`../ant-field-wake-2026-09-19.md`](../ant-field-wake-2026-09-19.md)
and [`../gpu-field-design-2026-09-19.md`](../gpu-field-design-2026-09-19.md).
This note is what another lane or the coordinator needs and not a summary of
those.*

**Head SHA / PR: see the foot of this note.**

## The four answers, in one line each

1. **§8.2's mechanism is confirmed by line, and it has one correction that
   matters.** Two of the five arrays `rebuild_blocked` derives — `glow` and
   `beam` — *are* read from a creature cell, unconditionally, and answer zero
   only because no creature material sets either. So the switch's predicate is
   keyed on the emission, not on `MaterialKind::Creature`.
2. **The stopwatch is built and gated** (`PIXEL_PHYSICS_PHASE_CLOCK=1`), woven
   through `sim::frame::step` and printed in the chronicle CENSUS row beside
   `awake_chunks`. Bit-identical and no measurable cost when off, against
   `origin/main` over six paired reps.
3. **The wake skip is built and it does not work, for a reason worth knowing.**
   Its bit-identical half ships; its solve-set half is not bit-identical and
   removes zero tiles. See below — this is the finding.
4. **The GPU field's condition is not met and the share was the wrong
   condition.** Design note only.

## What another lane should take from this

**The lab bed's field already solves half the world, so a wake rule has nothing
to remove.** 65.4 of 128 tiles a tick, and 65.4 with the skip on. Every chunk an
ant stands in is inside the one-tile halo of a tile the field was solving for the
plants or the sky anyway. If you are proposing anything that works by *not
waking* a tile on this bed, measure `tiles/f` first — `examples/antcost.rs` now
prints it — because the halo will eat it.

**What the ants really add to the field is CA block rescans, not tile solves**
(45.9 → 63.3 a tick at two hundred ants), and about 85% of those are marks the
ants *earn*: a dig writes `Solid`, a spoil drop writes `Solid`, and the
soil-water pass re-marks disturbed ground. **Lane N**, this is yours if you want
it: the open question is whether that 14.8 is the dig/drop writes or the
soil-water re-marking, and an arm with the digging verb off separates them in one
run. I could not run it — creature behaviour is out of this lane's scope.

**`antcost` grew three columns and a control you can use.** `tiles/f` and
`blocks/f` (the field's response), a field hash per arm, and **`antglow=<v>`**,
which sets the colony material's `glow` on the live registry before founding.
That last one is the general pattern rather than a one-off: it constructs the
creature that breaks a switch's premise, so the guard can be watched going red.

**`lab_cost fielddump=<path>`** writes the raw six channels. Reach for it the
moment two field hashes disagree — the hash says *different*, this says *which
channel and by how much against its settle epsilon*. Nothing in `examples/`
called `field_channels` before.

**A trap that cost me the first hour, and it is general.** The stocking loop
saturates: `ants=300`, `600` and `1000` on the 1024 bed at `rounds=60` produce
**byte-identical world hashes** — one population wearing three labels. Read the
`stocked` column, not `want`. The brief's own command has this in it.

**And `awake/f` at the fastest rep is not stable across arms of the same bed.**
The `off` run reports 27.9 / 31.1 / 27.9 for three identical beds, because
`best_i` picks whichever rep was quietest and each rep is a different frame
window. The review's §8.3 range "27.9–31.1" is that, not three populations.

## The bug I filed, which is not about creatures

**`open-bugs-handoff.md` §Z31.** `field::step`'s carry decision keys on
`Chunk::is_settled()`, on a premise its own comment states and which measurably
does not hold: a settled chunk can hold a live `stale_blocks` mark, and then
inherits its derived arrays from before the write for one frame.
`FIELD_CARRY_STALE=1` is the reproduction and it moves the field hash **on a bed
with zero creatures in it** (`0x77761fab0c35bfce` → `0xd67e172d6d16b8a4`).

It was inside the creature switch's first version, where the same term was
needed for an unrelated reason, and it is split out and default-off because a
creature switch that moves the field on an ant-free bed makes every number about
it a measurement of two changes. Ruled out by ablation, not argument: the
soil-moisture phase's placement, and `mark_moist_dirty`'s own stale mark. The
surviving mechanism is `end_sweep`'s promotion racing a cross-chunk write.

**Anyone touching `field::step`'s carry decision, `Chunk::end_sweep`, or
`parallel.rs`'s remote-write replay: read §Z31 first.**

## Files this lane touched

`src/sim/field.rs` (the switches, the solve-set gate, the carry gate),
`src/sim/chunk.rs` (`set_world` gains `field_relevant`; `mark_dirty` split;
`has_stale_blocks`), `src/sim/material.rs` (`Material::field_inert`,
`MaterialRegistry::field_relevant_write`, two guards),
`src/sim/world.rs` (**one line plus a comment**, in `write_cell`, plus the
`fill_run` and `MoistureView` call sites — deliberately minimal, see below),
`src/sim/parallel.rs` (the same at `ChunkView::set`), `src/sim/frame.rs` (the
stopwatch), `src/lab/census.rs` (the chronicle columns and addendum),
`examples/antcost.rs`, `examples/lab_cost.rs`. No creature behaviour, no
`assets/species/ant.ron` — Lane T and Lane N's files are untouched.

**`branchcheck --who-touched`, quoted from the tool and not from memory**, run
before writing into either contested file:

- **`src/sim/world.rs` — hot.** 39 changes landed on `main` in seven days, the
  most recent `339f0ad8` ("A gallery with an ant in it is still a gallery")
  **six hours** before this branch was cut. That is why the change there is one
  registry call in `write_cell` and nothing else, and why the decision about
  *what* is field-relevant lives in `material.rs` rather than in `world.rs`.
- **`src/sim/field.rs` — quiet.** Two changes in seven days, most recent
  `82429040` ("Water can dim the light field now"), five days before.

Branch cut from `5f92c761`. `origin/main` moved 14 commits during the lane and
was merged in before landing.

## What I did not do, and why

- **Not the creature parallel switch** — the owner is testing it on his own
  machine (the brief says so outright).
- **Not flipping `FIELD_CREATURE_WAKE=blocked` on by default.** It is free and
  bit-identical, and it recovers 2.6 of 17.4 block rescans, which does not show
  above run-to-run spread at 52 ants. The number that would justify the default
  is the one only the owner can take: the stopwatch on his own colony, with
  `blocked` set and unset.
- **Not tuning the solve-set gate to beat the halo.** The halo is one ring and
  `step_advection`'s clamp is what makes one ring sufficient; narrowing it is how
  a tile comes to sample a neighbour the frame never populated.
- **Not fixing §Z31.** It wants a decision more than code, and it is not this
  switch's to make.

## For the coordinator

Gates before push: `cargo clippy --all-targets --release --locked -- -D warnings`
clean (one lint fixed, `mem_replace_with_default`); `cargo test` green
(1,870 lib + the `tests/*.rs` binaries, 0 failed); `bash scripts/docscheck.sh`
clean; `python3 scripts/bugindex.py` regenerated after §Z31.

**The one thing to carry into the next brief**: the review's §8.3 row does not
reproduce at its stated size on this box — the field goes **0.081 → 0.117 ms**
for 52 ants (1.44x), not 0.090 → 0.188 (2.09x), same command and bed. The
direction is right and §8's argument survives; the multiplier should not be
quoted again without a re-measurement. And the field is **20–21% of the tick**
on the shipped lab bed at 52 ants, with `ca_sweep` at 56–60% — so the phase
worth three times as much attention is the sweep, which is where the parallel
switch the owner is testing sits.
