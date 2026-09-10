# The evolution lab — coordinator note

*`cargo run --release --bin lab` is the second game: a sealed box of soil under
grow lights where the shipped plants and ants live. Design of record:
[`../evolution-lab-design-guide-2026-08-30.md`](../evolution-lab-design-guide-2026-08-30.md),
with [`../evolution-lab-feasibility-2026-08-30.md`](../evolution-lab-feasibility-2026-08-30.md)
under it.*

**Read this before picking the lab up.** Twenty-six rounds have been run here
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

## Round twenty-six, 2026-09-10 — the box gets its first relationship

*Coordinator `session_01NGdywxc1ACg3L5scK7xBTc` (tags `evolution-lab`,
`coordinator`). State at open: phase one landed (round twenty-five); PR #293
measured the breeding clock (queen-only is a thirteen-fold collapse and is
off the table; individual against graded is the owner's taste call, §6.1 of
that report); articulated creature bodies are in flight on
`claude/creature-evolution-engine-67lhjp` (Opus, running — it owns
`creature.rs`, `organism.rs`, `render.rs`, `ant.ron` and `hopper.ron` until it
lands); the eusociality branch holds one unlanded scenario. Both were poked
with the handover the recorded way and both fires landed (`cse_<lane>`).*

**The direction, in one paragraph.** Phase one made the box legible; what it
still lacks is a *relationship*. One organism eating another is the entire
ecology, a flower is colour with no function to any animal, and the seed
inside a carried fruit dies on the trip. The owner's own long-horizon
complaint — *over a long enough time everything dies out* — has an
ecological answer nobody has built: close the loop **fruit → animal → nest →
seedling** and the colony becomes the plants' distribution network and the
plants the colony's renewable larder. **The colony that gardens survives.**
It is visible at play zoom (a ring of herbs around a nest, flowers turning
to fruit after visits, fruit carried home in a line, a bed whose plant map
records where the colony foraged), it is graded by construction (a bonus to
setting seed, never a requirement — `dead-ends.md`'s seed-limited trap), and
it is the patchy larder that §Z7's recruitment finding said no bed had.
This is the owner's own first thought (*flowers and fruits and interactions
between plants and creatures*) with the player's reason for it attached.

**Lanes this round** — separate worktrees, file-disjoint, the coordinator
merges on CI green and runs `docscheck` after each:

| lane | delivers | owns |
|---|---|---|
| measure (Sonnet) | `scenario=` on `windfall_probe` and `chronicle`, both echoing their parameters — this coordinator ran `chronicle scenario=played_bed` and got the frame-0 bed with no warning, the silent-ignore gotcha; then the fruit loop censused on the played bed, three seeds × 120,000 frames: set, dropped, on the floor, eaten / carried / rotted, and where fruit-borne seedlings stand relative to the nest | `examples/windfall_probe.rs`, `chronicle.rs`, world-side counters only |
| design (Opus) | the ecology design of record: gut/midden dispersal, nectar and pollination as a graded bonus, pollen as gene flow (an owner ruling, posted as a card with the BRUSH hand version), palatability co-evolution, a niche by height; build order with the creature-side hooks specified as one call each and placed last | `Reports/` |
| legends (Sonnet) | the chronicle written out on reset and quit (the direction report's first big swing), and a HISTORY page of the lines that ended — the graveyard the owner rated 5, given a place | `ui.rs` (new panel), `mod.rs` reset path, `world.rs` log region |
| rain (Sonnet) | measure whether the played bed dries over a session, then `RAIN OFF / LIGHT / STEADY / HEAVY` from the lid through the water tool's own placement, default set from the measurement, a counter beside it (Arc B3, owner-asked) | `mod.rs` water tool, `ui.rs` BOX rows, a scenario `Setting`, one `labstats` line |

**Ruled since round twenty-five, and where the ruling lives:** the breeding
trade is **graded suppression** — the owner answered it in chat, and it is
recorded as commit `e5792206` on the eusociality branch (PR opened by this
round), not as a card. **That is a rule, not an accident: the owner's
standing instruction is *"if it does not require a visual, just ask
questions here"*** — a text question goes to the owner in chat, and the
queue is for things judged by eye. Two conditions travel with the ruling
before graded ships as the default: the breeder lookup must scale (today it
scans every organism in the world, plants included, once per tick for every
animal that can afford a child — `crowded_bench.ron` is the pessimal case
built to price it), and `GRADED_MAX_SUPPRESSION` is a provisional 6.0 that
nothing has swept. The extra-breeder anomaly (#293) has one live
hypothesis: a non-finite `bank`/`reachable` passing `bank + reachable <
f32::INFINITY`; the instrument is a finiteness `debug_assert` at the
precheck and a print at the `children` increment under `queen`.

**Landed or opened so far, and what each overturned** (kept current as the
round runs):

- **design → PR #296**, `evolution-lab-ecology-design-2026-09-10.md`, filed
  under *Creatures and ecology* by the index's own rule. Three measurements
  that change the build: **the fruit pipeline is budget-limited, not
  animal-limited** — a ripe fruit went unfilled 18,867 times in 40,000 frames
  against 56 that dropped, and the refusals double with no colony, so a
  pollination bonus goes on the ripening *price*, never the clock;
  **nothing has ever eaten a flower** — the best mouthful in 40,000 frames
  was a 960 J fruit, so the 1,440 calibrates a printed ceiling and a comment
  and nectar is cheap; and **`labshot scenario= seed=` silently ignores the
  seed**, so every played-bed contact sheet to date is seed 1 (the same
  defect `labforage` fixed on itself; three lines). Build order **A0 → A1 →
  A2 → B1 → B2 → C on the ruling → D → E**, plant side first, every
  creature-side change one call at a named line (`creature.rs:4954` the
  bite, `:5001` the clear, `:5137` the drop) after the bodies branch lands.
  Its recommendation on pollen: **the player's BRUSH first**, animal-carried
  pollen only on an owner reversal, because it costs the
  cluster-in-genotype-space definition that is the lab's only operational
  test for plant speciation. Cards `…6dfed9` (pollen) and `…cba50c`
  (dispersal form) are the two rulings, asked in chat as well.
- **measure → PR #297**, `Reports/lanes/evolution-lab-ecology-measure.md`.
  `windfall_probe` and `chronicle` now take `scenario=` and echo it. On the
  played bed, three seeds × 120,000 frames: **32 windfalls produced, standing
  stock zero at every 30k checkpoint** (a fallen fruit lasts 354–518 frames),
  fates eaten-or-carried 19 / rotted 14 / unclear 17 (the positive control
  `handout=1000` moved eaten-or-carried 2 → 81, so the instrument reads),
  and **2 of 3,089 germinations came from windfall**. The loop the round is
  named for is not closing through fruit, and the first-order reason is not
  reach (89% / 7% / 17% of standing organs within ground reach by seed — a
  property of the draw, not a bottleneck). **The finding under it: windfall
  cells arrive on the floor already ownerless** (`organism_id = 0`, credited
  to neither producer, 7 and 15 unexplained departures on seeds 1 and 3),
  and an ownerless windfall can never germinate because `germinate()` is
  reached only through organism-scheduled dispatch. Eight causes ruled out
  by direct check; the line was not found. Reproduction lives behind
  `WF_DEBUG=1` in `windfall_probe` (frame 1,184, cell (456,158)). **This sits
  under the whole arc**, so it got its own lane at once.
- **A1** (Sonnet, running): the `labshot` seed fix as its first commit, then
  `pip.ron` at 40 J (the shipped gut cannot see it: 40 × 0.25 < 12), `plant::
  seed_survives_bite` at `seed_gut_survival: 0.6`, four counters, the one-line
  hook as its own final commit for re-placing after the bodies branch, the
  paired played-bed sweep. Told mid-flight to count bites on ownerless
  windfalls separately so a null is attributable.
- **W** (Sonnet, running): the ownership bug — instrument every windfall
  write site, follow the cell from the repro frame, fix at the line, a guard
  that fails unfixed, a bug section with the letter from `bugindex.py`.

- **rain → PR #298**, `src/lab/rain.rs`. Measured first: the played bed loses
  **5.7% / 4.1%** of its soil water over 120,000 frames on two seeds, against
  a bare-soil control that lost exactly nothing — so `RAIN` ships **OFF**,
  with a live `LIGHT / STEADY / HEAVY` ladder (50 / 150 / 400 cells per 1,000
  frames) on key `8` (every letter was bound), placed through the WATER
  tool's own call, jittered from the world's RNG, counted by
  `World::rain_cells`. A "water still in the air" instrument read 30 with
  rain off — the lid's own condensation — and was pulled for that reason.
  Card `…f2fb5b` (STEADY against OFF, a GIF) asks whether the rate reads.
  Also built: `examples/labgif.rs`, the lab's missing headless GIF capture.
- **Landed to main this round so far:** #296 (design), #297 (measure), #295 (breeder index and the recycled-slot fix), #298 (rain) — merged in that order 04:12–04:55, `docscheck` clean after each.
- **The eusociality lane, un-poked, kept going and found a shipped bug**
  (four commits on PR #295, CI running): an organism id is
  `(generation << 12) | slot`, and both breeder-scan loops iterated bare slot
  indices, so **the breeding rule was blind to every animal in a recycled
  slot** — which is the whole of #293's extra-breeder anomaly (a queen in a
  reused slot was invisible, so a second animal bred). Found by running the
  old scan and its replacement, a **per-colony breeder index (33x fewer
  organisms visited at 12,000 frames on the crowded bench, 11.7x at
  40,000)**, as two arms of one binary over 40,000 frames: identical for
  26,100 frames, then one birth apart. Fixed; all six queen seeds now report
  exactly one breeder; the thirteen-fold collapse is unchanged and now tight.
  PR #295 is therefore code, not docs; it landed before the bodies branch, which must merge `main` before anything else happens to it.
- **The articulated bodies are built and they do not walk** — the creature
  lane's own §7, pushed and then the session went idle at ~$145. Ant 5
  segments / 7 cells, hopper 7 / 8, expressed from four heritable rules
  each; **blocked on 43.9% of moves on `flat` and 96.8% on `rolling`**
  (hopper 74.6% / 91.9%) against a plain `Chain(6)` control at 2.5% / 12.4%;
  `ascii` red ("the colony has gone sessile"); 4 of 52 ants founded on
  `scene=colony`. One real deadlock fixed (the lateral sat on the segment
  ahead after any upward step) and it moved the number barely; three
  hypotheses recorded as moving nothing. The clue: the hopper has one
  lateral and is blocked more than the ant's two — **spine length**, not
  laterals, is the suspect. Also fixed on that branch: `corpse.ron`'s
  `food_energy` 480 → 120, a 3.5x energy creation on the burnt-corpse path
  the re-pricing exposed. **Not landable; no PR.** Its §7d names the one
  next step — an env switch that places a `Segmented` body's laterals or
  not, inside one binary — and **lane B ran it (PR #299, stacked on their
  branch): the laterals are the whole cause and the segmented spine is
  exonerated.** Same binary, `PIXEL_PHYSICS_BODY_LATERALS=0`: ant **43.9% →
  2.0%** flat, **96.8% → 14.8%** rolling (the `Chain(6)` control 2.5% /
  12.4%); hopper 74.6% → 5.5%, 91.9% → 40.7% with a residual that is not
  plain length either (`Chain(7)` 13.7%). A `Segmented` body of length 6
  with no laterals is **byte-identical** to `Chain(6)` on every counter, so
  `segmented_body_after_step` degenerates to `chain_follow` exactly. The fix
  is a design decision — what a lateral does when its cell is not placeable
  (`lateral_for`, `landing_is_placeable_through_tissue`) — so the creature
  session was poked with a bounded brief: a lateral may never block a move
  the bare spine could make; it takes the other side or **tucks** when there
  is no room (a body that squeezes, the ethos' middle); re-run the table,
  `ascii` green, the moving card, then merge `main` and open the PR.
  The ingestion question is answered in its lane note: **two clear sites**
  (the mouthful into the crop, `creature.rs:5001` on main; brood
  provisioning, which clears and credits in one block) and the drop site
  where a carried load becomes a whole cell again (`:5137`) — A1 was told to
  hook both clears.

**Put to the owner this round** — in chat, per the rule above, with the
priced readings on cards as a second copy: pollen as gene flow (the animals
carry it, the player's BRUSH carries it, or both with the animals off); the
seed inside a taken fruit — left on the spot, or carried home and put down
where a laden ant puts things down (a ring of herbs around the nest, at about
three times the work); the rain rate and default; the HISTORY page. Still
unanswered from round twenty-five: the tree in the bed (the owner's mix was
grass, herb and shrub, so a *no* closes it), the marks three-way, and *is the
box empty*.

**Inherited from the phase-one coordinator, kept or dropped here:** the
food-trail half of §Z7 (a decay sweep via `labforage bdecay=`);
`scent_spread > 0` as the condition under which castes are adaptive; `CROSS`
on the shelf; outdoor-to-lab collection. None is started this round.

**Outside the box, offered and not started** — each is a question for the
owner before it is a lane: **BRUSH**, pollinate by hand — the shelf's `CROSS`
verb realised physically in the bed, on the same mechanism the animals would
use, so the breeding fantasy arrives through the ecology; **sound** — the
ethos lists *no sound* among what leaves an event unfinished, the lab has no
audio dependency, and it needs a ruling on the dependency before anything
else; **a nectar-feeding hopper** that lives up the stems — a niche by height
(every food is worth the same at every height today), the first animal that
is not a ground ant in body, verb and food, after articulated bodies land;
**palatability co-evolution** — expensive dark leaves defended, the colony's
gut evolving against the bed's defences, two kingdoms evolving against each
other in colour on screen; **the bed as a record** — do not sweep it between
experiments; **wild collection** (Arc C3).

**Environment, learned this round:** a poke's fire response names where it
landed — `cse_<lane id>` both times here, so the session-programs correction
of 2026-09-09 holds. **A trigger's prompt bound to another session cannot be
edited afterwards** (`update_trigger` refuses), so every new message is a new
trigger, and the old one is deleted the moment it has fired so a stale prompt
cannot be re-fired. And a lane whose status reads *"awaiting … results"* is
usually in the sub-agent trap the note above names — it woke, resumed its
build lane, and went idle again waiting for a turn that will never end.

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
