# Plant implementation plan: the work, split by work

**Status: EXECUTABLE. All nine owner calls in
`Reports/plant-evolution-design.md` §8 are signed off (2026-08-19), and
the owner has directed that work be split by *work package*, not by file
ownership — this supersedes the file-ownership table in
`plant-work-split.md` §4. Files below are listed as collision notes, not
as territory.** The rule that replaces ownership: whoever holds a work
package edits what that package needs, lands small and fast in any file
another session also touches, and rebases before every land.

Written for implementing sessions to execute cold. Every package
states its spec, its acceptance protocol, and its failure protocol —
follow them as written; where a package says "measure first, then set
the bar," do not invent a number. `CLAUDE.md` applies in full,
especially: look before you measure; rebuild between sweep points
(species and materials `.ron` are `include_str!`); never `git add -A`;
work in your own worktree; capture sheets at noon (multiples of 3,600);
test both drivers; paired comparisons over remembered numbers.

**Two sessions, same model tier — split by context, not capability.**
Session G is the genome session: it holds the root-economy
measurements, the genome re-map's edit history through `germinate()`
and the `Grow` arm, and the megastudy harness in context, so it takes
the packages that lean on that knowledge. Session F is the form
session's implementation successor (this branch,
`plant-ecology-design`): it holds the ground-layer design, the
four-axes bar and the probe specs, so it takes those. **Two sessions
rather than one** because the lanes are genuinely parallel (G's
megastudy alone is 3.5 hours of wall clock that would serialize F's
entire queue behind it) and each lane's context would cost the other
session real time to rebuild. **Collapse to one session** if G's queue
drains first, or if plant.rs coordination overhead starts exceeding
the parallelism — the merge contract above is the tripwire: two
conflicted rebases in a row on the same region means stop and
serialize.

---

## 0. Order and merge contract

```
Day 0:   WP-B1 (G) — three-field materials change. Tiny, lands first,
         byte-identical guard. Both sessions rebase on it.
Then, in parallel:
         Session G: WP-A  (root repair)          -> land -> WP-A2 (megastudy)
                    WP-D  (population plumbing)   after WP-A
         Session F: WP-C  (form probes)           any time, no dependencies
                    WP-B2 (litter)                after WP-B1
                    WP-B3 (grass)                 after WP-B2; re-tune roots
                                                  after WP-A lands
Later:   WP-E (foliage mass)   after WP-B3 and WP-A2 — either session
         WP-F (age channel)    design note first; the note is planning
                               work and goes back to the planning
                               session, not into either queue
```

`src/sim/plant.rs` is the collision surface: WP-A works the root branch
gate, WP-B2 the three abscission sites, WP-B3 nothing (pure data), WP-C
two one-line edits. Different regions, but the rule stands: **commits in
plant.rs are small, land the same session they are written, and every
land is preceded by a rebase.** If a rebase conflicts, re-read the
function, not the diff (the stash-restored-blob gotcha).

### Disposition of the genome session's remaining queue (decided here)

The genome session's own handover (`plant-work-split.md` §3) listed five
verification rows plus the blocked megastudy. Decided disposition, so
nothing runs twice and nothing runs that should not:

| Their item | Disposition |
|---|---|
| Root economy call | **Answered by the owner** — becomes WP-A, theirs. |
| Megastudy | **Theirs, as WP-A2, only after WP-A** — running it before wastes 3.5 hours against numbers the repair re-baselines. |
| Slot 5 depth histogram | **Do not run standalone now.** Root architecture moves under WP-A; folded into WP-A2. |
| Slot 6 stand spread | **Do not run standalone at all.** The megastudy's stand histograms answer it for free; folded into WP-A2. |
| Slot 8 penetration (sand-bank scene) | **Deferred behind WP-A** — penetration cost is a root-economy quantity; running it against the starved economy measures the gate, not the locus. After WP-A, theirs; the sand-bank scene doubles as WP-B3's dry-substrate scenery, so build it once. |
| `seed_cost` endowment curve | **Deferred behind WP-D2 (seed decay)** — the immortal seed bank is load-bearing for recruitment today, so an endowment curve measured against it re-baselines the moment decay lands. Run it after, theirs. |
| §8e wet-stomatal anomaly | **Theirs, sequenced between WP-A and WP-A2** — an unexplained anomaly on a live locus (slot 7) contaminates the megastudy's read of that locus; chase it before launching the long run, not after. |

Everything else the form session takes over: litter (their handover
conceded the abscission gap but had no package for it), grass, the form
probes, and later foliage mass. Nothing else from their queue is
dropped.

---

## WP-B1 — Species-declared materials (Session G, day 0)

**Spec** (`plant-evolution-design.md` §3c, signed off as call 1):
`SpeciesDef` gains three `#[serde(default)]` fields —
`shoot_material: "wood"`, `root_material: "rootwood"`,
`leaf_material: "leaf"` — read at exactly the three seeding sites:
germinate's shoot (plant.rs:3800), germinate's companion root
(plant.rs:3852), the Grow arm's leaf cluster (plant.rs:2195). Keep the
existing `unwrap_or(cell.material)` fallback per site. Touch nothing
else: propagation-by-parent-copy is load-bearing and stays
(plant.rs:3845–3851's comment survives verbatim, amended to say the
three seeds now come from species data). `Reproduce`'s `"seed"`
(plant.rs:745) and `plant_moss_seed`'s `"moss"` are NOT part of this
package.

**Acceptance:** with no `.ron` edited, the standard probe run (8 trees /
30,000 frames / worldseed=1) is **byte-identical** to the pre-change
run, both drivers — the defaults resolve to the same material ids and no
RNG is touched, so any digit moving means the edit did more than it
claims. Plus one unit test: a species declaring a nonexistent material
name falls back exactly as the old code did.

**Cost:** ~20 lines. Land as its own commit; do not bundle with WP-A.

---

## WP-A — Root-branching repair (Session G)

**The decision, made by the owner — do not re-argue** (recorded in
`plant-work-split.md` §6.6): the allowance economy is intended (root
tips cannot earn carbon, live on functional-balance allocation, spend at
first affordance — do NOT lower root `Grow.cost` globally, do NOT touch
the allowance rate); the double-affordance branch gate is the accident.
Repair the shape of the purchase: **the primed-site model** from
`PLAN.md`'s M16 research note — a tip primes a branch site every N
steps; the site branches when local resource clears a threshold, so the
price is spread over time instead of demanded twice in one tick.

**Invariants the implementation must satisfy** (mechanics are G's
choice; these are not):

1. No single tick requires the tip to hold two steps' worth of carbon.
2. The full branch still costs a step's carbon, paid when the branch is
   actually taken — priming spreads the *decision*, not the bill.
3. Genome slot 1 multiplies the priming rate or its threshold — a
   consumer re-pointing; nothing in the slot map renumbers.
4. Prime state lives in the `OrganismCell` sidecar, never in `Cell`
   bits (all 16 aux bits are taken; the sidecar is the rule).

**Acceptance protocol, in order:**

1. Before any sweep: confirm `MAX_ROOT_FRACTION` binds (§8c's extreme
   setting converted most of the bed; the cap is the runaway guard).
2. `branch_chance: 0.04` was calibrated against a gate that opened
   twice in 12,000 frames — it is a different quantity now. Sweep it,
   rebuilt per point; prove the edit anchor unique before the sweep
   (the `crowding_weight` sed lesson).
3. Slot 1's paired comparison at draws −1 / 0 / +1 must finally order
   `root_cells` — write the guard test
   `root_and_shoot_branching_read_different_slots` now (its premise is
   finally true); keep `print_root_branch_slot_pairing` as the
   reproduction. Deliberately break the repair and confirm the guard
   fails.
4. By eye: §8c's two architectures (sparse diving strands vs fibrous
   mat, 10% vs 55% of root cells with 3+ root neighbours) must be
   reachable by the genome at sane settings — sheets at noon with the
   neighbour counter printed beside them.
5. Root mass moving moves water capacity (∝ `root_cells`): re-run the
   §8d wet/dry 2×2 in the same session; do not carry its numbers.
6. Frame cost: `ascii` worst-frame paired against a baseline
   re-measured the same session.

**Failure protocol:** if the primed-site shape fails in a way that
points back at the allowance economy itself, STOP and report — that
reopens the owner's decision (a) and is not the session's to reopen.

**WP-A2 — megastudy re-run,** only after WP-A lands: rebuild, confirm
the probe's first line echoes `worldseed=`, then 3 species × 8 seeds ×
16 plants × 45,000 frames, sheets at noon multiples. Gate cross-species
claims on the shape descriptors per `genetic-variability-study.md` §6.
The slot-5 and slot-6 verification rows re-run here too — root
architecture moved under both.

---

## WP-B2 — Litter (Session F, after WP-B1)

> **STATUS 2026-08-21: COMPLETE.** Litter material, all three abscission
> sites switched over, and the blocker fixed. Decay sites are now scheduled
> at the awake->settled transition instead of at cell creation, which is what
> the rule always meant -- weathering happens to matter that has come to rest.
> `Material::decays_into`/`decay_reseeds` make both ends data; the dedup
> index covers `Decay`. Acceptance: litter drains to 0 (strict-decrease
> guard), a world where nothing sheds holds exactly 0, and the paired
> settled-forest frame cost is **240.60 ms vs 257.74 ms baseline -- no
> regression**. Pending sites 105 -> 12,056, converging: a standing forest
> floor, not a leak. One cosmetic question is with the owner -- litter's
> palette may be too close to soil's to read. Full record in
> `Reports/open-bugs-handoff.md` §0.
>
> Not run: the edible-cells-near-surface count. It is a creature-side
> quantity and the creature branch sets its own bar when it consumes this.

---

## WP-B3 — Grass (Session F, after WP-B2; root re-tune after WP-A)

> **STATUS 2026-08-21: COMPLETE — all six acceptance items met.**
>
> Grass grows, reads as a sward (owner: *"looks different from trees"*, 4/5),
> reproduces, and holds a bank. Sheets:
> `target/filmstrips/wpb3-grass-sward.png` (64 founders, both drivers) and
> `wpb3-grass.png` (8 founders -- kept because it is the one that shows why a
> density knob was needed).
>
> **Item 3, the paired slope** (`plant::tests::
> a_rooted_bank_sheds_less_soil_than_a_bare_one`), measured both arms:
>
> | | bare | sod | |
> |---|---|---|---|
> | soil that left the bank | 327 | 305 | -7% |
> | bank surface still standing | 185 | 235 | **+27%** |
> | grassroot cells in the bank | 0 | 135 | (did it fire) |
>
> Both numbers are true and say different things: total spill barely moves
> because roots thread the *top* of a bank and the unrooted bulk dominates a
> whole-bank count -- which is what real sod does -- while the surface the
> roots actually reach keeps a quarter more material. Bar set at +10% with
> headroom, not on the measured value.
>
> **Item 5, slot pressure:** 40 founders, 45,000 frames, 2560-wide world ->
> 77 live organisms and 49 seeds set, so ~89 slots consumed against the 4095
> ceiling. No pressure; grass does not need WP-D to ship. Generations
> **[gen 0: 40, gen 1: 29, gen 2: 6, gen 3: 2]** -- four generations in one
> run, which is the number every §6 evolution experiment was waiting for and
> which trees cannot supply.
>
> **Item 4, the paired burn: done.** Speeds measured over identical strips
> (`fire::tests::a_fire_front_crosses_grass_faster_than_foliage_and_not_at_
> all_over_soil`), time for a front to cross 170 cells:
>
> | surface | frames | cells/frame |
> |---|---|---|
> | **grass** | **127** | **1.34** |
> | tree foliage | 161 | 1.06 |
> | leaf litter | 180 | 0.94 |
> | bare soil | never crossed | — |
>
> Grass is ~27% faster than canopy foliage and bare soil stops a front dead,
> so the "carries fire across open ground between two stands" claim holds.
> `filmstrip` gained `ignite=x,y,radius,frame` for the visual, and the GIF is
> with the owner.
>
> **The finding that came out of it, and it is emergent rather than
> authored: sward *connectivity*, not moisture, decides whether grass burns
> at all.** Fire spreads to 4-neighbours only, so at 64 founders the gaps
> between tufts stop a front dead (ignition landed — 10 cells lit — and went
> straight out), while at 200 founders it is continuous and the fire runs the
> whole world. A patchy meadow is a firebreak; a closed one is a fuse. Worth
> an owner call on whether that is wanted before anything is tuned around it.

---

## WP-C — Form probes (Session F, any time; no dependencies)

> **STATUS 2026-08-21: COMPLETE, and the answer is no.** All three probes
> built, rendered and owner-judged through the review queue (two blind).
> Verdicts: `weeping` *"same plant"* as tree; `creeper` vs `prostrate`
> *"Not that different"* (2/5). **Tally at most one against a bar of two, so
> the envelope claim is not validated** and the owner's original pushback
> stands. `weeping.ron` and `prostrate.ron` are retired, and the four-line
> order-0 tropism change in `plant.rs` is reverted with a do-not-retry note
> in place. `creeper.ron` survives. The finding worth carrying forward:
> every group change came from the size budget (`turgor_source`), none from
> an architectural knob. Full register in `plant-evolution-design.md` §4a.


**Spec** (P1a, signed off): three probes, each one `.ron` + one noon
filmstrip, judged against the four-axes bar — does it read as a new
form *class*, or as another small tree?

1. **Creeper:** pure values — `heading_inertia` ~0.0–0.1, low
   `upward_weight`, existing materials. No code.
2. **Weeping:** first *read* the Grow scorer to check whether a
   negative `upward_weight` is accepted or clamped (report which,
   with the line); if clamped, the probe includes the one-line range
   widening. Then a tree variant with negative upward weight on
   orders ≥ 1.
3. **Prostrate:** allow `Plagiotropic` at order 0 (today the tier
   reference applies at order > 0 only — a small, targeted change;
   prove trees are byte-identical after it, since no shipped species
   declares order-0 plagiotropy), then a low-turgor, tiny-internode
   species hugging the ground.

**Acceptance:** the sheet, plus the standard counters, plus an explicit
verdict per probe against the four-axes bar. **Failures are a
deliverable, not a problem:** a probe that reads as another small tree
goes to the dead-end register (`plant-evolution-design.md` §4a) with
its sheet, at filmstrip cost instead of milestone cost. Two successes
validate the envelope claim; report the tally either way.

---

## WP-D — Population plumbing (Session G, after WP-A)

**Spec** (P0, signed off): three items, each its own commit.

1. **Plant organism reclamation:** a liveness check —
   `reachable_from_anchors` from the organism's remaining anchors
   finding zero live `GrowingTip`/`Leaf`/`RootTip` — calls the
   existing `free_organism` free side (built, tested, never called
   for plants; §13m and issue #8 both name the gap). Guard: kill a
   plant, confirm its slot is reused and a stale scheduled site
   resolves to `None` without resurrecting or panicking. Count
   generation wraps in debug builds (the 4-bit wrap at 16).
2. **Seed decay:** seeds stop being immortal — a decay schedule in
   species data (the immortal seed bank is a measured leak). Guard: a
   sealed scene's seed count reaches 0; and the standard stand's
   establishment numbers are re-measured, not carried, since the seed
   bank was load-bearing for recruitment.
3. **Generation counter:** the probe prints founders vs
   inherited-genome establishments per run — the "did evolution even
   get generations" counter that gates every §6 experiment in the
   design doc, and the measurement the owner's call 7 (founder
   keying) waits on.

---

## WP-E — Foliage mass (either session, after WP-B3 and WP-A2)

Signed off as call 9's second step; spec'd at medium detail on purpose
— it starts with a measurement, not an edit. Foliage is 26–31% of
cells and renders as garnish; the goal is foliage reading as *mass*
(a conifer branch as a plume, not a stick with dots). Start by
sweeping the knobs that exist (`leaf_cluster`, `shade_death`,
`plastochron`) before proposing new mechanism — and check the
`tree.ron` discrepancy first: the sweep comment says `shade_death`
0.03 "<-- here" while the shipped value is 0.003; retune or typo,
answer it with git history before sweeping. Known coupling: foliage
carries transpiration, so mass moves the water economy — the §8d 2×2
re-baselines *again* here, which is why this waits for WP-A2's fresh
baseline. Bole legibility is the known trade (`shade_death` bought it
by cutting foliage 11,179 → 2,336 once); the acceptance judge is the
owner's eye on paired sheets, per the runtime-selector convention if
settings genuinely compete.

## WP-F — Age channel (design note first; not in either queue)

Signed off as call 9's third step, explicitly gated on its own short
design note answering *which object age grades* — a cell, a lateral,
or a tier — before any storage lands. The storage is trivial (a
birth-frame stamp in the sidecar); the consumer touches allocation and
the vein-polarity machinery. The note is planning work, not
implementation work: it goes back to the planning session when WP-E's
results exist to inform it. Do not attempt this package without the
design note, and do not write the note mid-queue in an implementation
session — the which-object question is precisely the one this repo
answers wrongly when it is asked as a rider on other work.

---

## Reporting

Every package reports per house convention: the numbers before and
after, what was tried and rejected, sheets beside counters, and the
commit message carrying the measurement. A package that cannot meet an
acceptance line stops and reports rather than tuning around it.
