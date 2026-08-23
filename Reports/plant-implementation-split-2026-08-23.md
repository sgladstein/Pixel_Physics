# Plant implementation split — lanes, packages, session briefs (2026-08-23)

**Status: execution plan for the queue in `plant-project-review-2026-08-23.md`.**
That report defines every item referenced here by ID (A1, B3, C1, …); this one
says who runs what, in what order, and why the parallelism is shaped the way it
is. Written for Opus-grade implementation sessions started fresh — each brief
below is self-contained enough to paste as a session's opening prompt.
Successor to `plant-work-split.md` in role; it inherits that document's one
surviving doctrine (split by work package, not by file ownership) and the
lesson its merge taught (`open-bugs-handoff.md` §A–§D: two plant lines merged
cleanly by git and still left one red test and three unmeasured cross-line
inconsistencies).

---

## 1. How much parallelism the codebase actually supports

**Three lanes, not one session and not a swarm.** The bound is not agent
count, it is two shared substrates:

1. **`src/sim/plant.rs` + `organism.rs`.** Crown recession (C1), the water
   book's absorb fix (E1), drought gating (E2), root thickening and dominance
   (B2/B3), resprout (D4), mortality and seed decay (A2) all edit the same two
   files — `plant.rs` alone is ~7,800 lines and 40% tests. Two sessions in it
   concurrently re-create the 2026-08-22 merge fallout.
2. **The carbon/water economy.** C1, E2, E4 and B3 all move what a unit of
   carbon buys. Tuned in parallel, each session re-derives constants against a
   baseline the other is moving — the exact double-tuning waste the
   substrate-v2 plan re-sequenced polarity to avoid. Economy constants get
   **one owner and one re-derivation pass** (inside P2, below).

What *is* disjoint, by the collision table's own clusters: the structural/
destruction files (`structural.rs`, `rigid.rs`, `load.rs`, fracture), the
worldgen files (`src/worldgen/*`), `weather.rs`/`fire.rs`, and pure-data
species fields. Hence:

| Lane | Owns | Never touches |
|---|---|---|
| **P — plant core** | `plant.rs`, `organism.rs`, economy/root constants in species `.ron`s, `plant_probe` internals | worldgen, rigid/structural internals |
| **S — structural verbs** | `structural.rs`, `rigid.rs`, `load.rs`, `explosion.rs`/`fire.rs` damage routing, `filmstrip` scenes | plant/organism economy code, all `.ron` constants |
| **W — world & species data** | `src/worldgen/*`, `weather.rs`, palette/band and identity fields in species `.ron`s, cards/decisions | economy constants, `plant.rs` internals |

**Within a lane: strictly one session at a time.** Across lanes: all three run
concurrently. A fourth code lane has nothing disjoint left to own.

Shared-file rules that make the three-way split survivable:

- `examples/filmstrip.rs` is the most-collided file in the repo. Scene
  additions are append-only; whoever touches it **lands within the session**
  (open the PR the same day — CLAUDE.md's contested-file rule).
- Species `.ron`s are split by *field*: P owns economy numbers
  (`cost`/`rate`/`pipe_ratio`/`turgor_*`/root blocks), W owns identity fields
  (`foliage_bands`/`bark_bands`, `leaf_cluster`, `plastochron`,
  `branch_angle`). Git merges disjoint fields cleanly; the `include_str!`
  rebuild trap does not care whose field changed — **rebuild before every
  measurement**.
- `Reports/open-bugs-handoff.md` takes entries from every lane: append your
  own section, never rewrite another lane's, land quickly.
- `PLAN.md`, `README.md`, `CLAUDE.md` stay untouched by implementation
  sessions except the doc-sweep package (P0) and each package's own README
  status paragraph at landing time.

**Landing cadence:** one PR per package, opened when the package's acceptance
is met — never held across packages. Standing authorisation to open PRs is in
CLAUDE.md. Run `bash scripts/branchcheck.sh` at session start and pull `main`
in mid-session; the `behind × files > 300` rule decides when to stop and land.

**CI reality every session must know before reading its own PR as broken:**
three known-red lanes are quarantined by design (`bug A` slot-1 test, `bug Y`
wood acceptance case, `bug H` ascii scene — all `continue-on-error`, red until
their bugs close), and as of `a0fa433` **main itself is red** on
`a_forced_vault_world_is_sealed_and_arrives_at_rest` (the world-scale phase-2
merge's, not yours — attribute against main's run at your base commit before
touching anything).

---

## 2. The lanes and their packages

Every package = one fresh session = one branch = one PR. Order within a lane
is a dependency chain; do not reorder without recording why.

### Lane P — plant core (the serialized lane)

- **P0 (optional, tiny): the doc sweep.** Review report §3's list: the expired
  CLAUDE.md amputation gotcha, four stale source comments, two "not merged"
  lines, `wiki/ants.md` litter paragraph, the ant-sweep filename,
  `grass.ron`/`creeper.ron` economy notes. One commit, zero behaviour. Can run
  any time; touches contested files, so land same-day.
- **P1: instruments + the water book.** E1 (conserve the un-drunk remainder in
  `absorb_water`; rain soak through zero-capacity litter in `weather.rs`;
  bare-soil evaporation sink for §F8) — each with its paired one-number
  measure. The `break_root_tips` firing counter at `plant.rs:3017` (one
  instrument, two bugs: §U and §A). E3: recalibrate bug A off the 8-seed order
  statistic and delete the CI exclusion. C4: the distinguishability component
  count + lineage birth/death rates in `plant_probe`, calibrated against the
  two answered §Z cards. **No economy constant changes in this package.**
- **P2: crown recession + the single economy pass.** C1 superlinear upkeep on
  `q_peak` (exponent anchor 1.5; *flat is a recorded dead end*), E4 night
  income (`0.25 + 0.75·daylight_fraction`, decisions stay noon-normalised),
  E2's income-gating for drought — then **one** re-derivation: `pipe_ratio`,
  canalization contrast (the audit requires it re-derived in `grove`),
  ensembles only, establishment rate + stem-above-base + the picture. Post
  paired grove cards before declaring done. This is the lane's L-sized item.
- **P3: the generation loop (A2).** Grass mortality path, seed decay clock,
  the 4,095-slot release-mode check, dead-trunk slot reclamation. Unblocks
  W-lane's grass sowing and the endowment curve. Measure: standing
  `OrganismState` count over 60k frames (baseline: 455 immortal seeds), grass
  generations still ≥ 4 per 45k run.
- **P4: root differentiation mechanisms (B2 + B3).** `can_widen` displaces
  soil (conserve water — `displace_soil_water` is the prior art; the recorded
  revisit condition for relocation is open); allocation below the collar
  weighted by conductance, contrast exposed as a genome locus. Guard:
  contrast-at-zero reproduces today's fibrous behaviour exactly (the
  `VEIN_GAIN = 0` discipline). Judged per `root-morphology-findings.md`'s
  method: single plants at zoom or N-per-treatment grids, never stand medians.
- **P5: resprout (D4).** Keep `q_now` beside `q_peak`, flush a `DormantBud`
  at `order = 0` on divergence; acceptance is the dormancy-reversibility cut
  test; watch the `juvenile_size` caveat. Coordinates with S-lane: uses their
  `cut=` scene, and its landing is what makes S-lane's felling *answered*.

### Lane S — structural verbs

- **S1: instrument + licence (D1 + D2).** `filmstrip` felling scene printing
  standing-organism census **and** `chunk_bodies` count; then route organism
  cells into `strike`/`mine_swept`/brush/fire damage
  (`rigid.rs:1151`-pattern skips are the target), letting `anchor_support`
  declare the severed region. Acceptance: chopping a trunk brings the crown
  down at all, with a nonzero body count printed beside the sheet. Post the
  GIF however ugly.
- **S2: fell as pieces (D3).** Severed subtree promotes as one piece or a few
  log-scale pieces + debris distribution; `BodyCell` carries the organism id
  through promotion; wood's fracture ladder stops shredding trees. Ethos gate:
  a felled tree must read as *pieces of a tree*, never sawdust — judge the GIF.
- **S3: topple v1 (D5).** Lateral impulse at the cut + normal fracture on
  impact; v2 (torque from support asymmetry) only if v1 reads fake on the GIF.

### Lane W — world & species data

- **W1: sow the flora + species identity (A1 woody + C2/A3-data).** Worldgen
  sows shrub/conifer/creeper by the conditions materials already express
  (grass waits for P3); palette bands spread to disjoint ranges; the two or
  three live levers differentiated per species as points on trade-off axes.
  Guard with a seed sweep gating an order statistic (worlds are procedural —
  the guard must sweep the procedure). Cards: generated-world panorama with
  per-species establishment counts; B1's shallow-vs-deep-fibrous render
  (slot 5, no code) rides along here.
- **W2: fire and the desert (E5 + E6).** Moisture gates grassfire spread
  (why does `MOISTURE_IGNITION_RESISTANCE` change nothing?), a burn that
  reads as fire (GIF card against §G's standing verdict); the §X desert
  decision card with the three levers costed — decided together with B4's
  water-table niche.
- **W3: sow grass (A1 grass half) + the divergence instrument (A4).** After
  P3 lands. Two-patch wet/dry scene, per-patch allele census per generation;
  the §8d crossover predicts the drift direction — watch it happen. This is
  the acceptance test for "an evolution framework exists".

**Second wave, unassigned until wave 1 lands:** C3 whorls (P or W), A5
dispersal (W), B4 niches (W+P), F4 megastudy re-run (only after P2 — it moves
composition), F5 age channel (design note first), D5 v2.

---

## 3. Session briefs (paste-ready)

Common preamble for every brief — include it verbatim:

> Work in this repo follows `CLAUDE.md` — read it first, then
> `Reports/plant-project-review-2026-08-23.md` §1–§4 for the queue you are
> executing, then `Reports/open-bugs-handoff.md` §A–§D before touching any
> plant constant. Run `bash scripts/branchcheck.sh` at start. One branch, one
> PR for this package (standing authorisation in CLAUDE.md), landed when
> acceptance is met — never held open past the package. Rebuild before every
> measurement (`cargo build --release --examples`); ensembles in `grove`,
> never the 40-row scenes; paired comparisons, order statistics over seeds;
> post every visible change as a review card (`scripts/review.py`) with the
> event count in `meta`. Known-red CI lanes (bugs A/Y/H) are red by design;
> main is currently red on the world-scale vault test — attribute before
> fixing anything that isn't yours. Never `git add -A`.

Then per session, the package block from §2 plus:

- **P1:** "Your package is P1. Deliverables: the three water-book fixes with
  paired measures; the `break_root_tips` firing counter and what it says
  about §U and §A; bug A's bar re-derived from `print_root_branch_slot_seed_sweep`
  and the CI exclusion deleted; the C4 probe calibrated against the §Z cards.
  Do not change economy constants; record what the counter implies for P2."
- **P2:** "Your package is P2. Read `PLAN.md`'s bud-break postmortem and the
  dead-ends entries for flat respiration, stock-division and ratio bounds
  before writing code. Deliverables: superlinear upkeep on `q_peak`, night
  income, drought income-gating, then the single economy re-derivation with
  before/after grove cards. Judge on establishment, stem-above-base,
  wood:leaf trajectory, and the picture — never `rows >1 cell wide`."
- **S1:** "Your package is S1. Read `Reports/felling-blockers.md` (its §1
  premise is superseded — `anchor_support` landed) and review report Arc D.
  Deliverables: the `cut=` felling scene with census + body count, organism
  cells routed into tool damage, a GIF card of the first fall."
- **W1:** "Your package is W1. Read review report Arc A/§2.0 and
  `plant-species-authoring.md`. Deliverables: woody species sown by worldgen
  (grass explicitly deferred to P3 — do not sow it), disjoint palette bands,
  species differentiated on live levers only, the seed-sweep guard, the
  panorama card, the B1 fibrous-axis card. Touch identity fields only; the
  economy numbers belong to lane P."

Subsequent packages get their briefs cut the same way when their turn comes —
each names its prerequisite PR and reads the predecessor's landing notes in
`open-bugs-handoff.md` / the PR description.

---

## 4. Coordination

- **The review session** (this one) stays up as integrator: watches the PRs,
  sequences merges (P before its dependents, S/W independent), keeps the
  queue report current as packages land, and arbitrates when two lanes need
  the same file after all. Implementation sessions do not merge each other's
  PRs; the owner (or the integrator on the owner's word) does.
- **Merge authority — standing delegation, owner, 2026-08-23.** The
  integrator merges lane PRs that meet this bar: the PR's own gating jobs
  green with every remaining red attributed to the base by paired
  measurement; `docscheck` clean; no unresolved review threads or
  unanswered owner comments; merge order respects lane dependencies (P
  first where others measure against it), with a main merge-back — done by
  the PR's own authoring session, never the integrator — after each
  landing. A bar-met merge by the integrator is also the wave-2 dispatch
  signal.
- **Lane P order amended 2026-08-23**, after the owner signed off the three
  morphology calls (`plant-morphology-reach-2026-08-23.md` §7): **P3 (the
  generation loop) runs before P2 (crown recession)** — annuals are
  turnover, fruit-borne seeds are dispersal, and grass sowing (W3), the
  organ package and the divergence instrument (A4) all gate on P3. The
  organ/determinacy package (morphology-reach §6) enters the lane after
  P3, carrying the three decided defaults; P2 follows.
- **Model/effort:** these are build-and-measure packages against a documented
  queue — Opus at high effort with `auto` permission mode fits (never `plan`
  mode for an unattended session; it stalls at the approval prompt).
- **When a package's session dies or drifts:** a fresh session re-reads the
  brief + the branch; packages are sized so restarting one loses hours, not
  days.
- **What would change the shape:** if P-lane throughput becomes the
  bottleneck (it will — it has five packages), the pressure valve is moving
  P0 and the *measurement-only* halves of packages (counters, probes, cards)
  into a fourth non-code session, not splitting `plant.rs` between two
  writers.
