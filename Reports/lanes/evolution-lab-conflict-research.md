# Lane note — conflict research and assessment (round 35, lane D)

*Coordinator: `session_01CwPq5vyYcSBnT7pe2nfAc2`. The owner's ask: "Do some
research on when and why different types of ants, bugs and bigger creatures
fight and how that might relate to our game. Implementation."*

**The report is
[`../animal-conflict-research-2026-09-14.md`](../animal-conflict-research-2026-09-14.md)**
and it is the deliverable; this note is the handover.

## The three things to read first if you read nothing else

1. **A stranger is already food, with no `Attack` weight anywhere.** `ant`
   material carries `food_class: 1.0` and the shipped ant's gut is neutral,
   so the moment two colonies fall outside each other's tolerance each is
   prey to the other's **ordinary mouth**. Measured: total `eats` goes
   **54 → ~750** on a 4,000-frame bed from nothing but turning kin
   recognition on, and cross-colony kills (9) outnumber `Attack` kills (4)
   two to one. **Lane C
   should know this before the switch ships: turning rivalry on does not
   produce a war, it produces predation.**
2. **`scent_spread = 1.0` does not make two colonies strangers; use 2 or
   more.** The acceptance radius is `tolerance + 1` = 1.0 and the founding
   offset is uniform in `±spread`, so at 1.0 only **9.3%** of ordered ant
   pairs read as non-kin and **one seed in four never produces an
   encounter** — which in a single run reads as "the mechanism does not
   work". From `spread=2` every seed meets and everything plateaus. Report
   §8.3 has the six-row table.
3. **`nearest_foe` targets any living non-kin *organism*, and a plant is an
   organism.** An armed ant bites herbs. Pre-existing, invisible because no
   shipped species authors `Attack`, found by the new arena's specificity
   control in its first minute. The `attacks` counter includes those
   closures, so anything reading escalation off `attacks` is wrong — the
   arena prints `fights` and `plantbites` separately for this reason.

## What landed

| file | what |
|---|---|
| `Reports/animal-conflict-research-2026-09-14.md` | the report: literature, mapping, measurements, and §10, what I deliberately did not build |
| `src/sim/contest.rs` | **new.** Assessment before commitment — pure arithmetic, six tests, no world access |
| `src/sim/creature.rs` | **~50 lines, all inside the existing `attack_urge > 0.0` block**, plus `nearest_foe` now returning the local odds, plus **one re-derived bar** in `a_maximally_armoured_ant_is_graded_only_when_the_reach_allows_it` (100 → 300, measured 73 off / 126 on; report §8.6 has the whole derivation and why the guard's own claim is *better* satisfied than before). Lane B's file — see below |
| `src/sim/world.rs` | two counters appended to `CreatureStats`: `contests`, `displays` |
| `src/sim/mod.rs` | the module line |
| `examples/conflict_arena.rs` | **new.** Two colonies in one bed; the first instrument here that can ask what they do to each other |

## The creature.rs diff, for merge sequencing

**It is small and it is in one place.** Everything is inside `act`'s existing
fight branch, which no shipped genome reaches (`attack_urge` is 0 for every
species that ships), plus `nearest_foe` returning an `Encounter` struct
instead of a bare `(i32, i32)` — one caller, changed.

Three properties that should make it cheap to sequence against Lane B and
Lane C:

* **Byte-identical for everything that ships.** The gate is one float
  comparison and nothing below it runs.
* **Byte-identical when not assessing, down to the random stream.** The
  commitment roll sits inside the branch rather than being taken
  unconditionally against 1.0, so `PIXEL_PHYSICS_CONTEST=off` and every bite
  at a plant consume no draw and run the original path exactly. That is what
  makes the A/B an A/B rather than a reshuffle.
* **It sits downstream of whatever raises `attack_urge`**, so Lane C's
  `ThreatNear` wire and this compose rather than collide.

## How to run it

```text
cargo run --release --example conflict_arena -- control=selftest      # both halves of the control, ~1 min
cargo run --release --example conflict_arena -- seeds=6 frames=24000
cargo run --release --example conflict_arena -- seeds=6 assess=off    # the other arm, same binary
cargo run --release --example conflict_arena -- seeds=4 spread=4      # strangers, properly
cargo run --release --example conflict_arena -- numbers=0             # strength only
cargo run --release --example conflict_arena -- boldness=0            # hesitation without information
```

Pin `RAYON_NUM_THREADS` before comparing two runs — every column is a
counter.

Dials, all shipped **on** at the values in the table, all overridable without
a rebuild: `PIXEL_PHYSICS_CONTEST` (on), `..._BOLDNESS` (4.0), `..._NUMBERS`
(1.0), `..._DISPLAY` (40, against 240 for a wound). The arena takes each as
an argument and sets it before any world exists.

**The in-game params row is not mine.** `src/lab/params.rs` is Lane B's this
round; each dial is a plain float behind a `OnceLock` and a row is a one-line
addition whenever that lane wants it.

**One thing to know before reading any dial sweep here:** in a bed of two
*identical* colonies `boldness=` and `numbers=` return byte-identical
summaries, because the strength half of the assessment is exactly zero
between two animals of one species. They are the same knob until the sides
differ, which is what `armour=` is for. Report §8.5.

## Review cards

| id | what |
|---|---|
| **`20260914T052317396Z-d3ddba`** | **the real one.** Frame sequence, 32 frames at 768x448: two colonies meeting at a border, yellow against blue, with the encounter/display/fight counts in `meta` |
| `20260914T052215239Z-aaff61` | **ignore this one.** Same run, posted before I noticed that repeated `--image` makes 42 separate items rather than one scrubbable sequence. There is no retract verb in `review.py`; it is noise in the queue, not a second question |

Read verdicts with `review.py get <id>`, never `inbox` — the coordinator note
records cards missing from `inbox` entirely and a card reading as unanswered
with three marker notes on it.

## What I did not build, and why it is in the report rather than here

§10 of the report is six rejections with reasons. The two that matter most
for whoever picks this up:

* **Interference competition proper — holding a place against a rival — is
  the biggest remaining gap and is not a combat mechanic.** It needs a reason
  to *stand* somewhere with no food in reach, which is a spatial goal the
  brain has no output for. **But the cheaper half of it is not code at all**:
  the literature says territoriality is a function of the *bed* — defence
  pays when food is dense and predictable — and `LabBox`'s eight evenly
  spread founders are the least defendable larder that can be built. Run the
  colony on two rich patches before anyone writes a hold-position behaviour.
* **A new brain input for "a rival is over there" is the obvious next step
  and I argue against doing it first.** The eye is not blind to rivals; it
  reports them through `prey`/`threat`, because those predicates are diet
  questions and a non-nestmate ant is food. Whether a distal *rival* channel
  buys anything is exactly what the arena can now measure, and the
  contest-assessment literature's own meta-analysis finds more support for
  self-assessment than for mutual assessment. Adding an input also widens the
  genome, which re-derives every species' `mutation_rate` and invalidates
  `main` as a control arm — round 33 paid that bill.

## Gates run on this branch

`cargo clippy --all-targets --release --locked -- -D warnings`,
`cargo test --release --lib`, `cargo test --release --test worldgen --test
determinism`, `bash scripts/docscheck.sh`,
`python3 scripts/deadendindex.py --touching`, and
`conflict_arena control=selftest`. Results in the PR body.

**`dead-ends.md` was grepped for the mechanism before building**, per
`CLAUDE.md`: `assess`, `retreat`, `withdraw`, `hawk`, `rival`, `territor`,
`soldier`, `caste`, `swarm` and `Caution` return nothing relevant — the one
`Caution` hit is `PERSIST_MAX`'s entry, which is about the *footing* output
and is the reason this lane did not take the coordinator's suggested route
(see below).

## One correction to the brief

The brief's guess was that the missing thing is `Caution`, *"a shipped,
working, unauthored input"*. **`BrainOutput::Caution` is not an assessment
knob — it is the foothold preference**, scaled by `FOOTING_MAX = 1.2` and
read only where `step` scores the three forward candidates. Authoring it
changes how willing an animal is to walk off a ledge and touches the fight
nowhere. The brief's *instinct* was right and its lever was not, which is why
this lane built the assessment rather than authoring a weight.
