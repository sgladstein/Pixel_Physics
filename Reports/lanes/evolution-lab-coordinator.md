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
by a line that does not outlive a session. Every creature result in archived
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

## Round twenty-one, 2026-09-08 — the bed has teeth, and the larder was the wrong suspect

*Started from "the colony starves, so nothing can be selected for" — round
ten's open problem and the standing Gate 2 caveat. **Both halves of that
sentence turned out to be wrong**, and the way they were wrong is the
transferable part.*

**Gate 2 passes for creatures.** `creature_arena arm=lethal` — a zeroed brain
against the shipped one — 24,000 frames, six seeds, run on the harness default
bed **and** a fed one: the zeroed brain takes **0.0% of animals on 12 of 12
seed-runs**, and the harness prints *the bed has teeth*. The standing caveat in
"What binds on anything you do here" is discharged above. It is a maximal-effect
test and licenses only what it measures: the bed can turn a large fitness
difference into a population difference. It cannot resolve a small one, and
that — not the ecology — is why the flight races nulled.

**And a single seed nearly shipped a false headline, in a session that had
just written the rule down.** One run at `founders=8` read **52 ants → 1** at
48,000 frames and 52 → 54 fed; that was drafted as "the harness bed goes
extinct". Five seeds:

| plants | alive at 48k | births | generations reached |
|---|---|---|---|
| 8 | 1, 4, **18**, 19, **87** | 651–1687, median 1519 | 4, 5, **5**, 7, 8 |
| 48 | 48, 50, **54**, 64, 96 | 2063–2855, median **2570** | 3, 6, **7**, 7, 9 |

The `alive=1` was the worst of five and one starved seed reached **87 — above
two of the fed seeds**. What survives the sweep is **births**, where every fed
seed beats every starved seed (5 of 5, ~1.7x), and standing population at ~3x
the median. What does *not* survive is the generation-depth claim: the ranges
overlap and one fed seed reached only 3.

**Two effects were being confused, and separating them is the finding.**

- **At a fixed horizon, food does raise generation depth.** The arena at
  24,000 frames: median depth **2 starved against 4 fed**, and fed ≥ starved
  on 6 of 6 seeds. Colony size 15 → 74 median in the same runs.
- **Over a long enough horizon the food effect is swamped.** At 48,000 frames
  both beds reach 4–9 generations and the two overlap. **The horizon dominates
  depth; food dominates population.**

So the practical rule for anyone measuring evolution here: **12,000 frames is
about one generation** and is the horizon nearly every result on this line was
read at, including the flight null and every armour number. Selection needs
generations; if the question is evolutionary, 24,000 is a floor and 48,000 is
where depth stops being the binding constraint.

**What was NOT changed, deliberately.** `LabBox::default()` stocks 52 ants on
8 plants and every harness but `labnest` (0) and `selection_arena` (16) rides
it. It is tempting to raise, and the measurement does not support doing it:
the default bed **passes Gate 2 as it stands**, so there is no correctness
argument, and moving it would void comparability with every number in these
notes for a benefit that is instrument convenience. `bin/lab.rs` opens at
`founders: 0, colonies: 0` regardless — the player stocks the box — so this
was never a statement about the shipped game. Round ten already wrote the
rule this round re-learned from the other end: **an instrument's default
scene is an input like any other.** Pass `founders=` explicitly and say what
you passed.

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
