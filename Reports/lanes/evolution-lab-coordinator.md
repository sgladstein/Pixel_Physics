# The evolution lab — coordinator note

*Brief, owner, 2026-08-30: build the first part of the new game direction —
the biosphere the shipped plants and ants run in, with the speed-up
function, and stats on screen. Design of record:
`Reports/evolution-lab-design-guide-2026-08-30.md` (PR #158, unmerged).
Branch `claude/evolution-lab-game-interface-9vf0fl`, PR #166.*

**Read this before picking the lab up.** It says who owns which file, what
the owner has already decided, and what is deliberately not being built yet.

## Round one, compressed — the skeleton and who owned what

Superseded; kept as one paragraph because the *reason* still binds.
`sim::frame::step` is the tick sequence moved out of `App::update` so both
games call one copy, and the thing it prevents is a second copy of a phase
order whose every line records an ordering constraint. Its guard
`frame_step_matches_the_sequence_app_update_ran_before_extraction` holds a
hash taken on `origin/main` **before** the extraction — a number from the
other side of the change. **If it goes red, either a phase moved
deliberately (re-take the number and say what moved) or a phase was added to
one binary's loop and not to `frame::step`**, which is the failure the module
exists to prevent.

The round-one file split (`time.rs` / `scene.rs` / `stats.rs` per lane,
`mod.rs` and `bin/lab.rs` coordinator-only) is **no longer in force** — PR
#170 landed all of it and later rounds have edited every one of those files.
Re-derive ownership from the open PR list, not from here.

The first bed's census and the fruiting-crop finding are in
[`../evolution-lab-gate-1-2026-08-30.md`](../evolution-lab-gate-1-2026-08-30.md).
Its headline correction is worth carrying: #162's *"no fruit or flower cell
stands in any sampled world"* is a statement about **worldgen** worlds and
**does not transfer to a hand-built herb bed** — a flower does stand in the
lab bed.

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
- **The lab window can be captured with no display.**
  `PIXEL_PHYSICS_SCREENSHOT_AFTER_FRAMES=N` under `xvfb-run` with lavapipe
  (`VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json`). `labshot`
  renders the *world*; only this renders the interface, which is most of what
  the binary is. `examples/labui.rs` renders the bar headlessly and can
  script clicks.
- **`/tmp` is shared between agents in this container** and the screenshot
  hook writes `$TMPDIR/pixel_physics_lab.png` — one lane captured another
  lane's frame. **Set a private `TMPDIR`.**
- **`review.py inbox --mark-seen` marked all 199 cards seen**, not just the
  caller's. Use `get <id>` rather than trusting an empty inbox.
- **Several agents in one container makes every timing untrustworthy** — two
  byte-identical `ascii` runs have disagreed 2.42x here. Prefer counters, and
  note that a counter is only load-independent at **fixed parallelism**:
  the same baseline binary gave 610 digs idle and 278 loaded. Pin
  `RAYON_NUM_THREADS`, or compare two arms inside one run.
- **Cloud lanes cannot be messaged.** `SendMessage` does not resolve a
  `create_session` child, so a brief cannot be narrowed once it is running.
  Write briefs that degrade well: say what to land first and to report the
  rest.
