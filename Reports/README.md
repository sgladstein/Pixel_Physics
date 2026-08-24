# Reports index

One line per report: what it holds, and where it stands. Status is
provenance, not a fence — a "settled" call can still be reopened with the
owner, but check the status *before* trusting a report or re-deriving what
one already holds. A report whose subject has since shipped is marked so;
the report stays valuable as the *why* behind the code.

House rules: a session that adds a report adds its line here in the same
commit; a session that supersedes one updates the superseded line.
`scripts/docscheck.sh` flags any report missing from this index.

Division of labour with the two working files: `open-bugs-handoff.md` owns
*"is this broken?"*; [`dead-ends.md`](dead-ends.md) owns *"was this
tried?"*.

## Method and architecture — read these first

- [design-philosophy.md](design-philosophy.md) — **settled.** The short,
  opinionated statement the other reports imply: constants, hardcoding,
  scope boundaries, outcome-vs-mechanism. Read before arguing about any of
  those.
- [emergent-world-architecture.md](emergent-world-architecture.md) —
  **direction agreed.** Thin agents, rich world; the priority reshuffle; §8
  is where determinism was reversed to *required*.
- [documentation-audit.md](documentation-audit.md) — **executed
  2026-08-21/22.** What the docs told an agent that wasn't true, re-ranked
  for the agent consumer; carries the in-flight doc inventory, and the
  cold-agent benchmark that verified the outcome — 3/3 in 8 file-opens, no
  source reads, one stale work order correctly refused. Re-run that before
  claiming a later docs pass improved routing.
- [documentation-overhaul-plan.md](documentation-overhaul-plan.md) —
  **executed; four CLAUDE.md items deferred.** The plan the audit's findings
  were executed against: the agent-consumer framing, read-cost measurements,
  the drift protocol, and the two refusals that leave no trace in the tree
  (no README reorder, no `Reports/archive/` — both had been chosen the other
  way first).
- [claude-md-recommendations.md](claude-md-recommendations.md) — **nine
  landed, four open.** The thirteen-recommendation review of `CLAUDE.md` as
  always-loaded infrastructure. 5, 6, 7 and 12 are approved and blocked only
  on `load-share`, `plant-branch-angle` and `perf-lock` merging; execute
  them when those land.
- [pixel-physics-issues.md](pixel-physics-issues.md) — **mostly closed.**
  The twelve-issue backlog; nine-plus closed, #11 (slice-identifier on
  `ChunkCoord`) has a land-before-save-format deadline and #12 (grass does
  not spread) was owner-filed 2026-08-24.
- [instruments.md](instruments.md) — **living index.** What each of the 25
  `examples/` binaries can answer, and which of them generalise past the
  question they were built for. **Grep this before building a measurement
  harness** — the file exists because they were being rebuilt.

## Destruction and structure

- [fracture-mechanics-design.md](fracture-mechanics-design.md) — **design;
  its load/torque step has since landed** (`load.rs`). Why rock breaks the
  way it does and why three earlier support models were wrong.
- [load-model-handoff.md](load-model-handoff.md) — **superseded by
  landing**: the step it hands off shipped (`7e13e42`); kept as the
  rationale written before the work.
- [load-model-fit-review.md](load-model-fit-review.md) — **review.** Does
  the load model carry mining, blasting and building; written right after
  it landed.
- [prior-art-destruction.md](prior-art-destruction.md) — **research.** How
  Red Faction, Teardown, Noita et al. actually do structural failure, and
  where this engine's approach sits.
- [destruction-plan.md](destruction-plan.md) — **plan.** "Destruction
  eager, building forgiving" — the synthesis of the two documents above.
- [explosion-stone-review.md](explosion-stone-review.md) — **shipped, and
  the live design record for blasting.** Why a blast in stone looked wrong
  and what each of five rounds did about it; §15-16 are the joint fabric
  (rock has a grain and a blast wakes it), §17 the containment retraction.
  Carries the measured dead ends for this area — read it before touching
  `explosion.rs`, `fracture_field.rs` or the confined branch of
  `structural.rs`.
- [building-rethink.md](building-rethink.md) — **proposal, not built.**
  Building should not be a physics puzzle; from a direct playtest steer.
- [load-concentration-review.md](load-concentration-review.md) — **merged,
  and its numbers predate `main`.** The load-concentration change (handoff
  §2d): a wall is judged at the worst of its cross-section, so both faces
  carry the roof. Every figure in it was measured against `origin/master`
  `0c7ad58`, before the powder-surcharge, starved-walk and staged-collapse
  commits; §9 (a column's strength is quadratic in its width) is the
  largest thing still open here.
- [load-concentration-review-response.md](load-concentration-review-response.md)
  — **review response.** The second opinion on the above.
- [load-concentration-review-reply.md](load-concentration-review-reply.md) —
  **reply to the response**, and the shorter read of the two: what the
  review changed, and the one finding it overturned.
- [next-session-handoff.md](next-session-handoff.md) — **live handoff.**
  The unzip: what it was, what is left, what must not be retried.
- [explosion-mechanics-diagnosis.md](explosion-mechanics-diagnosis.md) —
  **diagnosis; the rebuild it prompted shipped** (README M15 status). The
  look-first measurements behind it.
- [underground-definition.md](underground-definition.md) — **settled,
  implemented.** What "underground" means, fixed at worldgen time; the
  wiki's world-cycles page describes the visible result.
- [dark-bands-diagnosis.md](dark-bands-diagnosis.md) — **diagnosis, all
  three cases fixed.** Overhangs, objects and open-cast digs: the
  per-*column* baseline `underground-definition.md` settled, measured
  (`examples/underground_probe.rs`) and replaced by a per-cell genesis map
  plus a `ground_datum` for shading. Extends that report rather than
  superseding it — its rejection of *inference* stands, and this stores more
  history instead. Its postscript records the depth grade going off by
  default on a playtest, and two framing errors in how a fix was shown.
- [sky-light-design.md](sky-light-design.md) — **design round; shipped on
  `F12`, /4 the default.** Measures the candidates for the open-cast-dig
  case (`examples/sky_light_probe.rs`): why `field.rs`'s light channel
  cannot drive it, why seeded propagation can, why block size 4 rather than
  `FIELD_SCALE`'s 8, and — tested later — why a stored incrementally
  maintained field is *not* worth it. Two of its own claims were wrong and
  are corrected in place at the bottom.
- [prior-art-underground-lighting.md](prior-art-underground-lighting.md) —
  **research.** How Terraria (a per-tile wall layer, then 0.91/0.56 light
  propagation) and Noita (a coarse blurred fog of war, no classification at
  all) answer "is this dark", and which of the two the still-open
  open-cast-dig case needs.

## Liquids and granular

- [liquid-simulation-research.md](liquid-simulation-research.md) —
  **research, round 1.** Why poured water piled like sand; SPH → PBF →
  PIC/FLIP survey.
- [liquid-simulation-research-r2.md](liquid-simulation-research-r2.md) —
  **research, round 2 (Report B of four).** The three method families
  round 1 didn't survey.
- [liquid-heightfield-design.md](liquid-heightfield-design.md) —
  **design; step 1 implemented, promotion reverted.** The heightfield
  bodies in `liquid.rs` are test-only today — their bugs are latent until
  promotion lands.
- [granular-mechanics-research.md](granular-mechanics-research.md) —
  **research (Report A of four).** The two-angle model; why BTW avalanche
  toppling is deliberately not planned.
- [coupling-research.md](coupling-research.md) — **research (Report C of
  four).** Rigid body ↔ grid coupling for M8; §4 is why chunk bodies run
  serially.

## Plants and trees

- [plant-simulation-research.md](plant-simulation-research.md) —
  **research.** Growth, evolution and biology directions past M16.
- [organism-substrate-design.md](organism-substrate-design.md) — **design,
  implemented.** The shared cell-typed organism model that retired
  `TreeState`/`CreatureState`.
- [tree-rewrite-design.md](tree-rewrite-design.md) — **design,
  implemented; superseded for current work by the tree-architecture set
  below.**
- [tree-shape-problem-statement.md](tree-shape-problem-statement.md) —
  **problem statement** the whole tree-architecture set answers.
- [tree-architecture-research.md](tree-architecture-research.md) —
  **research.** Why the canopy read as a mass instead of a tree.
- [tree-extension-biology.md](tree-extension-biology.md) — **research.**
  How a real tree keeps extending and what stops it.
- [tree-procedural-prior-art.md](tree-procedural-prior-art.md) —
  **survey.** How other systems build trees; converges with the biology
  pass.
- [tree-extension-audit.md](tree-extension-audit.md) — **measured audit.**
  Why growth actually stops, instrumented.
- [tree-diagnosis-review.md](tree-diagnosis-review.md) — **adversarial
  review.** Why the original tree-shape diagnosis did not hold up.
- [tree-architecture-variety-review.md](tree-architecture-variety-review.md)
  and
  [tree-architecture-variety-review-verification.md](tree-architecture-variety-review-verification.md)
  — **review + second-pass verification** of `plant-substrate-v2`.
- [tree-architecture-implementation-plan.md](tree-architecture-implementation-plan.md)
  — **plan of record for the `plant-substrate-v2` line; merged.**
- [plant-substrate-v2-design.md](plant-substrate-v2-design.md) — **design,
  implemented; merged.**
- [plant-species-authoring.md](plant-species-authoring.md) — **live
  guide.** What authoring a second plant species actually needs.
- [root-blob-and-uptake-surface-2026-08-23.md](root-blob-and-uptake-surface-2026-08-23.md)
  — **measurement, no mechanism.** Sizes the owner's "a root cell not
  touching soil cannot benefit the plant and has a cost" proposal before it
  is built: a third of every root system is walled in, but the share does
  not rise with mass, so the cost is a flat tax rather than a brake. The
  argument that survives is the unpriced 51-79% per-plant contact spread.
- [plant-economy-rederivation-2026-08-23.md](plant-economy-rederivation-2026-08-23.md)
  — **shipped, package P2.** The single economy re-derivation: superlinear
  maintenance respiration on `q_peak`, night income, the root-blob economy
  (a root cell not touching soil earns nothing and costs something),
  anchorage as what root investment buys, and whole-plant die-back. Carries
  the paired eight-seed before/after at two horizons — a quarter smaller,
  foliage share up, 8 of 8 founders still establishing — and, more usefully,
  the **six mechanisms it built, measured and withdrew**, including the
  per-cell die-back that read a spanning-tree artifact as biology and the
  §U solvency gate that produced a death spiral. §7 and §9 are the two
  negative results: adult mortality has a cause that fires and still kills
  nothing, and selection throughput moved the wrong way.
- [world-flora-sowing-2026-08-23.md](world-flora-sowing-2026-08-23.md) —
  **implemented.** Worldgen sows four woody species instead of the
  hardcoded `"tree"`, by weights over terrain facts that already existed;
  the palette arithmetic that forced both material palettes longer; and the
  decision to sow `creeper` while leaving its superseded root knob to
  lane P. Carries the measured terrain spreads the niche bands were cut
  from, and **§6 is item B1's answer**: shallow-fibrous against
  deep-fibrous is reachable on genome slot 5 alone, measured, with the
  sign of the axis the opposite of the obvious reading.
- [grass-sowing-and-divergence-2026-08-23.md](grass-sowing-and-divergence-2026-08-23.md)
  — **implemented.** The grass half of A1 and the whole of A4. Grass is sown
  as its own layer rather than a fifth woody species, weighted by the
  *unclamped* woody sum — the clamped one saturates at p10, which is the
  measurement that chose the rule. Paired against main, all four woody
  species come out bit-identical. The second half is
  `examples/divergence.rs`, the two-patch instrument
  `physical-trees-design-2026-08-23.md` §11.6 is waiting on: same founders,
  one axis, scored on root:shoot and slenderness, with an exact
  identical-patch control and §8's list of what it takes to point it at wind.
  **§11 is the card-design postmortem** — the first review card was a null,
  and counting rather than looking found why: the rendered window held 125
  grass cells against 7,853 woody, while its `meta` honestly quoted the
  whole-world total. §13 is the handoff, including what W4's exposure work
  has to provide before the instrument can be aimed at wind.

The genome and appearance set, merged from `plant-substrate-v2` /
`plant-genome`. Two of its claims did not survive contact with main's
field rework — see `open-bugs-handoff.md`.

- [plant-genome-design.md](plant-genome-design.md) — **design, signed off
  2026-08-18, implemented.** The positional genotype slot map; §8a is the
  measurement behind the primed-site root repair.
- [plant-genome-implementation-handoff.md](plant-genome-implementation-handoff.md)
  — **handoff.** What the genome work left for the next session.
- [plant-genome-handoff.md](plant-genome-handoff.md) — **handoff**,
  earlier than the above and superseded by it for anything still open.
- [plant-genome-review-request.md](plant-genome-review-request.md) —
  **review request.** The questions put to the owner about the slot map.
- [root-morphology-findings.md](root-morphology-findings.md) —
  **findings.** Why root morphology is inexpressible through the
  architectural knobs, and that thickening is the reason.
- [roots-and-breakage-handoff.md](roots-and-breakage-handoff.md) —
  **handoff.** Roots and breakage, written before the genome work landed.
- [branch-angle-and-the-width-bound.md](branch-angle-and-the-width-bound.md)
  — **measured study.** Branch angle, the straightness budget, and the
  width bound they ran into.
- [genetic-variability-study.md](genetic-variability-study.md) —
  **measured study.** How much a genome actually moves an individual;
  the spread that makes single-run comparisons here unsafe.
- [plant-appearance-design.md](plant-appearance-design.md) — **design.**
  Why relabelling a cell cannot move a silhouette that texture and colour
  set; the report behind `CLAUDE.md`'s "ask which *pixels* a lever moves".
- [plant-night-session-handoff.md](plant-night-session-handoff.md) —
  **handoff.**
- [plant-project-review-2026-08-23.md](plant-project-review-2026-08-23.md) —
  **review + proposed queue.** The whole plant record read together after the
  2026-08-22 merge and the "one big mass" verdict; revised same-day on the
  owner's direction to lead with the evolution framework and root
  differentiation; the stale-record list; the finding that worldgen sows only
  `"tree"` and moss.
- [plant-implementation-split-2026-08-23.md](plant-implementation-split-2026-08-23.md)
  — **execution plan** for the review's queue: three parallel lanes (plant
  core, structural verbs, world & species data), one session per package,
  with paste-ready briefs; successor to `plant-work-split.md` in role.
- [plant-morphology-reach-2026-08-23.md](plant-morphology-reach-2026-08-23.md)
  — **design note, answering a direct owner question.** Can this substrate
  reach a sunflower, a tomato, a climbing vine? Yes: the random walk is the
  variation mechanism, not the ceiling — the four missing primitives are
  organ cell types, determinate axes, rosettes/whorls, and a climbing
  tropism with attachment as data. Three open owner calls in §7.

## Creatures and ecology

- [creature-direction.md](creature-direction.md) — **direction agreed
  (2026-08-17).** Cell-chain ants, the caged brain, the heritable genome;
  decision record plus implementation plan.
- [creature-evolution-plan.md](creature-evolution-plan.md) — **plan,
  S1–S4 implemented (merged 2026-08-23).** The staged route from a scripted
  ant to an evolving one: the 584-slot genome, food worth living on the
  material rather than on the eater, corpse worth in `Cell::aux`, and the
  edible forest floor. Its "As built" notes carry the measurements; every
  S4 number in them predates the litter merge and is superseded by it.
- [creature-review-2026-08.md](creature-review-2026-08.md) — **review +
  proposed plan, written the day S1–S4 merged.** Where the creature line
  stands, the decisions never posted to the queue (E5, the abundance dial),
  and the re-prioritised to-do list: gates first, the two meat-accounting
  holes before S6, traffic/range as new work, the canopy as an S7 option.
- [creature-implementation-handoff-2026-08.md](creature-implementation-handoff-2026-08.md)
  — **execution plan for the review above, written to be run cold.** Ten
  work packages with file anchors, steps, measurements and landing
  checklists; the scope guard on what must not start before the owner's
  verdicts (S6, S7's larder, new channels).
- [foraging-range-measurement.md](foraging-range-measurement.md) —
  **measured record, instrument landed via `da252dc`;** §0 and §5 corrected
  on landing, **§3 corrected 2026-08-23** by WP-9 arm 1's re-test. Why
  `nest_visits` counted loitering and what replaced it: the `forage_reach`
  profile, `FORAGE_TRIP_MIN` derived from a sessile control, the 19-cell
  bubble, and the litter-in-the-canopy finding with the owner's call and the
  paired table it produced. §3's correction records that the probe's 55-ant
  scene plants at 2-cell spacing — the recorded gridlock — so its "`>=32` at
  zero" figure describes that scene rather than a founded colony.
- [stigmergy-research.md](stigmergy-research.md) — **research,
  implemented.** Deposit → diffuse → decay → follow; the ant colony is
  built on it.
- [population-dynamics-research.md](population-dynamics-research.md) —
  **research (Report D of four).** Why the ecology will go extinct and
  what prevents it; §7b answered by ecological-lod-design.
- [ecological-lod-design.md](ecological-lod-design.md) — **recommendation,
  not settled.** How an ecology survives a world that is not simulated
  (off-camera catch-up).
- [plant-evolution-design.md](plant-evolution-design.md) — **design, all
  nine §8 calls signed off 2026-08-19; partly implemented.** The plant
  ecology: litter, decay, grass and the creeper; §4a's register holds the
  probe verdicts and the do-not-retry notes.
- [plant-implementation-plan.md](plant-implementation-plan.md) — **the
  executable split** (WP-A/B/C/D/E/F) the design above is delivered
  through.
- [plant-work-split.md](plant-work-split.md) — **work split.** How the
  plant queue was divided between concurrent sessions.
- [grassfire-and-the-desert-2026-08-23.md](grassfire-and-the-desert-2026-08-23.md)
  — **W2: E5 shipped, E6 is a decision card awaiting the owner.** Why the
  grassfire did not spread (a sward that looks continuous is 71 separate
  4-connected islands, and contact ignition reaches one of them) and why
  `MOISTURE_IGNITION_RESISTANCE` measured as inert (its channel reads
  exactly 0.000 at 96.8% of fuel cells at *every* ground wetness, because a
  field block holding a `Plant` cell is `blocked` and never diffuses). Ships
  a flame body, a fuel-wetness gate, and `examples/fire_probe.rs`; costs the
  three §X desert levers, two of which have changed since the record.

## Worldgen and world

- [worldgen-design.md](worldgen-design.md) — **direction agreed,
  implemented** (`src/worldgen/`). The M10 redesign: 2D play through 3D
  coarse worldgen.
- [prior-art-worldgen-slicing.md](prior-art-worldgen-slicing.md) —
  **research.** A 2D world cut from a 3D one, elsewhere.
- [weather-handoff.md](weather-handoff.md) — **handoff; weather has since
  shipped** (`weather.rs`, wiki). Check its open items against the wiki
  page's "not here yet" list before trusting either.
- [worldgen-erosion-design.md](worldgen-erosion-design.md) — **design;
  implemented** (`worldgen/erosion.rs`). Plan-space erosion — the mechanism
  behind the mesas and benches nobody has complained about, and the one the
  owner's verdicts keep pointing back to.
- [field-settling-2026-08.md](field-settling-2026-08.md) — **settled, and
  rewritten twice.** What the coarse field costs per frame, and what
  decides it. Read the "measure it against the clock, not across it"
  section before quoting any field timing; both earlier versions of this
  file were wrong because they did not.
- [world-review-2026-08.md](world-review-2026-08.md) — **review.** A
  multi-lens pass over the generated world.
- [cave-beauty-review-2026-08.md](cave-beauty-review-2026-08.md) —
  **review.** Why the caves do not look good, ahead of rounds 5 and 6.
- [pass-interference-2026-08.md](pass-interference-2026-08.md) —
  **investigation.** Which worldgen passes overwrite each other.
- [world-scale-handoff.md](world-scale-handoff.md) — **handoff; start
  here for the 4x world.** What round 7's performance work landed, the one
  target it missed and why, the question waiting on the owner, and Phases
  2-5 written to be picked up cold. Phase 2 is done; Phases 3-5 stand.
- [springs-in-generated-worlds.md](springs-in-generated-worlds.md) —
  **shipped; look still open.** Why no generated world had ever had a river
  (nothing placed one), the measured +2.645 ms/frame standing bill, and the
  three placement models — two of which were dead on arrival, one drowned by
  `ponds` and one buried in talus. Carries the owner's standing request to
  explore sources beyond cliff faces.
- [world-scale-phase-2.md](world-scale-phase-2.md) — **shipped.** The world
  at 8192x2560 and the caves scaled into it: what was re-derived, what
  deliberately was not and why the handoff's own list is not self-consistent,
  the three things that were silently broken at the new size, and the paired
  census Phase 3 should measure itself against. Carries the finding that a
  feature left at its old size in a 4x world loses *fourteen* times its
  significance, not four.
- [worldgen-round6-handoff.md](worldgen-round6-handoff.md) — **handoff.**
  What round 6 landed, what was ruled, and what the owner then rejected on
  the review cards. The starting point for rounds 7+.
- The round task files, one per worldgen round, each holding its own
  findings and measurements:
  [round 1](worldgen-implementation-tasks-2026-08.md),
  [round 2](worldgen-implementation-tasks-round2-2026-08.md),
  [round 3](worldgen-implementation-tasks-round3-2026-08.md),
  [round 4](worldgen-implementation-tasks-round4-2026-08.md),
  [round 5](worldgen-implementation-tasks-round5-2026-08.md),
  round 6 [caves](worldgen-implementation-tasks-round6-caves.md) and
  [formations](worldgen-implementation-tasks-round6-formations.md).

## Character

- [m9-gnome-character-plan.md](m9-gnome-character-plan.md) — **build plan;
  shipped** (`player.rs`, wiki/the-gnome.md). Historical.
- [felling-blockers.md](felling-blockers.md) — **investigation; §1
  superseded by the plant-line merge, §2 superseded by
  `physical-trees-design-2026-08-23.md`.** Why cutting a tree down was not
  in yet. §3's ordering still holds.
- [physical-trees-design-2026-08-23.md](physical-trees-design-2026-08-23.md)
  — **design; §8's T1 stage is built** (see the line below), the rest not.
  Sway, impact breakage and a tree that falls
  over, from the owner's "it reads as a tree disintegrating into dust".
  Carries the measurement that prices render-side sway at +8.0 ms/frame over
  a grown stand, and the deleted prototype that takes a fell from 1.7% to
  58% of the severed mass coming down as pieces. Supersedes
  `felling-blockers.md` §2. **§11 is a later addendum** — wind-throw, with
  roots as anchorage and slenderness as an independent failure mode, staged
  as T6. Its three scheduling calls are decided (the economy half moves into
  P2; wind geography is dispatched as W4; plasticity is built as a heritable
  reaction norm), and §11.6a establishes from source that the genome already
  inherits and mutates — the gate is the slot ceiling. **§11.5 is discharged:**
  W4 landed terrain-derived exposure, so the sheltered-valley outcome it called
  unreachable is now reachable, and the section carries what T6 inherits — a
  world 37% calmer per gust, and the sampling trap that makes an arbitrary
  frame read flat. The rest of §11 is unmeasured and unrendered.
- [physical-trees-t1-implementation.md](physical-trees-t1-implementation.md)
  — **built; does not meet its own acceptance bar, and §4f says why.** The
  build half of `physical-trees-design-2026-08-23.md` §8's T1 stage: the
  fragment ladder's floor, the 8-connected flood for organism tissue, the
  three debris tiers and the `log` material, `BodyCell::organism_id`. The
  ladder works and is confirmed by the owner in motion (1.7% → 99% of
  severed mass comes down as pieces); the settled pile still reads as dust,
  because **`leaf` is 56% of the tree's cells and every one becomes a
  `Powder`**, which no fragment ladder reaches. Also carries three defects
  the new material found that the design could not have known — including a
  fallen log anchoring the tree it fell off — and, in §4c–§4g, three rounds
  of owner review including two framing failures of the session's own.

## Open working files

- [open-bugs-handoff.md](open-bugs-handoff.md) — **open bugs.** Working
  reproductions, what has been ruled out by measurement. Read before
  touching a listed area.
- [dead-ends.md](dead-ends.md) — **live index.** 546 tried-and-reverted
  approaches, each with the condition its rejection depended on and where
  the full record lives. Grep your area's section before proposing or
  retrying anything in it; a revert adds its entry in the same change.
- [perf-lock-recovery-2026-08-24.md](perf-lock-recovery-2026-08-24.md) —
  **recovery record; acted on.** Three artifacts this index pointed at that
  existed on no remote branch, all found in untracked worktrees on one
  machine and now pushed. Also settles the `plant-branch-angle` question —
  merged, its ref simply never pushed — and records one thing nobody asked
  for: 50 uncommitted lines of `plant-appearance-design.md` §6a, rescued to
  `claude/plant-appearance-6a-recovery`.
- [water-phase-merge-plan.md](water-phase-merge-plan.md) — **merge handoff;
  the run it briefs has been done.** The prompt for merging the water-phase
  branch into the trunk: the measured conflict inventory, the files that
  auto-merge while changing meaning, and the shipped behaviours to check by
  their guards rather than by reading the diff. Kept because those traps are
  general — two of them (a key binding colliding in an auto-merged region,
  and both sides fixing one early-out differently) fired for real on the run.

## Elsewhere in the repo

- `../docs/future-directions.md` — **historical, written at M3.** Its
  premises (4-byte `Cell`, no scheduler) are two milestones stale; kept as
  the feasibility argument that shaped M12–M18.
- `../research/` — raw source material for M16 / M18 / M19, in good order.

## In flight — exists on an unmerged branch, not in this directory yet

When one of these merges, move its line into the sections above (docscheck
flags the mismatch).

- `performance-audit.md` — **recovered 2026-08-24**, branch
  `claude/perf-audit-recovery`. It was never on `perf-audit`, which is zero
  commits ahead of `main`; it was an untracked file in that worktree. The
  branch also carries the five harnesses its numbers came from and a patch of
  two instrumented source files left uncommitted. Not built, not a merge
  candidate as it stands — see
  [perf-lock-recovery-2026-08-24.md](perf-lock-recovery-2026-08-24.md).
- `measurement-under-contention.md` — **pushed 2026-08-24**, branch
  `origin/perf-lock` (`bdda4a9`), committed there along with `scripts/perf.sh`
  and 91 lines of CLAUDE.md that `main` does not have. Until it merges,
  `main`'s CLAUDE.md and the working checkout's are different files.
