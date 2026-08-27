# Plant project review — a fresh queue, 2026-08-23

**Status: review + proposed queue.** Written from the whole plant record — `src/sim/plant.rs`
/ `organism.rs` and the species assets as they stand on `main` (`a0fa433`), all
twenty-four plant/tree reports, `PLAN.md`'s plant sections, `PLAN-log.md`,
`README.md`'s status sections, `wiki/plants.md`, the review-queue verdicts, the
open PRs, and `git log`. It proposes a **new** queue and says where it deviates
from `plant-implementation-plan.md`'s WP order and why. It builds nothing and
retunes nothing; every claim about code was verified by reading the code, not a
report about the code.

**Revised the same day, on the owner's direction.** The first draft led with
tree refinement. The owner's response: the plan ignored two more important
pushes — (1) a genetic/evolution framework that produces *many kinds of
plants* rather than "tree and small tree", and (2) root differentiation, where
`root-morphology-findings.md` shows whole families are inexpressible, and the
owner's standing constraint is that morphologies must **develop or evolve to
fill niches, not be hardcoded**. The queue below is restructured around those
two pushes; the tree arcs survive but no longer lead. The revision also added
one decisive finding (§2.0) the first draft missed.

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
morphologies must be able to develop or evolve, never hardcoded
(`root-morphology-findings.md`, reaffirmed 2026-08-23); slow growth at night
(`plant-night-session-handoff.md` §2 directive 4, 2026-08-17 — income ×
`0.25 + 0.75·daylight_fraction`, decisions stay phase-free; never
implemented); do not lower root `Grow.cost` or the allowance rate (the §6.6
economy call — WP-A must work inside it); rain-wetting rate "keep on the to do
list" (weather card, 2026-08-22); §X's correction that a species wilting point
would do nothing for the desert.

---

## 2. The diagnosis this queue is built on

**2.0 The world contains one woody species.** `worldgen`'s `life_scatter` sows
exactly two things: moss, and `plant_tree_species(x, above, "tree")` — the
string is hardcoded (`passes.rs:3880`). Conifer, shrub, creeper and grass have
**never been planted in a generated world**; they exist only in probe scenes
and review cards. The owner's "we basically have tree and small tree" is
literally the world's contents, and the diversity that already exists in
`assets/species/` has never had the chance to appear, compete, or fail. Found
only during this review's revision; nothing in the record flags it.

**2.1 Architecture levers don't move pixels.** Sympody, tropism, acrotony all
fired (counters proved it) and moved nothing the owner could see; the two
probe species died blind A/Bs; WP-C's own conclusion was "every group change
came from `turgor_source`". Composition and colour outperformed all of them
(`plant-appearance-design.md` §5, the CLAUDE.md rule).

**2.2 The mass is composition, not shape.** The growth trajectory passes
through a genuine tree (~frames 3,300–5,100) and then fills in; foliage
plateaus while wood compounds. The named, deferred, best-candidate mechanism —
crown recession via superlinear maintenance respiration — now has its retry
condition met, because `q_peak` (girth) exists (`plant.rs:3013`).

**2.3 The verb is missing.** Nothing the player holds can hurt a plant (§D1),
a severed tree would dissolve into sawdust (`break_free` → one `deadwood`
powder cell), a cut trunk cannot topple (`ChunkBody` spin accrues from
*speed*), and a topped tree never regrows (`plant.rs:2731` — the gate is
backwards for recovery; `q_now` is computed and discarded at
`plant.rs:3013-3014`).

**2.4 The economy lies in specific, cheap-to-fix places.** §U/§F1/§F3/§F8 all
corrupt the water book that every measurement above sits on, and §A is
entangled with the same `water_status` path. Tuning anything before these is
tuning against a moving target — and **selection breathes through the same
economy**: an evolution framework built on an economy where drought grows
bigger trees will select for nonsense.

**2.5 Root form is inexpressible, not mistuned** (`root-morphology-findings.md`).
Two hard gaps: `thicken()`'s `can_widen` requires an `EMPTY` or own-`Leaf`
neighbour, and a root is buried in `Powder`, so **no root can ever thicken**
— the whole taproot family (conical/fusiform/napiform) is off the map, since
those are *thickness* forms. And `allocate_to_frontier` is democratic — a
primary axis and a third-order lateral get the same treatment — so **nothing
can make one root dominant** and a fibrous mat is the only possible outcome.
Two negative owner verdicts stand against tuning-level root variety; the
constraint on record is a *system* where morphologies develop or evolve.

---

## 3. Stale records this review found (fix-in-passing list)

**Status as of 2026-08-27 — 8 items, all now closed, but four of them stood
for four days after this list named them with the address attached.** Closed
by the session that noticed the pattern: the amputation gotcha, `wiki/ants.md`
and `grass.ron` were fixed within a day *because each sat in a file someone
had to open anyway*; `branch-angle`'s "NOT merged", `debug_tree_variants`'s
panic, `plant-appearance-design` §6 and the root-level sweep all sat in files
nobody had reason to open, and none moved. **That is the finding this list
accidentally produced: a defect named in prose does not get fixed, a defect a
check can see does.** Three checks now cover these classes —
`docscheck` 3b (widened), 3c (a report outliving its branch) and 5b (an
example emitting a field the engine no longer has) — plus `--selftest`, so
their green means something. Individual items below are left as written; the
record of what was wrong is worth more than a tidied list.

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
  `ByOrder` now demands lists, and its scene (bare stone floor) could not
  germinate anything even if it parsed. The harness the economy pass was tuned
  on is dead weight; `plant_probe` took over ensembles.
- **`roots-and-breakage-handoff.md`'s "roots are optional / transpiration is
  free" predates the water-currency work** and is superseded: canopy
  transpiration demand is live (`plant.rs:3572`), the epiphyte guard was
  deleted *because* the economy now kills epiphytes emergently, and
  `wiki/plants.md` documents that behaviour as shipped. Its still-live items
  are the megastudy re-run and the seed-bank leak, both queued below.
- **`genome-sweep-2026-08-18.txt` (repo root) is an ant sweep** — *(closed 2026-08-27: moved to `docs/ant-genome-sweep-2026-08-18.txt`.)*
  `creature_space` stdout, nothing to do with plants, committed incidentally.
  Move it under `docs/` with a name that says ants, or delete it.
- **`wiki/ants.md` still says litter does not rot**; it has rotted since
  `5a9e594` (2026-08-23). One stale paragraph under a current freshness date.
- **`plant-appearance-design.md` §6 says `wiki/plants.md` does not exist.** It
  does, and its freshness note is current.
- **`grass.ron` runs an undocumented economy**: `plastochron: [0,0]` means no
  nodes, no leaves, income permanently zero, `BudBreak` unreachable — grass
  works by a path nobody wrote down. Document or rationalise it as part of
  Arc A's grass work. `creeper.ron` likewise silently runs the *superseded*
  in-tick branching path (`branch_chance` with no `branch_priming`) — decide
  before sowing it.

---

## 4. The queue

Five arcs and a floor. The first two are the owner's pushes and lead; the tree
arcs survive behind them; the economy arc feeds everything and interleaves.
Every item that changes the screen ends with a review card (paired or blind,
count in `meta`), per the house rules; every canopy measurement is an ensemble
in `grove` (never the 40-row scenes), rebuilt before running.

### Arc A — many kinds of plants: the evolution framework

The end state, per the owner: variety comes out of genetics and selection, not
out of hand-authored species files. Grass was the first step and the work
stopped. What that takes, in dependency order:

**A1. Put the flora that already exists into the world.** §2.0: worldgen sows
`"tree"` and moss, nothing else. Sow the rest by the conditions the materials
already express — grass on soil by moisture, shrub toward drier/rockier
columns, conifer by its own band, creeper where footing suits it — clustered
by the same squared-noise device `life_scatter` already uses. **Grass is
gated behind A2's mortality fix** (§F4: a plantable grass that cannot die is
an organism-slot leak ending in silent id corruption at the 4,095 ceiling);
the woody species carry no new hazard and can be sown first. Guard with a
seed sweep (worlds are procedural; the guard must gate an order statistic),
measure per-species establishment, and post a generated-world panorama card.
Cheapest diversity win in the backlog — the species, genome and colour system
all exist and have simply never been planted. Size: S–M.

**A2. Close the generation loop: death, seed decay, slot hygiene.** Nothing
kills a healthy adult (stands close canopy and freeze; selection stops), seeds
never decay (455 immortal `OrganismState`s standing at 60,000 frames), grass
has no mortality path at all (§F4), and the organism-slot ceiling is a
`debug_assert` over an id encoding that does not mask. Give grass an
abscission-equivalent (or a senescence path that fits its economy once A1
documents it), seeds a decay clock (WP-D item 2), the ceiling a release-mode
check, and let disturbance + the Arc-C respiration deficit be the adult
mortality paths (call 6 chose emergent mortality over a lifespan constant —
this queue honours that). **Without generations there is no evolution
framework to build**; grass, at four generations per 45,000-frame run, is the
proving ground. Size: M.

**A3. Make the genome span a strategy space, not a stat sheet.** Live,
outcome-moving axes today: turgor (height budget, r ≈ −0.75 twice replicated),
leaf economy (the wet/dry crossover — wet favours acquisitive +21% mass, dry
favours conservative +51% foliage), wood density (both sides measured).
Dead or unproven: slot 1 root branching (bug A — Arc E revives it), slot 5
root tropism (never measured in-world), seed strategy (deferred by owner call,
endowment plumbing already landed). The work: revive the dead root loci, land
the seed-endowment response curve behind A2's seed decay, and express the
trade-offs as *data* so species are points on axes (Grime's CSR triangle is
the frame `ecological-lod-design.md` §9 recommends) — "a fern, a bush, a grass
as three looks with the same underlying economics produces one winner and two
also-rans", which is exactly what the near-clone `.ron`s currently are
(`shrub` and `creeper` even share both palette bands). Size: M, mostly data
and measurement.

**A4. Prove selection moves: the divergence instrument (P3).** A two-patch
scene (wet bank / dry bank), allele census per patch per generation, run long.
The §8d crossover already says which way frequencies should drift; watch them
do it. This is the acceptance test for "an evolution framework" — not a new
mechanism, a measurement that the existing ones compose. Also the first
consumer of `population-dynamics-research.md`'s warnings: an absorbing
extinction state needs the seed bank as reservoir (it already is one, once
seeds decay rather than accumulate). Size: M.

**A5. Dispersal (P5d).** Per-species seed material — mass, float, carry — so
strategies sort in space and patches can differentiate. Wind/water do the
rest with mechanics that already exist. After A4 proves in-place selection.
Size: M.

### Arc B — root differentiation

The constraint on record: *"create a system where these types of morphologies
can develop or evolve naturally"* — not authored root types. §2.5 names the
two mechanism gaps; the sequence below opens the space, then hands it to the
genome.

**B1. Render the axis that already exists.** Shallow-fibrous vs deep-fibrous
(turf vs prairie) is reachable today: slot 5 (root tropism gain) is live and
the water economy already prices surface moisture against a deep table. The
findings report's own method correction applies: compare **single plants at
high zoom or an N-per-treatment grid**, never stand medians (root cells span
90–1,435 in one stand; a median is not a shape). Post the card before
building anything — it establishes the one heritable root axis that exists
and gives Arc B its baseline. Size: S.

**B2. Let roots thicken.** Extend `can_widen` so a root can displace soil the
way root *growth* already does (`displace_soil_water` is the named prior art —
conserve the water, push the soil, never delete either). `SecondaryThicken`
already runs on roots and does nothing — machinery present, gate unreachable.
This single change makes the taproot *thickness* family expressible at all.
Watch the recorded near-miss: soil relocation was once rejected as a two-cell
write in the least-verified subsystem, with the explicit revisit condition
"if soil visibly disappearing under a mature tree reads badly in play" —
displacement into neighbours is the shape that landed for growth; reuse it.
Size: M.

**B3. Break the democratic frontier — with the mechanism the shoot already
uses.** Nothing can make one root dominant because `allocate_to_frontier`
cannot tell a primary axis from a lateral. The engine already owns the answer:
canalization — use-strengthened per-face conductance — is running on every
organism cell today. Let allocation below the collar weight by conductance
(or by the supply direction it already derives), and dominance becomes
*emergent*: the root that carries more flow strengthens, exactly the
"develop naturally" the owner asked for. Then expose the contrast as a
genome locus: low contrast → fibrous, high contrast → taprooted, and the
form becomes **heritable and selectable** rather than authored. Guard: the
same `VEIN_GAIN = 0`-reproduces-isotropic discipline the shoot polarity
shipped with. Size: M–L. This is the centrepiece of the push.

**B4. Give root form something to be selected on.** A heritable axis without
a niche is jitter. Two niches are already half-built: a deep water table
(taproot country — and one of §X's three desert levers is "let a root reach
the water table", so the desert decision and this item should be made
together), and surface-moisture country (fibrous). Anchorage becomes the
second selection axis the day Arc D's felling and storms exist (root mass =
uptake *and* holding on — `roots-and-breakage-handoff.md`'s point, still
true). Depends on Arc E first: §U must be fixed and bug A revived, or root
genetics select on a lying economy. Size: M, mostly scenes + measurement.

### Arc C — a stand you can read (answers §Z)

**C1. Crown recession: superlinear maintenance respiration.** Charge upkeep on
girth (`q_peak`) at an exponent > 1 (Takenaka's 1.5 is the cited anchor);
income already scales with foliage. Interior and overtopped wood runs a
deficit and dies back; the crown hollows; the bole clears; the fill-in mass
finally costs something — and Arc A2 gets its emergent adult mortality from
the same mechanism. Ranked the #1 missing mechanism in
`tree-architecture-research.md` §1, deferred twice, both recorded blockers now
gone. The dead-end register requires *superlinear* — flat respiration is a
recorded dead end. Judge on wood:leaf trajectory, stem-thickness-above-base,
and a paired grove card; re-derive `pipe_ratio` and the canalization contrast
in the same single pass (the audit already demands the contrast be re-derived
in `grove`), with **E4 (night income) folded in first** so the economy is
re-derived once, not twice. Size: L. Still the biggest single silhouette item.

**C2. Five species, five looks — data only.** The four woody `.ron`s are
near-clones (§2.1, A3); spread the palette bands to disjoint ranges and
differentiate the two or three constants that are known live levers
(`turgor_source`, `leaf_cluster`, `plastochron`). Folds into A3's
strategy-space work — listed separately because it is also the cheapest
visible move against "I cannot identify individual trees" and needs no design.
Size: S.

**C3. Whorls / rhythmic growth.** The one deferred lever that changes
*texture* (conifer tiers). Priced in the verification report (`whorl_count`
3–5, `lineage_step` as modulus, watch its 255 saturation); the only appearance
lever with a real frame cost, so quote `ascii`'s worst-frame number. Size: M.

**C4. A distinguishability probe that can fail.** §Z's candidates (connected
canopy components at field resolution; sky-gap width) have never been built;
the last metric said yes while the owner said no. Calibrate against the two
answered §Z cards (A = fail, B = partial); print lineage birth/death rates
while in there (Phase 0d's "only non-circular pair", still unprinted). If the
metric fails to track the eye, record that and fall back to cards-only,
explicitly. Size: S.

### Arc D — a tree that answers the axe

The amputation blocker is gone (§3); this line is unblocked. Order matters:

**D1. Instrument: a felling scene** in `filmstrip` printing the
standing-organism census *and* the `chunk_bodies` count under the sheet — a
coherent-looking collapse with a body count of zero has fooled this project
once. Size: S.

**D2. Let tools hurt plants (§D1).** `rigid::strike`/`mine_swept` `continue`
on `organism_id != 0`; brush and fire record no disturbance. Route organism
cells into the damage path and let `anchor_support` declare the severed
region. First shippable state: chopping a trunk brings the crown down *at
all*. Size: M.

**D3. Fell as pieces, not sawdust.** `break_free` converts to single
`deadwood` powder cells (the design-philosophy §0a failure verbatim); wood's
fracture ladder (uniform ≤32-cell rungs, `MAX_BODY_CELLS = 400`) can only
shred a tree. Promote the severed subtree as one piece or a few log-scale
pieces plus debris — a *distribution*; `BodyCell` must carry the organism id
through promotion. Size: M–L.

**D4. Resprout: keep `q_now` beside `q_peak`.** The instantaneous conduit
vector is computed and discarded (`plant.rs:3013-3014`); the recovery gate is
backwards without it (`plant.rs:2731-2748`: losing foliage *reduces* the
drive to rebuild — measured, two topped trees, 7,400 frames, nothing). Flush
a `DormantBud` at `order = 0` where `q_peak − q_now` crosses a threshold.
Simultaneously: the plan of record's damage item, the verification's
"cheapest high-value change", the *sanctioned* form of bud break
(event-triggered — the revert postmortem's own recommendation), and the
second half of the felling verb. Acceptance: cut at frame 10,000, neighbouring
buds restart. Watch the `juvenile_size` caveat (it gates on whole-organism
cell count). Size: M.

**D5. Topple.** `ChunkBody` cannot rotate a just-cut trunk (spin accrues from
speed; rotation is quarter-turn snaps needing full clearance). Stage it: v1, a
lateral impulse at the cut and normal fracture on impact; v2, real
torque-from-support-asymmetry only if v1 reads as fake. Judge on
`filmstrip gif=1`, never stills. Size: M (v1), L (v2).

### Arc E — an economy that doesn't lie

Selection (Arc A), root niches (Arc B) and every tuning pass above breathe
through this economy; these come early and interleave.

**E1. The water book, three entries.** §F3: conserve the un-drunk remainder of
an absorbed water cell. §F1: rain soak must not stop dead at a
zero-capacity litter cell — mulch currently *seals* the ground it lies on;
paired storm (littered vs bare) is the specified measure. §F8: give
unplanted soil its missing sink (bare-surface evaporation) so the substrate
stops ratcheting to field capacity. Each is small with a one-number paired
measure. Size: S each.

**E2. Drought must cost (§U).** Instrument the `break_root_tips` firing
counter first (the same counter §A asks for at `plant.rs:3017` — one
instrument, two bugs), then make stress gate income before it gates
architecture. The wiki's ratio claim (root-heavy when thirsty) already works;
the absolute must invert. Paired dry/wet ensemble. Size: M.

**E3. Bug A: recalibrate with the counter, then un-quarantine.** The owner
judged the visual "not obvious", so treat as test calibration: establish
whether the amplifier is shut (the +67%-uptake theory), then re-derive the
bar from an 8-seed order statistic (`print_root_branch_slot_seed_sweep`
exists) with headroom — never a single seed. Delete the CI exclusion the same
day, per `0a345c4`'s own instruction. Feeds Arc B directly: slot 1 *is* the
root-branching gene. Size: S–M.

**E4. Night slows growth.** The unactioned 2026-08-17 owner directive: income
× `0.25 + 0.75·daylight_fraction`, decisions stay noon-normalised. Expect a
large stand-size shift — which is why it lands *inside* C1's single economy
re-derivation, not after it. Size: S.

**E5. Grassfire (§G).** Standing negative verdict with an explicit steer:
moisture/dryness gates spread (find why `MOISTURE_IGNITION_RESISTANCE`
changes nothing), and the burn must read as fire. A meadow that carries fire
when dry is also a turnover mechanism for Arc A2 and a niche edge for Arc B4.
GIF card. Size: M.

**E6. The desert decision (§X).** Not a bug — a niche with no lever. Put the
three candidates to the owner as one card with costs: sand gets a small
`water_capacity` (the conservation tallies must learn about held water
first), roots reach the water table (= Arc B4's taproot niche — decide these
together), or stored-rain events. A species wilting point is already ruled
out on the record. Size: decision first.

### The floor — instruments, guards, records

- **F1.** Fix or delete `examples/debug_tree_variants.rs` (panics on start,
  stale schema, sterile scene). If `plant_probe` is the ensemble harness now,
  delete it and say so in the commit.
- **F2.** Re-measure the stranded-leaf component walk (`plant.rs:4107`) with
  `anchor_support` live; delete the workaround if its 26× justification died
  with the old search.
- **F3.** The doc sweep from §3: CLAUDE.md gotcha, the four stale source
  comments, the two "not merged" lines, `wiki/ants.md`'s litter paragraph, the
  ant sweep filename, `grass.ron`'s economy note, `creeper.ron`'s superseded
  branching path. One commit, no behaviour.
- **F4.** Megastudy re-run (WP-A2) — after C1 + E4 land (both move
  composition and mass). 3 species × 8 seeds × 16 plants; gate cross-species
  claims on crown profile and foliage centre, never height; rebuild first and
  check the echoed parameter line (the study has been void once already).
- **F5.** Establishment / age channel (WP-F): 5/16 trees and 4/16 conifers end
  zero-leaf; the named fix is `branch_chance` high-while-juvenile, which
  `ByOrder` cannot express because order is position, not age. Design note
  first ("which object does age grade — cell, lateral, tier?"), per the plan.

### Deliberately not now, with reasons

- **λ (excurrent/decurrent split) and any new shoot-architecture lever** —
  behind C1/C3; no architecture work until a card proves which pixels it
  moves. (Arc B3 is not this: it changes *allocation*, i.e. where mass goes.)
- **The auxin/apical-dominance channel** — the plan of record marks it
  redundant with the allocation pass: do not build both.
- **Hand-authored root-type species** — explicitly against the owner's
  constraint; Arc B exists so root forms come out of mechanism + selection.
- **Moss overhaul** — deliberately deferred (call 4); moss still has no
  economy; fine while it stays decoration.
- **Live-state colour (drought pallor, autumn, bark aging)** — appearance §7's
  own precondition (foliage a visible fraction) is only half met; revisit
  after C1.
- **`light_weight`/`phototropism_dir`** — inert by construction; the honest
  fix is a lateral light gradient in the field, with a frame cost, and
  nothing needs it until a shade-niche species exists. Keep the parked note.
- **Snow-defoliation (§F2 bug)** — observe once in winter weather before
  deciding; "nobody designed deciduous winters; they may be lovely."
- **Idleness-triggered bud break** — the do-not-retry stands in full; D4 is
  the sanctioned, event-triggered form.
- **Ecological LOD / off-screen catch-up** — after the on-screen ecology has
  generations at all.

---

## 5. Deviations from the written plans, stated

- **The owner's 2026-08-23 direction supersedes the first draft's ordering**:
  the evolution framework and root differentiation lead; tree refinement
  follows. This is recorded here so the next session does not "correct" the
  order back.
- `plant-implementation-plan.md` queues WP-A (root repair) first. This queue
  runs it as E2/E3 behind the instrument, because the owner's verdict ("not
  obvious") reframed it from regression to calibration — but it gains a
  second justification: slot 1 is the root-branching gene Arc B needs alive.
- WP-E (foliage mass) is folded into C1's judgement rather than run as its
  own tuning pass — `leaf_cluster` already moved share to 26–31%, and the
  remaining mass problem is die-back, not leaf count.
- The tree-architecture plan's queue item 2 (re-test `light_weight`) is
  dropped, not deferred — the verification proved it inert by construction.
- `felling-blockers.md`'s "six schedulers are amputation triggers" premise is
  re-scoped by `anchor_support` landing; D2 treats scheduling checks on
  organisms as the *goal*, not the hazard.
- The night-session handoff's directive 4 (night growth) is promoted from
  buried handoff bullet to E4, because it is an explicit owner instruction
  that predates and outranks everything tuned since.
- `root-morphology-findings.md` closed with "render the fibrous axis, build
  nothing yet". Arc B keeps its render-first step (B1) but **does** queue the
  two mechanism builds (B2/B3), because the owner has now asked for the push
  directly; the findings' method corrections (single plants at zoom, never
  stand medians) carry into every B measurement.
