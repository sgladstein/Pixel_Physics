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

- [why-changes-cost-so-much-2026-08-27.md](why-changes-cost-so-much-2026-08-27.md)
  — **method finding, from a live instance.** Why every change here seems to
  demand a global retune: most large levers have **no counterweight**, so
  every constant is calibrated against every other constant's current
  behaviour and any change reallocates a fixed budget. Costs are not a feature
  competing with features — they are what makes features composable. Also
  names the bias that compounds it (judging a change before its retune
  systematically rejects the changes worth making, ratcheting toward a local
  optimum the owner has already rejected), and proposes a `CLAUDE.md` rule
  rather than making one.
- [agent-communication.md](agent-communication.md) — **rule landed
  2026-08-29.** How sessions write to the owner, censused over 158 review
  cards, 347 image panes and 283 commit subjects: 56% of PR subjects need a
  document opened before they parse, no message in any corpus states a
  direction, and the one clean channel is the one with a required shape.
  Commissioned by the owner, who runs several sessions at once; produced
  `CLAUDE.md` §"Writing to the owner" and `scripts/plaincheck.py`. Records
  which channels were deliberately left alone, and why.
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
- [claude-md-recommendations.md](claude-md-recommendations.md) — **all
  thirteen landed.** The thirteen-recommendation review of `CLAUDE.md` as
  always-loaded infrastructure. 5, 6, 7 and 12 are approved and unexecuted.
  Only **rec 6** is provably unblocked: `load-share` is merged into `main`
  (and is now deletable clutter). **`plant-branch-angle` and `perf-lock`
  have no remote ref at all**, so recs 5, 7 and 12 are blocked on work that
  cannot be tested for merge — absence is equally consistent with "merged
  and deleted" and with "unpushed on one machine". One owner question
  settles all three; see `agent-documentation-audit-2026-08-24.md` §5b/§5e.
- [pr89-review.md](pr89-review.md) — **adversarial review of PR #89,
  2026-08-28.** Twelve findings against `agent-strategy.md` and its machinery.
  Three reverse a conclusion: §6's cache saving mixed premises across the two
  arms and had the wrong sign; `readguard.py` denied every `README.md` in the
  repo; the 28,000-token ceiling was enforced by nothing and its check went green
  on a 58%-over file. Findings 1, 2, 3, 5 and the record corrections are fixed;
  6-12 stand as recorded.
- [agent-strategy.md](agent-strategy.md) — **measured and partly executed
  2026-08-27.** Which to use — one session, parallel lanes, or a manager with
  sub-agents — and what enforces it once they are running. The axis that
  separates them is whether the agents' reading *overlaps*, not the topology:
  reading is **74%** of a measured agent run against 26% for the auto-loaded
  prefix. Carries the manager and worker brief templates, the session-lifetime
  rule (`files > 300`, not a token ceiling), and the finding that sub-agents and
  workflows default to a **5-minute** prompt cache while the main session gets
  an hour. Recommends the ~67,000-token `open-bugs-handoff.md` split and says
  why it did not do it.
- [agent-documentation-audit-2026-08-24.md](agent-documentation-audit-2026-08-24.md)
  — **findings; the mechanical half executed (`fbc10e6`), §5 awaiting an owner
  call.** The companion to the three above, asking the other question: not *is
  the documentation true* but *what does an agent pay to find what it needs*.
  Carries the corpus measurements (~1.08M tokens; the six routed documents are
  ~306k), the `CLAUDE.md` append-drift figure (5,475 lines added against 56
  ever removed) and its section budget, the bug register's open/closed split,
  and a re-run of the cold-agent benchmark above against today's tree. **§4b
  records the three identifier collisions in `open-bugs-handoff.md`, now
  resolved — §Z had been resolving to two different bugs in two different
  reports — §5a records that
  the repo version-controls none of its Claude Code configuration, and §5e that
  two in-flight reports and `perf-lock` exist on none of the 49 remote
  branches.**
- [pixel-physics-issues.md](pixel-physics-issues.md) — **mostly closed.**
  The twelve-issue backlog; nine-plus closed, #11 (slice-identifier on
  `ChunkCoord`) has a land-before-save-format deadline and #12 (grass does
  not spread) was owner-filed 2026-08-24.
- [concurrent-sessions.md](concurrent-sessions.md) — **living record.** The
  narratives behind `CLAUDE.md`'s "Working alongside another session" rules:
  the incidents, their measurements, and the forensics for recognising each
  again. Split out of `CLAUDE.md` by rec 5 — read it when a manoeuvre there
  has gone wrong, not before.
- [session-programs.md](session-programs.md) — **living.** The coordinator ↔
  lane protocol, moved out of `CLAUDE.md` 2026-08-25: how a coordinator
  reaches a lane (`SendMessage` fails; a poke-only trigger works), why a
  woken lane cannot reply, why the return path must be files, and the four
  failures that cost an evening. **Read it only if you are coordinating
  sessions or were spawned by one** — it applies to a minority of sessions
  and cost every one of them ~2,200 always-loaded tokens while it sat in
  `CLAUDE.md`.
- [measurement-under-contention.md](measurement-under-contention.md) —
  **evidence landed; the mechanism it designs was deliberately not.** Why two
  runs of a byte-identical binary disagreed 2.42x and reversed the
  serial/parallel ordering, and how busy a shared box actually is (8% quiet
  over 45 minutes). The machine-wide timing lock it specifies lives only on the
  unmerged `perf-lock` branch and is **not** in the tree — the two findings
  that generalise are `CLAUDE.md` rules instead. Read it before designing any
  timing harness; read its header before reaching for anything it names.
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

- [structural-reconvergence-design.md](structural-reconvergence-design.md) —
  **design, nothing built.** The scope for §S: converge the support field over
  what actually changed rather than over a box, why both withdrawn attempts
  failed the same way, and why the delay timer is nearly free while the
  convergence is not. Read with §S and `dead-ends.md`'s scheduler section.
- [arch-vs-lintel-measurement.md](arch-vs-lintel-measurement.md) —
  **measured 2026-08-26.** An arch really does outspan a lintel in this
  engine, with nothing added: **1.63x further at equal material**, and a flat
  roof needs **3.1x the stone** to hold the arch's widest span. Carries the
  two controls that make it trustworthy — a cell-count-matched lintel and a
  triple-thickness lintel — and the caveat that the engine models no lateral
  thrust, so the arch wins for a different reason than masonry does.
  Instrument: `examples/arch_probe.rs`.
- [design-load-telegraph.md](design-load-telegraph.md) — **design, nothing
  built.** Dust, creak and hairline cracks driven by `torque / capacity`, a
  continuous ratio the engine computes every frame and discards. The repo has
  no structural overlay and no audio at all, so a ten-second warning
  (`CHAIN_WINDOW_FRAMES`) is currently spent in silence.
- [design-props-and-shoring.md](design-props-and-shoring.md) — **design,
  nothing built.** The collapse delay exists so the player "can get supports
  in first", and there is no support to get in. A one-cell prop that adds a
  route to `support_count`, and why painting stone is not the same thing.
- [design-sounding-and-pillars.md](design-sounding-and-pillars.md) —
  **design, nothing built.** `Failure::at` already names the keystone and the
  engine throws it away; a light hammer swing that reports load instead of
  doing damage, plus ore placed in load-bearing rock. Also the note that
  `Cell::attached` is never regained, so worked rock is permanently weaker —
  an unused survival loop already in the data.
- [design-roots-hold-slopes.md](design-roots-hold-slopes.md) — **design,
  nothing built.** Corrects the obvious reading first: soil is a `Powder` and
  the support field cannot hold it, so the mechanism is `friction_angle`, not
  the support DAG. The feature is the *timing* — `rot_remains` means the bank
  fails after the trees are gone, not with them.
- [structural-support-model.md](structural-support-model.md) — **design and
  measurement, nothing built.** Sizes §S's sketched replacement for the
  support field. The coarse-layer half is confirmed within 1% (5,169 nodes,
  and the potential packs into the existing `u16`); the saturating-gradient
  half is falsified (a horizon of 64 strands 95.38% of body cells); and the
  error's *sign* is the other way round from the reading in circulation — the
  damaged region reads stale-**low**, not high, which makes it a load sink.
  Carries the instrument (`examples/support_census.rs`) and the cheapest
  falsifying experiment for what is left standing.

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

**Start here, not from the list below.** Plants are 42 of this directory's
110 reports and about **269,000 tokens** — no session reads them, and the
list is ordered by provenance (design / research / handoff), which is not
the question you arrived with. This table is: *what am I about to do?*

| If you are about to… | Read, in this order |
|---|---|
| **change how a plant grows** — candidate scoring, branching, tropism | `plant-substrate-v2-design.md` — the largest report here at ~29,800 tokens, so use its contents table: §7 (polarity) is 9,754 tokens on its own and §2 (growth mode) is 2,729 |
| **change the economy** — income, maintenance, what kills a plant | `plant-economy-rederivation-2026-08-23.md`. §7 and §9 are its two *negative* results, and it records six mechanisms built, measured and withdrawn |
| **make species look different from each other** | `plant-appearance-design.md` **first, before designing anything**. Three architectural levers were built, fired 46–2,750 times each, and moved nothing: a lever that relabels a cell cannot move a silhouette that texture and colour set |
| **author or change a species** | `plant-species-authoring.md` → `plant-genome-design.md` for the slot map. Slots are **positional forever** |
| **touch litter, the forest floor, or where a plant's mass goes when it dies** | `soil-accumulation-and-the-carbon-cycle.md` — it is a source with no sink, and the 5% yield only changes the slope |
| **change roots** | `root-morphology-findings.md` (what the engine structurally cannot express, and that thickening is why) → `root-blob-and-uptake-surface-2026-08-23.md` (sizes the cost before building) |
| **put plants into the generated world** | `world-flora-sowing-2026-08-23.md` (woody) → `grass-sowing-and-divergence-2026-08-23.md`, whose §11 is the postmortem of a review card that came back a null because the rendered window held 125 grass cells against 7,853 woody |
| **fell, cut or break a plant** | README's `Felling status` → `physical-trees-design-2026-08-23.md` §8 (T1, built) and §11 (wind-throw, not) |
| **work on evolution, selection or speciation** | `plant-evolution-design.md` → `genetic-variability-study.md` for how far one genome moves an individual |
| **know what is open, or what to pick up** | `plant-project-review-2026-08-23.md` §4 (the queue) → `plant-implementation-split-2026-08-23.md` (the lanes) |
| **measure anything** | `instruments.md` §Plants — seven harnesses already exist and their names do not say what they answer. `divergence` is axis-agnostic |

**The seven longest plant reports carry a generated contents table** with a
line number and a token count per section — `scripts/reporttoc.py`, gated by
`docscheck` 8b. They are 43% of the plant corpus by tokens, and the token
column is the point: it turns "read the report" into a priced decision.

Three things that are not on this table and apply to all of it: grep
[`dead-ends.md`](dead-ends.md) for the **mechanism** before proposing one;
read [`../wiki/plants.md`](../wiki/plants.md) for the bar the change is
judged against; and check this file's status line for any report before
trusting it — `scripts/docscheck.sh` now gates a report's own header
against this index (3b), against the branch it claims to live on (3c), and
an example against the fields the engine still has (5b), but only the index
carries *why* a report stands where it does.


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

**The plant-evolvability line (2026-08-26/27). Read the handoff first —
it carries the reading order, what is established, and an owner-caught
drift that two of these documents still reflect.**

- [plant-organs-handoff-2026-08-28.md](plant-organs-handoff-2026-08-28.md)
  — **handoff; read first if you are building the organ package (Phase 4).**
  What Phases 0–3 landed and what machinery they give you, the six owner
  decisions not to re-litigate, and seven traps each already paid for. Its two
  load-bearing findings: gate 1 says `becomes` and `lateral` — the fields a
  determinate axis and a truss need — took 34 mutations without one failure,
  while `child` killed 5 of 6; and **a label change has now failed to read five
  times**, so the organ's *material* is the load-bearing half of the phase, not
  a detail of it. Names the retune budget, including `shoot_cells`' two-sided
  effect on the reproduction economy that no other report records.
- [plant-fate-viability-2026-08-28.md](plant-fate-viability-2026-08-28.md)
  — **gate 1, measured, and it passes.** 92% of effective point mutations to a
  species' production rule still produce a plant that establishes and breeds,
  against §7a's literature prediction that most structural mutations are
  nonviable. **The shape matters more than the rate**: every failure was a
  `child` mutation on a frontier type — the only way to kill the plant is to
  destroy the frontier — while `becomes` and `lateral` took 34 mutations
  without a single failure. Those two are exactly the fields a determinate
  axis and a truss need, so the organ work is aimed at the tolerant half.
  Records two instrument bugs caught by controls rather than by inspection,
  one of which would have published a decisive, false 0%.
- [plant-organs-2026-08-29.md](plant-organs-2026-08-29.md)
  — **built: Phase 4, the organ package.** Flowers and fruit as cell types with
  their own materials, a determinate axis that terminates in one, and a carbon
  price on building them, plus two authored habits (`herb`, `scrambler`). The
  materials came first, deliberately: a label change has failed to read five
  times and the one lever that ever read changed material. Two accounts, and
  the split is the useful finding — **construction** is charged at the decision
  from the acting cell, while **ripening** had to move to the reproductive
  budget because `allocate_to_frontier` makes an organ a permanent *donor*, so
  a flower charged against its own carbon can never set: 35 flowers against 2
  fruit, where the clocks predict 58. Records four failures caught by looking
  rather than by a number, including a turgor bound that cut the axis short of
  its own metamer count.
- [plant-heritable-fates-handoff-2026-08-29.md](plant-heritable-fates-handoff-2026-08-29.md)
  — **handoff; read first if you are continuing the plant-evolution line.**
  The production rule is heritable now: every organism carries its own
  `FateGenome`, founded from its species file, read ahead of it, and
  copied-then-mutated when a seed is borne — so a lineage can move its
  developmental program, which nothing could do before. The operator is the
  flexible one by owner's call (retarget / recondition / insert / delete), and
  **only retarget has a viability gate**; the rate is a guess; and throughput
  is still the blocker, since a tree reaches generation 1 in 8 of 8 seeds and
  never more. Its §4a is the one to read before writing any guard here: with
  the mechanism disabled outright the two obvious tests stayed **green**,
  because a founder and its species agree by construction whichever table was
  read.
- [plant-evolvability-handoff-2026-08-27.md](plant-evolvability-handoff-2026-08-27.md)
  — **handoff; read first when picking this line up.** Reading order for the
  six documents, what is actually established, and **§3: an owner-caught
  drift** — the session slid from *build the missing machinery* to
  *characterise the machinery we have*, which are different projects. The
  morphospace census answers the second and is **not on the critical path**
  to the first; its honest value is hedging one reviewer's challenge and
  building the descriptor set the machinery work will need anyway. Also: why
  founder allele variance is not a prerequisite for a sampling harness, and
  why a disturbance process must be a neutral hazard rather than an age cull.
- [plant-phototropism-lateral-2026-08-27.md](plant-phototropism-lateral-2026-08-27.md)
  — **finding; change built, proved, and NOT landed.** `phototropism_dir`'s
  codomain held no horizontal component, so `light_weight` was inert *by
  construction*. The repair `dead-ends.md` prescribes was built and its guard
  proved non-blind — then withdrawn, because it takes tree reproduction to
  **zero**: the authored `light_weight` values (up to 0.6) were calibrated
  against a lever that could only say "up", and given a real 2D direction they
  steer plants sideways until they never reach `seed_maturity`. The working
  patch, a committed 6-seed baseline sweep, and the re-derivation the fix
  needs are all banked for the next session.
- [plant-equilibrium-costs-2026-08-27.md](plant-equilibrium-costs-2026-08-27.md)
  — **audit: does every lever have a cost and a benefit?** Every `Behavior`
  field and `SpeciesDef` scalar traced to its consumer and sorted into four
  verdicts — *both* (self-limiting), *benefit with no cost* (runs away, held
  by a hand-placed fence), *cost with no benefit* (a pure tax that detonates
  the day its benefit is switched on), and *neither*. **The mechanism of the
  global retune, in one sentence: the engine prices tissue upkeep and almost
  never prices tissue construction**, so a morphology lever's cost is not
  charged where the decision is made but deferred into one pooled maintenance
  bill that every other constant is calibrated against. Leaves and secondary
  thickening — the two largest tissue producers — are both free to build and
  say so in their own comments. **Two corrections to the survey's §4a,
  measured over four world seeds:** `turgor_source` is not free, it is
  *unrewarded* — it costs 0.5 → 1.3 in bill-to-income and 200x in starvation
  shedding while nothing reachable moves — and at crown-contact spacing
  (`trees=24 width=512`, no build needed) the same two arms separate cleanly
  on who survives at all, 17–23 established of 24 against 11–16. Also: why
  stand seed totals can never have been the signal (pinned by world width);
  fences against prices, with `turgor_taper` as the in-repo model for a
  graded stop; the unnormalised direction score as a second retune mechanism
  that is not costs at all; and why a costs pass buys equilibrium but not, on
  its own, diversity. **§10 carries three owner decisions taken 2026-08-27**
  — charge construction *at the decision* (and the biology behind it: the
  engine is missing growth respiration entirely, and has wood inverted,
  expensive to keep and free to build where biology is the reverse); the
  three reproduction options, with the finding that seeds and growth draw
  from **separate accounts** today so `seed_cost` is not a price at all; and
  that wood density should relocate onto the material rather than stay a
  named locus.
- [plant-heritability-survey-design-2026-08-27.md](plant-heritability-survey-design-2026-08-27.md)
  — **method, not results.** How to decide *which authored parameters should
  become heritable* by measurement rather than by a fifth design opinion:
  point `plant-species-authoring.md`'s existing lever-table method at the
  ~25-30 authored constants that set phenotype and are not heritable today.
  Carries the two criteria — it must move the phenotype **and** have a
  counterweight, since a free lever made heritable produces uniformity — plus
  both required controls and the traps.
- [plant-recruitment-measurement-2026-08-27.md](plant-recruitment-measurement-2026-08-27.md)
  — **measurement, 16 paired runs.** Overturns the three-way review's
  unanimous "nothing can evolve yet": **grass reaches generation 2 in 7 of 8
  seeds; tree reaches generation 1 in 8 of 8 and never more**, despite
  setting 5-8x more seeds. Fecundity is not the bottleneck, establishment is.
  Also: the 4,095-slot ceiling is at 0.7-0.9% with 0 refusals, and branch
  angle and internode are `1` in every morph of all 16 runs — review C's
  monomorphism finding confirmed on live data, and shown to survive a working
  generational loop. Logs in `Reports/data/recruitment-2026-08-27-*`.
- [plant-evolvability-three-reviews-2026-08-27.md](plant-evolvability-three-reviews-2026-08-27.md)
  — **review synthesis, disagreements preserved.** Three reviewers with
  different mandates, two firewalled from the design note. Unanimous and from
  two independent measurements: **generation depth is 1 and inherited-genome
  establishment is 0**, so nothing can evolve yet and every genome question is
  downstream of turnover. They split on architecture: A and B call it spent
  (B proposes an organ-allocation budget instead), C shows the claim is
  untested because `plant.rs:718-719` hardcodes four of six discrete loci
  monomorphic in every founder, and the one architectural positive is buried
  under a stale "NOT merged" header. Cheapest next measurement, ~20 min on an
  existing binary: grass establishment, never measured.
- [plant-evolvability-facts-2026-08-27.md](plant-evolvability-facts-2026-08-27.md)
  — **facts, not conclusions.** Verified `file:line` ground truth for the
  plant-evolvability question, written so reviewers spend budget on judgement
  rather than re-deriving engine facts, and so no reviewer takes another's
  word for anything. Deliberately carries no recommendation. Corrects three
  claims made in the 2026-08-26 note and its review (the `ByOrder` field
  count, the "two dispatch sites" blast radius, and the identity guard's
  supposed blindness).
- [plant-morphology-evolvability-2026-08-26.md](plant-morphology-evolvability-2026-08-26.md)
  — **findings note, answering a direct owner question**; successor in role
  to the reach note above. Can those forms *evolve*, or must they be
  authored? The deciding question is what the genome *holds*: a parametric
  genome (today) buys a blob around whatever ancestor was authored however
  many loci it gets, while a **developmental** genome — the `ByOrder`
  production set made heritable and structurally mutable — is where novel
  body plans come from. Records the three structural stops, why the engine
  is already an L-system, the Bornhofen/Ochoa precedent from
  `plant-simulation-research.md` §7, how far creature decision **D4**
  transfers (one objection of three — plants are asexual, so no crossover to
  protect), and the clade-as-inventory / program-as-genome split. §5a
  withdraws its own first draft's loci recommendation and says why. Four
  gates in §7, three open owner calls in §8.
- [soil-accumulation-and-the-carbon-cycle.md](soil-accumulation-and-the-carbon-cycle.md)
  — **yield landed 2026-08-27; the sink is not built.** Why trees were being
  buried in their own leaf litter: a plant fixes carbon out of light into a
  solid cell, litter rotted 1:1 into permanent soil, so the floor is a source
  with no sink and has no equilibrium at any rate. Carries the real-world
  account (most of a leaf's mass leaves as CO2 — 1–10% humification, plus ~2
  orders of magnitude of volume collapse) and §4's ranked menu for an actual
  fix, of which **bioturbation is the recommendation**. Read §4's dead-end
  warning before building the obvious "litter enriches the soil" version,
  and §5 before trusting any soil census: the global count has the *wrong
  sign* for the first 60k frames because root growth eats soil, and over
  212k frames the two yields bracket the defect rather than fixing it —
  1.0 accumulates and buries, 0.05 depletes to half the world's soil and
  stalls.

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
- [frame-cost-audit-2026-08.md](frame-cost-audit-2026-08.md) —
  **measurement of record for whole-frame cost.** The first attribution of
  `App::update` as a whole, at the shipped 8192x2560 size: 30.1 ms amortised
  with nobody playing, 79% of frames over the 16.6 ms budget. Reranked the
  performance backlog — issue #2 down (the sweep is a tenth of the frame),
  `plant::step_organisms` up (a quarter of it, and in no plan). Read it
  before quoting any per-phase cost; `field-settling-2026-08.md` remains the
  record for the field's *internal* split and is not contradicted here.
- [world-review-2026-08.md](world-review-2026-08.md) — **review.** A
  multi-lens pass over the generated world.
- [cave-beauty-review-2026-08.md](cave-beauty-review-2026-08.md) —
  **review.** Why the caves do not look good, ahead of rounds 5 and 6.
- [pass-interference-2026-08.md](pass-interference-2026-08.md) —
  **investigation.** Which worldgen passes overwrite each other.
- [resolution-step-2026-08-29.md](resolution-step-2026-08-29.md) —
  **handoff; start here for the resolution step**, which
  `world-scale-handoff.md` names as "higher resolution later". The render
  was three times the simulation and nobody was watching it, because
  `scale_probe phases=1` times `App::update` and `Renderer::draw` is not in
  it; it is now parallel and a **1024x640 redraw costs 18.6 ms against the
  old 512x320's 21.2 ms**, so doubling the framebuffer is paid for. Carries
  the owner's verdict on which of the two readings of "resolution" is meant
  (the apparent scale must not change), the measured finding that **cutting
  stone depth buys load time and memory but not frame rate**, and the 261
  cell-valued sites across 29 files that the content half has to move.
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

## Licensing and distribution

- [dependency-license-audit.md](dependency-license-audit.md) — **settled
  2026-08-24.** Why the project's own MIT licence was reversed to proprietary
  (MIT grants everyone the right to *sell* it, and the game is meant to be
  sold), what that reversal cannot undo, and the 301-package third-party
  inventory: all permissive, no copyleft. §3 covers attribution —
  `THIRD-PARTY-NOTICES.txt`, which **must ship with the executable** — and why
  compatibility and attribution are separate obligations that get conflated.
  **Read this before touching `LICENSE` or `Cargo.toml`'s licence fields** —
  restoring MIT would look like housekeeping and is not.
  `scripts/licensecheck.sh` and `scripts/notices.sh --check` are the live
  re-checks.

## Open working files

- [open-bugs-handoff.md](open-bugs-handoff.md) — **open bugs.** Working
  reproductions, what has been ruled out by measurement. Read before
  touching a listed area.
- [dead-ends.md](dead-ends.md) — **live index.** 546 tried-and-reverted
  approaches, each with the condition its rejection depended on and where
  the full record lives. Grep your area's section before proposing or
  retrying anything in it; a revert adds its entry in the same change.
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
flags the mismatch). Keep prose in this section free of backticked report
filenames that already exist in `Reports/` — check 4 matches any such name
here and cannot tell a listing from a passing mention.

**All three former entries here were recovered and pushed 2026-08-24** — they
had been in untracked worktrees on one machine, reachable from no remote. The
recovery record is `perf-lock-recovery-2026-08-24.md` on
`claude/perf-lock-recovery` (`369ec1d`). The old entries were wrong in three
ways worth remembering: perf-lock's files were already committed, not
untracked; the branch `perf-audit` is **zero commits ahead of `main`** and
never held the report at all; and a worktree's name is not its branch's name
(plant-branch-angle lives in a worktree called `plant-crown`).

- ~~`origin/perf-lock`~~ — **RETIRED 2026-08-25, not landed.** Its report is
  now in this directory (indexed above) and its two generalising findings are
  `CLAUDE.md` rules. What stays unlanded is the machine-wide timing lock
  itself: 1,546 insertions across 11 files including a 317-line rework of the
  CI-gated `ascii` example, from a branch 547 behind scoring `BxF 6,017`. Its
  premise is several sessions contending for four cores, which cloud
  containers do not do. The branch is left standing as the implementation
  should the condition return; the decision and that condition are in the
  report's header and in dead-ends under `other`.
- The frame-cost audit and four measurement harnesses —
  **`origin/claude/perf-audit-recovery`** (`f7bebae`). A rescue, not a merge
  candidate: not built, not checked, its worktree's `src/` changes held as a
  `.patch` rather than committed, and its five new `examples/` binaries have
  no rows in the instruments index yet (docscheck check 5 will fire).
- §6a of the plant appearance report —
  **`origin/claude/plant-appearance-6a-recovery`** (`8c35cff`). 50 lines that
  were on no branch anywhere: the `thicken`/`stem_run` axis bug and a re-swept
  `shade_death`.
