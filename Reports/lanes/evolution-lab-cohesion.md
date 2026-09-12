# Lane O — one odour per nest (fission B1)

*Round twenty-nine. Design of record:
[`../evolution-lab-fission-design-2026-09-12.md`](../evolution-lab-fission-design-2026-09-12.md)
§1, §3, §5, §6, §7 B1. Owner ruling 2026-09-12: ship drift on.*

## What landed

A nest patch is a **place that holds an odour**. `World::nest_sites` carries
one `NestSite` per `paint_nest_patch` call; an ant whose `AtNest` input is 1.0
exchanges odour with the nearest one (`creature::blend_with_nest`), a
trophallaxis contact does the same between two ants, and each site's own
odour wanders once per 1,000 frames. `ant.ron` ships `scent_drift: 0.15`.

## What not to re-derive — B1 (#347) and the precondition (#350)

*Numbers and full accounts are in those PRs and in
`../evolution-lab-fission-design-2026-09-12.md`. What a later session would
otherwise pay for again:*

- **Cohesion of the odour works and is strong.** At `scent_drift = 1.0` over
  500 generations every ant stays within **0.123** of its nest's odour against
  a radius of 1.0; with the blend removed the same bed spreads to **3.36**.
- **`TRAIT_TOLERANCE` drifts at `scent_drift` too and is deliberately not
  blended**, so "no setting can make a cohered nest eat itself" holds for the
  odour, not for the allele that judges it. Register, do not tune — blending
  it erases the asymmetry `TRAIT_TOLERANCE` is built around.
- **The design's §3 divergence arithmetic is a free-nest calculation**; a
  nest's residents anchor it, so a site-only wander relaxes to
  `sigma*beta/(gamma*n + beta)` — 0.580 measured where §3 says 1.01.
  `World::carry_nest_wander` is the fix. **Do not "improve" it back into a
  per-site-only wander.**
- **One crossing ant a thousand frames holds two nests together**, twelve
  seeds under 0.18 against a cut-off median of 1.19. Polydomy is the default.
- **The colony leaves its painted patch on two beds in three**, and it tracks
  §T2: where the round trip closes (seed 3, 268 deliveries) it stays and
  cohesion is live all run; where it does not, every blend lands in the first
  10,000 frames and the centroid walks 28–130 cells out. **This bounds
  budding as hard as it bounds cohesion.**
- **Time away does not make an ant a stranger.** An ant's odour moves when it
  is **born**; coming home undoes it. The owner asked for the other shape and
  **the repair is costed and not built** (his call): gate `carry_nest_wander`
  and the blend on proximity, and re-derive sigma with it.

## Environment notes

- **`cargo test --lib` hits the stale-incremental link error** here
  (`rust-lld: undefined hidden symbol: anon.…`, named from `creature::act`).
  `rm -rf target/debug/incremental` clears it; it is not a code error.
- **A test bed needs `scheduler::step` beside `update::step`.** `update::step`
  begins and ends the frame and sweeps but dispatches no creature sites, so a
  bed driven with it alone ticks no animals and every creature counter reads
  zero — which looks exactly like a dead mechanism.
- **A helper that filters on the colony label silently stops reaching the
  animals a `regroup_by_scent` mint has just renamed** — measured as a cloud
  of 2.645 that was really 0.123.

## What B2 needs from here

`World::nest_sites` and `World::register_nest_site(x, y, half_width)` are what
a budded satellite pushes into. `register_nest_site` treats anything within
`half_width` of an existing centre as a repaint rather than a new nest, so
`bud_distance = 120` is comfortably outside it. `NestSite::seeded` is lazy on
purpose — the ground is painted before a founder exists — so a satellite whose
party carries the parent's odour needs no `bud_scent_offset` plumbing at all:
the first ant to stand on it seeds it.

## The played-bed baseline shift, 2026-09-12 — it is #347, and it is chaos

**Full account and every number: `../open-bugs-handoff.md` §Z14.** The short
form, because three lanes were about to build on the wrong reading:

**Attribution confirmed.** A pair on one commit (`c7ee0f40`, seed 1, 500k,
`RAYON_NUM_THREADS=1`, differing only in a `drift=` argument that defaults to
the species file) reproduces lane M's "before" and "after" **digit for digit
on all 21 stops**. #344–#346 are not involved.

**Three mechanisms proposed, all three measured false**: not combat (the
shipped arm kills **0** of its own at every stop to 500k; the drift-0 arm
kills 6), not "the colony became strangers" (**0.00%** of pairs not mutually
family at the frame they part, 0.18% at its worst), and not suppressed
trophallaxis — which was *this lane's own* hypothesis, and the drifting colony
turns out to share **179 times per ant against 91**.

**Not the extra random draws either.** A drift of **0.0001** consumes exactly
the draws 0.15 does and reproduces the drift-0 arm exactly. The draws are not
the channel; the magnitude is. Nothing reads a scent slot continuously — the
only consumer is a threshold at radius 1.0 — so a perturbation either flips a
kin decision somewhere or does nothing at all.

**What it actually is: this bed's 500k trajectory is chaotic and 0.15
re-rolled it.** Inside the single drift-0 arm, adjacent stops read
**3,099 → 16**. A system that swings two hundred-fold on its own cannot have
one trajectory treated as a baseline. **A single-seed 500k run on this bed is
not a baseline** and cannot be compared across any change that perturbs
behaviour; use an order statistic over seeds, paired within one binary.

**And a control shorter than the onset proves nothing** (lane M's rule, worth
repeating): the two arms are identical for the first fifth of a session and
part at 120,000 frames. Any check to 100,000 would have called #347 inert.

**The shipped value is not this lane's to change** — the owner's design call.
What 0.15 costs on a bed where the colony leaves its patch is the §Z14 table;
the alternatives are drift 0, gating `carry_nest_wander`/the blend on
proximity (costed above), or a nest that follows the colony.

## The fresh baseline on current main, 2026-09-12 — and the dial is inert on it

`latecensus`, `played_bed`, seeds 1–3, 500,000 frames, `sample=20000`,
`RAYON_NUM_THREADS=1`, on main `a4359300` (which carries seed cargo #342).
Shipped arm and `drift=0` arm, paired.

| seed | peak ants | extinct at | born | died | starved | killed | arms identical |
|---|---|---|---|---|---|---|---|
| 1 | 12 | 420,000 | 86 | 122 | 40 | **82** | 26 of 26 stops |
| 2 | 12 | 100,000 | 39 | 55 | 18 | **37** | 26 of 26 stops |
| 3 | 212 | 220,000 | 835 | 872 | 713 | **159** | 26 of 26 stops |

**This replaces the late-game §0 census, which every lane has found unusable.**

Three things it says, none of them small:

- **`scent_drift` is inert on the current trunk.** Every stop of every seed is
  identical with the dial on and off. The colony never reaches the size where
  a kin flip has anything to amplify — peak **12** ants on seeds 1 and 2
  against the **3,182** the same seed 1 reached at `c7ee0f40`. So §Z14's effect
  is a property of *that* bed, not of the dial, and re-measuring it on this
  trunk would have found nothing at all.
- **The colony now dies on every seed**, by 100,000 frames on seed 2. The
  late-game boom the design was written around is gone from this bed.
- **Deaths are now mostly KILLINGS, and it is not scent drift.** Seed 1 is 82
  killed against 40 starved; seed 3 is **159 killed** — which is lane J's own
  unexplained figure at 500,000 frames — and the `drift=0` arm reads the same
  159. **Lane J's KILLED channel is excluded from scent drift by direct
  control**, on the trunk, at the seed and frame count J measured.

## Who kills whom on the played bed, 2026-09-12 — nobody

**Full account: `../open-bugs-handoff.md` §Z16.** The colony's `KILLED`
deaths, which outnumber its starvations on the trunk, are **not killings**.
`reconcile_chain` books `DeathCause::Killed` wherever a deciding cell goes
away, whatever took it — its own comment has always said so — while
`tally_kill` fires only for an attributable bite. Across seeds 1–3 at 500,000
frames: **384 booked, 2 attributable**, both ants of the same colony. About
half the rest is the vital cell going **empty** and about two fifths is a
**plant growing into the ant's head**. The colony is overgrown, not eaten.

**The positive control that caught it was the pair** — the attacker log read
against the cause tally. 216 against 2 on the first run. A kill counter with
no second counter beside it would have been quoted as a war.

## For round 30: where to look, and what is already ruled out

*Read-only, no fix started — round 30 owns this. §Z16 is the finding.*

**Read §Z15 beside §Z16.** Lane N landed it in the same window: *a plant holds
a creature up and also blocks it, so a bed of foliage is a cage*. That is the
same collision between plant cells and creature cells from the other side —
theirs is the living animal that cannot step, mine is the dead one whose head
cell became a pip. Whoever picks either up should read both; they may be one
repair.

**Plants do not grow into ants, so growth is not the suspect.**
`plant::growable` (`src/sim/plant.rs:245`) is the gate every growing tip
passes, and it refuses an occupied creature cell twice over: a shoot
(`penetration_force <= 0.0`) takes the cell only when `cell.material ==
material::EMPTY` and otherwise returns false outright, and a root takes it
only when the material is a `Powder` soft enough to push through. A creature
cell is neither. **So the pip and the grassblade standing in a dead ant's head
were not grown there — the ant's own cell was converted in place, or claimed
by a path that does not go through `growable`.** That is the half of the
search space round 30 can drop.

**The three placement paths that write a plant cell without `growable`**, in
the order they are worth checking:

- `plant::seed_survives_bite` (`plant.rs:2360`) writes a `pip` **over the
  bitten cell in place** rather than clearing it, and the creature bite path
  calls `reconcile_chain` on the victim immediately after. Whether that cell
  can ever be an animal's is the question; if it can, the pip is the ant.
- `plant::germinate` (`plant.rs:11957`) converts a standing pip **in place**,
  which would turn any such pip into the `grassblade` the table also shows.
  That makes grassblade a *consequence* of the pip case rather than a second
  route, and testing the pip case tests both.
- Whatever sets a carried seed down (`pips_set_on_soil` / `pips_set_on_nest`
  are its counters) — does the drop test the target cell for an occupant?

**Hypotheses for the `EMPTY` half, which is the larger half and is not
diagnosed.** A vital cell reading `Empty` was vacated by something that left
nothing behind:

- a powder or liquid swapping through a creature cell in the sweep;
- the animal's own move ordering — a body vacating its head before the rest
  of the chain is reconciled;
- `creature.rs`'s own `world.set(tx, ty, Cell::EMPTY)` on the bite path.
  **Partly ruled out already**: that site reads the victim's identity *before*
  the clear and calls `tally_kill` when the chain fails to reconcile, so it
  should have been attributed, and 384 deaths produced 2 attributions. It can
  only contribute where one of the two lookups returns `None`.

**The cheapest instrument that would separate them** is the one this branch
already has: widen `World::note_vital_loss` to record whether an attack was in
progress on that cell this frame, and the EMPTY column splits into "a bite
nobody could attribute" and "everything else" in one run. That is a
measurement, not a fix.
