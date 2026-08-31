# Where a dead plant's mass actually goes

*2026-08-31. Answers the owner's question — "after a plant or tree dies does
every part of it actually degrade to soil?" — with a ledger over the lab's own
bed, and lands the one defect the measurement found. Extends
[`soil-accumulation-and-the-carbon-cycle.md`](soil-accumulation-and-the-carbon-cycle.md),
which is still the report of record for the litter yield itself; this one
closes that report's own named next measurement.*

## 1. The answer

**No. About a tenth of it does.**

Measured on the lab bed — eight `herb` founders, grown 9,000 frames, every
plant killed through `World::mark_organism_senescent`, then left to rot to a
standstill. Twelve world seeds, every control green:

| fate | share of everything that died | across 12 seeds |
|---|---|---|
| reached `soil` | **9%** | 7.1–14.3% |
| rotted away to nothing | **55%** | 49.0–58.8% |
| **locked in `deadwood`, for ever** | **33%** | 30.1–38.9% |

The instrument is `examples/labmass`, built for this and described in
`Reports/instruments.md`.

**The third row is the finding.** The first two were expected — they are
`litter.ron`'s 0.05 yield doing exactly what it was set to do in August, and
that number is not in question here. The third was not known to anyone:
`deadwood` is the only plant-derived debris material in the game with **no
`decays_into` at all**, so a snapped branch is matter that can never become
soil, never become food, and never change again as long as the world runs.

## 2. Why the number is trustworthy

`CLAUDE.md` names the failure this shape of measurement keeps producing — a
number that is arithmetically correct and about the wrong thing — so the
harness is built around controls rather than around a result.

- **Specificity.** `control=empty` runs the same bed with nothing alive. Every
  ledger figure is 0 and the mineral bed moves **exactly zero cells**. That
  second half also settles the oscillator question for this census: the
  `±1,700`-cell water-cycle swing that `CLAUDE.md` records on `cells lost`
  does not reach the bed's mineral count, because the lab holds its sky at
  noon and pins the weather clear.
- **Sensitivity.** `yield=1` must report a 100% return and `to_nothing 0`;
  `yield=0` must report exactly 0. Both do. Without this arm a 9% return is
  indistinguishable from a decay channel that never fired — which is the null
  `World::rotted_to_solid`'s own doc warns about.
- **Plateau.** The last two stops must agree on the mineral count, or the
  census caught the pool mid-drain and is reading a delay as a loss
  (`CLAUDE.md`'s cascade rule). Two seeds failed this at a 18,000-frame rot
  budget and were re-run at 30,000.
- **A positive control on the *model*, not just the instrument.** Roots occupy
  a bed cell by overwriting it, so the bed's fall plus the soil made during
  growth must be at least the standing root count. It is, in every arm.

### 2a. The instrument lied first, and the mechanism is worth carrying

The first run reported a **34%** return. It was wrong by 4x, and nothing about
it looked wrong.

`World::rotted_to_solid` counts every decay that *left a solid behind*.
`deadleaf` decays into `litter` at the default yield of 1.0 — so every shed
leaf on its way down the chain scored a `rotted_to_solid` that produced no
soil whatever. **450 of 620** solid-leaving decays in that rot phase were that
intermediate step. The counter was arithmetically correct throughout and had
been answering a different question than the one asked since it was written.

It was caught by the ledger *also* censusing the grid: soil in the bed rose by
172 cells where the counter claimed 620.

The fix is `World::rotted_onward`, and the shape of it is the reusable part:
it keys on whether the **product** itself has a `decays_into`, which is data
rather than a material name. `decay.rs` stopped hardcoding `ash` and `soil`
years ago for the same reason, and a name test would have gone stale on the
very change this report lands. Read `rotted_to_solid - rotted_onward` for
"reached the end of a chain".

## 3. What was landed

**`deadwood` now decays into `litter`**, at `decay_chance_damp: 0.08` /
`decay_chance_dry: 0.01`.

**This is an omission being closed, not a default being tuned**, and the
distinction is checkable three ways. `deadwood.ron` predates
`Material::decays_into` — `decay.rs` was ash-only and hardcoded when M17
wrote deadwood — and was never revisited when litter made the channel data.
Every other debris material argues its rates at length; this one carried no
decay commentary at all. And `Reports/dead-ends.md` has no entry for it, so
it was never tried and rejected either.

The contrast that proves the point is `corpse`, whose silence on this **is**
deliberate and is recorded — dead-ends.md: *"Decay needed no hook:
`corpse.ron` declares no `decays_into`"*, because `EnergyLedger::meat_lost`
books it instead. That one was left exactly as it is.

**`litter`, not `soil`, and the target is the point.** Three reasons:

1. It makes a **chain** rather than a teleport. `wood → deadwood → litter →
   soil` is four states at four rates, so a branch pile visibly crumbles to
   leaf mould before any of it is ground. `CLAUDE.md`'s ethos asks for a
   middle; this is where the middle is.
2. `litter` is on `ant.ron`'s food list and `deadwood` is not, so a fallen
   branch becomes food eventually instead of never.
3. It leaves the mass charge where the owner put it. The loss belongs once, at
   the terminal `litter → soil` step. Charging a fraction at every hop would
   make a long chain arbitrarily lossy, which is a property of the chain's
   length rather than of anything physical — `deadleaf → litter` is 1.0 for
   exactly this reason.

**The rate is a branch's.** `litter` runs 0.5/0.1 and `log` 0.02/0.002; these
sit between, nearer the log. At `DECAY_TICK_INTERVAL` of 200 frames the dry
rate is a mean lifetime near 20,000 frames — ten times a leaf's, a fifth of a
bole's. That spread in *time* is as much of the graded outcome as the chain
is: the floor after a die-back is leaves, then branches, then bare ground,
rather than one brown mat that never changes.

### 3a. What it does not do, stated plainly

**It does not increase the return.** Paired, one seed, two binaries built
either side of the change:

| | locked for good | reached soil | bed after one full cycle |
|---|---|---|---|
| before | 34.0% | 9.42% | −102 cells (−0.25%) |
| after | **0.0%** | 9.39% | −231 cells (−0.57%) |

Matter that used to sit on the floor for ever now enters the lossy chain and
mostly leaves the world instead, so the sealed box ends one cycle very
slightly emptier. That is the honest cost and it is in front of the owner on
review card `20260831T033121764Z-872b88`.

**Read the `locked` column for what it says.** It counts cells in a material
with **no way out**, so after the change it is 0 by construction — deadwood
now has a way out. The direct read is the standing deadwood count, and at the
30,000-frame budget above it is **15 cells against 847**, still falling.

**And the chain now takes longer to finish, which is the middle working
rather than a defect.** `deadwood → litter → soil` is a longer road than
`litter → soil`, so a rot budget sized before the change catches the pool
mid-drain — the harness's plateau control fires on exactly that, and did, on
one seed of four at 30,000 frames. Given 66,000 the same seed converges with
every control green: **deadwood 847 → 4**, return 9.7%, bed −222 (−0.55%). A
rot budget under about 50,000 frames is now too short to read this ledger at
all, and the plateau control is what says so rather than the reader having to
know it.

The judgement behind landing it anyway: a material that can never be soil,
never be food and never change is the binary outcome the ethos rules out, and
nothing would defend it. The bed difference is 0.3% of a 40,320-cell bed on a
quantity that, undisturbed, plateaus and creeps back up either way (§4).

## 4. The bed does not run down — and what actually stops a die-back recovering

This closes the question
[`soil-accumulation-and-the-carbon-cycle.md`](soil-accumulation-and-the-carbon-cycle.md)
ends on: *"whether a second and third cohort keep drawing the floor down is
not measured, and it is the question this report would ask next."*

Run the lab bed on its own life cycle for 120,000 frames with no cull
(`labmass cull=0`), and the stand reaches generation 2 and holds at ~240
plants. The bed goes 40,320 → **39,596**, −1.8%, and the last four stops are
39,530 → 39,539 → 39,561 → 39,596: **it has stopped falling and is creeping
back up.** Deadwood pins at 275 cells from frame 45,000 and does not grow.

So an undisturbed lab bed is not running down in any way that threatens a
multi-generation experiment. Before this change a single cull of half the
stand left 963 deadwood cells standing for the remaining 84,000 frames and
for ever after, which matters because a graded cull is a **button on the
lab's own control bar** rather than a harness invention.

### 4a. Two claims withdrawn, 2026-08-31, and the retraction is the useful part

**"Die-back is what costs" and "the stand went extinct after one cull" both
came from one repeated-cull run, and that run is an artifact of the
harness.** `labmass`'s `cull_half` filters *creatures* out of the organism
list and nothing else — and a seed is an organism. At frame 12,000 the bed
held 427 organisms of which **385 were seeds**, so "cull half the stand" was
overwhelmingly "delete half the seed bank", and the seed count falling
385 → 3 over the next 12,000 frames is that deletion rather than a die-back.

`CLAUDE.md`'s *ask what your number counts*, on the third instrument in one
day — and note what let it through: the cull reaches for a **shipped** seam
(`World::mark_organism_senescent`, the lab's own bar button), so the call was
correct and only the *selection* was wrong. Reaching for a real seam is not
the same as reaching for the right objects.

Nothing in §1–§3 depends on it: every figure there comes from the
single-cohort arm, where the cull is `cull_all` and the denominator is
standing plant tissue rather than an organism count.

### 4b. What does stand: the owner's own diagnosis, measured directly

**A deadwood mat blocks germination outright.** `seedbed_probe`, 2026-08-31:
**16 of 16** seeds germinate on bare soil and **0 of 16** on a deadwood mat
that is still 85% standing when they land (1,506 cells laid, 1,285 left).

Two gates do it, and they have different repairs. The germination gate reads
plant-available water in the cell beneath the seed, guarded on
`water_capacity > 0` — deadwood sets none, so it reads bone dry however wet
the world is. And `plant::growable` will not let a root enter it either:
`penetration_resistance` **defaults to 100**, against a herb root's force of
1.0.

**The binding number is the seed bank, which is what makes this a rate
question rather than a material one.** Deadwood's dry lifetime is ~20,000
frames against herb's `seed_half_life` of **14,000**, so the ground is still
covered when the seeds it buried expire. Litter, at ~2,000 frames, clears
seven times over inside the same window — which is why litter has never
caused this and deadwood does, and it is a stronger reading than "litter is
permeable": measured, litter is **not** permeable, it has simply already
rotted (1,506 cells laid, 436 left when the seeds land).

**And the obvious fix is measurably not one.** A mat of *soil* — penetrable,
water-holding, the thing one would convert debris into — blocks identically
(**0 of 16**, still dry after 6,600 frames), and so does deadwood given a
`water_capacity` of 400 and a `penetration_resistance` of 0.3 (**0 of 16**,
available water 0.000). Fresh ground is created dry and does not wet from the
damp soil below in time, so the sterilising agent is **dryness rather than
material** and converting debris to soil would not help. The real repair is
whatever lets fresh ground wet up, which lives in the soil-water code rather
than in the material files.

**That repair has since been made, in two further changes, and it needed a
third thing nobody had asked for.** Capillary rest was made conditional on the
drainable band so unsaturated soil equalises finely (#189) — but that alone
drains the bed to the wilting point, because the evaporative sink was
unbounded. Bounding it took making drying scale with how much water is left
(#189) *and* closing the local half of the water cycle (#191): evaporated
water is now added to the air block above the cell, so drying ground damps the
air it is drying into. Until that last one, humidity was a *reading of the
ground* rather than a state of the air, so soil that began to dry made its own
air drier and dried faster still — one plant was enough to parch a 512-wide
bed, 99.4% of it evaporation rather than the plant's own draw. The mechanism
and its numbers are in `src/sim/evaporation.rs`'s module doc under *The ground
was drying itself*; the three ways of getting it wrong are in
`Reports/dead-ends.md`.

## 5. What is deliberately not done

**`litter`'s 0.05 was not touched.** It is the owner's call from 2026-08-27,
it has a report behind it, and it is close to real humification efficiency.
More directly, the lane note carries the owner's standing direction — *"stop
balancing, start exposing. A default that looks wrong is something to register
and report, never to tune."* Changing it is precisely the tuning that forbids.
And §4 says it is not what breaks the lab.

**It should become a dial instead, and that is the registered ask.**
`decay::decay_yield_override` already reads a `DECAY_YIELD` environment
variable — it exists because materials are `include_str!`d — but nothing in
the game can reach it. That is the same gap the coordinator note names as the
largest one between the lab and *"I can figure it out myself"*, and it belongs
to whoever owns the parameters panel, not here.

**A per-cell graded return is not representable today**, and it is worth
saying so rather than leaving it as an obvious next idea. There is no
fertility, nutrient or organic-matter channel on soil — checked — so a cell
cannot become "half soil". Until such a channel exists with a named writer
*and* a named reader, the graded outcome has to live in fates and rates, which
is where this change puts it. `soil-accumulation-and-the-carbon-cycle.md` §4
ranks what a real fertility channel would have to look like and records the
dead end the obvious version walks into.

## 6. Provenance

Every figure from `examples/labmass` at `grow=9000 rot=30000` (§3a's
convergence run at `rot=66000`), `RAYON_NUM_THREADS=4` pinned. All of it
re-run after merging `main` in, twice — the second time carrying **"roots
drink soil, not air"** (#185), which is the change most likely to move these
numbers, since it is what decides how deep a root system goes and roots are
the ledger's only sink on the mineral bed.

It does not move them. Four seeds on the merged tree, `rot=30000`, with
`deadwood`'s decay taken back out to reproduce the "before" state:

| | reached soil | locked | rotted to nothing |
|---|---|---|---|
| original 12 seeds | 7.1–14.3% | 30.1–38.9% | 49.0–58.8% |
| after #185, 4 seeds | 7.4–9.6% | 31.5–37.2% | 50.6–60.0% |

The post-fix arm on the same tree reads 10.0–13.1% to soil, 0.0% locked, and
every control green. Recorded rather than assumed, because a fate table
measured before a root change is a table about a tree nobody has. Twelve seeds for the fate table, one paired seed for §3a with a
separate release build either side of the change and the two logs diffed to
prove the binary moved (`CLAUDE.md`'s stale-example gotcha). §4 is one seed at
120,000 frames. Guards: `decay::tests::deadwood_rots_into_litter_instead_of_
standing_for_ever` and `an_intermediate_decay_is_counted_apart_from_a_chain_
ending_one`, both watched going red against three injected faults — the field
removed, `decays_into: "soil"` (the plausible wrong fix, which fails on
`onward 0 of 17`), and the `rotted_onward` increment removed.
