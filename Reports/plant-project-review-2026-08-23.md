# Plant project review — a fresh queue, 2026-08-23

**Status: review + proposed queue.** Written from the whole plant record — `src/sim/plant.rs`
/ `organism.rs` and the species assets as they stand on `main` (`a0fa433`), all
twenty-four plant/tree reports, `PLAN.md`'s plant sections, `PLAN-log.md`,
`README.md`'s status sections, `wiki/plants.md`, the review-queue verdicts, the
open PRs, and `git log`. It proposes a **new** queue and says where it deviates
from `plant-implementation-plan.md`'s WP order and why. It builds nothing and
retunes nothing; every claim about code was verified by reading the code, not a
report about the code.

Three documents remain authoritative for their own domains and this review does
not restate them: `Reports/open-bugs-handoff.md` (reproductions),
`Reports/dead-ends.md` (do-not-retry conditions), `plant-implementation-plan.md`
(the signed-off ecology calls).

---

## 1. Where the work actually stands

The three-week plant run (substrate v2 → polarity → economy → genome → ecology →
appearance) ended 2026-08-22, and **nothing plant-side has been attempted since
the owner's last verdict**, which is therefore the standing state of the world:

> "No. Everything has merged together into a big mass. I cannot identify
> individual trees." — absolute card, `open-bugs-handoff.md` §Z
>
> "Big improvement but not a full solve." — the blind A/B, same entry

That pair is the review in miniature: the *deltas* keep winning (crown fusion 95
→ 41, establishment doubled, palette bands landed, branch angle reached 63–87°
against a 70° target) while the *absolute* standard — the wiki's own "trunks are
bare at the bottom … neighbouring crowns stay mostly separate" — still fails.

**What landed and holds** (all verified in source): the cell-typed organism
substrate and per-organism sidecar; canalized carbon transport with per-face
conductance; per-column Beer-Lambert light and the noon-equivalent economy;
`path_len` hydraulic turgor gating (`organism.rs:2273`, `plant.rs:1458`); real
leaves, litter, decay-to-soil, grass, the heritable 15-locus genome with colour
as a readout; `branch_angle`/`internode` (despite two documents still saying
they never merged — §3); root penetration, hydrotropism, allometry, primed-site
branching; and — load-bearing for everything below — **`plant::anchor_support`**,
the Dijkstra-from-anchors support field that replaced the hop-bounded search.
The amputation landmine that contaminated every Phase-3 damage result is
**structurally gone** (`plant.rs:2865`, `structural.rs:888`).

**The live defect set**, one line each (full entries in `open-bugs-handoff.md`):

| Ref | Defect | State |
|---|---|---|
| §Z | Stand reads as one mass; no metric can even fail for it | open, owner verdict |
| §A | Slot-1 root lever measures dead (0.90 ± 0.056 over 8 seeds); test CI-quarantined | open, cause unconfirmed |
| §U | Drought makes a tree *bigger* (982 vs 734 cells) — inverts dendrochronology | open |
| §Y | Gnome travels 98/200 through a wood; litter is 97 of the shortfall, mechanism open | open, PR #12 narrows it |
| §D1 | Pick and chisel **cannot damage a tree at all** (`rigid.rs:1151` skips organism cells); brush/fire license no collapse | open |
| §F1 | A litter blanket blocks rain soak entirely (`weather.rs` stops at the first `water_capacity == 0` cell) — mulch inverted | open |
| §F3 | Root drinking a water cell destroys the un-absorbed remainder | open |
| §F8 | Soil moisture has three sources and one sink; unplanted soil ratchets to field capacity | open |
| §F4 | Grass cannot die (no `Leaf`, so no abscission path), latent until grass is plantable; 4,095-slot ceiling is a `debug_assert` | latent, severe |
| §G | Grassfire: "looks like you are cycling colors … doesn't spread at all"; owner wants moisture to gate spread | open, standing verdict |
| §X | Desert is dead because arid ground is *sand* and only soil declares `water_capacity` — a design call, not a bug | needs an owner decision |
| §V | `a_tree_eventually_stops_growing` retired by owner decision; termination now unguarded | accepted, noted |

**Unactioned owner directives found in the record**, so they stop being lore:
slow growth at night (`plant-night-session-handoff.md` §2 directive 4,
2026-08-17 — income × `0.25 + 0.75·daylight_fraction`, decisions stay
phase-free; never implemented); do not lower root `Grow.cost` or the allowance
rate (the §6.6 economy call — WP-A must work inside it); rain-wetting rate
"keep on the to do list" (weather card, 2026-08-22); and §X's correction that a
species wilting point would do nothing for the desert.

---

## 2. The diagnosis this queue is built on

Four findings, each already paid for, that together say where the next unit of
effort goes:

1. **Architecture levers don't move pixels.** Sympody, tropism, acrotony all
   fired (counters proved it) and moved nothing the owner could see; the two
   probe species died blind A/Bs; WP-C's own conclusion was "every group change
   came from `turgor_source`". Composition and colour outperformed all of them
   (`plant-appearance-design.md` §5, the CLAUDE.md rule).
2. **The mass is composition, not shape.** The growth trajectory passes through
   a genuine tree (~frames 3,300–5,100) and then fills in; foliage plateaus
   while wood compounds. The named, deferred, best-candidate mechanism —
   crown recession via superlinear maintenance respiration — now has its retry
   condition met, because `q_peak` (girth) exists (`plant.rs:3013`).
3. **The verb is missing.** Nothing the player holds can hurt a plant (§D1),
   a severed tree would dissolve into sawdust (`break_free` → one `deadwood`
   powder cell), a cut trunk cannot topple (`ChunkBody` spin accrues from
   *speed*), and a topped tree never regrows (`plant.rs:2731` — the gate is
   backwards for recovery; `q_now` is computed and discarded at
   `plant.rs:3013-3014`). This is the ethos gap: destruction of the most
   destructible-looking thing in the world delivers no consequence.
4. **The economy lies in specific, cheap-to-fix places.** §U/§F1/§F3/§F8 all
   corrupt the water book that every measurement above sits on, and §A is
   entangled with the same `water_status` path. Tuning anything before these
   is tuning against a moving target.

---

## 3. Stale records this review found (fix-in-passing list)

Each of these will misdirect the next session that reads it:

- **`CLAUDE.md`'s structural-check amputation gotcha has expired.** The
  hop-bounded `organism_is_supported` no longer exists; `anchor_support`
  schedules its own checks (`plant.rs:2933`). The gotcha's *conclusion*
  ("Phase 3 damage results are contaminated") described the old world; damage
  work is unblocked. Stale siblings in source: `organism.rs:97-107` (RootTip
  doc describes the dead search), `plant.rs:4107-4117` (the 26× stranded-leaf
  workaround justified by the dead premise — re-measure whether it is still
  needed), and the three "deliberately no `schedule_structural_check_around`"
  comments (`plant.rs:1917`, `4222`, `4586`).
- **`branch-angle-and-the-width-bound.md` says "NOT merged"; it is merged**
  (`branch_angle`/`internode` live in `plant.rs`, `organism.rs`, every species
  `.ron`). `PLAN.md:1748` ("plant-branch-angle has not [landed]") is the same
  staleness. Its §4 width-bound gap is also closed in code — `path_len` exists
  and the turgor gate reads it — while §V separately retired the guard test.
- **`examples/debug_tree_variants.rs` panics on start** — it generates species
  RON with the removed `moisture_threshold` field and scalar values where
  `ByOrder` wants lists, and its scene (bare stone floor) could not germinate
  anything even if it parsed. The harness the economy pass was tuned on is
  dead weight; `plant_probe` took over ensembles.
- **`roots-and-breakage-handoff.md`'s "roots are optional / transpiration is
  free" predates the water-currency work** and is superseded: canopy
  transpiration demand is live (`plant.rs:3572`), the epiphyte guard was
  deleted *because* the economy now kills epiphytes emergently, and
  `wiki/plants.md` documents that behaviour as shipped. Its still-live items
  are the megastudy re-run and the seed-bank leak, both queued below.
- **`genome-sweep-2026-08-18.txt` (repo root) is an ant sweep** —
  `creature_space` stdout, nothing to do with plants, committed incidentally.
  Move it under `docs/` with a name that says ants, or delete it.
- **`wiki/ants.md` still says litter does not rot**; it has rotted since
  `5a9e594` (2026-08-23). One stale paragraph under a current freshness date.
- **`plant-appearance-design.md` §6 says `wiki/plants.md` does not exist.** It
  does, and its freshness note is current.
- **`grass.ron` runs an undocumented economy**: `plastochron: [0,0]` means no
  nodes, no leaves, income permanently zero, `BudBreak` unreachable — grass
  works by a path nobody wrote down. Document it or rationalise it when §F4's
  mortality work happens.

---

## 4. The queue

Three arcs and a floor. Ordering *within* an arc is real; the arcs themselves
interleave — each has a cheap first step, and nothing in Arc 1 blocks Arc 2.
Every item that changes the screen ends with a review card (paired or blind,
count in `meta`), per the house rules; every measurement is an ensemble in
`grove` (never the 40-row scenes), rebuilt before running.

### Arc 1 — a stand you can read (answers §Z)

**1.1 Crown recession: superlinear maintenance respiration.** Charge upkeep on
girth (`q_peak`) at an exponent > 1 (Takenaka's 1.5 is the cited anchor);
income already scales with foliage. Interior and overtopped wood then runs a
deficit and dies back; the crown hollows; the bole clears; and the fill-in mass
finally costs something. This was ranked the #1 missing mechanism
(`tree-architecture-research.md` §1), deferred twice, and its two recorded
blockers are both gone: girth memory exists, and the scene/light confounds are
fixed. The dead-end register requires *superlinear* — flat respiration balances
at any size and is a recorded dead end. Judge on the wood:leaf trajectory,
stem-thickness-above-base, and a paired grove card; expect to re-derive
`pipe_ratio` and the canalization contrast in the same pass (the audit already
demands the contrast be re-derived in `grove` — do it once, here, not twice).
**Fold item 3.4 (night income) in before the re-tune so the economy is only
re-derived once.** Size: L. This is the queue's centrepiece.

**1.2 Five species, five looks — data only.** The four woody `.ron`s are
near-clones: `shrub` and `creeper` declare *identical* foliage and bark bands,
and every woody species wears the same `wood`/`leaf` materials. Spread the
band assignments to disjoint ranges (the machinery already guarantees
bit-identical growth), differentiate the two or three constants that are known
live levers (`turgor_source`, `leaf_cluster`, `plastochron`), and consider
per-species materials (only grass overrides today). This is the cheapest
visible move against "I cannot identify individual trees", it needs no engine
change, and it follows the ecological-lod rule: species must be **points on a
trade-off, not appearances**, or the shared economy picks one winner. Size: S.

**1.3 Whorls / rhythmic growth.** The one deferred lever that changes *texture*
rather than statistics — tiers for the conifer, flushes for the tree. Priced in
`tree-architecture-variety-review-verification.md` §6 (`whorl_count` 3–5,
`lineage_step` as the modulus, watch its 255 saturation). It is also the only
appearance lever with a real frame cost, so quote `ascii`'s worst-frame number
in the commit. Size: M. After 1.1 — texture on top of a readable silhouette,
not instead of one.

**1.4 Turnover: something kills a healthy adult.** Today nothing does; stands
close canopy and freeze, selection stops, and §Z gets worse with time. Call 6
chose emergent mortality over a lifespan constant, and the disturbance verbs
(Arc 2) are exactly the emergent path — fire, felling, breakage, plus the
density-dependent decline that 1.1's respiration supplies for free (an
overtopped tree now starves). No new mechanism proposed here: this item is the
*measurement* that the combination actually produces gaps — generation counts
and a stand-age histogram in `plant_probe` over a long grove run. Size: S
measurement over M prerequisites. `population-dynamics-research.md` 9e's bar
(prey persistence with doubled food) fires the day this works.

**1.5 A distinguishability probe that can fail.** §Z's metric candidates
(connected canopy components at field resolution; sky-gap width per stand row)
have never been built, and the last metric (contiguous runs) was measured
saying yes while the owner said no. Build the component count into
`plant_probe`, calibrate it once against the §Z cards the owner already
answered (A = fail, B = partial), and print lineage birth/death rates while in
there — Phase 0d's "only non-circular pair" is still unprinted. If the
component count also fails to track the eye, record that and fall back to
cards-only, explicitly. Size: S.

### Arc 2 — a tree that answers the axe (the ethos arc)

The amputation blocker is gone (§3); this whole line is now unblocked, and it
is the highest-satisfaction work in the backlog. Order matters:

**2.1 Instrument first: a felling scene.** `filmstrip cut=` exists; extend it
to print the standing-organism census *and* the `chunk_bodies` count under the
sheet. A coherent-looking collapse with a body count of zero has fooled this
project once already. Size: S.

**2.2 Let tools hurt plants (§D1).** `rigid::strike`/`mine_swept` `continue`
on `organism_id != 0` and brush/fire record no disturbance — the pick
literally cannot touch a tree. Route organism cells into the same
damage/disturbance path, let `anchor_support` (which already schedules on
rising distance) declare the severed region unsupported. First shippable
state: chopping a trunk makes the crown die and come down *at all*, even if
the fall is ugly. Size: M.

**2.3 Fell as pieces, not sawdust.** Two recorded blockers: `break_free`
converts to single `deadwood` powder cells (the design-philosophy §0a failure
verbatim), and fracture's wood ladder (`fragment_rungs: 5`, uniform ≤32-cell
pieces, `MAX_BODY_CELLS = 400`) can only shred a 2,000-cell tree. A severed
subtree should promote as one piece or a few log-scale pieces plus debris — a
*distribution*, per the ethos — and `BodyCell` must carry the organism id
through promotion or the fall re-triggers structural checks from inside
itself. The good surprise on file: hand-painted wood already promotes to
bodies; the barrier is only the `organism_id != 0` routing. Size: M–L.

**2.4 Resprout: keep `q_now` beside `q_peak`.** The instantaneous conduit
vector is computed and discarded (`plant.rs:3013-3014`); the recovery gate is
backwards without it (`plant.rs:2731-2748` documents the defect: losing
foliage *reduces* the drive to rebuild, measured — two topped trees, 7,400
frames, no regrowth). Flush a `DormantBud` at `order = 0` where
`q_peak − q_now` crosses a threshold. This is simultaneously: the plan of
record's queue item 3, the verification's "cheapest high-value change", the
*sanctioned* form of bud break (event-triggered, self-limiting — the revert
postmortem's own recommendation), and the second half of the felling verb (cut
a limb, the tree answers). Acceptance is the dormancy-reversibility test the
plan already specifies: cut at frame 10,000, neighbouring buds restart. Watch
the recorded `juvenile_size` caveat — it gates on whole-organism cell count,
so a reiterating limb inside a big tree gets the mature economy. Size: M.

**2.5 Topple.** `ChunkBody` cannot rotate a just-cut trunk: spin accrues from
speed, and rotation is quarter-turn snaps needing full clearance
(`felling-blockers.md` redesigns 1–2). Stage it: v1, a severed trunk gets a
lateral impulse at the cut and breaks on impact by the normal fracture path —
crash, debris, done; v2, real torque-from-support-asymmetry if v1 reads as
fake in the GIF. Judge on `filmstrip gif=1`, never stills — the question is
whether it *moves* right. Size: L (v2), M (v1).

### Arc 3 — an economy that doesn't lie

**3.1 The water book, three entries.** (a) §F3: `absorb_water`'s liquid arm
credits at most `rate` and destroys the remainder — conserve it (leave the
rest as partial fill). (b) §F1: rain soak stops at the first
`water_capacity == 0` cell, so a litter blanket *seals* the ground it lies on
— mulch inverted; let soak pass through zero-capacity cells or give litter a
token capacity, then run the paired storm (littered vs bare) already specified
in the bug entry. (c) §F8: soil moisture has three sources and one sink, so
unplanted soil ratchets to field capacity — add the missing sink (bare-soil
surface evaporation is the obvious one). Each is small, each has a one-number
paired measure, and together they stop the substrate from drifting wet under
every future measurement. Size: S each.

**3.2 Drought must cost (§U).** A water-stressed tree currently outgrows a
watered one on every absolute. The filed hypothesis — `break_root_tips` at
`water_status < 0.95` re-initiates roots without the stress throttling the
carbon that pays for them — is unconfirmed; instrument the firing counter
first (the same counter §A asks for at `plant.rs:3017` — one instrument, two
bugs), then make stress gate income before it gates architecture. The wiki's
"thirsty plant looks root-heavy" ratio claim is the part that already works;
keep it. Paired dry/wet ensemble is the measure; the ratio *and* the absolute
must both point the right way. Size: M.

**3.3 Bug A: recalibrate with a counter, then un-quarantine.** The owner
already judged the visual difference "not obvious", so this is a
test-calibration problem, not a visible regression. Use the firing counter
from 3.2 to establish whether the amplifier is shut (the +67% uptake theory);
then either re-derive `tree.ron`'s slot-1 calibration against the post-merge
quantity or set the bar from an 8-seed order statistic
(`print_root_branch_slot_seed_sweep` already exists) with headroom — never on
the single seed that flips with litter volume. Delete the CI exclusion the
same day, per `0a345c4`'s own instruction. Size: S–M.

**3.4 Night slows growth.** The unactioned 2026-08-17 owner directive: income
× `0.25 + 0.75·daylight_fraction`, decisions stay noon-normalised. Expect
~40% stand-size shift — which is exactly why it lands *before* 1.1's economy
re-tune, inside the same single re-derivation pass. Size: S.

**3.5 Grassfire (§G).** Standing negative verdict with an explicit steer:
moisture/dryness must gate spread, and the burn must read as fire rather than
colour cycling. Split the claims: behaviour (spread rate at play scale,
moisture gating — check why `MOISTURE_IGNITION_RESISTANCE` changes nothing)
from look (flame/ember/smoke read — M14 territory). A meadow that carries fire
when dry and stops it when wet is also the desert/wet-niche mechanic §X wants
to lean on. GIF card, not stills. Size: M.

**3.6 Seed bank and slot hygiene.** Seeds never decay (455 immortal
`OrganismState`s standing at 60,000 frames), grass can never die (§F4 — no
`Leaf`, so no abscission path reaches it), and the 4,095-slot organism ceiling
is a `debug_assert` above an id encoding that does not mask — release builds
corrupt organism identity when it overflows. Give seeds a decay clock (WP-D
item 2), give grass a mortality path, make the ceiling a real check. The
endowment-curve measurement (§8h) unblocks behind seed decay. Size: M.

**3.7 The desert decision (§X).** Not a bug — a niche with no lever. Put the
three candidates to the owner as one card with costs: sand gets a small
`water_capacity` (needs the liquid-conservation tallies taught about held
water first), roots reach the water table (plays to the existing capillary
fringe behaviour), or stored-rain events. A species wilting point is already
ruled out on the record. Size: decision first.

### The floor — instruments, guards, records

- **4.1** Fix or delete `examples/debug_tree_variants.rs` (panics on start,
  stale schema, sterile scene). If `plant_probe` is the ensemble harness now,
  delete it and say so in the commit.
- **4.2** Re-measure the stranded-leaf component walk (`plant.rs:4107`) with
  `anchor_support` live; delete the workaround if its 26× justification died
  with the old search.
- **4.3** The doc sweep from §3 above: CLAUDE.md gotcha, the four stale source
  comments, the two "not merged" lines, `wiki/ants.md`'s litter paragraph, the
  ant sweep filename, `grass.ron`'s economy note. One commit, no behaviour.
- **4.4** Megastudy re-run (WP-A2) — *after* 1.1 + 3.4 land, since both move
  composition and mass. 3 species × 8 seeds × 16 plants; gate cross-species
  claims on crown profile and foliage centre, never height; rebuild first and
  check the echoed parameter line (the study has been void once already).
- **4.5** Establishment / age channel (WP-F): 5/16 trees and 4/16 conifers end
  zero-leaf; the named fix is `branch_chance` high-while-juvenile, which
  `ByOrder` cannot express because order is position, not age. Needs the
  design note first ("which object does age grade — cell, lateral, tier?"),
  per the plan. Sized M, sequenced after the arcs' first passes.

### Deliberately not now, with reasons

- **λ (excurrent/decurrent split) and any new architecture lever** — behind
  1.1/1.3. λ genuinely moves mass, unlike the label levers, but the lesson
  stands: no architecture work until a card proves which pixels it moves.
- **The auxin/apical-dominance channel** — the plan of record marks it
  redundant with Phase 4's allocation: do not build both.
- **Root morphology / root thickening** — two negative owner verdicts; the
  taproot family needs root thickening (`can_widen` early-returns in soil) and
  soil displacement, a real mechanism build for a variety axis the owner
  ranked below canopy work. The reachable-now piece is a *render*: shallow- vs
  deep-fibrous via genome slot 5, posted as a card. Build nothing yet.
- **Moss overhaul** — deliberately deferred (call 4); moss still has no
  economy; fine while it stays decoration.
- **Live-state colour (drought pallor, autumn, bark aging)** — appearance §7's
  own precondition (foliage a visible fraction) is only half met; seasons and
  the temperature-channel treatment don't exist yet. Revisit after 1.1.
- **`light_weight`/`phototropism_dir`** — inert by construction (codomain
  `{(0,−1),(0,0)}`); the honest fix is a lateral light gradient in the field,
  which is field-pass work with a frame cost, and nothing needs it until a
  shade-niche species exists. Keep the parked note.
- **Snow-defoliation (§F2)** — observe once in winter weather before deciding;
  "nobody designed deciduous winters; they may be lovely."
- **Idleness-triggered bud break** — the do-not-retry stands in full; 2.4 is
  the sanctioned, event-triggered form.
- **P5 speciation, dispersal materials, ecological LOD** — all downstream of
  turnover existing at all (1.4).

---

## 5. Deviations from the written plans, stated

- `plant-implementation-plan.md` queues WP-A (root repair) first. This queue
  runs it as 3.2/3.3 *behind* the instrument, because the owner's verdict
  ("not obvious") reframed it from regression to calibration — and because two
  arcs of owner-visible work (1.1, 2.2) should not wait behind a test bar.
- WP-E (foliage mass) is folded into 1.1's judgement rather than run as its
  own tuning pass — `leaf_cluster` already moved share to 26–31%, and the
  remaining mass problem is die-back, not leaf count.
- The tree-architecture plan's queue item 2 (re-test `light_weight`) is
  dropped, not deferred — the verification proved it inert by construction.
- `felling-blockers.md`'s "six schedulers are amputation triggers" premise is
  re-scoped by `anchor_support` landing; 2.2 treats scheduling checks on
  organisms as the *goal*, not the hazard.
- The night-session handoff's directive 4 (night growth) is promoted from
  buried handoff bullet to 3.4, because it is an explicit owner instruction
  that predates and outranks everything tuned since.
