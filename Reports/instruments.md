# The instruments — what already exists to measure with

**Status: living index. Current as of 2026-08-25.**

Every measurement in this repo comes out of an `examples/` binary, there are
**25 of them**, and their names do not say what they can answer. This file
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

## Rendering and judging by eye

| instrument | answers | notes |
|---|---|---|
| `filmstrip` | A contact sheet of several frames, or `gif=1` for an animation | The acceptance harness *and* the review-card generator. `channel=` draws per-cell scalars. Reach for `gif=1` whenever the question is whether something *moves* right — a grid of stills cannot answer that, and a GIF has twice got a diagnosis where stills got a rejection |
| `ascii` | Headless behaviour plus **worst-frame timing** | The number to quote for frame cost. CI runs it |
| `viewshot` | What the *player's viewport* shows of a world larger than itself | The scale question a full-world render cannot answer |
| `pixel_stat` | How noisy a rendered region is, as a number | Compares two strips without squinting |

## Plants

| instrument | answers | notes |
|---|---|---|
| `divergence` | **Does one environmental difference produce two different-shaped plants?** | See below — the most reusable thing here |
| `plant_probe` | Every organism-owned cell's per-cell channels for a grown tree | The quantitative pair for any `channel=` overlay. Echoes its own parameters |
| `genome_drift` | **Per-slot population mean over generations** — whether a slot ever moves | Warns below generation 2, because a drift study on a population that never turns over cannot answer its question |
| `root_contact` | How much of a root system is actually touching soil | |
| `flora_census` | Which species a generated world actually contains, per seed | `where=1 focus=NAME at=X` audits one *window*. Built after a card came back "I don't see a difference" and the window held 125 grass cells against 7,853 woody |
| `litter_probe` | Where shed litter comes to rest, and whether it rots | |
| `debug_tree_variants` | Multi-variant comparison of `tree.ron`'s economy | Marked throwaway in its own header |

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

**`scale_probe load=blast:EVERY:COUNT` is what makes a cost question about a
*verb* answerable at all.** With charges still arriving, a queue that never
drains and one that drains slower than it fills look identical; `COUNT` fires
a fixed number and lets the world be watched afterwards. The same trick
applies to any load component added later -- measure the aftermath, not the
steady state.

**`flora_census where=/at=` is the answer to "I don't see a difference".**
Audit the rendered window before believing a card; a whole-world total in a
card's `meta` cannot say whether the thing is even in frame.
