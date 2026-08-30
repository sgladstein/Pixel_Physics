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

## Which game a report is about

**Two games run on this engine** — the outdoor sandbox and the evolution lab
(`two-games-one-repo-2026-08-30.md`) — and this index is one of the two
routing layers every session is sent to before it opens anything. So each
section below is tagged:

| tag | meaning |
|---|---|
| **`engine`** | **shared by both games, and most of this index.** That sharing is the whole argument for one repository: plants, creatures, fire, liquids and the sweep are the same code either side |
| **`outdoor`** | the lab builds no scene that reaches it — it has no rock, no worldgen and no gnome |
| **`lab`** | the sealed box, which the outdoor game has no equivalent of |

**It is a hint about where your time goes, not a rule about what you may
read**, and it is deliberately coarse: a whole section at a time, no file
moved, reversible in one commit. Getting one wrong costs a reader one
section. The alternative considered and *not* taken was moving the files into
`Reports/{engine,outdoor,lab}/`, which would take five non-recursive
`Reports/*.md` globs in `scripts/docscheck.sh` with it — including the one
that enforces this index — and would do it **silently**.

**`dead-ends.md` and `open-bugs-handoff.md` carry no tag on purpose.** Both
are grepped rather than read, both are explicitly cross-subsystem, and the
property that makes them work is that a mechanism tried on plants is findable
by somebody about to try it on creatures.

## Method and architecture — read these first  ·  `engine`

- [two-games-one-repo-2026-08-30.md](two-games-one-repo-2026-08-30.md) —
  **proposal, not yet built; revised after an adversarial review found its
  central recommendation was a no-op.** Answers whether two games on one
  engine means everything shared or everything separate. Neither — but the
  first draft proposed moving `CLAUDE.md`'s evidence into a `.claude/rules/`
  file with no `paths:` frontmatter, **which loads at launch exactly as
  `CLAUDE.md` does and saves nothing**, the same failure it had correctly
  named for `@imports` two paragraphs earlier. **And `contextbudget.py` would
  have certified it**: it counts `CLAUDE.md` only, so the CI gate would have
  gone green for a change that moved no context — this file's worst-recurring
  failure arriving *prospectively*. The revision carries the four places the
  evidence could actually go, the precedent this repo already set
  (`session-programs.md`, moved out for exactly this reason), two live Claude
  Code bugs that would void the saving **inside a git worktree, which
  `CLAUDE.md` mandates**, and the correction that `Reports/` is neither flat
  nor 14 MB of prose. Names the largest miss: **`README.md` is 71,561 tokens,
  the biggest document in the repo, overwhelmingly outdoor, and every agent is
  routed to it first.**
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

## Destruction and structure  ·  `outdoor`

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
- [explosion-menu-guide.md](explosion-menu-guide.md) — **reference, for the
  owner rather than for an agent.** What every row of the `O` -> EXPLOSION
  panel does, measured on one scene so the rows can be read against each
  other, plus the five charge types and what each spends its energy on.
  Written from the report that the panel was "too complicated"; it also
  records why nothing was deleted from it.
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
  `F12`, /4 the default — and *extended 2026-08-29* after play found the
  shipped model drew a broken-open hilltop black.** Its own Finding 2 table
  holds the defect: a 64-wide pit's floor and a 1-wide shaft scored 0.023
  against 0.010, because distance from a lit cell cannot tell a hole from a
  hollow. Read *What play found wrong with it* at the bottom first — the
  second term, the eight-ray fan, and what it cost.
  Below that, the original round measures the candidates for the
  open-cast-dig case (`examples/sky_light_probe.rs`): why `field.rs`'s light channel
  cannot drive it, why seeded propagation can, why block size 4 rather than
  `FIELD_SCALE`'s 8, and — tested later — why a stored incrementally
  maintained field is *not* worth it. Two of its own claims were wrong and
  are corrected in place at the bottom.
- [prior-art-underground-lighting.md](prior-art-underground-lighting.md) —
  **research.** How Terraria (a per-tile wall layer, then 0.91/0.56 light
  propagation) and Noita (a coarse blurred fog of war, no classification at
  all) answer "is this dark", and which of the two the open-cast-dig case
  needs. Its call — propagation, not a better boolean — was right and
  incomplete: propagation *by distance* is what `sky-light-design.md`'s
  2026-08-29 postscript had to add an aperture term beside.

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

## Liquids and granular  ·  `engine`

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

## Plants and trees  ·  `engine`

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
- [subpixel-rendering-2026-08-29.md](subpixel-rendering-2026-08-29.md) —
  **measurement + prototype.** The framebuffer does not have to be the cell
  grid: the window already gives every cell a 2x2 block of physical pixels and
  paints all four the same, and drawing the same world region into four times
  the pixels measures at **1.13x** the redraw (nine times: 1.32x), because the
  per-pixel work is under 10% of a full draw. Semantic sub-cell reconstruction
  — from *what the cell is*, which is why hqx/xBR cannot do this — and the
  ambient-occlusion and surface-normal terms that fall out of the same field
  for free. Carries the class rule (masses keep their per-cell grain, thin
  structures do not) and two interior-quilting bugs that a firing counter would
  have been green through.
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
- [plant-rule-drift-observed-2026-08-29.md](plant-rule-drift-observed-2026-08-29.md)
  — **§3d closed: a plant's production rule has been watched drifting.** Twenty
  coexisting rule tables in one `herb` stand at 45,000 frames, from a world that
  shipped with one, with all four operator signatures visible in a live
  population. Drift **plateaus** near 1% rather than accumulating. Returns a
  **null** on whether a drifted table costs a plant its establishment (3 of 385
  distinct plants against 4.5 expected) and says how underpowered that null is.
  Its §3a is worth reading on its own: the first version of that statistic
  pooled organism-*samples* and printed a headline finding of strong selection
  that was an artifact of counting persistent plants once per sample — caught on
  tidiness, two seeds at exactly 0.00%. Leaves one open discrepancy: observed
  drift runs **2.6x below** a per-birth model, with the settling measurement
  named (a mutation counter at `bear_seed_at`, whose return value is currently
  discarded). **That discrepancy is now closed — see the entry directly
  below, which finds the model at fault rather than the mechanism.**
- [plant-selection-teeth-2026-08-29.md](plant-selection-teeth-2026-08-29.md)
  — **does this world punish a plant that is worse? Yes, and gradedly.** The
  teeth-test: two genomes competing in **one** bed, 5 arms x 18 mirrored
  seeds, closing the gap the operator gate's §6 names. Built against a
  **false-negative** worry — measuring nothing in a world whose pressures are
  incomplete and recording it as "evolution does not work". The load-bearing
  row is `nobranch`: a plant that grows, flowers and sets seed loses **11
  points of the bed on 18 of 18 seeds** (p=0.0002), which is selection between
  two *living* plants; `lethal` at 0.0% is near-tautological and should not be
  quoted for that claim. Carries a **null that is a finding** — `norootbranch`
  reads 49.7%, p=0.86, and the bed is uniform field-capacity soil, so a root
  system has no scarcity to compete over. **States its own blindness up
  front**: the control's spread is ±9.3 share-points, so 18 seeds resolve ~7.5
  points and a 1% selection coefficient would need ~620 — the next experiment
  needs a frequency trajectory over generations, not more seeds. Its §2 and §5
  are the method value: the mirrored identical-arms control is **vacuous by
  construction** (exactly 50.0%, an algebraic identity), and three arms were
  silent before they were real — `lateral: None` is not "no lateral", and a
  herb shoot never places a lateral at all while its root does.
- [plant-water-scarcity-2026-08-30.md](plant-water-scarcity-2026-08-30.md)
  — **the teeth report's one untested prediction, tested and refuted: a dry
  bed does not bring root architecture under selection.** `norootbranch` reads
  **50.8%, 8 of 18 seeds, p=0.49** in a bed with **5.5x less plant-available
  water**, and 50.8%, 8 of 18, p=0.93 in a further bed where water genuinely
  limits — against 49.7%, 10 of 18, p=0.86 in the wet one; paired over shared
  seeds the beds differ by **+0.0 points**. Not an inert world — `nobranch`
  loses 13 points on **12 of 12** seeds and 10 points on **18 of 18** in those
  same dry beds. The three findings behind the null are the value. **A bed cannot be dried into
  drought**: every species' `Germinate` floor (`soil_water_threshold`) sits
  *above* the availability at which its own uptake becomes limiting — moisture
  246 against 191 for `herb`, 290 against 234 for `tree` — so below the floor
  you get an empty bed, and `tree` at 260 goes from 6 organisms to zero.
  **The lever that does bite is rooting volume**: same moisture, `soil=4`
  instead of 34, and water status falls **1.000 → 0.678** with uptake halved
  and the stand losing plants — a plant in a deep dry bed escapes downward
  into soil it has not drunk. And **root branching does not buy water here**:
  income is `rate x available` per *wet neighbour*, so contact with wet soil
  is what earns and in a drawn-down bed the bed sets it — the handicap costs
  23% of root cells and **3%** of uptake surface. Lands `sky=` and `bed=1` on
  `selection_arena`: a bed under live weather is rained on for **30% of the
  run** and ponds 74,674 units of free water, so "dry" needed a pinned sky
  before it meant anything. Its §5c names what would put roots under selection
  and why "wetter or drier" is the axis this rules out.
- [plant-mutation-counted-at-source-2026-08-29.md](plant-mutation-counted-at-source-2026-08-29.md)
  — **§4 closed: the 2.6x was the model, not a loss.** Counts fate mutations
  where they happen (`World::fate_mutation_rolls` / `_fired` / `_applied`) and
  rules out both candidate causes by measurement: the draw fires at **0.982%**
  pooled against a nominal 1.000%, and **96%** of draws change the genome. The
  gap is the comparison itself — the per-birth model needs **58%** of every
  mutation ever applied to be standing at the census, where an ordinary birth
  survives at **23%**; scaled by that mortality it predicts 62 against 77
  observed, missing in the *opposite* direction. Carries an **exact** control
  on the instrument: the new counter differs from `plant_probe`'s independently
  measured birth count by 16 on all three seeds, which is the founder count.
  Flags one thing to go and look at — seed 2 carries 2.3x more standing drift
  than its mutation count and mortality allow, the first sign in this programme
  of a mutation doing better than average rather than merely surviving. Also
  lands `FateLookup`, the runtime selector that makes the fallback fork
  renderable.
- [plant-throughput-herb-2026-08-29.md](plant-throughput-herb-2026-08-29.md)
  — **which species the evolution programme should actually run on, and the
  answer is `herb`.** Three seeds: deepest *established* generation 5, 7 and 3,
  with 88% of established plants carrying an inherited genome and 8,000–11,000
  seeds set per run, against `tree`'s 0-of-16 and `grass`'s best-ever 2. Put
  beside the operator gate it is the only species with **both** halves — a
  genome that can move (48% / 42% / 18% where tree and grass are 0% / 2% / 8%,
  because `Ripe` is the one condition `builtin_fate` does not backfill) and
  lineages that last long enough to move it. **Withdraws two standing
  conclusions**: *"run evolution experiments on grass"* (grass sets **zero**
  seeds on `main` today — `open-bugs-handoff.md` §1n, four scene controls) and
  *"the 4,095-organism ceiling is nowhere near binding"* (herb runs at 44–61%
  of it). Also records that `grass.ron`'s fate table is **byte-identical** to
  `tree.ron`'s, so switching to grass would never have bought mutability.
- [plant-fate-fallback-fork-2026-08-30.md](plant-fate-fallback-fork-2026-08-30.md)
  — **the fallback fork is decided and landed**: the owner answered *"No safety
  net"*, so `fate_for` reads the individual's genome and stops. A mutation that
  vacates a slot vacates it for real, which is what makes `delete` and
  `recondition` real operators; the gate report below is the baseline it is
  measured against. **Its §0 is the part to quote**: at the shipped mutation
  rate this is a **no-op** — 88,909 fate queries over 60,000 frames of `herb`
  with the net catching **0** of them, and `genome_drift` byte-identical
  between the old and new depths at both 0 and 10x. **Mutation volume**, not
  generation turnover and not the fallback depth, is the bottleneck — this
  line said "generation turnover (mean depth 2.04)" until 2026-08-30 and that
  attribution was withdrawn by the report itself in `ba3f723`, which corrected
  every other document and missed this one. The net's first bite is bracketed
  to `(0.1, 0.3]` by `plant-mutation-rate-2026-08-30.md`, below.
  **Withdraws one standing claim**:
  `builtin_fate` is *not* the absorber — at 90x all 1,305 saves went to the
  **species** layer and `builtin_fate` took 0; the two layers agree, which is
  why dropping the middle one measured identical. Also records why `moss`
  (empty fate table, 0 calls — it only `Divide`s) and `(RootTip, Node)`
  (unreachable at `plastochron: 0`) are safe, and that an emptied `Grow` slot
  makes a tip that **never retires** rather than one that cannot grow.
- [plant-mutation-rate-2026-08-30.md](plant-mutation-rate-2026-08-30.md)
  — **`FATE_MUTATION_CHANCE` re-derived, 0.01 → 0.30**, closing the item the
  fork report above leaves open. The old value was not low, it was **inert**:
  at 60,000 frames of `herb` the *whole log* is identical to the same world
  with mutation switched off — 873 live, 74 established, 5,858 births, same
  body sizes and slot means — while 45 mutations fired and **none of the 28
  individuals that carried one ever reached 20 cells**. The genome moved and
  no plant did. **The trade it was supposed to balance is nearly empty**:
  across 3 seeds and 7 rates, establishment, throughput and body size never
  consistently decline, *including at rate 1.0 where every birth mutates*
  (establishment there runs −13%, +13%, +28% — no sign). So the four-way
  trade collapses to "how much variation should a species carry", and 0.30 is
  the smallest rate that both puts variation in plants (29–40% of *bodied*
  plants, against **0%** at 0.01 on every seed and both budgets) and makes the
  owner's no-safety-net ruling non-vacuous — `GenomeOnly` and `Full` are
  byte-identical at 0.10 and differ at 0.30, bracketing the net's first bite
  to `(0.1, 0.3]`. **Its §5 is the method value**: a plausible 2x2 reported a
  **3x establishment penalty for drifted plants in a run whose stand was
  bit-identical to no-mutation at all** — confounded by age, kept in the
  harness with the caption rewritten, because the next reader would derive the
  same wrong number. Also records that `tree` is invariant to the entire
  ladder (generation 1, every mutant a seed that never germinates), so the two
  species do not want different rates, and corrects the stale attribution in
  the entry above.
- [plant-fate-operator-gate-2026-08-29.md](plant-fate-operator-gate-2026-08-29.md)
  — **all four mutation operators now have a viability gate**, closing §3a of
  the handoff below, and the answer is not the one its weighting hedged
  against. The three unmeasured operators are not dangerous — 46 of 46
  effective mutants lived — they are close to **inert**: on the woody base
  `delete` is 0% effective in 40 draws, `recondition` 2%, `insert` 8%. One
  mechanism explains it: `fate_for` falls back **per query**, so a slot a
  mutation vacates is refilled by the species table or the built-in rule, and
  first-match-wins shadows an insert that lands below an existing rule.
  `retarget` is the only operator that changes a rule *in place*. **Its §6 is
  the control that makes that a finding rather than a scene report**: the two
  extreme cells re-run at another world seed and 2.5x the frame budget, with
  the base stand moving 79 -> 206 seeds and `delete` not shifting by one
  mutant. Also: the
  harness had been measuring a **lookalike** of the operator, not
  `FateGenome`'s own — six replacement cell types on the woody base where the
  engine draws from eight — so the recorded 92% was about a mutation nothing
  performs. Consequence for the programme: `FATE_MUTATION_CHANCE = 0.01` is
  not the rate in effect.
- [plant-heritable-fates-handoff-2026-08-29.md](plant-heritable-fates-handoff-2026-08-29.md)
  — **handoff; read first if you are continuing the plant-evolution line.**
  The production rule is heritable now: every organism carries its own
  `FateGenome`, founded from its species file, read ahead of it, and
  copied-then-mutated when a seed is borne — so a lineage can move its
  developmental program, which nothing could do before. The operator is the
  flexible one by owner's call (retarget / recondition / insert / delete).
  **Its §3a is now closed** — all four are gated, see the report above, which
  also corrects this one's premise that the harness measured the shipped
  operator. The rate is a guess; and throughput
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

## Creatures and ecology  ·  `engine`

- [creature-motion-design.md](creature-motion-design.md) — **built
  2026-08-29; all four of §6's calls answered, §7's five guards green.**
  `BrainOutput::Impulse` ships: one verb, and the *body* decides what it
  does — launch speed is `sqrt(2W/m)` off cell count, descent is a drag law
  off the bounding box, and `ant_wide` and `ant_block` (same mass, 5x2
  against 3x3) launch identically and fall **2.3x** apart. E9's float limit
  came free with it. Motion is the axis nobody had measured:
  there is no jump, fly, swim or hop anywhere in the engine. Answers the
  owner's "how many impulse verbs" debate by asking of each candidate
  *decision or property?* — **five of nine need no verb at all**, because the
  ballistic physics already exists in `rigid.rs` (terminal velocity from
  weight-minus-buoyancy against drag, which is also E9's float limit already
  implemented). Recommends **one verb, one slot held with a stated condition
  for spending it**. **§4c reverses the report's own first recommendation**
  and now says *enlarge* the reserve, on the finding that `is_live_slot`
  gates on the live counts rather than the reserves, so the reserve is never
  drawn, mutated or evaluated — `live_slots()` stays at 268 either way and
  the whole cost is 2.1 MB at the population ceiling. §4d answers "can we
  name the held slot now": no, naming is what makes a slot live. Carries the
  principal risk: `step_chain` refuses gaps deliberately, after two attempts
  that put falls at 59–80% of all moves. **That risk was retired by not
  touching the walk at all** — the verb is a separate opt-in path, the
  shipped ant authors no weight into it, and falls per move on the foraging
  scene is unchanged.
- [creature-appearance-design.md](creature-appearance-design.md) —
  **design + measured study.** Why a two-cell ant cannot be found in a
  picture, and what does: extent is the only lever, the dark palette is
  already the best of the values tested, and a nine-cell pale body puts less
  on screen than the shipped two-cell dark one. The creature-side answer to
  `plant-appearance-design.md`.
- [creature-motion-decoys-2026-08-30.md](creature-motion-decoys-2026-08-30.md)
  — **measured study, and a qualification of the report above.**
  `creature-appearance-design.md`'s whole body-size case rests on `decoys`,
  which is computed on a **single still**, and a decoy is a rock edge or a
  leaf — something that holds still, while the animal does not. Adding the
  motion axis (`examples/motion_look.rs`: a decoy that does not change
  between two frames is not competing for the eye) finds the decoy field is
  **entirely static**: a body that moves has **0–2** competitors at every
  size from 1 to 16 cells, so a walking two-cell ant is already better off
  than a stationary sixteen-cell one, in every sky measured and on four
  seeds. Does not overturn the recommendation so much as split it — **22–42%
  of ants never move across a 384-frame horizon**, and for those the static
  ladder is the whole story, which is the owner's *"ants are mostly visible
  with there motion"* arriving as a number.
- [creature-birth-grant-2026-08-30.md](creature-birth-grant-2026-08-30.md) —
  **built and landed 2026-08-30.** `birth_grant` as a heritable slot, E14's
  `start_energy` cut (900 -> 200), and the measured finding that **the two
  together cannot make the shipped ant breed and no setting of either
  closes it**: the binding term is the 960-point body stamp, which is
  invariant to both, and cutting the budget lowers the bank ceiling faster
  than it lowers the bar. What E14 buys is not what it was authorised on:
  **`deaths` did not read "0 everywhere" before** — the uncut ant dies at
  36,000 frames and keeps dying, and the cut converts that unbounded
  run-down into an early cull that settles (§4a). Sharpens
  `creature-reproduction-economics.md` §3.6 and corrects the direction
  `ant.ron`'s own comment stated. **§6 superseded the same day** by
  `creature-gate0-births-2026-08-30.md`: the gap closed without the stamp
  term, because the "one mouthful" in this report's ceiling was a property of
  the feeding rule rather than of the animal. §2's arithmetic stands.
- [creature-gate0-births-2026-08-30.md](creature-gate0-births-2026-08-30.md) —
  **built and landed 2026-08-30. Ants breed.** The block was neither the
  economy nor the gut: `act` fed an animal only below its satiety line and
  made it carry everything after, so **the largest bank any ant could hold
  was the satiety line plus one mouthful** — 1,060 against a 1,041 birth cost
  and an 1,100 bud threshold. An animal short of a child's price now finishes
  the meal (out on the route only for a mouthful that pays by itself, at the
  nest for as long as the larder lasts), and `adjacent_food` returns the
  **best** neighbour rather than the first. With food on the ground the
  shipped ant at the shipped neutral gut reaches **generation 13**; the
  worldgen colony goes 0 -> 1 birth against `origin/main` in a paired A/B.
  **Supersedes `creature-birth-grant-2026-08-30.md` §6** — closing the gap
  did not need the stamp term after all. What still blocks the unfed lab bed
  is measured to the cell: **1,651 pickups, 4 deliveries, an empty larder**,
  isolated to `act`'s out-of-nest drop rule with a controlled probe and filed
  as a bug. Ships `best_offer` / `best_bite` / `peak_bank` and
  `examples/windfall_probe.rs`.
- [creature-body-extent-2026-08-30.md](creature-body-extent-2026-08-30.md) —
  **built and landed 2026-08-30.** The body is priced per cell at last:
  nothing in the cost path read `chain.len()`, so **E10's premise that
  "per-cell metabolic cost already prices a longer body" was false** and a
  longer body was strictly free — measured at a difference of *exactly zero*
  by injecting the old behaviour back into this change's own guard. Also
  ships `ShadeRule::Countershade`, the appearance report's §7 seam, off by
  default. **The finding that reframes the extent lever**: at the shipped
  seed and horizon **no chain above two cells leaves a living colony**, at
  the old flat bill as much as the new one and on a flat slab as much as on
  the world — so the collapse is upstream of both the pricing and the
  palette, and the blind A/B `creature-appearance-design.md` §6 asks for is
  held until it is understood. Prices the arms that report measured.
- [creature-chain-head-loss-2026-08-30.md](creature-chain-head-loss-2026-08-30.md)
  — **diagnosis, 2026-08-30. Closes `open-bugs-handoff.md` §R3, and the
  answer is neither of the two effects that entry named.** The colony above
  two cells never dies: `built - deaths = registry` exactly in every arm, so
  nothing was ever unaccounted for. A `Chain(n >= 3)` loses its
  `CellType::Head` marking — `body_after_step` can put one position in the
  next body twice when a head steps into its own tail, and `relocate_chain`
  writes a trailing Segment over the Head — so every instrument that finds
  an ant by looking for a head reports an empty world over a living,
  feeding, delivering population. **Cannibalism is ruled out with both
  controls**: `kinfood=off` is byte-identical to shipped, while
  `eatskin=on` moves `meat_lost` 0 -> 40,320, so the null is a real one and
  not a blind instrument. The `food in reach: ant 480` dump §R3 rests on
  never applied the kin gate, and three of its four entries are the
  animal's own tail. Placement is real but is the harness on the slab (a
  two-cell founder pitch: 28 bodies at `pitch=2`, **46** at `pitch=4`).
  **The extent lever is recoverable.** No fix attempted — `creature.rs` was
  another lane's.
- [creature-stamp-routes-2026-08-30.md](creature-stamp-routes-2026-08-30.md) —
  **priced, nothing built; a decision document.** The three routes past the
  960-point body stamp, each with a number against it, over 12 pre-registered
  seeds. **What decides a birth is one number — `ceiling - bar`** — and it
  governs regardless of mechanism: every negative margin gave *exactly zero*
  births and every positive one bred, with two arms sharing no mechanism but
  the same +99 margin breeding at the same rate. Three corrections to the
  standing account: economics §3 prices every route against a satiety line of
  **450** that E14 has since cut to **100**, which reverses two verdicts;
  **neither stamp route removes the stamp, both defer it** (480 against a
  220 bank), so **route 3 is the precondition for routes 1 and 2 rather than
  an alternative** — and **creatures cannot grow**, so both stamp routes need
  a verb the engine does not have. The recommendation is **none of the
  three first**: `fruit` (960) and `flower` (1,440) exist and no world
  contains one, because worldgen sows `creeper`/`shrub`/`conifer`/`tree` and
  the only fruiting species, `herb` and `scrambler`, **are never planted** —
  which answers the experiment economics §7 calls its cheapest. Route 2 is
  reported as **unpriced**, with the reason. **Corrected 2026-08-30 by
  `evolution-lab-gate-1-2026-08-30.md` §4.3: the recommendation does not work
  as written** — in a box that does flower and fruit the matched gut takes the
  margin to **+500 and still gives zero births**, because the flower stands 22
  to 40 rows up a stem and `windfall` never exceeds 1. A *reach* problem, which
  is the failure case §5 names; the margin model holds, and what it does not
  contain is whether the mouthful can be got at.
- [creature-direction.md](creature-direction.md) — **direction agreed
  (2026-08-17).** Cell-chain ants, the caged brain, the heritable genome;
  decision record plus implementation plan.
- [creature-evolution-plan.md](creature-evolution-plan.md) — **plan,
  S1–S4 implemented (merged 2026-08-23).** The staged route from a scripted
  ant to an evolving one: the 584-slot genome, food worth living on the
  material rather than on the eater, corpse worth in `Cell::aux`, and the
  edible forest floor. Its "As built" notes carry the measurements; every
  S4 number in them predates the litter merge and is superseded by it.
- [creature-reproduction-economics.md](creature-reproduction-economics.md) —
  **design research, 2026-08-29; nothing built, nothing timed. Awaiting an
  owner ruling.** Why S6's budding never fires on the shipped ant, and what
  a reproductive economy that *can* fire would look like. Corrects the
  standing account in two places: the bank ceiling is **570, not 930** (the
  neutral gut draws 120 from a 480 leaf through S5's matched filter), and
  there are **two independent gaps** — the body stamp, and a grant term that
  is unreachable even with a free body, so fixing the birth cost alone
  leaves a 3.75x shortfall. Nature's answer in four parts (Smith–Fretwell
  offspring size, mass versus progressive provisioning, the claustral queen
  metabolising her own flight muscles, trophallaxis and repletes), five
  candidates scored against this engine's arithmetic and its ledger, and a
  recommendation that is a **fork**: `birth_grant` first in both arms, then
  either a solitary altricial income breeder (E5's own ancestor) or a
  colonial mass provisioner (E8's other half). Names three genes with
  two-sided trade-offs and **refuses a fourth** — heritable body size
  ratchets to one end whichever way it is priced, because `idle_cost` and
  `move_cost` are flat per organism, which corrects **E10**'s premise that a
  longer body is already priced. Six experiments named and none run.
- [creature-export-design.md](creature-export-design.md) — **built and
  landed 2026-08-29.** The dev-tool exit decision **E8** asks for and nothing
  implemented: an evolved individual written back out as a species `.ron` the
  existing loader reads to the same animal. Why it is a species file rather
  than a genome sidecar, the genome block (hidden self-recurrence) that
  **24% of sampled genomes carry and no species file could describe**, and
  what the manifest stamp does and does not catch.
- [creature-motion-baselines-2026-08-29.md](creature-motion-baselines-2026-08-29.md) —
  **measured baselines, 2026-08-29, on `3c4cc2b`; §0 corrected the same day.**
  Re-takes `creature-motion-design.md` §7's falls-per-move, stale since
  `d007c156` and `4c95233`: **1,217/8,812 = 13.8%** against the quoted 14.8%,
  with a seeded order statistic (`seeds=12 frames=12000`, median 0.225, max
  0.334) to gate against, and the warning that it **does not fully settle**
  (0.239 → 0.225 → 0.215 across 6k/12k/24k), so a gate must fix its frame
  budget. Establishes moves-per-1,000-frames, which did not exist: **27.2** for
  a colony on real terrain against 113.5 for a lone ant, with ticks scheduled
  equal to ticks executed everywhere. Corrects **"~60% of moves blocked"** in
  `creature-review-2026-08.md` to **3.4%**, a 17x error still standing as a
  premise for traffic work, and a denominator trap in two harnesses (`ascii`
  plants 55 ants and runs 27; `forage_probe` runs 46 and divides by 55).
  **Its §0 is corrected in place and the original left standing**: it ruled out
  scheduler starvation, which is true of the undisturbed colony it measured and
  false once a pick is in the world — see PR #118, which reproduces the
  starvation on demand. Kept as the worked example of CLAUDE.md's *ask what your
  number counts when nothing is wrong* failing in the scene rather than the
  arithmetic.
- [creature-rebaseline-2026-08-29.md](creature-rebaseline-2026-08-29.md) —
  **measured re-baseline, 2026-08-29, across `ba6fc98` and `f96c08d`.** Every
  creature figure in the record predates this week's worldgen work, so none of
  it was checkable. Re-measures the §4 guards (foraging pays +0.427/+0.459,
  ants fed 0.68/0.75, reference genomes 0.696/0.299, determinism identical,
  frame cost a bigger scene rather than a slower one), re-sets `ascii.rs`'s
  `forage_reach` trip bar (comment said 98, measured 23, bar 14 → 6 with the
  trade stated), prices `ant_ablation` (868 s at its defaults, which cannot
  answer the feeding question), and **overturns the reading of
  `eats 6 / deaths 0`** — the energy ledger shows food supplied 2.9% of the
  colony's energy and the scene stops at 45% depletion against a 50% hunger
  threshold, so the cause is the horizon, not free food. Supersedes the
  `Reads today` column of `creature-evolution-plan.md` §4 and the `98`/`18`
  figures quoted in `examples/ascii.rs`.
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
- [creature-vision-sizing-2026-08-30.md](creature-vision-sizing-2026-08-30.md)
  — **measured pre-flight, 2026-08-30; instrument `examples/vision_probe.rs`,
  no behaviour changed.** Sizes **E15**'s sight sense before anyone builds it,
  by tracing the geometry that already exists: **build it at radius 64,
  all-round, seeing over the floor litter**, and it costs **~0.005 ms of a
  frame** — 0.15–0.22% of `ascii`'s 2.94 ms mean, below what a wall clock
  resolves, and under 10% of a frame only past a few hundred predators. Every
  geometry number in it was measured on **four different trees** as `main`
  landed underneath — worldgen, tree-breaking, the creature economy. The first
  three were byte-identical; the fourth moved only in the third decimal and
  **every median and p10 the recommendation rests on held**. The radius argument is
  the **p10 seed** rather than the median: the stranded beetle sees prey
  0.108–0.260 of the time at r32 and 0.240–0.389 at r64, over three presets
  and 18 seeds each. Two findings the design has to carry: what blocks a
  sight line is **floor clutter, not landscape** (seed, litter, corpse, soil
  — 28.1% of pairs on `wetland`, 8.6% with the eye one cell up), and making
  **foliage a binary blocker costs half the sense** (0.667 → 0.350) with no
  eye height buying it back, which is `CLAUDE.md`'s *an outcome is a
  distribution, not a binary* arriving on the creature line. Does not answer
  whether a beetle acts on a sighting; `predation_probe`'s control already
  says the kill works at contact.
- [creature-sight-sense-2026-08-30.md](creature-sight-sense-2026-08-30.md)
  — **the build of the report above, shipped 2026-08-30.** Two brain inputs,
  `PreyNear` and `PreyBearing`, written by a 16-ray fan at radius 64 from one
  cell above the head; the beetle authors `sight_range: 64` and one weight,
  and **nothing else in the world has eyes**. The pre-flight transferred:
  predicted 0.572 of samples with prey in sight, built sense reads **0.50**
  over 8 generated seeds. Over that sweep pursuit moves two independent
  far-side counters together — mean sighted range **15.2 → 12.5 cells** and
  prey caught **302 → 323**. It costs about **twice** what the pre-flight
  priced — 1,020–1,100 cells read per cast against 485, because prey must be
  tested in the un-lifted frame and blockers in the lifted one — which is
  still 0.3% of a frame and is an honest correction to §5 of the report
  above. Three things to carry: the obvious
  effect counter (**did it move closer this tick**) *cannot* fire on the
  ticks the sense exists for and falls where catches rise — a number that is
  arithmetically correct and about the wrong question; the stronger pursuit
  lever, releasing straight-ahead persistence, is **measured, better on the
  field, worse in a corridor and deliberately not shipped**, with the
  question put to the owner; and **`BrainOutput::Turn` is nearly inert for a
  surface walker on level ground** — both turning candidates fail, one on
  passability and one on foothold — which is a movement finding filed as
  `open-bugs-handoff.md` §R4, not a perception one.
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
- [evolution-lab-feasibility-2026-08-30.md](evolution-lab-feasibility-2026-08-30.md)
  — **feasibility measurement, not a plan.** Could a second game live on this
  engine with the gnome, worldgen, rock and destruction stripped out — a
  sealed lab box of plants and creatures under grow lights, run fast enough
  to watch evolution? Yes, and not for the expected reason: an empty box
  costs 0.001 ms/frame, cost is ~0.7 µs per living plant cell and **not** a
  function of world size, and the phases the concept deletes already measure
  0.000–0.001 ms. 5–7 herb generations measured at 4 min 17 s. The blocker is
  biological — the shipped ant reaches **generation 0**.
- [evolution-lab-design-guide-2026-08-30.md](evolution-lab-design-guide-2026-08-30.md)
  — **design guide; calls and open questions, not a plan.** How you would
  build the lab game the feasibility report says is affordable. Its §0 is the
  reframe: the concept **is owner decision E8** ("evolution is a dev tool as
  well as a mechanic") with a player in it, and `species_export` already
  ships the keep. §2 turns each measurement into a design consequence — no
  sky, population as the frame budget, depth as an honestly-priced upgrade,
  equipment as what switches the idle air simulation on. §3 is five gates,
  of which Gate 0 is "an ant reaches generation 2". §5 answers "reward
  interesting behaviour" without naming a behaviour. §1a finds the deadlock in
  the owner's own opening premise — the ant *cannot* be fed over its birth
  cost, because its bank ceiling is the hunger line — and argues the lab wants
  that deadlock as its first puzzle where the outdoor game wants it fixed.
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

## Worldgen and world  ·  `outdoor`

**The 2026-08-29 revamp program** — six audits and a plan, written the day
the owner said six rounds had not made the world interesting and asked for a
revamp rather than another round:

- [worldgen-revamp-plan-2026-08-29.md](worldgen-revamp-plan-2026-08-29.md) —
  **the plan.** What the four audits below add up to, the workstreams in
  priority order, the sequencing and the stop gate. Start here; the audits
  are its evidence.
- [worldgen-appearance-audit-2026-08-29.md](worldgen-appearance-audit-2026-08-29.md)
  — **audit; measured.** What the player actually sees, in rendered pixels:
  the landform passes are at most 0.6% of the view and the palette passes are
  90%. Killed the "every world looks the same" premise (presets differ 4.76x
  in colour) and replaced it with a sharper one — they differ by repainting,
  not reshaping. Its distance matrix is **owner-calibrated**: it predicted
  "identical" and "2, maybe 3 kinds of country" before either verdict existed.
- [worldgen-architecture-ceilings-2026-08-29.md](worldgen-architecture-ceilings-2026-08-29.md)
  — **audit; structural.** What no tuning inside this pipeline can produce.
  The root: the generator has no representation of a feature — a heightfield
  and a cell grid with nothing in between. Also refutes the "hard boundary at
  1/3 and 2/3" reading, and finds that every sharp vertical face in the world
  is a residual.
- [worldgen-prior-art-and-dead-ends-2026-08-29.md](worldgen-prior-art-and-dead-ends-2026-08-29.md)
  — **audit; research.** The do-not-retry list with each rejection's
  *condition* (and whether a revamp voids it), the six-round ledger, and the
  outside prior art. Records that the 3D coarse map of `worldgen-design.md`
  §0 was designed in detail and never built.
- [worldgen-visual-interest-2026-08-29.md](worldgen-visual-interest-2026-08-29.md)
  — **audit; measured.** How our world differs from beautiful landscape,
  ranked and split into worldgen/render/content. Killed the "shading terrain
  is the cheapest large win" hypothesis with a positive control — a shading
  term needs a surface and 95% of the ground on screen is interior — and the
  owner then killed it in his own words (*"it is the build"*). Carries the
  cutaway reframing: most of the screen is a cross-section through rock, so
  landscape photography is the wrong referent for it.
- [cave-redesign-2026-08-29.md](cave-redesign-2026-08-29.md) — **design.**
  Why six of the owner's eight cave verdicts are one fact (a cave is a Worley
  field thresholded in a box, and retuning can only zoom the lattice), the
  replacement (rooms grown by roof collapse, conduits routed through the
  strata), and the finding that **there is no cave entrance in this game** —
  so every cave verdict on record was given on a place the player cannot
  reach. Also repairs two instruments that could not see a cave at the
  shipped world size.
- [rock-vocabulary-design-2026-08-29.md](rock-vocabulary-design-2026-08-29.md)
  — **design; prototyped and measured.** The ground made of six rocks instead
  of one rock with tints painted on it. Six rocks move **24.89%** of the
  player's view — more than `soil_blanket`, and ~40x the whole landform
  programme of six rounds — while running **2.2x faster**, because the old
  region tint sampled two 2-D fBm fields per cell over 18.7M cells. Finds that
  tint as a blob *cutting across the bedding* (why the underground reads as
  camouflage), and that `pockets` and `boulders` both tested for grey stone by
  identity and so silently did nothing. Overturns the revamp plan's own
  demotion of rock: the case is strength, not colour.
- [worldgen-drains-2026-08-29.md](worldgen-drains-2026-08-29.md) —
  **Phase 0; shipped.** The pass conflicts that were deleting other passes'
  output. Fixes R4-1 (`brows` taking the air `boulders` needed, recorded
  2026-08-20 and live for nine days) — **boulders exist for the first time**,
  3 cells across all presets to 777, median over 6 seeds at the shipped size —
  and makes `talus` realise 2.5–3.5x more. Adds `scripts/worldgencheck.sh`, a
  CI gate that fails when a pass writes nothing or appears only when another
  is switched off, with a selftest that restores R4-1 and requires the gate to
  *name* it. Withdraws its own `pockets`→`vaults` fix with a control, because
  it makes overlapping cave systems' independent waterlines fire — that goes
  to the cave lane. Corrects two errors in the revamp plan: `springs` was
  never at zero, and the shipped world is 128x the small one, not 256x.
- [worldgen-relief-2026-08-30.md](worldgen-relief-2026-08-30.md) — **W1;
  shipped.** The centrepiece: the ground gets a surface. A slope-free lowering
  term coupled to the *section* resistance under the surface, taken as a
  contrast against a running mean over +/-200 columns, plus a `ridged_1d`
  massif and a long fold with horst-and-graben faults in `strata_offset`.
  **The formation-scale band roughly doubled** — local relief at reach 30,
  p90 over 6 seeds: arid 21→46, canyon 54→83, rolling 27→50, terraced 30→47,
  wetland 14→26 — and the starved passes switched on with it (brows 4.9–55x,
  talus 3.2–66x, `boulders` writing at all on four of five presets).
  Generation +290 ms on ~2.1 s; no per-frame code changed.
  **Two traps recorded.** A mountain does not fit in a 320-row world, so
  `filmstrip` and every 512x320 scene are **bit-identical to before** — do not
  judge this from a filmstrip. And the obvious form of the erosion term
  improved every number while rendering as the flattest world this generator
  has made; looking at it is what caught that.
  Also deletes a **blind guard** (`an_old_world_is_smoother_than_a_young_one`
  was false at the median on both arms and passed on two hand-picked seeds),
  replacing it with a paired eight-seed guard watched going red, and adds a
  test three comments had cited by name without it existing.
- [worldgen-caves-rebuilt-2026-08-29.md](worldgen-caves-rebuilt-2026-08-29.md)
  — **W3; shipped** (`src/worldgen/cave.rs`, new). The cave generator replaced
  rather than retuned: **nothing in it reads a noise field to decide where
  rock is absent.** A room is a dissolution lens flooded through a removal
  cost built from the strata, then a roof that falls in until it reaches a bed
  strong enough to hold its span; conduits are paths through an anisotropic
  cost field; systems are given a way in — **though not all of them get one:
  6 of 11 over 8 `rolling` seeds, corrected 2026-08-30, pre-existing and not
  moved by the guard work.** `bed_span` is read off the material —
  42 cells in mudstone to 308 in basalt — so two rooms differ by a factor of
  seven with no parameter moving.
  Census over 16 seeds x 5 presets: worlds with **no cave 2–4 → 0**, largest
  connected walkable region **36–39% → 98%**, median open column **13–16 → 60–72**
  against a 14-cell gnome, and systems with a way in **0 → all of them**.
  Margin came *down*, 802 → 780.
  **Two corrections the programme needs.** `cave_probe`'s census window was
  `WORLD_HEIGHT/2`, below most of the depth band — the earlier "8 or 9 of 16
  worlds have no cave" is really **2 to 4**. And the pillar question the
  revamp plan flagged as its largest open risk is **answered and the risk does
  not exist**: `support_census` could not see it (it reads the field and never
  cuts a hole), so `cave_probe` gained a `span=1` mode, and the roof does not
  come down at any width up to 2,048 cells — `load::capacity` is quadratic in
  section and multiplied by `attached_span_bonus`, so `max_unsupported_span:
  16` never reaches the scale the plan assumed. Positive control (`lid=6`)
  reads 0 → 207, so the instrument is not blind. Pillars stay as a design
  choice, not a structural necessity.
  Also: the owner's "sky is coming into the cave" was **the renderer, not
  geometry**.

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
  **measurement of record for how `App::update` divides.** The first
  attribution of `App::update` as a whole, at the shipped 8192x2560 size:
  30.1 ms amortised with nobody playing, 79% of frames over the 16.6 ms
  budget. Reranked the performance backlog — issue #2 down (the sweep is a
  tenth of the frame), `plant::step_organisms` up (a quarter of it, and in no
  plan). Read it before quoting any per-phase cost; `field-settling-2026-08.md`
  remains the record for the field's *internal* split and is not contradicted
  here. **Its headline is no longer the frame** — see the next entry, which
  extends rather than supersedes it.
- [frame-cost-the-render-half-2026-08-29.md](frame-cost-the-render-half-2026-08-29.md)
  — **measurement of record for the whole frame, simulation *and* render.**
  `App::update` has fallen to 18.9 ms (±1%, measured on this job), so the
  simulation did not regress — but `Renderer::draw` is not in `App::update`
  and nothing had ever counted it: it is **~40 ms**, the larger half of the
  frame by 2:1. Bisects the owner's "the game feels slow" to `39e6f36`
  (PR #94) and to the **sky** half of it, worth ~29 ms a redraw and ~2 ms of
  simulation, with the soil half worth nothing. The cost was a *cliff* rather
  than a price per sky pixel, which is what said it was a defect — and it
  was: `rebuild_near_glow` hashed a `ChunkCoord` twice for each of ~615 disc
  cells of each of ~6,900 glowing cells, on every forced full redraw. Fixed
  here, **~42 ms -> ~7.5 ms**, and PR #94 stays. **Then the "on every forced
  full redraw" half went too**: that trigger read `force_full`, which is true
  whenever the cursor is over the window, so the rebuild happened on ~100% of
  frames for a reason that has nothing to do with cells. Removing it takes the
  redraw to **2.4 ms** and the rebuild to a frame where a crystal actually
  changed — §7, with `Renderer::forget_world` for the one thing `force_full`
  was covering by accident, and the phase split
  (`PIXEL_PHYSICS_DRAW_TIMING`) that no instrument here had. **Read its "every
  image-level check of this is blind" section before verifying any change to
  the glow splat by rendering** — a deliberate off-by-one left two renders
  byte-identical and four existing guards green. Carries a separate ~8 ms
  finding for rain.
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

## Character  ·  `outdoor`

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

- [plant-mechanics-handoff-2026-08-29.md](plant-mechanics-handoff-2026-08-29.md)
  — **handoff; its §5.1 is built** (`tree-fall-2026-08-29.md`), the rest
  stands. **Read it with the plan below.**
  What happened *after* the plan: PR #102 landed (the plan, the debris tiers,
  colour preservation), and six findings that change what to build next. The
  one to carry: **what has always looked like a crown collapsing was leaf
  powder running downhill, not a fall** — nothing makes a severed piece
  travel sideways, which is why the previous attempt kept measuring well and
  looking wrong, and which makes the fall the critical path rather than a
  polish item. Also: the load model has **no `topple` outcome**, proved by a
  control that falsified the session's own source reading (giving logs the
  tipping test crushes them and leaves *more* standing); ask 1 is **buckling,
  not bending**, because a balanced stem's base reads 0.0–0.3 against a max
  of 7,171; and rotation seeds from angular acceleration, not torque. §4 is a
  code map of the promotion path with the constants and commands, §5 the
  staged work, §6 the traps, §7 the standing owner rulings verbatim.

- [tree-fall-2026-08-29.md](tree-fall-2026-08-29.md) — **shipped, and one of
  its two claims is not established.** The fall: a severed piece now turns as
  it comes down, at a rate read off its own mass about the joint that gave
  way (`alpha = g*sum(m*d)/sum(m*r^2)`, reducing to `3g/(2L+1)` for a limb
  breaking at one end, with nothing tuned), and a piece that lands
  overhanging its footing goes over instead of standing there. Opens with the
  counter that reframes it: `scene=fell` asked for **0 quarter turns** over a
  whole felled tree, so the rotation mechanism had never once fired on a
  falling tree. Closes the felled-tree half of `open-bugs-handoff.md` §Q by
  supplying the *outcome* §Q found missing — the tipping test was never the
  gap — and the numbers agree, 109 topples against 21 in-flight turns over
  twelve scenes. **Read §4 before quoting §3**: pooled over ten paired scenes
  the per-piece lying share moves **28% → 36%** and pieces left standing on
  end **364 → 287**, up in 8 of 10 and flat in 2, while the *cluster* census
  the repo already had does not move at all because it folds touching logs
  into one blob. Also carries a raft regression the suite caught, why
  `FALL=off` had to exist for the control to be real, the re-promotion loop
  (§Z3) the negative control turned up in *both* arms, and §4c's measured
  price of extending any of it to rock.

- [tree-mechanics-plan-2026-08-29.md](tree-mechanics-plan-2026-08-29.md) —
  **plan; one change landed** (`3bdf674`). Structure and physics for *all*
  plants, from the owner's five asks — a top-heavy tree falling, a rock
  breaking a limb, chopping, wind-throw, and a lateral that bends before it
  breaks. **One stress number, two material properties**: stiffness decides
  how far a thing bends, strength decides when it snaps, and bending
  *relieves* the moment — which is why grass bends and never breaks, from
  the same arithmetic that fells a tree. Carries the measurement that the
  base of a balanced stem reads ~0 bending stress, so **ask 1 is buckling,
  not bending**, and the shipped `max_cantilever_reach` rule is a
  slenderness rule with its width term missing. §9 states what
  `examples/beam_probe.rs` can and cannot carry (52% of cells disagree
  between three section definitions), and **§10 tabulates twelve corrections
  the three reviews and the owner made to the first draft** — read that
  before trusting anything the draft said. Owner rulings recorded in the
  header: growth and the carbon economy are out of scope, every plant
  participates, and leaves must never become a powder.

## The evolution lab  ·  `lab`

The second game: a sealed box of soil under a grow light, run at speed, where
the shipped plants and creatures live under conditions a player sets. Design
of record is the design guide (in flight, below); `two-games-one-repo-2026-08-30.md`
above says what the two games share.

**A report here is about the box, not about the biology in it.** Plant and
creature findings stay in their own sections even when the lab measured them
— the whole premise is that those are the same organisms either side, and
filing them by which game happened to observe them would hide that.

- [evolution-lab-gate-1-2026-08-30.md](evolution-lab-gate-1-2026-08-30.md) —
  **measured, nothing built beyond the two harnesses.** Gate 1 of the
  evolution-lab program: the census and the frame cost of `lab::scene::LabBox`,
  the first scene in this repo running plants and creatures together.
  **The box lives** — plants reach generation 5, 285 born against 266 dead in
  90,000 frames — **and the colony is the biggest thing acting on it**: a bed
  with no ants holds all eight founders and 2.9x the organisms, **12 seeds of
  12, every column**. Three corrections. **The founders that go missing are
  eaten, not ungerminated and not merely too small to draw.** **Gate 0 is a
  reach problem, not an economy one**: the matched gut that
  [creature-stamp-routes-2026-08-30.md](creature-stamp-routes-2026-08-30.md)
  prices takes the margin from −820 to **+500** and produces **zero births**
  over 48,000 frames, because the flower it unlocks stands **22 to 40 rows up
  a stem** and the ground-level form of it, `windfall`, stands at 1–2 cells
  for two census tiles out of sixteen. And **frame cost in the lab is the
  field's solve set, not biomass** — correlation with plant cells **−0.02**,
  with tiles solved **+0.90** — so the box gets *cheaper* as it runs, 5.1x
  real time early to 11.5x settled, which is the opposite of the sizing rule
  it was measured against. The organism ceiling is a footnote: 66 slots of
  4,095 used, 0 refused. Timings taken under five-to-nine-times
  oversubscription and labelled as such; the render term is reported as
  **not measurable** rather than guessed.

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
