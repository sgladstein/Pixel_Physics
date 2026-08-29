# The instruments — what already exists to measure with

**Status: living index. Current as of 2026-08-26.**

Every measurement in this repo comes out of an `examples/` binary, there are
**36 of them** (recounted 2026-08-29; the 31 this line carried was the
2026-08-26 census), and their names do not say what they can answer. This file
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
| `viewshot` | What the *player's viewport* shows of a world larger than itself | The scale question a full-world render cannot answer. `quarry=WxD` cuts an open-cast pit into the skyline **on the shot after the one it names**, so `shots=2` is the minimum or the cut is never drawn; `aim=` puts it on high ground, without which it fills with sea |
| `uishot` | **What the app's own HUD and panels look like**, and what a held sky or a pinned weather actually does | The only thing here that calls `App::draw` — every other renderer builds frames from `Renderer::draw` and never sees a panel. Drives `App::update`/`App::draw` as `main.rs` does, then sets the state a keypress would have. `sheet=menus\|sky\|weather`; a fresh deterministic world per tile, and per-tile counters (sun, water, gusts and delivered force, freezes, bolts, ice **in the drawn window**) printed under each. That last column exists because FROST measured 3,844 freezes against a picture identical to CLEAR's — the world is 8192x2560 and the frame shows 1/128th of it |
| `scale_covariance` | **Is the same seed at `k` times the cell resolution the same landscape, `k` times as large?** | The question the resolution step's whole content half rests on, and it was worth asking before hand-editing 46 parameters. Reports the rescaled elevation residual against **two controls in the same run** -- an unrelated seed, which is what "no relationship" looks like, and a `region_variation=0` arm, which names the cause when it fails. It did fail: 39.1 rows against a 42.5-row control, diagnosed in one run to `region::COMPOSITION_WINDOW` being a hardcoded 512 |
| `pixel_stat` | How noisy a rendered region is, as a number | Compares two strips without squinting |
| `subpixel` | **What the picture looks like when the renderer has more pixels than the simulation has cells** — the same world drawn at `scale` pixels per cell with each cell's shape reconstructed at sub-cell resolution, against `arm=baseline`, the shipped 1:1 render magnified by the same factor | Not plant-specific and not really a renderer: it is **a testbed for any question of the form "what if a cell were not a square"**, because the colour source is the shipped `Renderer` at 1:1 and the background is a second pass of the same renderer over the same world with the class under test emptied — so no arm can invent a colour the engine would not have drawn, which is what makes an A/B off it admissible. `arm=plants|all` decides whether terrain goes through it too, and that is the comparison worth running before believing any smoothing proposal: with colour blending on, **the soil grain is destroyed**, and the grain is why soil reads as soil. The `ao`/`shade` knobs are the general finding rather than a plant feature — a kernel field over *any* occupancy channel yields a thickness (`cov`) and a surface normal (`grad cov`) at sub-cell resolution, which is volume shading for anything the lattice draws flat. **Set both to 0 first when an interior looks quilted**: two separate lattice-noise bugs came out that way and nothing else caught either (`Reports/subpixel-rendering-2026-08-29.md` §5d). Echoes its own parameters, and prints the plant-cell count beside the image |
| `subpixel_cost` | **What drawing the same world region at more pixels per cell costs**, as a paired comparison over one camera | Distinct from `render_cost`'s `viewport_scaling`, which grows the *viewport* at `zoom == 1` and so shows **more world** — its own note says the extra is cheap underground stone. This holds the visible region fixed (asserted, not assumed) and varies only the output lattice, so it answers the resolution question rather than a content question. Reads 1.13x at four times the pixels and 1.32x at nine, because the per-pixel work is under 10% of a full draw and a finer lattice repeats none of the per-draw setup. Generalises to any "is this cost per-pixel or per-draw" question about `Renderer::draw`. Its "under 10% of a full draw" was the fixed cost `renderer-frame-cost-2026-08-29.md` later named -- 94% of that draw was one call outside the pixel loop, so read this row's ratios against a redraw whose phase split you have seen |
| `render_cost` | **Where a full-screen redraw spends its time**, broken down, **and what a bigger viewport costs** | The full branch measured 12.07 ms mean on the shipped 2048x640 world -- 54% of a frame -- and runs on ~100% of frames while the gnome walks, because a camera move invalidates every pixel. Its `viewport_scaling` section draws one world at 512x320, 768x480 and 1024x640 -- the resolution question -- and carries **two uniform-world controls beside it**, because the camera is clamped at the world origin and everything a taller viewport adds is cheap underground stone: on the generated world the scaling reads 2.41x at 4x the pixels, and on the all-stone control it reads 3.68x. The control is the number. **Set `PIXEL_PHYSICS_DRAW_TIMING=1` on this or any harness that draws** and `Renderer::draw` prints its own phase split -- preamble, horizon, sky light, glow splat, pixels, overlays. Nothing else in this table can attribute a redraw *internally*, and this row's own breakdown is reads-vs-colour-work rather than phases: it read 3% for the per-pixel chunk lookup while **94% of the frame sat in one uncounted call** (`Reports/renderer-frame-cost-2026-08-29.md`) |
| `frame_profile` | **Which phase a frame's time went to**, timed separately with a distribution | The thing `ascii` cannot answer: it reports a worst frame, which says "does this fit in 16.6 ms" and nothing about *where* it went. Runs the exact phase list `App::update` runs |
| `camera_snap` | Whether the camera moves discontinuously through the path the **app** actually uses | Drives `App::update`/`App::draw` as `main.rs` does, rather than calling `Renderer::follow` directly -- so it catches what a harness calling the API itself cannot |
| `weather_duty` | How often it is raining, swept across seeds and a long window | Built because a single 1,200-frame run measured 89% and that was a sample from inside one wet epoch, not a duty cycle. Generalises to any "is this a duty cycle or one epoch" question |

## Plants

| instrument | answers | notes |
|---|---|---|
| `divergence` | **Does one environmental difference produce two different-shaped plants?** | See below — the most reusable thing here |
| `fate_viability` | **What fraction of mutations to a species' production rule still produce a plant that lives and breeds?** | Gate 1 of the evolvability programme, and it is allowed to answer *no*. Generalises past plants and past fates: it is really *"register N variants of a species at runtime, grow each, and classify the outcome"*, so any question of the form **how much does this substrate tolerate being changed** can be asked by swapping what `mutate` perturbs — a creature genome, a material table. Its real contribution is the **three-way** classification: viable / lethal / **silent**, where silent means the stand came out identical to the base and the mutated field is therefore never read in that scene. A two-way rate counts silent mutations as tolerance and overstates robustness by however many of them there are (measured: 7 of 48 were no-ops from drawing the value already held, and a further set were unread fields). Both controls are mandatory and print on every run — the unmutated table must live, and a shoot-child-to-`Seed` table must die — because a viability rate with neither is exactly the arithmetically-correct-and-about-the-wrong-thing shape. The positive control has already earned its keep once: it reported the *unmutated* table as 0/3 and caught a harness bug that would have published a decisive, entirely false 0%. **It calls the shipped operator rather than imitating it**, which it did not until 2026-08-29: it mutated its own copy of the table with its own code, drawing from six cell types on the woody base where `FateGenome`'s operators draw from eight on every base — so its number was about a mutation nothing in the engine performs. Route is now genome -> `FateGenome::mutate` -> `to_table` -> RON -> founder genome. **`op=all` (default) or `op=retarget|recondition|insert|delete`**: `all` reproduces the shipped 60/15/15/10 mixture, and forcing one operator is what makes the rare ones measurable — 40 mutants of the mixture spend about four draws on `delete`. **Four outcome classes, not two**, and the two extra are where a naive rate leaks: *declined* means the operator itself changed nothing (a redraw that never found a different value, `delete` at the one-rule floor, `insert` at `MAX_FATES`), *silent* means the genome changed and the stand came out identical anyway. Declined is a subset of silent; both are excluded from the denominator, because a mutant that establishes only because it *is* the base is the positive control quoted back as a result. **`base=tree` (default) or `base=herb`**: `tree.ron` declares no organ material, no `Ripen` behaviour and no `Ripe` rule, so a mutant reaching `Flower`/`Fruit` there measures that gap as well as the substrate — the harness no longer prevents it (that was the divergence above) but counts it in its own column. Note the negative control finds its rule by `when == Grew` rather than by index, because a determinate base carries an extra rule ahead of it and a control that poisons the wrong rule fails *open* |
| `plant_probe` | Every organism-owned cell's per-cell channels for a grown tree | The quantitative pair for any `channel=` overlay. Echoes its own parameters. Also carries the four **organ** effect counters (built / axes terminated / fruit dropped, plus three separate *binds* ratios), which appear only for a species that has organs |
| `genome_drift` | **Per-slot population mean over generations** — whether a slot ever moves — **and, since 2026-08-29, whether the *production rule* moves** | Warns below generation 2, because a drift study on a population that never turns over cannot answer its question. The continuous slots and the fate table need different treatments and get them: a population mean is meaningful for a draw and meaningless for a *program*, so the rule table is censused instead as *how many individuals carry a table that is no longer their species'*, split by whether it grew (`insert`), shrank (`delete`) or changed in place (`retarget`/`recondition`), plus the count of distinct tables alive. **Two free controls, and read them first**: `gen0` must be 0, because founders take the species table verbatim, so anything else means the census is reading the wrong object and every other column is void; and `empty` must be 0, because an empty genome falls back to the species table and a population of them reports *no drift* while meaning *no genomes* — the channel-with-no-writer failure wearing a plausible number |
| `root_contact` | How much of a root system is actually touching soil | |
| `flora_census` | Which species a generated world actually contains, per seed | `where=1 focus=NAME at=X` audits one *window*. Built after a card came back "I don't see a difference" and the window held 125 grass cells against 7,853 woody |
| `litter_probe` | Where shed litter comes to rest, and whether it rots. **Read the `SUSPENDED (air underneath)` line, not the on-terrain/against-plant split** | `out=` writes a classification overlay — magenta on a branch, cyan on the ground — and **`crop=x,y,w,h zoom=N` on top of it, which is what makes that overlay judgeable**: the answer is read by eye and a 512x320 sheet with the interesting part 180 px wide is not. `plain=1` drops the markers and the dimming and draws the scene in its own colours -- the marked overlay says which cells are held off the ground, `plain=1` says whether a person would call it a forest floor, and those are different questions. **Two of its columns are confounded and the third is not**: `against-plant` scores a drift banked against a trunk and a leaf stuck up a tree identically, and the height bands measure against a `terrain_top` that excludes litter, so the top of a deep mat reads as high up. `SUSPENDED (air underneath)` is the unconfounded one and is what to quote |
| `crown_census` | **What the brown cells in a crown actually are** — every material standing above the ground line, split into 40-row bands | Built because soil, litter, deadwood and thickened wood are one mid-brown speckle at contact-sheet zoom, so the eye cannot separate them and only a count can. Generalises well past plants: it is a material-by-height histogram, so it answers *what is stacked where* for any vertical structure — a collapse's debris column, a drift's profile. Echoes its own parameters |
| `beam_probe` | **Would a bending-stress model say anything useful about this plant?** Dumps per-cell moment, section and stress for the largest organism, with a parent forest rooted at the structural anchors | Written as a positive control *before* the mechanism, which is the whole reason it exists — it answered "yes, and the base of a balanced stem reads ~0", which redirected the plan (`tree-mechanics-plan-2026-08-29.md` §4). **Reports three section measures side by side and they disagree on 52-61% of cells**, which is the thing to know before quoting any number off it; §9 of that report says why. Echoes its own parameters |
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
| `ant_ablation` | Is the authored brain doing anything, or is it the substrate? | The control that separates a mechanism from its scaffolding. **~15 min at its defaults** (20 arms x 5 seeds x 6,000 frames, ~1.39 ms per arm-run-frame, measured 2026-08-29 on 4 cores); progress goes to stderr. Its defaults answer the *locomotion* question only — `deliv` and `eats` are 0.0 in every arm on a corpse pile. Use `terrain=world food=trees` for anything about foraging |
| `forage_probe` | Does the colony actually range, and how far? | |
| `predation_probe` | **Can a beetle smell an ant?** The T6 pre-flight: how much channel-A and channel-B pheromone there is world-wide, what fraction of prey and of predators stand within a sensor offset of a nonzero cell, and how far a beetle is from the nearest one | Answers a question no fitness harness can: `creature_space` reports per-ant survival on the same scene and `creature_probe` reports one creature's inputs per tick, and neither can say what a *plane* looks like world-wide. Generalises past predation — `mode=preflight` is a **stigmergy census** and would answer "is there a trail where the ants are" for any signal-following species. `mode=control` is the positive control in the same binary (paint a saturated trail; stand a beetle on an ant), `mode=ab` re-measures the `beetles=0`/`beetles=9` null, `mode=cost` times a whole frame on a settled world. Echoes its own parameters |
| `creature_look` | **Can you find this thing in the picture?** `ink` is how much luminance a body puts on screen that the ground would not have, from a paired with/without render; `decoys` is how many *other* places in the frame are at least as different from their surroundings as the body is | **Not creature-specific — it answers "is this findable" for anything drawn into this world**, which is the question `plant-appearance-design.md` had no instrument for. `decoys` is the one that explains a picture: contrast says a body differs from its background, and this says how many things in the frame differ just as much. Pins daylight (a luminance number sampled at an arbitrary hour is a statement about the hour) and builds on a `WorldgenPresets` preset rather than `build_terrain`, whose bare skyline makes every surround reading the sky. Echoes its own parameters |
| `gnome_depth` | Does the gnome weave *through* a formation, or get sliced *by* it? | |
| `species_export` | **Can this individual be kept?** Writes a genome + traits out as `assets/species/<name>.ron` and reads it back through the loader | Not a measurement — the dev-tool exit for E8. `genome=rNNN` takes the same labels `creature_space` ranks, so a good row in that sweep can be saved as a species. `verify=1` (default) is the round trip |

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

**`FALL=off` reverts the 2026-08-29 fall — the rate seeded from the break,
the tipping test on landing, and the centroid pivot — leaving everything else
alone**, so a paired run can hold the semantics fixed instead of comparing
two *binaries*. That distinction is what the switch exists for: every
instrument added alongside a mechanism is missing from the arm it is being
measured against, and the first reading of this change fell back on a
cluster-level statistic for exactly that reason. Verified against the
pre-change binary on `scene=fell`: every physics line identical.

**`filmstrip channel=bend` draws the plant bending stress**, and its
quantitative pair is the `bending stress over N cells` line in the felling
census — median, peak, **where the peak is**, and how many cells read exactly
zero. Read them together: a ramp can only say "brighter", and whether the
hottest cell is at a trunk's base or out on a twig is the entire claim of the
model. **Not `channel=stress`**, which is `load::evaluate`'s rock stress and
a different quantity.

**The bend census is four lines and none of them substitutes for another.**
`cells leaning this tick` counts what *wants* to move (`|deflection| >= 1`);
`cells moved / refused` counts what did; `cross-sections blocked` and `would
have torn` split the refusals by *reason*, and that split is the one that can
be acted on. A grass stand once read **0 moved against 302 refused** — a
mechanism inert in a real world with every guard over it green — and the two
causes want opposite fixes. Blocked is a crowded stand doing its job; torn is
the one-piece rule turning a swing down because no cross-section could move
without stranding a cell. **Read them against `wind on that tissue`**, which
names the gust and the exposure range the moments were measured under: half
the moment now comes from the weather, so a lean count with no wind figure
beside it is that frame's phase plus the mechanism, inseparable.

**And the moment line is split per material**, because `stiffness` is a
per-material constant and one pooled distribution cannot fit two of them.
Trunks dominate a stand's pooled quantiles and foliage sits two orders of
magnitude below — leaf p90 260-348 under a stand p90 of 455-636 — so a
stiffness read off the pooled line is fitted to the wrong tissue. Each row
prints its own material's stiffness and how many of its cells want to lean,
which is the "did this constant connect to anything" reading.

**`BEND=off` holds every plant rigid** and is the control for all of the
above. It exists because both errors in this mechanism were found by holding
the semantics fixed and changing nothing else. Comparing two *binaries*
cannot do it: the counters that catch the error are added alongside the
mechanism, so the arm being measured against does not have them. Note the two
arms diverge — the sim differs — so a difference read thousands of frames
apart is two different worlds, not a measurement of the mechanism.

**`HINGE_PROBE=1` prints the felling hinge's own arithmetic** — the region's
size and mass, the stump it pivots about, `broke_at` beside it, the centre of
mass **as a vector from the pivot**, the second moment and `alpha`. Read the
vector, not `alpha`: a hinge whose centre of mass is level with its pivot
swings straight *down*, which on a contact sheet is indistinguishable from
the piece simply falling. That is not hypothetical — it is what the first
build did, for a whole render and a wrong reading, and no image could have
told anyone.

**`filmstrip`'s "foliage by steps to the nearest wood"** is a multi-source
BFS out of every woody cell, crossing only foliage, so bucket 1 is exactly
the set `update::on_a_branch` holds. It exists because "the leaves fall off
the branch" and "the rule that holds them is one cell too shallow" produce
the identical picture; the histogram separates them, and it is what set the
clinging depth (36% at one step, 53% at two). `no path` is foliage with no
route to wood through its own kind — already run clear, and no depth
recovers it.

**Two orientation censuses on `scene=fell`, and they answer different
questions.** `settled log pieces` folds settled `log` into 8-connected
clusters, so two logs that land touching are one "piece" whose orientation is
the *pile's*; its own doc records the largest "piece" going from 49x48 to
99x71 purely because the pile packed tighter. `how pieces came to rest`
(2026-08-29) is asked of each body as it lands, once, at the only moment its
extent is unambiguous. Reach for the second whenever the question is about
the pieces and the first when it is about the pile — and note that a change
which packs the pile tighter moves the first one *against* itself.

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
