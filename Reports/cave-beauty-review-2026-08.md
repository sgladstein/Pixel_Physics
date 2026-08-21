# Cave beauty review — what real caves have that ours don't yet

2026-08-20, reviewing session. The owner asked what makes beautiful cave
photographs beautiful, and how our merged round-3 caves measure against
that. Note on sources: the sandbox's network policy refused image
downloads (gateway 403), so the criteria are drawn from the canonical
photographic subjects — Carlsbad's Big Room, Luray's Dream Lake, Son
Doong's dolines, Waitomo, Naica — described from knowledge rather than
from freshly fetched photos. If reference photos get dropped into the
repo, re-run the comparison against exactly those.

Ours were judged from fresh merged-tree renders: `viewshot vault=1` on
rolling s7 (geode vug), canyon s1 and wetland s3 (full systems), zoomed
4x (`target/filmstrips/beauty_*_zoom.png`, plus
`merged_cave_glow_s1.png` from the merge review).

## Seven features of beautiful natural caves

1. **Vertical exuberance at every scale.** Speleothem *forests*: one
   monumental anchor formation, many mid-size, thousands of soda-straw
   fingers. Heavy-tailed size distribution, never even spacing.
2. **Pairs and near-misses tell time.** A stalactite reaching for its
   stalagmite across a tall room; fused columns. The gap is the drama.
3. **Rooms with necks.** Compression and release: tight dark passage,
   then the ceiling leaps and the floor falls away. The photograph is
   always taken at the release point.
4. **One light source, darkness preserved.** The best shots are mostly
   black; a single shaft or lamp lights part of the scene and the
   falloff gives depth. Full illumination kills it.
5. **Still water doubles everything.** A glass-flat pool mirroring the
   formations above it (Luray's Dream Lake).
6. **The rock has grain and flow.** Flowstone drapes, ribbed curtains,
   bedding lines; ceilings break along structure into stepped, blocky
   profiles; breakdown boulders below explain where the roof went.
7. **Color restraint with one accent.** Ochre/grey everywhere, so one
   patch of white calcite, crystal, or blue pool reads as treasure.

## Verdicts on ours

**Already strong:** (4) darkness with pooled crystal glow is our best
feature — the wetland pair of glowing stalagmites in a black chamber is
the postcard; (7) restraint-plus-accent holds by construction; (6) is
half-earned (systems stretch along the bedding; floors are rubble).
The rolling s7 water-filled vug — glowing pale ring around dark blue
water at a shaft's foot — is a legitimate small jewel.

**Gaps, in order of what they cost the picture:**

1. **No vertical exuberance (the big one).** ~125 speleothems per
   system, mostly small teeth at enforced ≥4-column spacing — reads as
   a comb, not a forest. No monumental anchor formation anywhere. Want:
   one huge fused column or floor-to-ceiling near-pair per system, plus
   5–10x more soda-straw-scale fringe, clustered (drip concentration)
   rather than spaced, with a heavy-tailed size draw.
2. **Chamber/passage contrast too small.** Canyon s1 has real
   pillar-divided rooms; wetland s3 is a root-web of near-equal-width
   strands — no compression/release. Chambers should be conspicuously
   taller than passages; some passages tighter.
3. **Water is inert.** Pools are correct but a thin blue line. No
   renderer reflections (don't chase them) — but formations standing
   *in* the pool (glowing crystal at the waterline, its light already
   spills across water) buys half the Luray effect.
4. **Pairs too short to be dramatic.** The near-meeting rule shipped
   but at 2–7 cells; the money formation is a rare tall pair, two-thirds
   of chamber height each side, one-cell gap.
5. **Ceilings are noise-smooth.** Worley curves with no structural
   grain; stepped/blocky ceiling breaks along the strata would also
   explain the breakdown mounds below.

## Suggested round-5 scope (not yet specced or approved)

All five gaps are realise-pass work on the existing anatomy — no new
mechanism. Formation exuberance + heavy-tailed sizes + clustering (1, 4),
chamber/passage contrast via a second threshold shaping pass or
per-junction chamber dilation (2), waterline formations (3), ceiling
grain along the strata (5). Owner has not yet approved this round;
judge against the criteria above by rendering, not by counters alone.

---

## Measured addendum, 2026-08-20 — the review above was generous

The owner's read on seeing the round-3 caves in play: they need
significant improvement. That is correct, and the analysis above could
not have shown why, because it was conducted at contact-sheet zoom
against the pass's own write counters. Two instruments landed since
(`viewshot zoom=/crop=`, `examples/cave_probe`) and the corrections
below are all measurements, 16 seeds x 7 presets at the shipped
2048x640 unless stated. Images: `Reports/img/cave-anatomy/`.

**Every verdict above that was a compliment is downgraded, and three of
the five "gaps" are misattributed to tuning when the cause is
structural.**

### Corrections to the verdicts

| Claim above | Measured |
|---|---|
| "~125 speleothems per system, mostly small teeth" | **17** free-standing formations per system. 125 is the pass's *write counter in cells*; a write counter is not a visibility counter. |
| "small teeth" / gap 1 wants "a heavy-tailed size draw" | Height is **median 3, p90 6, max 7** over 539 formations. The draw is `2 + unit * 6` — uniform, ceiling 8. There is no tail to make heavy; the ceiling has to move first. |
| "the near-meeting rule shipped but at 2–7 cells" | **0–2 near-pairs per 16 seeds**, tallest combined **7 cells**; zero in `canyon` and `arid`. Not "short", effectively absent. |
| "one system, rarely two, a fifth of worlds none" (round-3 ruling) | Worlds with **no** system: arid 13/16, canyon 11/16, rolling 9/16, terraced 9/16, wetland 7/16. The pass's `systems` counter counts geode vugs as systems, which is why it read otherwise. |
| "(6) half-earned: systems stretch along the bedding" | Every surviving system is **179 x 65–69**, in every preset and every seed. It stretches because the envelope is 181 x 71 and the carve fills it. The envelope is the shape. |
| "(2) chamber/passage contrast too small" | Median open column **30** cells, p95 58, in a 69-tall envelope. There are no passages to contrast with — the whole system is one bore. |
| "canyon s1 has real pillar-divided rooms" | Those are the ceiling-span guard's stone teeth (`MAX_CEILING_SPAN`), dropped into roof runs over 36 cells. They are the artefact of a bound, read as architecture. |

### Three causes, none of them a tuning knob

**1. One grain of sand deletes a cave.** `cave_system`'s seal requires
every cell of the void *and* its 2-cell dilation to be `stone` — 12,851
cells — and rejects the whole system otherwise. Instrumented and
reverted: every rejection in canyon/rolling/wetland is a single `sand`
or `gravel` cell from a `pockets` lens. That is the wholesale-rejection
shape of CLAUDE.md's twice-written landmine — *a size cap must bound
work, never gate whether something happens* — wearing a different
costume: a seal check should reject a **breach**, not a system.

**2. The envelope holds ~9 Worley lattice cells, and the threshold
floods them.** `CAVE_CELL 52` against a 181x71 envelope squashed 2x is
3.5 x 2.7 lattice cells. `CAVE_THRESHOLD 0.34` on `F2 - F1` opens ~53%
of that. So "chambers linked by passages" is one open lens with rock
islands in it, which is exactly what `reveal-canyon-s1.png` shows and
what the census's median-30 open column says. The anatomy is set by the
ratio *envelope / lattice cell*, and at 9 cells there is no anatomy to
have.

  `cave_probe field=` dumps the rule itself. At `cell=22 t=0.09` the
  same one-threshold field gives 1–3 cell corridors opening into rooms:
  open column **med 4 / p95 13 / max 28, contrast 3.2x** against the
  shipped world's **med 30 / p95 58 / contrast 2.0x**. Nothing new is
  needed for gap 2 — the shipped mechanism produces it at a lattice
  scale that has more than nine cells in it.

**3. A formation may never bridge floor to ceiling** — stated in
`passes.rs` and enforced by leaving two open rows, with the reason "a
column splits the passage the player walks". That rule forbids gaps 1
and 4's money shot outright. It is worth keeping *for passages* and
wrong for chambers: a fused column against a chamber wall blocks
nothing, and it is the single most photographed object in a cave.

### One hypothesis raised and refuted here, so it is not raised again

Round 3 rejected a second sub-threshold because a disc around a Worley
feature *point* never touches the boundary web. That reasoning is
correct for a disc and does **not** carry to `F3 - F1`, which is small
at lattice *vertices* — and a vertex lies on the web by construction.
Measured on the union: largest component keeps **94% at t3=0 rising to
99% at t3=0.34**, i.e. the second threshold *improves* connectivity
rather than adding sealed satellites.

It still should not ship, for a different reason the same dump gives:
it widens everything, not junctions. Contrast **falls** 3.2x → 2.1x as
t3 rises, and median open column doubles. It buys size, not drama. The
"junction cells not on the web" statistic that first looked damning
(31–46%) was the wrong metric — an off-web bulge cell is still in the
room, reached through its neighbours; only the union's connectivity
answers the question. Recorded because both halves cost time.

### What this does to the suggested round-5 scope

The scope above ("all five gaps are realise-pass work on the existing
anatomy — no new mechanism") does not survive. Gaps 1, 2 and 4 are
downstream of the three causes: decorating a 179x69 lens that exists in
a third of worlds, with formations capped at 7 cells, is redecoration.
The corrected scope is in
`Reports/worldgen-implementation-tasks-round5-2026-08.md`.

---

## Round-5 verdict, 2026-08-21 — the bars were met and the cave got worse

Round 5 landed tasks 1–5 (task 6, ceiling grain, never started — the
implementation agent stopped). Reviewed line-by-line and empirically:
merged into a review worktree, `cave_probe` over 16 seeds × 7 presets,
and a paired render against the pre-round strip **through the same
renderer**, so only worldgen differs.

### The structural half worked

| | before | after | bar | |
|---|---|---|---|---|
| worlds with a cave | arid 3/16, canyon 5/16, wetland 10/16 | **12/16 every preset** | ≥12/16 | met |
| median open column | 30 | **4–5** | 3–8 | met |
| contrast p95/median | 2.0x | **5.2–5.8x** | ≥3.0 | far exceeded |
| tallest chamber, per-seed p50 | 64 (the envelope) | **44–47** | ≥40 | met |
| formation height | med 3, max 7 | med 2–3, p90 18, **max 43** | p90 ≥10, max ≥25 | met |
| near-pairs / 16 seeds | 0–2, tallest 7 | **45–53, tallest 44** | ≥1/world | met |

Three missed: formations/system **29–36** against 60; `wetland` contrast
p10 **2.43x** against 3.0; waterline formations ~1.5/system against 8
(and *unverified* — the agent stopped before writing that finding, though
its code comment already refers to one).

**A metric-definition trap, resolved rather than adjudicated.** The
agent reported tallest-open-column p50 45–48; `cave_probe` reports
median 35–41. Both are right: theirs is the **per-seed** maximum, mine
the **per-system** median, and the spec's "p50 over 16 seeds" supports
theirs. Recomputed per seed: **44–47**, matching. The bar is met. Worth
recording because the two numbers differ by a third and either could be
quoted as "the" chamber height. Note also n=12, not 16 — the p50 is over
the worlds that *have* a system — and the per-seed minimum is 24–30, so
roughly a third of worlds have no room taller than 40.

### The decoration half is a regression

The passage network is a real improvement — corridors, junctions, two
systems in a frame that used to hold one lens. But the chamber now reads
as a **picket fence**: many adjacent tall pale columns of similar width
and height. The owner reported it independently before this review ran
("large uniform gray blocks"), and the paired render confirms it.

**The cause is this document's own round-5 scope, not the
implementation.** It asked for heavy-tailed heights (4a) *and* clustering
(4b) *and* a minimum spacing of "1–2 columns" — and tall formations at
1–2 spacing are a wall. Each task hit its own bar; the composition was
never checked. That is the same shape as the landmine CLAUDE.md records
about sweeps: *anything that rode along with the mechanism is part of
every data point* — here, three mechanisms that were each verified alone
and never verified together.

### "Nothing here looks like a crystal" — and why

Owner's verdict when asked which half of the complaint was the block:
*the light and the block, the shape and the flatness. Nothing here looks
like a crystal.* All four, plus an identity failure nobody had named.

Crystal's four palette tones are luma ≈ 205, 224, 240, 250 — **all four
in the top fifth of the range**, so there is no dark inside the object to
read a facet against — and they are assigned **randomly per cell**
(`loose_shade`). A crystal is therefore a 1–2 cell white bar with noise
sprinkled on it. Nothing in that says crystal.

**The general rule this exposes, which is bigger than crystal**: a
*shape* needs **coherent** shading, and this codebase assigns per-cell
random tone almost everywhere. That is correct for bulk material — it is
the Sandspiel trick, and it is why rock reads as rock — and it destroys
every object that has a silhouette, because a facet is a contiguous
region and noise is not. The same root cause is already recorded twice
elsewhere under different names: open bug **0b** (`palette_family`'s
per-cell coin flip dithering two rock families) and the flowstone blocks
above. Three symptoms, one rule.

So a crystal wants: an **angular** silhouette (chisel tip, wider base, a
small cluster at differing angles rather than one bar), **contiguous**
facet regions rather than per-cell noise, and **internal** value contrast
— its darkest facet below the surrounding rock, not above it. None of
those is a size question, which is why raising the heights made it worse
rather than better.

### The finding that overturns the recommendation: the cave is not enterable

Owner, on the paired render, unprompted: *"It doesn't look like I could
even enter it."* Measured, and they are exactly right.

`PLAYER_WIDTH x PLAYER_HEIGHT` is **7 x 14**, and crouch is marked
*"Reserved: crouch on ground, swim-down in water (phase 3)"* —
unimplemented. So the character needs a clear 7x14 opening to move at
all. `cave_probe` now reports a morphological opening: keep every
position where the whole 7x14 box is void, then measure the void within
one box of a kept position.

| preset | reachable before round 5 | after |
|---|---|---|
| arid | 70% | **8%** |
| canyon | 70% | **4%** |
| rolling | 76% | **0%** |
| terraced | 75% | **0%** |
| wetland | 64% | **0%** |

**The median round-5 cave is 0% reachable in three of five presets.** Not
"cramped" — the player cannot enter it at any point.

**This is my task-2 bar, and it is the boulder mistake repeated.** I set
"median open column in 3–8" from a field dump, chose it purely to
maximise the contrast ratio, and never once checked it against
`PLAYER_HEIGHT`. The same error as capping a boulder at 4 cells against a
14-cell character: a number picked in the plan's own units with no
reference to the thing that has to walk through it. The pre-round-5 cave,
for all its dullness, was **passable** — median open column 30, twice the
player's height. Round 5 traded a walkable cave for a beautiful plan.

It also explains the owner's other remark on the same render — *"it
looks like a single room instead of a cave system"* — which had seemed
inconsistent with a census showing a branching network. It is not
inconsistent: the network exists, and the only part of it the player can
occupy is the one chamber. From inside, a cave you cannot leave **is** a
single room.

### Recommendation (revised — the earlier one was wrong)

An earlier draft of this section said "merge the anatomy (tasks 1–3), it
is unambiguously better". That was written before the reachability
measurement and it is **wrong**: task 2 is what made the passages 4–5
cells, so the anatomy is not separable from the defect.

The split that survives measurement:

- **Task 1 (seal a breach, not a system) — merge.** Presence 3–10/16 →
  12/16 on every preset, no effect on shape. Pure win.
- **Task 3 (one chamber per system) — merge.** Gives the rooms; the
  reachable maxima (54–73%) are chamber volume.
- **Task 2 (lattice and threshold) — retune, do not revert.** The
  mechanism is right and the constants are not: they were derived
  against a contrast ratio instead of against the character. New bar:
  **reachable ≥ 50% (p50 over seeds)**, set below the 64–76% the
  pre-round world managed so there is room to also keep contrast,
  and contrast ≥ 3.0 *subject to* it. Passages want to be walkable
  (≥ ~16 tall) with chambers at 40–60, which is a wider spread than the
  round-5 setting achieves by making passages tiny.
- **Task 4 (formations) — respec.** Owner's direction, verbatim: the
  columns *"are all 1 pixel thick. They should have a taper and be
  thicker but fewer of them."* Plus: it is *"totally full of stuff"*.
  So: fewer, wider, tapered — the opposite of 4b's clustering, which
  should not ship in its current form.
- **Task 5 (waterline) — hold**, unverified and downstream of the above.
- **Task 6 (ceiling grain) — blocked.** Structural grain on the ceiling
  of a cave nobody can walk into decorates the wrong problem.

**And a rule for the respec, which is the real lesson**: every bar in
this round was met, and the round failed. Bars on per-column statistics
cannot see composition (three formation mechanisms that each passed
alone and form a picket fence together) and cannot see the player (a
contrast bar that maximised itself by shrinking the world below the
character's size). The respec is judged on a **composed render** and on
**reachability**, and the per-column statistics are diagnostics, not
bars. **Respec the formation half before any of task 4b's clustering
ships**, and treat task 6 (ceiling grain) as blocked behind that respec:
adding structural grain to ceilings above a picket fence decorates the
wrong problem. The formation respec should be judged on a composed
render, never on three separately-verified bars.
