# The decoy field is static, and an ant is not

**One sentence.** Motion does not move the size at which a creature becomes
findable — it **removes the size axis**: in ordinary weather a body that
moves has **0–2** competitors at *every* size from 1 to 16 cells, so a
walking two-cell ant is already better off than a stationary sixteen-cell
one, and the size question only governs the **22–42% of ants that never
move at all**.

`Reports/creature-appearance-design.md` concludes that *extent is the only
lever and it has to roughly quadruple*, and the whole of that case rests on
one number — `decoys`, *"how many other places in this frame are at least as
different from their surroundings as the animal is"*. **The word doing the
work is *frame*.** `decoys` is computed on a single still, and a decoy is a
rock edge or a leaf: something that **holds still**. The animal does not.
Nothing in that instrument distinguishes a stationary distractor from a
moving target, and the owner has now twice said the distinction is what
actually finds an ant — most recently on review card
`20260830T031945607Z-7e0999`, asked whether a dead ant can be told from a
living one: *"visually, I cannot tell anything from these. **ants are mostly
visible with there motion**."*

**This report does not say the appearance work ignored motion.** It did not:
the body-plan card `20260829T045336581Z-34c3d3` is a twelve-frame animated
sequence, posted precisely because an earlier creature card came back *"need
an animation to tell"*. The owner has judged body plans **in motion**. What
had never been *measured* is whether motion changes the decoy count — the
number, not the impression. That is the only gap this fills.

## 1. The instrument

`examples/motion_look.rs`. It reuses `creature_look`'s `luma`, its
`SURROUND` ring, its pinned daylight and its window geometry **verbatim**,
and adds exactly one axis.

- A **still decoy** is `creature_look::decoys` unchanged: a window where
  `|inner mean − surround mean| >= contrast`.
- A **moving decoy** is a still decoy in which at least one pixel changed by
  at least `motion` luma between two frames `gap` apart.

**Composition, not replacement.** A thing that moves but has no contrast is
not a candidate for the eye either, and a thing with contrast that holds
still is exactly what a moving animal is being separated *from*. Both counts
come out of **one loop** over one window set, so `moving <= still` is true by
construction and no difference between them can be a difference in how the
frame was walked.

**The definition is deliberately generous to decoys.** One changed pixel
anywhere in the window qualifies it, so drifting sand, a settling pool, a
swaying twig and every falling raindrop all count. A stricter rule — coherent
displacement, a whole body moving together — would cut the moving count
further and flatter the answer this exists to test.

Defaults: `preset=rolling`, `warmup=2400`, noon pinned (`DAYLIGHT = 1.0`),
`contrast=80` (the live column of the appearance report, *"about the contrast
a dark body actually achieves against ordinary ground here"*), `motion=12`,
`gap=8`, medians over 6–8 samples 40 frames apart.

## 2. The size ladder, with the motion axis

`seed=1`, median of 7 samples. The `still` column is the appearance report's
own ladder, re-measured on this tree:

| body size | still decoys | **moving decoys** |
|---|---|---|
| 1 cell | 371 | **1** |
| **2 cells — ships today** | 141 | **0** |
| 4 cells — the shipped beetle | 57 | **0** |
| 6 cells (3x2) | 36 | **0** |
| 9 cells (3x3) | 15 | **0** |
| 16 cells (4x4) | 0 | **0** |

The still column reproduces the published shape (342 / 127 / 55 / 32 / 15 / 0
on `a7b2dd9`; 371 / 141 / 57 / 36 / 15 / 0 here — `main` has moved, and the
whole point of re-measuring it is that both columns come off one tree).

**Read the second column across, not down.** It does not fall with size
because there is nothing in it to fall: the field of distractors the size
argument exists to escape is *entirely static*. Extent buys a great deal
against a still world and nothing at all against a moving one.

## 3. Real ants, four seeds

`mode=live`, the shipped body, the shipped walk, 40 attempted placements,
600 frames, `gap=8`, 8 samples. Windows touching an ant are skipped, so no
ant is its own decoy.

| seed | placed | still decoys | **moving decoys** | ants' share of moving pixels | ants the motion channel sees per gap | **never moved in any gap** |
|---|---|---|---|---|---|---|
| 1 | 31/40 | 148 | **1** | 90% | 29% | **42%** |
| 2 | 27/40 | 190 | **2** | 72% | 25% | **41%** |
| 3 | 31/40 | 277 | **2** | 64% | 41% | **33%** |
| 7 | 37/40 | 290 | **0** | 97% | 37% | **22%** |

Two findings, and the second is the one that keeps the first honest.

**The moving decoy count is essentially zero for the shipped body.** A
walking two-cell ant has **0–2** competitors where a standing one has
**148–290**. On this evidence the report's recommendation — quadruple the
extent to get from 127 decoys to 15 — is buying, at real cost in mobility and
in the colony's energy ledger, an improvement that motion already delivers
two orders of magnitude of, for free.

**And 64–97% of every moving pixel in the frame is an ant.** The animals are
not competing with the world for the motion channel; on a settled world they
very nearly *are* the motion channel.

**But the animal only supplies the cue a fraction of the time.** 25–41% of
ants register any motion in a given 8-frame gap, and over a ~384-frame
horizon **22–42% of ants never moved once**. For those ants the static ladder
is the entire story — which is precisely the owner's dead-ant observation,
arriving as a number.

The integration window is the knob, `seed=1`:

| gap (frames) | ants the motion channel sees | moving decoys at 2 cells |
|---|---|---|
| 1 | 0% | 0 |
| 2 | 0% | 0 |
| 8 | 32% | 1 |
| 16 | 48% | 2 |
| 32 | 58% | 3 |

An ant does not step often enough to change a pixel between consecutive
frames at all. Waiting longer finds more ants and costs almost no decoys —
the ratio stays overwhelming out to a second of play.

## 4. The sky is the one thing that fills the motion channel

Every world above is *settled*: warmed up, no player, a seeded weather cycle
that mostly leaves it alone. **A quiet test bed would make this result by
construction**, so `weather=` pins the sky and re-runs. `seed=1`, gap 8,
median of 7 samples, 2-cell window:

| sky | ambient moving pixels per gap | still decoys | **moving decoys** |
|---|---|---|---|
| live (seeded cycle) | 5 | 141 | **0** |
| clear | 17 | 146 | **0** |
| rain | 2,870 | 105 | **2** |
| storm | 6,316 | 87 | **0** |
| **blizzard** | 1,516 | 509 | **13** |

**Rain and a storm move thousands of pixels and produce no decoys**, because
a rain streak is thin and low-contrast: it fails the contrast half of the
test that the ant passes. They also *lower* the still count, by darkening the
scene.

**A blizzard is the exception and it is worth stating on its own.** Snow both
falls and settles, so it adds moving pixels *and* lays down a high-contrast
speckle: the still count goes to 509 and the moving count to 13. That is the
worst sky for a small animal — and even there the comparison holds within the
frame, because the whole blizzard ladder moves together:

| body size | still | moving |
|---|---|---|
| 1 | 992 | 130 |
| 2 | 509 | 13 |
| 4 | 257 | 14 |
| 6 | 199 | 14 |
| 9 | 122 | 13 |
| 16 | 64 | 14 |

A **moving two-cell** body (13) still beats a **still sixteen-cell** one
(64). Size never overtakes motion in any sky measured.

## 5. The controls

Written before the numbers were believed, and three of them are asserts that
would abort the run.

- **Frozen pair.** Two renders of a world that was **not stepped**: *0*
  changed pixels, *0* moving decoys. So nothing here is render-side
  animation — `GrainMode::default()` is `Position`, a pure function of screen
  position, and `render` builds a fresh `Renderer` per frame so `Animated`'s
  frame counter is 0 in both halves of every pair. Every changed pixel below
  is the simulation moving. *(Asserted.)*
- **The counter fires.** At `contrast = 0` every window must count:
  windows = still = **157,752**. *(Asserted.)*
- **The motion axis reduces to the static instrument.** At `motion = 0`,
  still = moving exactly. So the two columns are the same instrument with one
  extra predicate, and not two implementations that might disagree.
  *(Asserted.)*
- **The target side, which is the one the whole comparison rests on.** A
  probe is painted on the world's own surface, rendered, then rendered again
  one cell along (the moving arm) and where it was (the still arm). The
  moving arm must be flagged and the still arm must not — otherwise `moving`
  is a census of a field the animal is not in:

  | probe | \|contr\| | pixels moved | moving arm | still arm |
  |---|---|---|---|---|
  | 1 cell | 114.6 | 1 | **MOVING** | still |
  | 2 cells | 111.3 | 1 | **MOVING** | still |
  | 6 cells | 73.3 | 3 | missed | still |
  | 9 cells | 124.1 | 7 | **MOVING** | still |

  The 6-cell row is *correct behaviour*, not a miss by the instrument: that
  probe landed where its own contrast is 73, below the 80 threshold, so it is
  not a decoy-grade object at all and neither column should count it. Two
  rows print `no step` — the 4- and 16-cell probes had no legal cell to step
  into, which is `creature-appearance-design.md` §5's mobility cost showing
  up in a harness that was not built to look for it.

  **One negative control fails, in one condition, and it is worth carrying**:
  in a **blizzard** the 1-cell still arm reads MOVING, because snow lands on
  a single-cell body and changes its only pixel. At 2 cells and above it
  holds. Read the blizzard 1-cell figures as an upper bound.

## 6. What this means for a lane that is building against the old finding

Lane D is implementing larger bodies on the strength of the static claim.
**This does not overturn that work; it splits it into two regimes, and only
one of them is the one the size argument answers.**

- **For an ant that is walking, extent is not the lever.** Motion already
  puts the shipped two-cell body below a still sixteen-cell one, in every sky
  measured, on four seeds. Nine cells buys nothing a walking animal did not
  already have — while still costing 4.5x the body energy at every hatch,
  4.5x the relocated cells per frame, about a third of the placement sites,
  and an **8–10x** blocked-movement rate for a rigid body (all
  `creature-appearance-design.md` §5). Against a *moving* target those are
  costs with no measured benefit.
- **For an ant that is standing still, the report is exactly right and this
  changes nothing.** 22–42% of ants never move across a 384-frame horizon,
  and for them the static ladder is the whole story: 141 decoys at two cells,
  15 at nine. That population is not small and it is not going away.
- **So the cheaper lever is behavioural, and unlike extent it is reachable by
  evolution.** §7 of the appearance report establishes that the two things
  deciding whether a creature is worth looking at — extent and palette — are
  exactly the two an individual cannot own, because `individual_as_species`
  copies the parent's body and the palette is a material keyed by species
  name. **`genome` and `traits` — the brain — are the only things an
  individual does own.** How often an animal moves, whether it rests in the
  open, how far it travels per step, are brain-side; and the number that
  matters for findability, *what fraction of the time is this animal
  supplying motion*, is on the side of the ledger that selection can actually
  reach. That is not a claim that any particular brain change would work — it
  is not measured here — but it is the first appearance lever the E5 question
  can be answered "yes" for.

**What would settle the split cheaply**, and is not built here: an arm in
which the *same* colony is rendered with its ants frozen, judged by eye
against the same colony walking. This report's numbers say the two pictures
differ by two orders of magnitude in candidates; only the owner can say
whether that is what finding an ant feels like.

## 7. What this number is not

It is a count of **distractors**, not a model of human search — the same
caveat `creature-appearance-design.md` §2 attaches to its own figure, and it
inherits every part of it. It says the eye has 1 candidate rather than 149;
it does not say a person finds the ant 149 times faster. The claim is ordinal
and the review card is what settles the rest.

Three further boundaries, stated rather than buried:

- **Daylight is pinned at noon.** It has to be — an unpinned pair straddling
  dusk reads the whole screen as moving, and `creature_look`'s first run
  landed at night and reported a surround luma of 28 against 153. The cost is
  that a real screen's slow sky change is excluded from the ambient count.
  At the gaps measured (≤32 frames) it is small; it is not zero.
- **No player, no digging, no collapse.** The commonest large motion event in
  actual play is the gnome, and he is not in these worlds. The weather arms
  bound one kind of world-scale motion; they do not bound that one.
- **512x320, one preset family.** Four seeds of `rolling` plus one `wetland`
  spot check. The still ladder is known to be a property of the world's
  texture grain and stable across worldgen changes (§2 of the appearance
  report); the moving ladder has not been tested that way.

## 7a. Re-taken after the merge, and why nothing moved

`main` landed **#142** (ants starve; a birth grant), **#145**, **#146** and
**#149** underneath this branch while the report was open, and #142 is
creature code. Every figure above was re-taken on the merged head
(`9323e2e`) rather than carried forward — `CLAUDE.md`'s rule that a baseline
measured on a tree nobody else has does not transfer.

**Every number is identical.** The whole probe ladder (371 / 141 / 57 / 36 /
15 / 0 still, 1 / 0 / 0 / 0 / 0 / 0 moving), the ambient motion figure, and
all four live seeds to the last percent — 148/1, 190/2, 277/2, 290/0, and the
never-moved fractions 42 / 41 / 33 / 22%.

**Identical output across a change that must have moved something is this
repo's tell for a stale binary, so that was checked before the result was
believed**: the example was rebuilt after the merge (binary 05:43:34 against
`src/sim/creature.rs` at 05:39:03) and the built binary contains
`birth_grant`, a field that exists only in the merged `assets/species/ant.ron`.
The binary is the merged code.

The reason it is unmoved is a **regime** difference worth stating, because it
also bounds this report. #142 governs the colony's energy ledger — who
starves, what a hatch costs — over long horizons. This harness places
founders and runs **600 frames**, in which nothing hatches and nothing
starves; it measures how a body *appears and moves*, and the merged changes
touch neither. A colony run long enough to starve is a different question,
and the fraction of ants that never move is exactly the number that would be
expected to shift there.

## 8. What is on this branch

Nothing here changes a shipped creature or any engine behaviour.

| | |
|---|---|
| `examples/motion_look.rs` | The instrument. `mode=probe` is the size ladder with the motion axis and the four controls; `mode=live` is real ants, the body share of moving pixels, and how many ants the motion channel actually sees; `out=x.png` writes a review-queue frame sequence and `out=x.gif` an animation. Not creature-specific — it answers *"would you notice this move"* for anything drawn into this world |
