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

## Round twenty-four, 2026-09-09 — consolidation: what the line is, and what is actually left

*Owner's call: stop and bank it. No new mechanism. One pass over the
documents so the next session starts from a current picture.*

**The creature design report now opens with a state-of-the-line table** —
every `§4` subsection against shipped / not built / rewritten — because it was
written in one evening and closed piecemeal over four rounds, and no single
place said what the line now *is*. Read that table before any section under it.

**What is actually open**, in the order it matters:

1. **The generation clock, ~12,000 frames**, against an owner target of 60–70
   generations. With §4f's geometry cap withdrawn and Gate 2 passed, this is
   the only named limit left on evolution in this bed.
2. **The colony cannot reliably reach its food** (round twenty-one), which is
   the thing standing between the box and a colony that looks alive.
3. **§4c `Regrow`** — injury is still a binary, and the owner is unconvinced.
   It needs a case made by picture rather than by argument.
4. A private trail plane per colony (§4d), never built.

**Four stale claims were carrying forward and are corrected.** README
documented `World::colony_rivalry` as a live rule (retired a week ago by the
scent signature); a "known limitation" said the heritable signature was
"designed and not built" **directly above the paragraph describing it
shipped** — the exact failure `CLAUDE.md` warns of, a limitation outliving its
fix; the `Persist`/`Tumble` note called its race unfixably confounded when
`padarm=on` now holds the synapse count equal; and Gate 2's pass was recorded
in no README section at all.

**And `wiki/ants.md` led with a claim about the shipped ant that is wrong.**
"No ant follows a trail — both smells are laid and neither is read" was the
page's freshness note. The trail-following circuit is in `ant.ron`'s **hidden
layer**, not its direct weights, and `eval_brain` runs it: a laden ant walks
up channel A, an empty one up channel B. Corrected there, in
`colony-starvation-separated-2026-09-08.md` (as a scoped note leaving that
lane's numbers and its first clause intact), and in `dead-ends.md`. **The
starvation finding survives it** — what changes is the fix, since the reader
already exists and the candidate is a cold start: channel B is only laid by an
ant already carrying. Flagged, still not measured; `labforage` is the
instrument.

**The transferable bit**: a network with a hidden layer has two places a sense
can be read, and an audit of the authored weight list sees one. Grep the
*sense*, not the weight list, and check the evaluator runs the layer you found
it in.

## The earlier rounds

All twenty-three are verbatim in
[`../evolution-lab-rounds-archive.md`](../evolution-lab-rounds-archive.md),
which prices each one and maps it to its owning report. **Read the one round,
not the file** — they are three concurrent lines braided into one sequence, and
knowing which is yours is most of the saving:

| line | rounds | design of record |
|---|---|---|
| the lab as an instrument — interface, shelf, rosters, persistence, soil, scenarios, forage | 3, 4, 5, 7, 9, 10, 11, 21 | `evolution-lab-gui-physics-2026-08-30.md` |
| frame cost and the speed dial | 2, 6, 8, 17, 18, 19 | `evolution-lab-frame-cost-2026-09-01.md` |
| creatures — groups, kin, armour, castes, verbs, gates | 12, 13, 14, 15, 16, 20, 22, 23 | `creature-signature-and-castes-2026-09-06.md` |

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
