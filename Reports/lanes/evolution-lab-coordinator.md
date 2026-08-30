# The evolution lab — coordinator note

*Brief, owner, 2026-08-30: build the first part of the new game direction —
the biosphere the shipped plants and ants run in, with the speed-up
function, and stats on screen. Design of record:
`Reports/evolution-lab-design-guide-2026-08-30.md` (PR #158, unmerged).
Branch `claude/evolution-lab-game-interface-9vf0fl`, PR #166.*

**Read this before picking the lab up.** It says who owns which file, what
the owner has already decided, and what is deliberately not being built yet.

## What landed first, and why that order

The skeleton is one commit and the load-bearing part of it is **not** the
binary. `sim::frame::step` is the tick sequence moved out of `App::update`
so both games call one copy. The guide's §7a names the fork risk from the
other end — the lab's speed comes from what is not in the *box*, not from
what is not in the *binary* — so nothing is stripped, and the thing to
prevent is a second copy of a phase order whose every line records an
ordering constraint.

**The move has a positive control, not a self-comparison.** Two copies of a
sequence agreeing proves nothing (`CLAUDE.md`: a superseded mechanism's
tests keep passing while testing nothing), so 120 ticks of a mixed
sand/water/stone scene were hashed on `origin/main`, through the inline
`App::update`, in a separate worktree, before the extraction landed:
`15147976901438684952` both sides. The constant in
`frame_step_matches_the_sequence_app_update_ran_before_extraction` is a
number from the other side of the change. **If it ever goes red, either a
phase moved deliberately — re-take the number and say what moved — or a
phase was added to one binary's loop and not to `frame::step`, which is the
failure the module exists to prevent.**

## File ownership

Nothing in the concurrent round shares a file. This is the discipline
`CLAUDE.md`'s *working alongside another session* section asks for, applied
up front rather than after a collision.

| file | owner |
|---|---|
| `src/lab/time.rs` | the speed dial |
| `src/lab/scene.rs` | the bed, and the lab interior render |
| `src/lab/stats.rs` | the census page |
| `examples/lab_cost.rs`, `examples/labshot.rs` | measurement |
| `src/lab/mod.rs`, `src/bin/lab.rs` | **the coordinator only** — a lane needing a signature changed asks rather than edits |

## Owner decisions taken here, 2026-08-30

- **The speed dial spans the full range with a marked crossover.** Not
  "watchable motion only" and not "skip-ahead only" — both halves are
  wanted, and the readout must say which half you are in. The crossover is
  the point where the display stops showing motion and starts showing
  fast-forward.
- **The lab interior gets built this round**, replacing the sky render. The
  air inside a sealed box currently draws through `sky.rs` — a day gradient
  with a star hash — which the guide flags at **27.4 ns/px against stone's
  6.7**. Wrong twice: it looks like a field at dusk, and it costs 4x per
  pixel for the privilege.

## What the box does, as built

`labshot`, 512x320, 80 rows of soil, 8 herb founders, one colony:

| frame | plant cells | organisms | seeds |
|---|---|---|---|
| 0 | 112 | 60 | 0 |
| 900 | 274 | 59 | 0 |
| 3,600 | 474 | 65 | 12 |
| 10,800 | 774 | 53 | 51 |

The stand lives and reproduces. **The organism count falling is the ants**,
and that is expected rather than broken — see below.

## The finding to carry forward: the lab bed may already be Gate 0's experiment

PR #162 merged 2026-08-30 and prices the ant-breeding deadlock. Three
things in it point straight at this bed:

1. What decides a birth is one number, `ceiling − bar`. Every negative
   margin gives **exactly zero** births across 12 seeds; every positive one
   breeds. The shipped ant sits at **−880**.
2. A matched gut on a **960-point fruit** gives margin **+99** — the same
   margin as the arm that bred on 12 of 12 seeds.
3. **No fruit or flower cell stands in any sampled world at any frame**, and
   not for want of time: the only two fruiting species, `herb` and
   `scrambler`, are never planted by `LIFE_SPECIES`.

**The lab plants `herb`.** So the cheapest route past the deadlock — a
fruiting crop, no creature code — may already be sitting in this bed.

**And it is. Corrected 2026-08-30 by the stats lane, which measured it
rather than inferring it from worldgen.** #162's *"no fruit or flower cell
stands in any sampled world"* is a statement about **worldgen worlds**,
where `LIFE_SPECIES` plants no fruiting species. It does not transfer to a
hand-built herb bed, and the lab's own breeding-margin row says so:

| bed | `ceiling − bar` | what the best standing mouthful is |
|---|---|---|
| ants only | **−880** | 120, the ant's own flesh — reproduces #162 exactly |
| the lab bed | **−640** | **360**, which at a neutral gut implies a 1,440-worth flower standing |

**A flower therefore stands in the lab bed** — real, and unlike any
worldgen world. **What was inferred from it was wrong**, and the round-two
section below records what replaced it. Full account, with the arms and the
controls, in [evolution-lab-gate-1-2026-08-30.md](../evolution-lab-gate-1-2026-08-30.md).

## Round two of the program: what the four lanes settled, 2026-08-30

All four landed. PR #170. What each one *overturned* matters more than what
it built, so that is what is recorded here; the builds are in the lane notes
and in `Reports/evolution-lab-gate-1-2026-08-30.md`.

**Gate 0 is a reach problem, not an economy one — and this is the round's
most important result.** The margin arithmetic above is right: a `-1.0` gut
draws a whole 1,440 flower against a 1,040 bar. Run rather than computed, it
gives margin **+500** and **zero births over 48,000 frames**, richest bank
**568** — the *leaf* ceiling. Flowers and fruit stand **22-40 rows up a
stem**, and `windfall`, the only ground-level form, never exceeds **one
standing cell**. The ants walk the floor beneath food they cannot climb to.
The gut buys survival (3 → 9 ants), never breeding. This is #162's own
caveat coming true — *a fruit has to be found, and the failure case is
foraging rather than economy* — and **the next measurement is a counter on
fruit → windfall, which is small.**

**The frame cost does not follow biomass.** Correlation **+0.03** against
biomass, **+0.92** with the field's solve set, **+0.93** with awake chunks.
The design guide's *"cost follows living biomass"* and its Gate 3 corollary
*"a mature box is the expensive one"* are **backwards in this bed**: the
multiplier **rises** through a session, **9.0x fresh to 17.8x settled**,
because the box quiets as it fills.

**The draw dominates the tick five to one** — 4.78 ms against 0.94 — and
**2.8 ms of that is `sky_light`, running in a box that has a ceiling.**
That is the largest single thing left for the speed dial and it is not in
the simulation at all. **Next optimisation, and it is not where anyone was
looking.**

**Partitions: containment reproduces, the speed-up does not.** `solved/f`
falls 39.4 → 25.1 (−36%) over 1 → 16 compartments, exactly §2c's mechanism,
but the frame goes 1.69 → 1.41 → **1.92 ms**, non-monotone, because the
field is only 54% of a 1.5 ms tick at 512 wide. §2c's 7.6x was measured on a
**fanned 2048-wide** bed. Keep partitions for isolation and scoring; **do
not budget a speed-up at lab scale.**

**The founders were being eaten, and a ceiling was helping.** 8 of 8 plant
and germinate. A colony takes the stand from 55 plants to 19 and then goes
extinct itself (52 founded, 52 dead, 0 born, gone by frame 66,000). And
separately, thickening the ceiling 4 → 7 rows to seat a lamp cost **45% of
the light on the bench and half the stand** — 468 plant cells to 286, 12
seeds to 0 — with **no gate going red**. `field.rs` passes light down a
column as `0.2^(depth/8)`, so shell thickness is a light knob nobody knew
they were turning.

**The grow lamps are not what lights the crop.** `labshot lamps=0` replaces
every fixture with stone and the stand is byte-identical. The glow decays
over a handful of field blocks; the bench is nineteen below. The room
*reads* the schedule, the shell *passes* the light. Recorded rather than
brightened away.

**The dial's crossover is 12 ticks per displayed frame**, so `M* = 12·D/60`:
**12x at 60 Hz, 4x at 20 Hz**. The falling-grain arithmetic overstates it
**~14x** because nothing in a sealed box is in free fall. Under **0.16%** of
the box changes even at the top of the dial.

### What to do next, in the order the evidence supports

1. **The fruit → windfall counter.** Small, and it unblocks the whole
   creature half of the game.
2. **`sky_light` in an enclosed world.** 2.8 ms of a 4.78 ms draw, bought by
   a check the room already knows how to answer.
3. **Gate 2 — does selection have teeth in *this* bed.** Still never run,
   and `selection_arena`'s whole finding is that a null there is a statement
   about the world rather than the genome, so it invalidates every evolution
   result measured in the bed until it is done.
4. **Gate 4's two verbs**, cull and partition, which the premise most
   depends on and which have no engine support.

## Deliberately not being built yet

Verbs (cull, plant-by-hand, partition-by-hand), the score, the economy.
Those are the guide's Gates 4 and 5, and it is right that they cannot be
specified until 0–3 exist to test them against. **Gate 2 — does selection
have teeth in *this* bed — is the next thing that matters**, and it has
never been run: `selection_arena`'s whole finding is that a null there is a
statement about the world rather than about the genome, so a bed that does
not punish a plant known to be worse invalidates every evolution result
measured in it.

## Environment notes that cost time here

- **The container's clippy is 0.1.94; CI runs 0.1.98**, and CI's command is
  `cargo clippy --all-targets --release --locked -- -D warnings` — stricter
  than the bare form. One `use super::*;` in a test module was enough to
  turn the PR red with every other gate green.
- **The lab window can be captured on a box with no display.**
  `PIXEL_PHYSICS_SCREENSHOT_AFTER_FRAMES=N` under `xvfb-run` with lavapipe
  (`VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json`) writes
  `%TEMP%/pixel_physics_lab.png`. `labshot` renders the *world*; only this
  renders the interface drawn over it, which is most of what the binary is.
- **Four agents in one container makes every timing untrustworthy.** Two
  byte-identical `ascii` runs have disagreed 2.42x under load here before
  (`Reports/measurement-under-contention.md`). Later rounds should run as
  separate cloud sessions, one machine each. Measured by the dial lane on
  this very box: the same bed reached **1.7-2.0x at 60 Hz under load 16 and
  3.4-4.0x under load 5**.
- **This container is suspended between tool calls, so a background job
  makes no progress while the session is idle.** Found by the dial lane
  after it cost hours, and it explains a failure this session spent three
  attempts on: `cargo test --lib` completed in 1,420 s when run alongside
  active work and then hit the wall at 1,800 s and 2,400 s when left to run
  while nothing else was happening. **A long job has to run in a foreground
  call, or CI has to be the gate.** CI runs on branch pushes as well as on
  pull requests, so pushing and reading the run is the reliable route --
  and it is the *only* route for the full suite, which does not fit in one
  foreground call at this build's `codegen-units = 1`.
