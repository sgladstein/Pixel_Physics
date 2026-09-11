# The evolution lab — coordinator note

*`cargo run --release --bin lab` is the second game: a sealed box of soil under
grow lights where the shipped plants and ants live. Design of record:
[`../evolution-lab-design-guide-2026-08-30.md`](../evolution-lab-design-guide-2026-08-30.md),
with [`../evolution-lab-feasibility-2026-08-30.md`](../evolution-lab-feasibility-2026-08-30.md)
under it.*

**Read this before picking the lab up.** Twenty-six rounds have been run here
since 2026-08-30. One to twenty-five are history and moved (2026-09-08, 2026-09-09, 2026-09-10) to
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

## Round twenty-five, 2026-09-09 — the instrument turned toward the player

*Verbatim in [`../evolution-lab-rounds-archive.md`](../evolution-lab-rounds-archive.md);
design of record
[`../evolution-lab-direction-2026-09-09.md`](../evolution-lab-direction-2026-09-09.md).
What still binds:*

**Owner's rulings:** trophallaxis is a brain output the genome can evolve,
shipped on, never a rule; **rest is the absence of a reason to act, not the
presence of a full stomach** (the bias comes down and never off; digging is
conditioned on a crowded nest); a queen is three authored values over
mechanisms that exist, **never a type the engine knows**, and the
eusociality lane's first deliverable is the generation-clock measurement;
staggered founder reserves are approved; the marker overlay was rejected on
sight; **movement, not stills, is how animals are seen** — a visibility
claim is judged on a moving sequence, never a contact sheet.

## Round twenty-six, 2026-09-10 — the box gets its first relationship

*Record: [`../evolution-lab-round-26-2026-09-10.md`](../evolution-lab-round-26-2026-09-10.md);
design of record
[`../evolution-lab-ecology-design-2026-09-10.md`](../evolution-lab-ecology-design-2026-09-10.md).
Pointer and what binds.*

**The direction.** Close the loop fruit → animal → nest → seedling: **the
colony that gardens survives.**

**What binds.** The breeding trade is **graded** (commit `e5792206`); ship
graded as the default only after `GRADED_MAX_SUPPRESSION` (a provisional
6.0) is swept. **A question that needs no visual is asked in chat, not the
queue.**

## Round twenty-seven, 2026-09-10 — the flower gets a customer it cannot yet reach

*Record: [`../evolution-lab-round-27-2026-09-10.md`](../evolution-lab-round-27-2026-09-10.md);
design of record
[`../evolution-lab-pollinator-design-2026-09-10.md`](../evolution-lab-pollinator-design-2026-09-10.md).
Pointer and what binds.*

**Landed on `main`:** #306 (the bed with thicket and tree), #307 (the
pollinator design; the species is `flitter`), #309 (HISTORY per colony),
#310 (rain LIGHT, control kept), #312 (nectar), #313 (the seed rides home),
#314 (the bloom sense, no species wired), #317 (re-bloom; shrub flowers). **The bodies landed as one PR, #320 (2026-09-11), on the
owner's ruling *"Go with A, but the long ant should be an option that I can
place"*:** the tuck, the founding walk and the flip ship for every body, the
shipped ant stays two cells, and the seven-cell body is `longant`, a species
placed from the COLONY chip or named in a scenario
(`played_bed_longant.ron`). The shipped colony forages better for it (bed,
60,000 frames: deliveries 9 → 20 on fewer moves). #303/#311/#315/#316 are
superseded. Left for the dig-and-bite lane: the long body's founding and
its whole-body bite; the moisture-gradient scene's pickups fell 416 → 78
and were not chased.

**What binds.** **Ants are not the pollinators** — nothing wires the ant to
the bloom sense, and the flitter (P2) comes before the scent plane (P1b),
because a ground animal with eyes and a pull tops out one row under the
flower (21 against 22). **The eye is heritable** (`TRAIT_SIGHT_RANGE`, cap
8); whether selection would pay for it against nectar is a control
measurement inside the flitter lane, not a lane. **Every mechanism of the
garden loop is in and its rate is zero** (bites in single digits per
120,000 frames; no pip has ever become a plant): the next lever is where
windfall lands and why a pip never germinates, not another mechanism. The
bodies' two reds — the §9 swarm guard and the chamber's `roofed > 0` — are
the owner's ruling: green them and land the stack, or park it and keep the
two-cell ant. Plant counts run lower under re-bloom in all six pairs: watch
it. **Conserve tokens** (owner, 2026-09-10): spawn a lane only for a build
the owner asked for or a landing needs; a finished lane resumes by
`SendMessage` with its context intact. `labforage`'s SUMMARY line is
contested by every lane — keep `main`'s fields, append yours, `cargo check`
before pushing.

**The round's cards are answered (2026-09-11, in the record):** the flip
is confirmed on sight; **bigger flowers with more variety** is a new ask
(C2's petal-colour locus and a size lever); a one-cell event cannot be
judged on a card even at 5x — mark the cell or follow it.

**Round twenty-eight, proposed and not yet ruled:** the flitter (P2, with
the eye-price control inside it); the garden loop's rate (where windfall
lands against where ants walk; why a dropped pip never germinates); the
bodies ruling; sugar water as the first player verb; the wider list is in
the record.

## The earlier rounds

Rounds one to twenty-five are verbatim in
[`../evolution-lab-rounds-archive.md`](../evolution-lab-rounds-archive.md),
which prices each one and maps it to its owning report; twenty-six and
twenty-seven have their own records, linked above. **Read the one round,
not the file** — they are three concurrent lines braided into one sequence, and
knowing which is yours is most of the saving:

| line | rounds | design of record |
|---|---|---|
| the lab as an instrument — interface, shelf, rosters, persistence, soil, scenarios, forage | 3, 4, 5, 7, 9, 10, 11, 21, 25 | `evolution-lab-gui-physics-2026-08-30.md` |
| frame cost and the speed dial | 2, 6, 8, 17, 18, 19 | `evolution-lab-frame-cost-2026-09-01.md` |
| creatures — groups, kin, armour, castes, verbs, gates | 12, 13, 14, 15, 16, 20, 22, 23, 24, 25 | `creature-signature-and-castes-2026-09-06.md` |
| the ecology — fruit, seed, nectar, flowers, the pollinator | 26, 27 | `evolution-lab-ecology-design-2026-09-10.md`, `evolution-lab-pollinator-design-2026-09-10.md` |

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
