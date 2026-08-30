# The evolution lab — coordinator note

*Brief, owner, 2026-08-30: build the first part of the new game direction —
the biosphere the shipped plants and ants run in, with the speed-up
function, and stats on screen. Design of record:
`Reports/evolution-lab-design-guide-2026-08-30.md` (PR #158, unmerged).
Branch `claude/evolution-lab-game-interface-9vf0fl`, PR #166.*

**Read this before picking the lab up.** It says who owns which file, what
the owner has already decided, and what is deliberately not being built yet.

## Round one, compressed

Superseded, but two things still bind. `sim::frame::step` is the tick
sequence shared by both binaries; its guard
`frame_step_matches_the_sequence_app_update_ran_before_extraction` holds a
hash taken from the other side of the extraction, so **if it goes red either
a phase moved deliberately (re-take the number and say what moved) or a phase
was added to one binary's loop and not to `frame::step`** — the failure the
module exists to prevent. And **round one's file-ownership table is no longer
in force**: PR #170 landed all of it and later rounds have edited every one of
those files. Re-derive ownership from the open PR list, never from here. The
first bed's census is in
[`../evolution-lab-gate-1-2026-08-30.md`](../evolution-lab-gate-1-2026-08-30.md).

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

## Round three, 2026-08-30 evening — read this first

### The owner's standing direction, which reframes the programme

> *"Your goals are not tweaking and optimizing evolution now. **Give me the
> tools, data, access to the parameters that need to be tweaked and I do that
> testing myself in the game. That is the game.** If I have access to food,
> water, can cull, can create plants, and creatures, I can figure it out."*

And: *"You don't need to tweak these things. The world starts with nothing,
but the user can add plants, creatures, water, food, soil, etc."*

**So: stop balancing, start exposing.** A default that looks wrong is
something to **register and report**, never to tune.

### Landed

**#176** the lab becomes drivable — two-row mouse bar (look, plant, colony,
cull, soil, water, species, brush, overlays), `SPACE` genuinely stops the
world, the box opens empty, a graded cull, hover readout, thicker dirt with
the cement slab gone, no nest stripe. **#174** ants breed — the blocker was
satiety, not the birth economy. **#178** `labnest`, the playtest report
reproduced.

### Three findings — detail in [`../evolution-lab-gui-physics-2026-08-30.md`](../evolution-lab-gui-physics-2026-08-30.md) §6

1. **Roots steer by air humidity, not soil water**, found by the owner from a
   screenshot. Hydrotropism reads the coarse *field* channel, which **does
   not diffuse inside solid ground** — so below the surface it has no
   gradient, which is why roots stop at 13 rows. Germination gates on it too.
   **The fifth coarse-field occurrence.** Diffusion is double-buffered and is
   **not** at fault — do not re-investigate. Lane:
   `claude/roots-drink-soil-not-air`.
2. **The tunnels were never collapsing** — `overcap` is 0 in every frame of
   both arms, so my one-unit-margin hypothesis is measured wrong. Real:
   **ants 52 -> 12 between frames 4,000 and 5,000**, digging stopping with
   the colony. That run had no food; **re-run now #174 has landed**.
3. **The dirt-depth regression does not reproduce**, and the light steer I
   gave that lane was wrong — bench light is flat at 0.447 across 48 runs and
   the figure that started the hunt is *downstream* of the stand. Deep soil
   is earned by **the ants**: roots max at 13 rows, galleries already at 35
   of a 40-row bed.

### Open PRs — land one at a time, merging `main` into each first

#175 and #180 both touch `field.rs`; #177 and #179 both touch `creature.rs`.
`wiki/ants.md`'s freshness note has now conflicted on **three** branches;
expect it, resolve keep-both.

| PR | what |
|---|---|
| #177 | The ants **do** tunnel — one connected gallery, unlit rather than undug. **See the block below** |
| #179 | `cell_scale` reaches the living. **Approved by eye** |
| #180 | Lamps light the bed; moving one moves what grows. **Approved by eye** |
| #181 | `FIELD_SCALE` 8 -> 16. **Approved by eye** |
| #175 | Half the draw, picture unchanged |
| #182 | This handoff |

### Verdicts from the 21:30 batch — one BLOCKS a PR

- **#177's lighting fix is REJECTED.** He chose **the shipped lighting**,
  *"the current light model is much better"*. **The finding stands and the
  fix does not.** Land the measurement and counters alone, or put the
  lighting behind a switch that is off. Making a standing gallery legible
  needs a different answer.
- **Parameters panel: rated 5, *"this is a great next step"***. Priority.
- **Double cell density confirmed twice** — he chose 1024x640 with light per
  16, and rated the same-physical-size card **5, *"Yes"***. **#179 and #181
  together are the direction**, not an experiment.
- **The 36-cell creature is not there yet, and the critique is shape.**
  *"Shape. It is a perfect cube. Are there perfect cube creatures in our
  world?"* and *"both are smudges but A is closer"*. **More cells did not by
  itself produce an animal** — `plant-appearance-design.md`'s lesson arriving
  on the creature line. Read it before trying again.

### Two gaps the owner will hit

**There is no save** — a parameter he changes and cannot keep is a toy. And
**species parameters are not in `tunables.rs` at all**: `dig_force`,
`hunger_fraction`, `gut_bias`, `reproduce_threshold`, `mutation_rate` are
compiled in via `include_str!` and unreachable at runtime. That is the
largest single gap between the lab and *"I can figure it out myself"*.

## Deliberately not being built yet

The score and the economy — the guide's Gate 5. **Gate 2, does selection have
teeth in *this* bed, has still never been run**, and `selection_arena`'s whole
finding is that a null there is a statement about the world rather than about
the genome. Until it passes, every evolution result measured in this bed is
unvalidated.

## Environment notes that cost time here

- **The container suspends between tool calls**, so a backgrounded job makes
  no progress while the session is idle. `cargo test --lib` does not fit in
  one foreground call. **Let CI be the gate on the full suite** — it runs on
  branch pushes as well as pull requests.
- **Every push cancels the in-flight suite and restarts a ~19-minute clock**,
  so batch commits. A run whose jobs all read "cancelled" two seconds in is
  the concurrency group superseding a push run with a pull_request run, not a
  failure.
- `rust-toolchain.toml` pins 1.98 and CI has a build cache, so plain
  `cargo clippy --all-targets --release --locked -- -D warnings` matches CI.
- **The lab window captures with no display** —
  `PIXEL_PHYSICS_SCREENSHOT_AFTER_FRAMES=N` under `xvfb-run` with lavapipe.
  `labshot` renders the *world* and shows no interface; `examples/labui.rs`
  renders the bar headlessly and scripts clicks.
- **`/tmp` is shared between agents in this container** and the screenshot
  hook writes `$TMPDIR/pixel_physics_lab.png` — one lane captured another
  lane's frame. **Set a private `TMPDIR`.**
- **`review.py inbox --mark-seen` marked all 199 cards seen**, not just the
  caller's. Use `get <id>` rather than trusting an empty inbox.
- **Several agents in one container makes every timing untrustworthy** — two
  byte-identical `ascii` runs have disagreed 2.42x here. Prefer counters, but
  a counter is only load-independent at **fixed parallelism** (610 digs idle,
  278 loaded, same binary): pin `RAYON_NUM_THREADS` or compare arms inside
  one run. Now in `CLAUDE.md`.
- **Cloud lanes cannot be messaged.** `SendMessage` does not resolve a
  `create_session` child, so a brief cannot be narrowed once it is running.
  Write briefs that degrade well: say what to land first and to report the
  rest.
