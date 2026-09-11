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
[`../evolution-lab-pollinator-design-2026-09-10.md`](../evolution-lab-pollinator-design-2026-09-10.md).*

**What binds.** Ants are not the pollinators; the eye is heritable
(`TRAIT_SIGHT_RANGE`); a ground animal with eyes tops out one row under the
flower (21 against 22). Plant counts run lower under re-bloom. **Conserve
tokens** (owner): a lane only for a build the owner asked for or a landing
needs. `labforage`'s SUMMARY line is contested by every lane — keep `main`'s
fields, append yours, `cargo check` before pushing. The landed list (#306,
#307, #309, #310, #312, #313, #314, #317) and the bodies' landing (#320, the
two-cell ant stays default, `longant` is placeable) are in the records.

## Round twenty-eight, 2026-09-11 — the pollinator arrives and eats the garden

*Record: [`../evolution-lab-round-28-2026-09-11.md`](../evolution-lab-round-28-2026-09-11.md).
Pointer and what binds.*

**Landed on `main`:** #318, #319, #320, #323, #324, #325 — petal colour
inherited, windfall's dead zone measured, the bodies, the dig verb no longer
shovelling pips, the nectar-only flitter, the pip's clock re-armed (the
first two plants ever from a pip). Open at the close: #322 (heads at 12 with
a heritable spread) and the midden; each is in the record.

**What binds.** **The flitter must float like a bee, not hop like a frog**,
and **flowers must be easier to find** (owner, on the cards): it reaches 106
rows and still cannot find a one-cell flower, so sustained, steerable flight
toward a bloom is round twenty-nine's first build — on top of `nectar_only`,
which stays, because a plant specialist's mouth eats the plant and no gut
setting avoids it. **A card of a one-cell event is unreadable even ringed
and zoomed** (three pip cards, three "cannot tell"): show the stand, the
door or the colony over a long span, or let the playtest judge. **Bigger
heads read, 12 as well as 16, and 9 is out** (decoded through `blind_was`).
The garden loop's last blocker is water at the nest patch (452 of 454
checks fail on it); the dig and the schedule are fixed. **Sonnet refuses a
brief dense in genetics vocabulary on a `[bio]` classifier** — write it in
the world's words or run the lane on Opus. **A resumed session loses its
in-process lanes** (`SendMessage` stops resolving): salvage the worktree
with a WIP commit and start a lane from it. Never `TaskOutput` a running
lane — it returns the transcript.

## The earlier rounds

Rounds one to twenty-five are verbatim in
[`../evolution-lab-rounds-archive.md`](../evolution-lab-rounds-archive.md),
which prices each one and maps it to its owning report; twenty-six to
twenty-eight have their own records, linked above. **Read the one round,
not the file** — they are three concurrent lines braided into one sequence, and
knowing which is yours is most of the saving:

| line | rounds | design of record |
|---|---|---|
| the lab as an instrument — interface, shelf, rosters, persistence, soil, scenarios, forage | 3, 4, 5, 7, 9, 10, 11, 21, 25 | `evolution-lab-gui-physics-2026-08-30.md` |
| frame cost and the speed dial | 2, 6, 8, 17, 18, 19 | `evolution-lab-frame-cost-2026-09-01.md` |
| creatures — groups, kin, armour, castes, verbs, gates | 12, 13, 14, 15, 16, 20, 22, 23, 24, 25 | `creature-signature-and-castes-2026-09-06.md` |
| the ecology — fruit, seed, nectar, flowers, the pollinator | 26, 27, 28 | `evolution-lab-ecology-design-2026-09-10.md`, `evolution-lab-pollinator-design-2026-09-10.md` |

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
