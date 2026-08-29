# The instruments — what already exists to measure with

**Status: living index. Current as of 2026-08-26.**

Every measurement in this repo comes out of an `examples/` binary, there are
**31 of them**, and their names do not say what they can answer. This file
exists because instruments were being rebuilt: a lane needs a number, does not
know a harness for it already exists, and writes a second one. The specific
case that prompted this — W3's `divergence` — turned out to answer at least
four questions nobody had asked it, none of them guessable from the name.

**Two rules before you build a new one.** First, grep this page. Second, if you
do build one, add its row here in the same change, and say what it can answer
*beyond* the question you built it for — that sentence is the whole point of
the file.

Two gotchas that apply to every row, both of which have produced whole invalid
studies:

- **`cargo build --release` does not rebuild examples.** Use `--examples`. A
  stale binary runs happily, prints plausible numbers, and has a newer mtime
  than the source you just edited.
- **An unknown argument is silently ignored.** A 3.5-hour study once produced
  eight byte-identical logs because the binary predated the flag it was being
  given. Make a harness echo its own parameters, and treat identical output
  across a change that must have moved something as a stale-binary tell.

## Agents, context and cost

Not an `examples/` binary — these read what an agent run already wrote down.

| instrument | answers | notes |
|---|---|---|
| `scripts/cacheprobe.py` | What the prompt cache actually did in a session: reads, and writes split by **TTL**. Separates the main conversation from sub-agents (`isSidechain`). | Reads `~/.claude/projects/*/<session>.jsonl`, which Claude Code writes for free — **no agent spend**. Beyond the question it was built for it also answers "did this session cache at all", "which TTL is really in use" (measured 2026-08-28: main conversation is **100% `ephemeral_1h`**, 983,153 tokens against 0 at 5m), and "how much did re-reading cost me". Its three questions — namespace, race, TTL — are deliberately reported apart; conflating them is what broke `agent-strategy.md` §6. `--selftest` needs no transcript. Must run on the machine that ran the agents. |
| `scripts/contextbudget.py` | What every session, agent and subagent pays before it starts; `--corpus` ranks the read side. | `--gate` is the ceiling (docscheck check 9b), `--check` the record's staleness (check 9). Token figures are bytes/4.0 and that divisor is **unvalidated** — the gate compares bytes for that reason. |

## Rendering and judging by eye

| instrument | answers | notes |
|---|---|---|
| `filmstrip` | A contact sheet of several frames, or `gif=1` for an animation | The acceptance harness *and* the review-card generator. `channel=` draws per-cell scalars. Reach for `gif=1` whenever the question is whether something *moves* right — a grid of stills cannot answer that, and a GIF has twice got a diagnosis where stills got a rejection. Its per-tile lines carry the **standing organ census** beside the organ event counters, which is the pair a flowering card needs: built says the mechanism fired, standing says there is anything in the picture to see, and a stand once read 1,126 organs built with no fruit on any plant |
| `ascii` | Headless behaviour plus **worst-frame timing** | The number to quote for frame cost. CI runs it |
| `viewshot` | What the *player's viewport* shows of a world larger than itself | The scale question a full-world render cannot answer |
| `uishot` | **What the app's own HUD and panels look like**, and what a held sky or a pinned weather actually does | The only thing here that calls `App::draw` — every other renderer builds frames from `Renderer::draw` and never sees a panel. Drives `App::update`/`App::draw` as `main.rs` does, then sets the state a keypress would have. `sheet=menus\|sky\|weather`; a fresh deterministic world per tile, and per-tile counters (sun, water, gusts and delivered force, freezes, bolts, ice **in the drawn window**) printed under each. That last column exists because FROST measured 3,844 freezes against a picture identical to CLEAR's — the world is 8192x2560 and the frame shows 1/128th of it |
| `pixel_stat` | How noisy a rendered region is, as a number | Compares two strips without squinting |
| `render_cost` | **Where a full-screen redraw spends its time**, broken down | The full branch measured 12.07 ms mean on the shipped 2048x640 world -- 54% of a frame -- and runs on ~100% of frames while the gnome walks, because a camera move invalidates every pixel |
| `frame_profile` | **Which phase a frame's time went to**, timed separately with a distribution | The thing `ascii` cannot answer: it reports a worst frame, which says "does this fit in 16.6 ms" and nothing about *where* it went. Runs the exact phase list `App::update` runs |
| `camera_snap` | Whether the camera moves discontinuously through the path the **app** actually uses | Drives `App::update`/`App::draw` as `main.rs` does, rather than calling `Renderer::follow` directly -- so it catches what a harness calling the API itself cannot |
| `weather_duty` | How often it is raining, swept across seeds and a long window | Built because a single 1,200-frame run measured 89% and that was a sample from inside one wet epoch, not a duty cycle. Generalises to any "is this a duty cycle or one epoch" question |

## Plants

| instrument | answers | notes |
|---|---|---|
| `divergence` | **Does one environmental difference produce two different-shaped plants?** | See below — the most reusable thing here |
| `fate_viability` | **What fraction of mutations to a species' production rule still produce a plant that lives and breeds?** | Gate 1 of the evolvability programme, and it is allowed to answer *no*. Generalises past plants and past fates: it is really *"register N variants of a species at runtime, grow each, and classify the outcome"*, so any question of the form **how much does this substrate tolerate being changed** can be asked by swapping what `mutate` perturbs — a creature genome, a material table. Its real contribution is the **three-way** classification: viable / lethal / **silent**, where silent means the stand came out identical to the base and the mutated field is therefore never read in that scene. A two-way rate counts silent mutations as tolerance and overstates robustness by however many of them there are (measured: 7 of 48 were no-ops from drawing the value already held, and a further set were unread fields). Both controls are mandatory and print on every run — the unmutated table must live, and a shoot-child-to-`Seed` table must die — because a viability rate with neither is exactly the arithmetically-correct-and-about-the-wrong-thing shape. The positive control has already earned its keep once: it reported the *unmutated* table as 0/3 and caught a harness bug that would have published a decisive, entirely false 0%. **`base=tree` (default) or `base=herb`**: the herb base adds `Flower`/`Fruit` to the draw set, and the organ types are offered *only* on a base that declares organ materials and a `Ripen` behaviour — on `tree.ron` a mutant pointing at `Flower` would grow a wood-coloured cell that never ripens, measuring a half-configured species rather than the substrate. Note the negative control finds its rule by `when == Grew` rather than by index, because a determinate base carries an extra rule ahead of it and a control that poisons the wrong rule fails *open* |
| `plant_probe` | Every organism-owned cell's per-cell channels for a grown tree | The quantitative pair for any `channel=` overlay. Echoes its own parameters. Also carries the four **organ** effect counters (built / axes terminated / fruit dropped, plus three separate *binds* ratios), which appear only for a species that has organs |
| `genome_drift` | **Per-slot population mean over generations** — whether a slot ever moves | Warns below generation 2, because a drift study on a population that never turns over cannot answer its question |
| `root_contact` | How much of a root system is actually touching soil | |
| `flora_census` | Which species a generated world actually contains, per seed | `where=1 focus=NAME at=X` audits one *window*. Built after a card came back "I don't see a difference" and the window held 125 grass cells against 7,853 woody |
| `litter_probe` | Where shed litter comes to rest, and whether it rots | `out=` writes a classification overlay — magenta on a branch, cyan on the ground — and **`crop=x,y,w,h zoom=N` on top of it, which is what makes that overlay judgeable**: the answer is read by eye and a 512x320 sheet with the interesting part 180 px wide is not. Its `against-plant` column scores a drift banked against a trunk and a leaf stuck up a tree identically, so read it with the height bands, never alone |
| `crown_census` | **What the brown cells in a crown actually are** — every material standing above the ground line, split into 40-row bands | Built because soil, litter, deadwood and thickened wood are one mid-brown speckle at contact-sheet zoom, so the eye cannot separate them and only a count can. Generalises well past plants: it is a material-by-height histogram, so it answers *what is stacked where* for any vertical structure — a collapse's debris column, a drift's profile. Echoes its own parameters |
| `debug_tree_variants` | Multi-variant comparison of `tree.ron`'s economy | **Parse fixed 2026-08-27, scene still dead — do not reach for this.** It emitted `moisture_threshold`, renamed to `soil_water_threshold` in `organism.rs` some time ago, so it panicked on start for anyone who tried; the rename is corrected but its scene is a bare stone floor that can germinate nothing, so it answers no question today. `plant_probe` took over ensembles. Kept, not deleted, because its header records why the shipped tree stalled at ~10 wood cells. Its row said only "marked throwaway in its own header" for the whole time it was broken, which is why check 5b now exists |

## World, terrain and weather

| instrument | answers | notes |
|---|---|---|
| `cave_probe` | What a world's caves actually *are*, over a seed sweep | |
| `wind_probe` | What `weather::exposure` reads across a landscape | |
| `sky_light_probe` | What a sky-visibility model would say, on the five geometries that decide it | |
| `underground_probe` | How much open air the renderer draws as cave interior | |
| `scale_probe` | What a bigger world costs, measured rather than extrapolated | **`phases=1` is the whole-frame one** — see below |
| `pass_ablation` | Which generation pass eats which other pass's output | |
| `field_cost` | What the coarse field costs per frame, and what decides it | |
| `film_probe` | Standing census of one-cell water films | Standing count, not creation rate — the distinction that solved the whisker hunt |
| `fire_probe` | The grassfire instrument | |
| `anchor_probe` | **Does it matter which code path last wrote a field?** One geometry, its anchor distances written three ways, swept for the margin | Built for `open-bugs-handoff.md` §S2; the shape generalises — see below |
| `arch_probe` | **Does the shape a player builds change what stands?** One opening, four roof forms, swept for the span each one drops at | Builds and runs scenes; changes no default. Refuses to print a margin it did not bracket — see below |
| `support_census` | **What the support field is made of, and what a replacement would cost.** The distance histogram, the coarse chunk layer's true node and edge count, and a cell-by-cell comparison of a candidate field's *load DAG* against the exact field's | Read-only — builds candidate fields beside the real one and never writes. `control=1` runs both controls; see below |

## Creatures

| instrument | answers | notes |
|---|---|---|
| `creature_probe` | What a creature is sensing and deciding, per tick | |
| `creature_space` | **How many distinguishable ways of being an ant does this system admit?** | |
| `ant_ablation` | Is the authored brain doing anything, or is it the substrate? | The control that separates a mechanism from its scaffolding |
| `forage_probe` | Does the colony actually range, and how far? | |
| `gnome_depth` | Does the gnome weave *through* a formation, or get sliced *by* it? | |

## The ones that generalise past what they were built for

**`divergence` is axis-agnostic, and this is the entry that stops it being
rebuilt.** Everything downstream of the axis — the two-world founder
construction, the exact-zero control, both metrics, the seed sweep, the
establishment-imbalance warning, the axis-survival check — does not know what
is being varied. **Adding an axis is one arm on `Axis` and nothing else.** It
therefore already answers:

- **Any single-axis morphology comparison.** `soil=`, `founders=`, `width=`,
  `species=`, `frames=` are parameters already, so "does soil depth change
  root:shoot", "does crowding change slenderness", "do two species differ in
  shape at the same size" are each a *run*, not a build.
- **Does a new genome locus move morphology at all?** Point it at two patches
  differing only in the locus and read the sign agreement. This is precisely
  the measurement `plant-species-authoring.md` §1 wanted when it found
  `light_weight` and `upward_weight` inert — and the measurement
  `plant-appearance-design.md` needed when three architectural levers all
  fired and moved no silhouette.
- **A determinism check, for free.** Its control asserts two identically built
  worlds diverge by *exactly* zero. A non-zero return on `control=1` means
  determinism has broken — which `PLAN.md` requires and which nothing else
  routinely exercises at whole-organism scale.

**`anchor_probe` is a *provenance* harness, and that is what generalises.**
It was built to ask whether `structural.rs`' three disagreeing anchor rules
matter, and everything downstream of "which function writes the field" is
indifferent to what the field is. Three properties worth reusing:

- **One geometry, N routes.** The obvious build — paint a structure, generate
  one, dig one — cannot answer the question, because the arms would differ in
  *shape* as well as in rule. Building once and writing the field several ways
  makes the scene a constant by construction, and the probe prints a material
  census per arm as the control that says so.
- **It sweeps for a margin, not an outcome.** Past its margin every rule
  agrees a structure falls; short of it every rule agrees it stands. A rule
  can only show itself in *where the margin is*, which is also the quantity a
  player feels — how far you can build before it comes down. The first run of
  this probe put the sand pile where the margin could not reach it and
  produced a null that said nothing about anything.
- **It prints the debug field beside the outcome.** That is what caught the
  two pointing opposite ways: under the brush's rule the deck reads as *better
  supported* (largest distance 9 against 82) and is the arm that collapses. A
  support overlay alone would have said the brush's field was the healthy one.

Reach for its shape for any "does this code path's version of X differ from
that one's" question — two writers of the same cached field, two builders of
the same state, a fast path against its slow reference.

**`arch_probe` is `anchor_probe`'s margin logic with the variable moved.**
`anchor_probe` holds one geometry and varies the *rule*; this holds one rule
and varies the *geometry*, and the transferable part is what sits between
them: **a comparison of two ways of doing something can only show itself in
where the margin is**, because on either side of it both arms agree. Three
things worth copying:

- **It refuses to report an unbracketed margin.** Its first sweep had every
  arm at 100% and it printed *"the sweep never reached its margin"* rather
  than a number. `anchor_probe`'s own first run produced exactly that null and
  did not say so.
- **It carries a control for each rival explanation, not just one.** The arch
  uses more stone than the lintel, so there is a cell-count-matched arm; and
  "it is really just depth" is a live alternative, so there is a
  triple-thickness arm. The second one *worked* (the margin went 56 → 96),
  which is what makes the arch's win over it meaningful.
- **It measures the scene rather than assuming it.** The clear span is read
  back off the built world as the widest empty run below the springing line,
  so two arms cannot silently be roofing different holes.

**`support_census` compares two fields by the *question their consumers ask*,
not by their values**, and that is the transferable part. `load.rs` never
reads a support distance as a magnitude — it reads four bits per cell, "which
of my neighbours are below me" — so two fields with wildly different numbers
can be identical to every rule downstream, and two fields with similar numbers
need not be. The census computes those bits under each field and diffs them.
Reach for the shape whenever a replacement is proposed for a cached quantity:
**diff the predicate the consumers evaluate, not the cache**.

Three things it does that are worth copying:

- **Two controls, opposite in sign.** A flat-zero field must read ~97%
  disagreement (the instrument can see a difference) and the exact field
  against itself must read exactly 100% same (it does not manufacture one). A
  set-comparison that is silently comparing a thing with itself reports
  perfect agreement, which is the answer a proposal wants.
- **It splits by whether anything ever looks.** At 8192x2560 only **10,344 of
  19.4 M** body cells pass `load::is_structurally_interesting`, so a
  whole-world agreement figure is 99.95% a statement about rock no rule
  evaluates. The headline and the meaningful number differed by 2.3x
  (50.49% against 21.82%).
- **It reports a chunk-boundary breakdown as a rate per band, never a
  count.** The interior band holds far more cells and wins on a count
  whatever the truth is; `CLAUDE.md`'s chunk-decomposition warning needs the
  rate to be checkable.

**`ant_ablation` and `pass_ablation` are the same idea in two domains**: turn
the mechanism off and see whether the outcome notices. Before concluding a
mechanism works, run the ablation — `CLAUDE.md`'s *a test can pass because the
code under it is dead* has an instrument, and this is it.

**`scale_probe phases=1` is the only thing that times a whole frame**, and it
was built because nothing did. Every other cost figure in this repo measures
*part* of a frame, and the three that existed were taken at three different
world sizes — `ascii` times the CA sweep at 512x320, `field_cost` the field at
8192x2560, `scale_probe`'s default mode the two together. So "the field is the
problem" was a reading off two numbers that had never been placed beside the
other nine phases. It runs `App::update`'s exact order, times each phase, and
buckets whole frames by sky-step and gust the way `field_cost` does. Beyond
the question it was built for it answers:

- **"Is this phase worth optimising?"** for any phase, since it prints each
  one's share. A phase at 2% cannot repay work whatever its internal cost.
- **"Does a new per-frame subsystem fit?"** — add it to the list and read its
  share against the 16.6 ms budget before it ships.
- **The idle cost of a loaded world**, which is what a player experiences most
  of the time and what M10's streaming has to hold down.

**`ORGANISM_PASS=<every N>` splits `step_organisms` seven ways** (in
`plant.rs`, same shape as `FIELD_PASS`), and prints `live`/`ticked`/`cells`
beside the timings. The counters are the point: they are what said the cost is
per *cell ticked* rather than per live organism, which killed a plausible
optimisation before it was written. Reach for it for any "is this cost the
item count or the item size" question.

**`SCHED_PASS=<every N>` splits `scheduler::step` six ways**, one per
`ActiveKind`, and prints `sites` / `produced` / `deferred` beside the times.
Same shape as `FIELD_PASS` and `ORGANISM_PASS`, and the counters are again the
point:

- **`produced` against `sites` is a leak detector.** If a batch schedules as
  many sites as it drains, each site is replacing itself and the queue is
  self-sustaining however fast it is served -- which is how
  `open-bugs-handoff.md` §S was found (~8,100 produced against a 2,000 cap).
  Reach for it for any "does this backlog drain" question, not only a
  structural one.
- **`deferred` is the whole heap after the batch**, not the capped remainder,
  so it has a meaningful idle value (~5,400 at 8192x2560: ordinary
  future-dated growth and evaporation sites) and a drained queue is one that
  comes back to it.
- **`[struct]`'s second line attributes the structural share to a branch of
  `structural::tick`** -- `worsened` / `improved` / `unmoved`, plus the two
  defer reasons and the largest distance written. A `max aux` that keeps
  rising with the world's material dead still is the count-to-infinity
  dynamic and nothing else.

**`scale_probe load=` is what makes a cost question about a *verb*
answerable at all**, and it now carries all three destructive ones --
`blast:EVERY[:COUNT]`, `strike:EVERY[:COUNT]` (the hammer) and
`mine:EVERY[:COUNT]` (the pick), at the app's own `brush_radius`. Three
things it answers that were not the question it was built for:

- **`COUNT` separates "never drains" from "drains slower than it fills."**
  With uses still arriving the two look identical; fire a fixed number and
  watch the aftermath. This is what turned `open-bugs-handoff.md` §S from an
  explosion bug into a bug in every destructive verb but the brush.
- **Two verbs are a control on each other.** The hammer removes *fewer* cells
  than the pick and costs *more*, which rules out material-removed as the
  driver without needing a third measurement. Reach for a paired verb before
  reaching for a new metric.
- **`cells actually removed` is printed beside uses taken**, and that is not
  decoration. The first run of this probe reported **200 cuts and 0 cells
  removed** -- `rigid::is_tool_target` refuses `Powder`, and the probe was
  aiming at the topmost `Solid | Powder` cell, which on a rolling world is
  soil. The queue sat flat and it read exactly like "the pick is fine". A
  counter of *calls* is not a counter of *effect*; any new load component
  needs its own effect counter before its null means anything.

**`RECONVERGE_AT=<frame>` on `scale_probe` is the *oracle* for any question
of the form "would converging this help?"** It runs one whole-world
`compute_world_distances` mid-run and prints the pending count, the scheduler
cost and the census either side of it. Two uses well past the one it was
built for:

- **It separates "the reactive path is slow" from "the reactive path never
  arrives".** No amount of tuning a reactive relaxation can say what the
  converged state costs, because the converged state is what it never reaches.
  One pass says it directly: on `load=blast:200:1` the scheduler went 12.49 ms
  -> **0.25 ms** and pending 53,077 -> **6,094**, and stayed there. That is the
  measurement that made `Reports/structural-reconvergence-design.md` a scope
  worth building rather than a hypothesis.
- **It sizes the fix, because it censuses every body cell's `aux` either side
  of the pass.** "How much of the world does one charge actually invalidate?"
  is otherwise unanswerable, and the guess in circulation (250,000 cells,
  inferred from woken chunks) was **3.7x** the censused figure of 67,100.
  Run it on an idle world first: that arm reads **45 cells out of 19.4 M**,
  which is what makes the loaded arm's number mean anything.

It is a probe, not a proposal — the pass takes ~2,000 ms and walks all 21 M
cells. Nothing would ship it per blast.

**Put it on the *last* measured frame, not in the middle.** `step` counts the
measured frames only (`for step in 0..frames`), so a `RECONVERGE_AT` above
`frames=` never fires at all and prints nothing — which reads exactly like a
run where the oracle had nothing to say. And the pass itself converges the
field, so every frame after it is cheap: at `RECONVERGE_AT=frames-1` the
timing table is untouched and the census is still taken at the end of the run.

**`scale_probe band=N` sets how wide a strip the pick and hammer work
along, and without it no two world sizes are comparable.** The walk puts 64
cuts at a stride of `band / 64`, so the default 384 spans the same absolute
384 cells whatever the world is — **4.7% of an 8192-wide world and 19% of a
2048-wide one**. A size sweep run that way damages the small arm four times
harder than the large one and then compares absolute counts, which is how
`Reports/structural-support-model.md` §6.5d briefly acquired a third bug
that did not exist. Pass `band=$((w * 384 / 8192))` to hold the fraction
fixed.

**Its default owes a positive control and it is cheap to run**: at
`band=384` the 8192 arm must reproduce **4,490 wrong / 10,033 pending**
byte-for-byte. That check earned its keep immediately — an initial default
of 378 floored to a stride of **5** against the historical **6**, which
would have silently moved every recorded number in that report while looking
like a no-op.

**`scale_probe` echoes its own landing-aux arms** (`landing aux: settle=…
particle=…`) as of 2026-08-27, because `SETTLE_AUX` and `PARTICLE_AUX` decide
where §S's false anchors come from and a log that does not name them cannot be
told from one written before they existed. Each takes `zero|max|seed`. A run
whose header lacks that line came out of an older binary.

**The `[struct]` census is a *per-frame sample*, not a total, and that makes
`grounded` a one-way instrument.** `scheduler::step` drains
`structural::take_tick_census()` every frame and prints only on reporting
frames — deliberately, since a running total wearing a per-frame label would
be worse — and the line is further gated on that frame having done work. So a
counter in it answers "how many fired on *this* frame", and on a scene where
the mechanism is rare it prints **no line at all**: measured 2026-08-27, the
hammer arm at `SCHED_PASS=20` over 600 frames produced zero `[struct]` lines
on both arms of an A/B whose oracle counts differed 26-fold.

Read it accordingly: **a non-zero reading is real evidence the mechanism
fires; a zero is nearly none.** That was written on 2026-08-27 and then
immediately violated by its own author: "`acceptance` cannot see the ground
root, `grounded` reads 0 through `rockdrop` and `ligament`" went into two
documents before anyone tested it properly. **The test that works is
differential** -- run the suite with the rule at both values and diff
everything that is not a wall clock. Three of the 22 cases turned out to see
it, and `scene=strike` turned out to be §S in miniature. See
`Reports/structural-support-model.md` §6.5e. `Reports/structural-support-model.md` §6.5's
"`grounded` reads 0 on every frame, so the ablation is vacuous" stands on the
byte-identical output beside it rather than on the counter. When the question
is "how much is left", use a whole-world census instead — `RECONVERGE_AT`'s
`of changed, was at 0 (tick ground-root)` is the one that answers it for this
particular rule.

**`filmstrip`'s `max_sites=N` asserts that the structural scheduler
*drains*, and it is the one gate in the suite that states
`open-bugs-handoff.md` §S directly.** A **final** count rather than a peak,
deliberately: §S is "pinned at its cap for ever", so the refutation is not
that the backlog stays small -- a real blow should spike it -- but that it
comes back down. On `scene=strike` the shipped engine reads 958 / 968 / 824
/ **289** across frames 2-182 and a reverted one reads 958 / 2747 / 5034 /
**7145**, so only the last reading separates them by more than 3x.

A counter and not a wall clock, which is why it can be trusted where this
suite's two `max_frame_ms` gates cannot: those flaked twice on a loaded box
inside one session, and this one is bit-identical under any load.

**`AUX_TRAP` is a *write-seam trap*, and the shape is the reusable part.**
It is not an example — it is an env-gated report inside `World::set` that
fires on any write matching a predicate and prints a backtrace. Reach for it
when you know *what* wrong state exists and not *who* wrote it, and when
guessing writers by name has already failed twice: `Reports/structural-support-
model.md` §6 ablated `rigid::settle` and `structural::tick`'s `grounded_root`
before trapping the seam, and the answer was neither (`particle.rs::
landed_cell`, twelve backtraces out of twelve). Three things that generalise:

- **Trap the invariant, not the caller.** `World::set`'s own doc already
  states the principle for a different problem — *"an enumeration that has to
  stay complete is the failure mode this project keeps rediscovering"* — and
  a predicate over (old cell, new cell, position) is complete by construction
  where a list of suspects is not.
- **State the predicate as the bug, not as the symptom.** Here: *this write
  makes a cell body material reading `aux <= 2` with no bedrock adjacent, and
  the cell it replaced was nowhere near an anchor.* That is "a false anchor is
  being created" in one line, and it fired 12 times in two frames with no
  false positives.
- **Cap the reports and print the neighbourhood.** The cap keeps the first
  report readable through a cascade; the neighbourhood is what showed the
  error spreading — the first traps sit beside `stone:2405` and the last
  beside `stone:0`.

**`flora_census where=/at=` is the answer to "I don't see a difference".**
Audit the rendered window before believing a card; a whole-world total in a
card's `meta` cannot say whether the thing is even in frame.
