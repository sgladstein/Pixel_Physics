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
- [documentation-audit.md](documentation-audit.md) — **findings, being
  executed.** What the docs told an agent that wasn't true, re-ranked for
  the agent consumer; carries the in-flight doc inventory.
- [pixel-physics-issues.md](pixel-physics-issues.md) — **mostly closed.**
  The eleven-issue backlog; nine-plus closed, #11 (slice-identifier on
  `ChunkCoord`) has a land-before-save-format deadline.

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
- [building-rethink.md](building-rethink.md) — **proposal, not built.**
  Building should not be a physics puzzle; from a direct playtest steer.
- [load-concentration-review-response.md](load-concentration-review-response.md)
  — **review response.** Answers `load-concentration-review.md`, which is
  *in flight* on branch `load-share` (`b4fb357`) and arrives at merge.
- [next-session-handoff.md](next-session-handoff.md) — **live handoff.**
  The unzip: what it was, what is left, what must not be retried.
- [explosion-mechanics-diagnosis.md](explosion-mechanics-diagnosis.md) —
  **diagnosis; the rebuild it prompted shipped** (README M15 status). The
  look-first measurements behind it.
- [underground-definition.md](underground-definition.md) — **settled,
  implemented.** What "underground" means, fixed at worldgen time; the
  wiki's world-cycles page describes the visible result.

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
  — **plan of record for branch `plant-substrate-v2`** (in flight).
- [plant-substrate-v2-design.md](plant-substrate-v2-design.md) — **design;
  implemented on branch `plant-substrate-v2`** (in flight).
- [plant-species-authoring.md](plant-species-authoring.md) — **live
  guide.** What authoring a second plant species actually needs.

## Creatures and ecology

- [creature-direction.md](creature-direction.md) — **direction agreed
  (2026-08-17).** Cell-chain ants, the caged brain, the heritable genome;
  decision record plus implementation plan.
- [stigmergy-research.md](stigmergy-research.md) — **research,
  implemented.** Deposit → diffuse → decay → follow; the ant colony is
  built on it.
- [population-dynamics-research.md](population-dynamics-research.md) —
  **research (Report D of four).** Why the ecology will go extinct and
  what prevents it; §7b answered by ecological-lod-design.
- [ecological-lod-design.md](ecological-lod-design.md) — **recommendation,
  not settled.** How an ecology survives a world that is not simulated
  (off-camera catch-up).

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
- [felling-blockers.md](felling-blockers.md) — **investigation.** Why
  cutting a tree down is not in yet.

## Open working files

- [open-bugs-handoff.md](open-bugs-handoff.md) — **open bugs.** Working
  reproductions, what has been ruled out by measurement. Read before
  touching a listed area.
- [dead-ends.md](dead-ends.md) — **live index.** 542 tried-and-reverted
  approaches, each with the condition its rejection depended on and where
  the full record lives. Grep your area's section before proposing or
  retrying anything in it; a revert adds its entry in the same change.

## Elsewhere in the repo

- `../docs/future-directions.md` — **historical, written at M3.** Its
  premises (4-byte `Cell`, no scheduler) are two milestones stale; kept as
  the feasibility argument that shaped M12–M18.
- `../research/` — raw source material for M16 / M18 / M19, in good order.

## In flight — exists on an unmerged branch, not in this directory yet

When one of these merges, move its line into the sections above (docscheck
flags the mismatch).

- `creature-evolution-plan.md` — branch `creatures-m18`.
- `load-concentration-review.md`, `load-concentration-review-reply.md` —
  branch `load-share`.
- `branch-angle-and-the-width-bound.md`, `genetic-variability-study.md`,
  `plant-appearance-design.md`, `plant-night-session-handoff.md` — branch
  `plant-branch-angle`.
- `plant-evolution-design.md`, `plant-implementation-plan.md`,
  `plant-work-split.md`, plus a new `wiki/plants.md` — branch
  `plant-ecology-design`.
- `plant-genome-design.md`, `plant-genome-implementation-handoff.md`,
  `plant-genome-review-request.md` — branches `plant-genome` /
  `plant-substrate-v2`.
- `performance-audit.md` — worktree `perf-audit` (untracked).
- `measurement-under-contention.md` — worktree `perf-lock` (untracked,
  with a CLAUDE.md edit adding `scripts/perf.sh`).
