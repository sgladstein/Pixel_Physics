# The organ package: flowers, fruit, determinacy and a price (2026-08-29)

**Status: built and landed.** Phase 4 of the plant-morphology programme,
against `Reports/plant-organs-handoff-2026-08-28.md`. The acceptance artifact
is a blind review card (`20260829T005132631Z-0b56d4`), posted before this was
called done; its verdict is not in yet and this report does not claim one.

## 1. What was built

Two organ cell types with their own materials, a determinate axis that
terminates in one, a carbon price on building them, and two authored habits
that use all three.

**None of it is a new growth engine**, which was the handoff's own framing and
is worth restating because it is what kept the change small: the production
rule was already data (`SpeciesDef::fates`, Phase 1), so organs are new
*values* in a table that exists plus the materials and the bill.

| piece | what |
|---|---|
| `CellType::Flower` / `Fruit` (8, 9) | inert cells; `is_organ()` is read by three rules |
| `flower.ron`, `fruit.ron`, `windfall.ron` | 8, 6 and 6 palette bands; the load-bearing half |
| `Fate::after_metamers` | determinacy, as a filter on the production rule |
| `FateWhen::Ripe` + `Behavior::Ripen { rate, cost }` | the organ clock: flower → fruit → drop |
| `Grow::organ_cluster` | the head: an organ is a cluster, not a cell |
| `ORGAN_CONSTRUCTION_MULTIPLE = 2.0` | charged at the decision, paired with four counters |
| `OrganismState::organ_cells` | organs counted apart from `shoot_cells` |
| `herb.ron`, `bramble.ron` | one erect determinate axis; sympodial repeated axes |

## 2. The finding that shaped it, and what it forced

Handoff §4b: **a label change has failed to read five times** — `weeping`
(*"same plant"*), `prostrate` (*"Not that different"*), sympody, tropism,
acrotony, and founder variance (*"These look almost identical"*) — every one
of them firing with counters printed. The single lever that ever read was
grass at 4/5, and grass changed **material**.

So the three `.ron` files were written before any of the machinery, and two
design decisions follow directly from that finding rather than from botany:

**2a. Three materials, not one.** An attached fruit has to hang, so it must be
`Plant` kind — an organism-owned `Powder` falls the tick it is created, which
is exactly what a seed does and exactly what a fruit must not do until it is
ripe. So the hanging fruit is `fruit` and the dropped one is `windfall`, and
the transition is the mechanism rather than a repaint.

**2b. `organ_cluster`, because one cell is the material half without the size
half.** A single bright pixel at the top of a stalk is a material change with
the size half missing, and size *plus* material is the only combination that
has ever read here. The head is built by a flood outward from the terminating
apex, charged per cell and truncated by what the apex can pay — the same shape
as the leaf spray, minus its exclusion rules, because a terminal organ has no
tip left to wall in.

## 3. Determinacy, and why it is a field on `Fate`

`FateWhen` is the lookup *key* and is fieldless by construction; a condition
carrying a number cannot be a variant of it without making the key a compound.
So `after_metamers` is a field on `Fate`, and rule ordering does the rest: a
species lists its determinate rule first and the ordinary rule underneath
catches every step before the count is met.

The count itself is free. `lineage_step` is the `plastochron` the active site
already carries, and metamers are `lineage_step / plastochron_interval`, so
determinacy costs no state and no traversal.

**The lookup is hoisted above every growth gate**, and that is load-bearing
rather than tidy. The resource gate, the tip cap and the turgor bound all
`continue`, so a fate lookup placed after them would have made "the axis
terminates in a flower" conditional on the axis being able to grow — which is
backwards. A determinate axis has *finished*; whether it could have taken
another step is no longer the question.

This is aimed squarely at the tolerant half of the substrate.
`plant-fate-viability-2026-08-28.md` §2: `becomes` and `lateral` took 34
mutations without one sterile plant, while `child` killed 5 of 6. A
determinate axis ending in an organ is a `becomes` rule and never touches
`child`.

## 4. Two accounts, and the split is principled

- **Construction** — building the organ cells — is charged **at the decision,
  from the acting cell's own carbon**, per the owner's 2026-08-27 ruling.
  `ORGAN_CONSTRUCTION_MULTIPLE = 2.0` is denominated against
  `LEAF_CONSTRUCTION_MULTIPLE` (1.2) and `WOOD_CONSTRUCTION_MULTIPLE` (0.8):
  the *ordering* is what carries meaning, and it is the biology's.
- **Ripening** — setting fruit, and provisioning the seed that rides down in
  it — is charged to the organism's **`reproductive_budget`**, which is
  `plant-equilibrium-costs-2026-08-27.md` §10b option C applied to the one
  case where it is unambiguous: filling a fruit is not construction, it is
  reproduction.

**The second half was not a design choice at the start; it was forced by a
measurement, and the measurement is the useful part of this report.**
`allocate_to_frontier` classifies every non-frontier, non-leaf cell as a
*donor* — carbon flows out of it toward the tips and nothing puts any back. An
organ is therefore permanently poor, so a flower charged against its own
carbon could never pay to set. Measured on six plants:

| | flowers standing | fruit standing |
|---|---|---|
| charged to the organ's own carbon | 35 | **2** |
| charged to `reproductive_budget` | 69 | **22** |

The two authored clock rates predict about 58 fruit at steady state. The
flowers were not waiting, they were **stuck** — and the picture showed a stand
that flowers and does not fruit, with every event counter looking healthy.
`World::organ_ripening_blocked` exists so that the two states are
distinguishable next time.

## 5. The `shoot_cells` exemption

The handoff names this as the sharpest single item in the phase and it is
right. `shoot_cells` feeds **three** consumers: `seed_maturity_met`, the
juvenile check, and the per-bearer denominator that spreads a tick's
affordable seeds over the crown. Counting organs in it would therefore
*advance* the maturity fence and *dilute* the seed rate at the same time — a
two-sided reallocation of an economy PR #84 calibrated to bind at ~79%,
arriving as a side effect of a materials change nobody would connect to it.

Organs are excluded and counted in `organ_cells` instead, so the exclusion has
a reader. They are also kept out of `crown_moment` and the collar/top span,
for the same reason stated at the site: those are calibrated against
vegetative shoot mass.

## 6. Measured

`plant_probe trees=8 frames=25000`, one world seed:

| | organs built | axes terminated | fruit dropped | construction binds | cluster binds |
|---|---|---|---|---|---|
| `herb` | 222 | 27 | 74 | 35.7% | 32.1% |
| `bramble` | 1,126 | 188 | 500 | 27.1% | 11.6% |

**The prices bind**, which is the thing that had to be checked before any of
those numbers meant anything: a construction charge that never binds is real
arithmetic that changes nothing, and a sweep over the multiple would report a
converged-looking null. Roughly a third of `herb` axes that reach their
metamer count cannot pay for a head, and a third of wanted head cells are
refused — so heads are a spread of sizes, which is the first ethos law.

Standing census on the card's own scene (three plants per species, 30,000
frames), which is a different question from the event counters and the one a
picture can be checked against:

| | flower | fruit | windfall on the ground |
|---|---|---|---|
| `herb` | 46 | 25 | 0 |
| `bramble` | 19 | 81 | 27 |

The two are opposite, which is the point: the erect habit is mostly in flower
at any moment and the sprawling one mostly in fruit.

## 7. Four ways this went wrong, all caught by looking rather than by a number

Recorded because each is a repeat of a failure `CLAUDE.md` already names, and
each looked like a different problem than it was.

**7a. The turgor bound cut the axis short of its metamer count.**
`turgor_source: 0.32` with `turgor_per_cell: 0.006` puts the height ceiling at
`(0.32 − 0.1) / 0.006 = 36` cells of path length, and 9 metamers at a
plastochron of 4 needs 36. So the axis reached its ceiling exactly as it
reached its count, with the taper slowing it for a long way before that:
**9 axes terminated across 32 established plants**, and a contact sheet of six
showed one flower. This is the determinacy trap's own symptom — a low counter
with no visible cause — with the cause outside the fate table entirely. **The
metamer count and the turgor bound have to be set together.**

**7b. `max_active_tips: 1` is not "one axis", it is *no growth*.** The cap gate
reads `active_tip_count >= max_active_tips` and the tip asking is itself
counted. A whole stand germinated, staled at one cell, went senescent for want
of a vital cell and rotted away inside 8,000 frames — population zero, which
reads exactly like a broken species file.

**7c. A stand that flowers and does not fruit.** §4 above.

**7d. The guard's own scene did not contain the situation.** A first version of
the determinacy test built its own soil bed and ran `parallel::step` alone —
no `step_active_sites`, no `field::step` — and reported **4 cells of tissue
standing**: four ungerminated seeds, which reads as "the mechanism does
nothing". `CLAUDE.md` records two Phase-2 "root bugs" that were the identical
mistake.

**And one arithmetic failure worth keeping separately**: the same test then
asserted `> 0` terminations over three plants and 14,000 frames, where the
measured rate (3.4 per plant per 25,000 frames) predicts **0.6**. It went red
for the arithmetic rather than for a fault. A termination claim's budget has
to come from a measured curve.

## 8. Guards

| test | what it is for |
|---|---|
| `after_metamers_gates_a_determinate_rule_and_leaves_the_ordinary_one_answering` | both sides of the boundary; a version checking only the "on" side would pass on an implementation that ignored the field |
| `an_organ_takes_the_species_organ_material_not_its_parents` | the load-bearing half, **and** that non-organ types still propagate their parent's material |
| `a_determinate_species_terminates_its_axes_in_organs_and_an_indeterminate_one_does_not` | paired: `tree` must read exactly zero, or the rule is a relabel of every tip |
| `organs_are_counted_apart_from_shoot_cells` | §5 |
| `a_ripe_fruit_detaches_as_a_powder_carrying_a_new_organism` | three distinct ways the drop could be broken with the counter still moving |

`an_authored_fate_table_agrees_with_the_builtin_rule` deliberately does **not**
cover the two new species: their whole purpose is to disagree with the
built-in rule, and adding them would make that guard fail for the right
reason while losing the proof that the five indeterminate species are
unchanged.

## 9. Naming, and a correction to the record

The two species shipped for one draft as `sunflower.ron` and `tomato.ron`,
taken from `plant-morphology-reach-2026-08-23.md` §6's sequencing note
(*"the sunflower is the acceptance artifact"*).

**That note is superseded**, and the superseding text is a heading:
`plant-morphology-evolvability-2026-08-26.md` §6, *"The acceptance artifact is
not a sunflower … Twelve tomatoes = it does not [work]."* The register is habit
words throughout — `tree`, `conifer`, `shrub`, `creeper`, `grass`, `moss` — and
a species file is a habit, not a portrait. Renamed to `herb` and `bramble`; the
mechanism was never the problem and did not change.

## 10. What this does not do

- **It does not make annuals.** Post-fruiting senescence is Phase 5, and the
  reach report's third owner call depends on it. A plant here flowers, fruits
  and then goes on standing.
- **It does not make organ colour heritable.** `flower_band` / `fruit_band` are
  drawn per individual from the species' declared range; there is no locus for
  them, so nothing for a mutation to move. Giving petal colour a locus is a
  genome change and belongs with the heritability survey
  (`plant-equilibrium-costs-2026-08-27.md` §9 step 6), not smuggled in beside a
  materials change. Recorded rather than hidden, because a heritable-but-
  immutable channel is exactly the defect the bark band was fixed out of.
- **It does not re-derive the economy the organ charge reallocates.** Costs
  §9a's *"absorbed is not calibrated"* applies here as it did to 3a: the gates
  are green, the stand works, and nobody has checked it works for the right
  reasons. The `shoot_cells` exemption removes the largest single term of that
  debt by construction; what remains is that `INCOME_PER_NODE`,
  `MAINTENANCE_PER_NODE` and `REPRODUCTIVE_BUDGET_CAP` now describe an economy
  with a third construction charge and a second draw on the reproductive
  account.
- **It does not measure whether organs change fitness.** Several arms of any
  such comparison would need an order statistic over seeds; that is gate 3's
  experiment, not this one's.
- **The climbing vine and the `attaches` bit are untouched** — reach report
  §2d, its own package.
