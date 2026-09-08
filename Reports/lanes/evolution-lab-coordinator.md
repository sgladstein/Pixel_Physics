# The evolution lab — coordinator note

*`cargo run --release --bin lab` is the second game: a sealed box of soil under
grow lights where the shipped plants and ants live. Design of record:
[`../evolution-lab-design-guide-2026-08-30.md`](../evolution-lab-design-guide-2026-08-30.md),
with [`../evolution-lab-feasibility-2026-08-30.md`](../evolution-lab-feasibility-2026-08-30.md)
under it.*

**Read this before picking the lab up.** Twenty rounds have been run here since
2026-08-30. One to nineteen are history and moved on 2026-09-08 to
[`../evolution-lab-rounds-archive.md`](../evolution-lab-rounds-archive.md) —
verbatim, with a priced contents table and a per-round map of which round belongs
to which of the three concurrent lines. What stays here is what still binds.

## What binds on anything you do here

**The owner's standing direction, which reframes the whole programme** (round
three, and it has not been superseded):

> *"Your goals are not tweaking and optimizing evolution now. **Give me the
> tools, data, access to the parameters that need to be tweaked and I do that
> testing myself in the game. That is the game.** If I have access to food,
> water, can cull, can create plants, and creatures, I can figure it out."*

**So: stop balancing, start exposing.** A default that looks wrong is something
to **register and report**, never to tune.

**Ship new behaviours as default** (round twenty). Reach is not behaviour:
nothing born swinging or listening is `CLAUDE.md`'s second law failing quietly.

**Read the bed at a session, not at a minute.** The owner's own framing — a
session is a few hundred thousand frames and a million is several sessions. At
that length **every shipped bed starves its ant colony**, on every seed, with
births plentiful: [`../open-bugs-handoff.md` §Z6](../open-bugs-handoff.md)
carries the table and the bar a fix has to clear. **§Z6 is upstream of the
castes question, the kin drift and Gate 2 alike** — a channel cannot be *found*
by a line that does not outlive a session; **round twenty-one answers its open
question.** Every creature result in archived
rounds twelve to eighteen was taken at a few minutes of play at most and is
true at that length only — **the owner's machine is the ruler, not this
container's**: a full box runs at 1–4x there and one plant at 40x, so 24,000
frames of the full box is two to seven minutes and 300,000 is a long session
(the container's 6x is where "one minute" came from, and it is wrong for the
owner). A session census costs six to eight minutes a bed on one core, so it
is the cheap default from here on, not the expensive exception.

**Deliberately not being built yet:** the score and the economy, the guide's
Gate 5. **Gate 2 — does selection have teeth in *this* bed — passes for
creatures**, and this paragraph carried the opposite claim for weeks:
`creature_arena arm=lethal`, a zeroed brain against the shipped one, 24,000
frames, six seeds, on the harness default bed **and** a fed one, puts the
zeroed brain at **0.0% of animals on 12 of 12 seed-runs** — the harness prints
its own verdict, *the bed has teeth*. It is a maximal-effect test and licenses
only that: the bed turns a *large* fitness difference into a population
difference. It cannot resolve a small one, and the arena's own 2.42–3.12x seed
noise — not the ecology — is why the flight races nulled, so those nulls are a
statistical-power problem. **Gate 2 for plants is untouched**, a different
harness on a different kingdom, and `selection_arena`'s finding stands there:
a null is a statement about the world rather than about the genome.

**Re-derive file ownership from the open PR list, never from a table in a
note.** Round one's ownership table was landed whole by PR #170 and every later
round edited those files; `CLAUDE.md`'s own rule says the roster you were
handed is a claim about the past. `sim::frame::step` is the tick sequence
shared by both binaries, and its guard
`frame_step_matches_the_sequence_app_update_ran_before_extraction` holds a hash
taken from the other side of the extraction — **if it goes red either a phase
moved deliberately (re-take the number and say what moved) or a phase was added
to one binary's loop and not to `frame::step`**, which is the failure the module
exists to prevent.

**The perf line's handed-forward list** (archived round nineteen, and
[`../evolution-lab-frame-cost-2026-09-01.md`](../evolution-lab-frame-cost-2026-09-01.md)
§18.5, in order): the **~21% in the kernel and rayon**, the largest block left
by a wide margin; then the moisture pass, where what remains is per-cell
arithmetic; then the pheromone `roundf`, which is cheap and is **not**
behaviour-free. `step_organisms`' three pure levers were priced at under 1%
between them and are closed. **Rebuild the baseline binary after every merge** —
a hash gate is worthless against a stale one.

## Round twenty-one, 2026-09-08 - Z6 separated: the colony dies twice

*PR #284; record in
[`../colony-starvation-separated-2026-09-08.md`](../colony-starvation-separated-2026-09-08.md);
Z6 stays OPEN, bar unchanged, nothing tuned.*

**Stages, not alternatives**: 0-4,500 is *reach* (41-46 of 52 founders starve
with 4-6x their endowment standing), then *grazing* (survivors eat the bed to
4-14% of `colonies=0`), so **a fix for either alone buys a later extinction**.
**Z6's "the plants are not the casualty" is wrong**: paired against
`colonies=0` the stand is at 61-68% of the unfed bed at frame 900, before an
ant has died. **The mechanism is one absence** -- no
`FoodNear`/`FoodBearing` in `brain.rs`, no ant weight on a pheromone plane, so
the trail is laid and never followed. Traps: `forage_probe` at 300,000
frames is identical to its 24,000 run, and *aloft* is not *out of reach*:
an unfed bed reads 80-86% aloft with nothing in it.

## Round twenty-two, 2026-09-08 — Gate 2 passes, and the page could not tell a stand from a colony

*Gate 2's answer, plus a retraction; full account in `dead-ends.md`.*

**Gate 2 passes for creatures.** `creature_arena arm=lethal` — a zeroed brain
against the shipped one, 24,000 frames, six seeds, on the harness default bed
**and** a fed one — puts it at **0.0% of animals on 12 of 12 seed-runs**. The
caveat in "What binds" is discharged above. It licenses only a *large* fitness
difference becoming a population difference, not a small one — which is why
the flight races nulled.

**`labstats`' `EVER` and `BIRTHS` are not animal numbers, and this round
published them as if they were.** `EVER` is `World::deepest_generation`,
written only by `plant.rs:2734`; `BIRTHS` is every organism ever allocated,
sprouted seeds included. Both sit three lines under `ANIMALS BORN`, which
*is* animals. **The control is one command**: the same bed at `colonies=0`,
no animal in it, reports `BIRTHS 311` against 220 with a colony — *higher*,
because the ants graze the stand. The tell was tidiness: 199 to 987 monotone
across `founders` 8 to 64, and `founders` **is** the plant count.

**Re-measured on the animal counters** (48,000 frames, five seeds, merged
head): births median **25 to 101** (4.0x, where the plant column read 1.7x),
alive **17 to 54**, animal depth **4 to 6** (ranges 2-5 against 5-12), every
column separating 5 of 5. So the claim that dies is this round's own headline,
*"the horizon dominates depth; food dominates population"* -- that was plant
depth, which saturates in any bed. **Food raises depth and population both**,
agreeing with round twenty-one from the other side.
Gate 2 is untouched: `creature_arena` maxes generation over ants only.

**Fixed so the page cannot say it again**: `World::deepest_animal_generation`,
written beside `creature_stats.births`; the row reads `EVER p/a`, help string included. `a_bred_colony_deepens_the_animal_counter_and_not_the_plant_one`
was watched red twice — write deleted, then pointed back at the plant counter.
Depth above is still among the **living** and so understates; read the new
counter instead.

**For §Z6's lane, one correction to round twenty-one.** *"No ant weight on a
pheromone plane, so the trail is laid and never followed"* holds for the
**direct** weights only: `ant.ron`'s `hidden_inputs` carry `(PheroAAlong, 0,
6.0)`, `(PheroAAlong, 1, -6.0)`, `(PheroBAlong, 2, 6.0)`, `(PheroBAlong, 3,
-6.0)` into units whose `hidden_outputs` drive `Move` at ±2.5, gated on
`Carrying` — A when laden, B when empty. That is a trail-following circuit,
wired end to end. **Your finding may survive in a stronger form**: B is
emitted only by a *carrying* ant, so a colony that never reaches food never
lays one, and the reader starves for want of a trail rather than a weight --
a cold start, not a missing reader, wanting a different fix. **Not measured
yet**; `labforage` is yours. The missing-sense half of your diagnosis is
untouched: there is no `FoodNear`/`FoodBearing` past the cells a head touches.

**12,000 frames is about one generation** on the starved bed — where nearly
every result on this line was read, the flight null and every armour number
included. If the question is evolutionary, 24,000 is a floor. And
`LabBox::default()`'s 8 plants were **deliberately left alone**: it passes
Gate 2 as it stands and `bin/lab.rs` opens at `founders: 0` anyway (reasoning
in `dead-ends.md`), so round ten's rule stands — **an instrument's default
scene is an input like any other.**

## The earlier rounds

All twenty are verbatim in
[`../evolution-lab-rounds-archive.md`](../evolution-lab-rounds-archive.md),
which prices each one and maps it to its owning report. **Read the one round,
not the file** — they are three concurrent lines braided into one sequence, and
knowing which is yours is most of the saving:

| line | rounds | design of record |
|---|---|---|
| the lab as an instrument — interface, shelf, rosters, persistence, soil, scenarios | 3, 4, 5, 7, 9, 10, 11 | `evolution-lab-gui-physics-2026-08-30.md` |
| frame cost and the speed dial | 2, 6, 8, 17, 18, 19 | `evolution-lab-frame-cost-2026-09-01.md` |
| creatures — groups, kin, armour, castes, verbs | 12, 13, 14, 15, 16, 20 | `creature-signature-and-castes-2026-09-06.md` |

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
  byte-identical `ascii` runs have disagreed 2.42x here. Pin
  `RAYON_NUM_THREADS` or compare arms inside one run; the general rule is in
  `CLAUDE.md`.
- **Cloud lanes cannot be messaged.** `SendMessage` does not resolve a
  `create_session` child, so a brief cannot be narrowed once it is running.
  Write briefs that degrade well: say what to land first and to report the
  rest.
