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

## What not to re-derive — B1, landed as #347

Full account and numbers: PR #347 and `../evolution-lab-fission-design-2026-09-12.md`.
The four findings a later session would otherwise pay for again:

- **Cohesion of the odour works and is strong.** At `scent_drift = 1.0`, 500
  generations, every ant stays within **0.123** of its nest's odour against a
  tolerance radius of 1.0; with the blend removed the same bed spreads to
  **3.36**, the whole axis. The design's `0.072 * |u|` predicted 0.125.
- **`TRAIT_TOLERANCE` drifts at `scent_drift` too and is deliberately not
  blended**, so §1's "no setting can make a cohered nest eat itself" holds for
  the odour and not for the allele that judges it. Blending it would erase the
  asymmetry `TRAIT_TOLERANCE` is built around. Register, do not tune.
- **§3's divergence arithmetic is a free-nest calculation**; a nest's own
  residents anchor it, so a site-only wander relaxes to
  `sigma * beta / (gamma * n + beta)` — measured median 0.580 where §3 says
  1.01, and about a ninth of that at forty ants. No sigma repairs it.
  `World::carry_nest_wander` is the fix: the wander moves the gestalt. **Do
  not "improve" it back into a per-site-only wander.**
- **One crossing ant a thousand frames holds two nests together**, every one
  of twelve seeds under 0.18 against a cut-off median of 1.19. Polydomy is the
  default outcome.

## What the owner's verdict added, 2026-09-12

Card `20260912T051541289Z-3b03d3`, and the question that bounds this lane:
*"where is the nest/home defined as because I see ant populations moved from
where they are originally placed and just live in the plants where there is
food."* **He is right on two beds in three**, and it tracks §T2: seeds 1 and 2
(deliveries 31 and 9) put every blend in the first 10,000 frames and then 7
and 8 in the remaining 110,000, the centroid walking 28–130 cells out; seed 3
(deliveries 268) stays, 15–47% within 32 cells, blends arriving to the last
window. **Where the round trip closes the colony lives at home; where it does
not, cohesion covers the founding cohort only.** Correlation over three seeds,
direction untested. It bounds B2 as hard as B1.

**Time away does not make an ant a stranger, and the owner asked for a build
in which it does.** An ant's odour moves when it is **born**; coming home
undoes it. Away from home it neither blends nor drifts and still tracks its
nest's wander, because `carry_nest_wander` steps every animal whose nearest
site it is at any distance. At the shipped 0.15 a lineage that never comes
home needs ~44 generations to reach a radius against the 5–11 a session
reaches. **The repair, costed and not built** (a design change, the owner's
call): gate `carry_nest_wander` and the blend on proximity, so an ant away
falls behind with *time* while ants at home still track it — the divergence
guard is unaffected, but σ would need re-deriving (a wholly absent ant takes
237 epochs at 0.065).

**Three arms on seed 3, one mechanism each** (40,000–100,000 frames), cards
`20260912T073328717Z-172fa8` and `20260912T074657942Z-a14201`: drift 0 with
blending off gives 86 alive and 0 own-kills; drift 1.0 blending off gives 155
and **675**; drift 1.0 blending on gives 32 and **181**. Blending cuts
own-killings 73%, and the colony that stops eating itself is a fifth the size
because on a starving bed 675 killings are 675 meals. **A bed term, absent at
the shipped 0.15 on every seed.**

**A number quoted from the wrong harness looks exactly like a result.** Card
172fa8 went out with living-ant counts carried from `labstats` at a different
frame span — 133 and 62 against the true 86 and 155, wrong in both directions
and wrong about which colony was bigger. `labgif` now counts its own.

## Environment notes

- **`cargo test --lib` will hit the stale-incremental link error** on this
  container (`rust-lld: error: undefined hidden symbol: anon.…`, referenced
  from `creature::act`). `rm -rf target/debug/incremental` clears it. It is
  `CLAUDE.md`'s documented gotcha and not a code error; it cost ten minutes
  here because the message names a real function.
- **A test bed needs `scheduler::step` beside `update::step`.** `update::step`
  begins and ends the frame and sweeps; it does **not** dispatch creature
  active sites, so a bed driven with `update::step` alone ticks no animals at
  all and every creature counter reads zero. That looks exactly like a dead
  mechanism. `App::update`'s order is `update::step` then `scheduler::step`.
- **`blend_a_lifetime`-style helpers must not filter on the colony label.** A
  `regroup_by_scent` mint *renames* the ants it splits off, so a label filter
  silently stops reaching exactly the animals a split has just made
  interesting — which reads as "cohesion failed" and is the helper failing.
  Measured: a cloud of 2.645 that was really 0.123.

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
