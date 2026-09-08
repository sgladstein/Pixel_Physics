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
rounds twelve to eighteen was taken at about one minute of play and is true at
that length only. A session census costs six to eight minutes a bed on one
core, so it is the cheap default from here on, not the expensive exception.

**Deliberately not being built yet:** the score and the economy, the guide's
Gate 5. And **Gate 2 — does selection have teeth in *this* bed — still has
never been run as designed**; `selection_arena`'s whole finding is that a null
there is a statement about the world rather than about the genome. Until it
passes, every evolution result measured in this bed is unvalidated. (The
2026-09-02 arena run in archived round two, item 3, is the closest thing to it
and answers only *past the founding grant*.)

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

## Round twenty, 2026-09-08 — the verbs ship on by default

*Owner's ruling: **ship new behaviours as default**. Everything the creature
line built on 2026-09-06 was reach rather than behaviour -- nothing born
swinging or listening -- which is `CLAUDE.md`'s second law failing quietly.
README's "Creature groups status" is the shipped half.*

**Two things came off their compatibility settings, and only one of them is
visible.** `TRAIT_REACH_DEFAULT` 1 -> 8 is a **ceiling**, not a starting
value: every animal is born at allele 0 and `trait_variance` moves a slot 0.15
a birth against ~8,600-frame generations, so a bed at reach 8 and a bed at
reach 1 are the same bed for a long time. Anyone comparing the two settings
and finding nothing has found the truth, not a bug. The visible half is two
authored weights on the ant.

**The finding worth carrying: the alarm was read on the wrong cell, and the
error was invisible in every test.** `BrainInput::Alarm` shipped reading the
cell *ahead* of the animal, because the two trail planes do and it looked like
a convention. Measured on the standard bed, three seeds: reading ahead gave
**8, 38 and 28 attacks**; reading at the animal's own cell gave **296, 258 and
266** — a factor of ten, and `eats` went *up* rather than down. The difference
is facing. **The general shape**: a *route* is read ahead because where it
lies relative to the head is its information; an *event* is read where you
are, because being in one is not a fact about which way you are looking. Every
guard passed at both readings, because a guard that asks "does the alarm fire"
cannot ask "does anything hear it".

**And the sign of `(Alarm, Move)` is the opposite of the obvious one.** Read
here, a positive weight means "move faster while you are in a fight", which is
*leaving* it. Measured against `-1.0`:

| `(Alarm, Move)` | attacks | cells off a beetle | eats | ants eaten |
|---|---|---|---|---|
| unwired | — | — | 2086/2916/2397 | 10/2/0 |
| +1.5 | 296/258/266 | 75/89/69 | 2276/2662/2204 | 9/5/0 |
| **−1.0** | **372/478/361** | **104/142/79** | 1703/2390/1858 | **4/4/2** |

Standing to fight takes ~40% more off a beetle, roughly halves what beetles
take back, and costs 15–22% of foraging. **The populations are inside the
bed's own spread**, so anyone reading this as a census will find nothing: the
trade is in the ledger.

**What it costs the outdoor game, isolated by control rather than argued.**
Nothing bites anything in a one-colony ant scene, so both weights contribute
exactly zero — and are still **billed**, because `synapse_fraction` charges
per active synapse and `eval_brain` counts a synapse active on its *weight's*
magnitude, not on what flows through it. `ascii`'s deposition gate moves
**1.36x -> 1.34x** (pickups 332 -> 309); the same build with the two weights
at `0.0` returns **1.36x on 237 drops from 2,962 laden ants, digit for
digit**. So the whole drift is the tax on two connections that never fire.
That is the price of "ship as default" on a shared species file, it is
measurable, and it is what `synapse_fraction` exists to let selection prune.

**Two harness repairs fell out of trying to photograph this.**

- **`labshot`'s `crop=` had never worked.** The tiles are cut to the crop and
  the sheet is sized from the crop, but the assembly loop walked them at the
  *view's* dimensions — so it panicked on the first row past the crop height,
  for every crop smaller than the bed. The one thing that argument was added
  for (a two-cell animal is invisible in a full-frame tile) was the one thing
  it could not do. Fixed; `zoom=` and `crop=` still do not compose, and
  `look=` is the argument that works with zoom.
- **`creature_arena` can weight-match its arms** (`padarm=on`). An arm wired
  with four named weights paid more `synapse_fraction` than one wired with
  two, every tick — so last night's flight race compared the wiring's *shape*
  confounded with its *size*. Padding is inert-but-taxed weights into a hidden
  unit whose outgoing row is silent, verified bit-identical through
  `eval_brain` rather than assumed.


## The earlier rounds

All nineteen are verbatim in
[`../evolution-lab-rounds-archive.md`](../evolution-lab-rounds-archive.md),
which prices each one and maps it to its owning report. **Read the one round,
not the file** — they are three concurrent lines braided into one sequence, and
knowing which is yours is most of the saving:

| line | rounds | design of record |
|---|---|---|
| the lab as an instrument — interface, shelf, rosters, persistence, soil, scenarios | 3, 4, 5, 7, 9, 10, 11 | `evolution-lab-gui-physics-2026-08-30.md` |
| frame cost and the speed dial | 2, 6, 8, 17, 18, 19 | `evolution-lab-frame-cost-2026-09-01.md` |
| creatures — groups, kin, armour, castes | 12, 13, 14, 15, 16 | `creature-signature-and-castes-2026-09-06.md` |

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
