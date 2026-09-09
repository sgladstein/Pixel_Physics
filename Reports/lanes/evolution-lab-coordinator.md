# The evolution lab — coordinator note

*`cargo run --release --bin lab` is the second game: a sealed box of soil under
grow lights where the shipped plants and ants live. Design of record:
[`../evolution-lab-design-guide-2026-08-30.md`](../evolution-lab-design-guide-2026-08-30.md),
with [`../evolution-lab-feasibility-2026-08-30.md`](../evolution-lab-feasibility-2026-08-30.md)
under it.*

**Read this before picking the lab up.** Twenty-five rounds have been run here
since 2026-08-30. One to twenty-four are history and moved (2026-09-08, 2026-09-09) to
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
is the cheap default from here on, not the expensive exception. **And run it
on the played bed** (`ants_at=`, round twenty-five): the frame-0 bed is the
harness talking.

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

## Round twenty-five, 2026-09-09 — the instrument turned toward the player, and two standing claims overturned

*The design of record from here is
[`../evolution-lab-direction-2026-09-09.md`](../evolution-lab-direction-2026-09-09.md);
this round is its pointer. Five lanes landed with it: line names and a
chronicle bounded per lineage, the clock reacting to line events, a hand in
the box (`I` scent, `J` alarm, `Q` fling, `U` lamp), the hunger sense wired
at last with the dig drive gated on a crowded nest, the jump given a species,
trophallaxis as a brain output with staggered founder reserves, and
`chronicle`, the run log as text.*

**Owner's rulings, all the same day, all binding here:** trophallaxis is a
brain output the genome can evolve, shipped on, never a rule; **rest is the
absence of a reason to act, not the presence of a full stomach** — the ant
must not become an animal that only acts when hungry, so the bias comes
down and never off, and digging is conditioned on a crowded nest; a queen
is built as three authored values over mechanisms that exist (a founding
rule, a founder who rests because she is full, sterile workers through the
caste channel), **never as a type the engine knows**, and the eusociality
lane's first deliverable is the generation-clock measurement, not a
feature; staggered founder reserves are approved; the marker overlay was
rejected on sight; and **movement, not stills, is how animals are seen** —
a visibility claim is judged on a moving sequence, never a contact sheet.

**Two claims this note carried are overturned by measurement.** *"Every
shipped bed starves its ant colony"* was the frame-0 harness bed: founded
at frame 6,000 on grown plants (`labshot`/`labforage ants_at=`), the same
colony holds 31 through 30,000 frames where the frame-0 bed holds 4 — take
every earlier §Z6 figure as the frame-0 bed, on eight herbs. And round
twenty-four's *"the trail circuit is live end to end"* is true and the
circuit is inert: the laden gate parks its unit at 30 on the squash curve,
where the `±6` trail term moves the chance of a step by about ±0.003 — a
laid trail moved a colony's near-target count by exactly zero, twice
(§Z7). The cold-start hypothesis sits upstream of a reader that cannot
read.

**Open, in order:** the trail-gate race (§Z7 carries the bar); the hunger
and sharing wires on the *played* bed at six seeds or more with the
composed Move row (`KinNeed` now exists) — on three seeds at 6,000 frames
the sharing arm ended with half the survivors of the arm without, inside
an eight-fold seed spread, and sharing undoes the stagger it shares down;
the hopper's jump rate (a bias of 2.0 is a hop on two ticks in three, and
it kills the animal in a session); then appearance and the eusociality
measurement.

**Environment, learned this round:** a sub-agent that "waits for a build
notification" has ended its turn and will wait for ever — a message
resumes it with its context intact; and six worktrees' `target/` filled
the disk — delete a merged lane's `target/` the moment it lands.

## The earlier rounds

All twenty-four are verbatim in
[`../evolution-lab-rounds-archive.md`](../evolution-lab-rounds-archive.md),
which prices each one and maps it to its owning report. **Read the one round,
not the file** — they are three concurrent lines braided into one sequence, and
knowing which is yours is most of the saving:

| line | rounds | design of record |
|---|---|---|
| the lab as an instrument — interface, shelf, rosters, persistence, soil, scenarios, forage | 3, 4, 5, 7, 9, 10, 11, 21 | `evolution-lab-gui-physics-2026-08-30.md` |
| frame cost and the speed dial | 2, 6, 8, 17, 18, 19 | `evolution-lab-frame-cost-2026-09-01.md` |
| creatures — groups, kin, armour, castes, verbs, gates | 12, 13, 14, 15, 16, 20, 22, 23, 24 | `creature-signature-and-castes-2026-09-06.md` |

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
