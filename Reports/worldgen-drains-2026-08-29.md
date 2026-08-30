# Worldgen: clearing the drains

2026-08-30. **Phase 0, lane 0a** of
[`worldgen-revamp-plan-2026-08-29.md`](worldgen-revamp-plan-2026-08-29.md) —
W5, *stop throwing away what is already computed*. Pure repair of measured,
documented defects: no new mechanism, no design judgement. It is a
prerequisite rather than a feature, because relief that lands while one pass
is still deleting every boulder pays into a pass whose output is thrown away.

Of the five pass interactions the plan names: **one was the defect it looked
like and is fixed** (`brows` deleting every boulder); **one is fixed, measured
and deliberately not landed** (`pockets` eating cave systems — §3, and it is
the most useful section here); **three are the generator working** and are
measured and recorded rather than changed, one of them under a standing owner
ruling (§7). A sixth loss, the talus deposit, was two losses stacked and the
smaller one is fixed.

**Nothing in this branch changes how many seeds a world contains**, per that
ruling — no file under `src/sim/plant.rs`, `assets/species/` or `life_scatter`
is touched. §7 carries the measurement instead.

Everything below is measured at the **shipped 8192x2560**, which is itself
one of the findings.

---

## 1. What was wrong, and what it is now

**Median over 6 seeds, every preset, at 8192x2560** — `pass_ablation seeds=6`,
the same command before and after.

| pass, cells per world | arid | canyon | flat | rolling | terraced | wetland |
|---|---|---|---|---|---|---|
| `boulders` **before** | **0** | **3** | 0 | **0** | **0** | **0** |
| `boulders` **after** | **42** | **364** | 0 | **157** | **137** | **77** |
| `brows` before → after | 251 → 242 | 10,078 → 9,336 | 0 | 3,488 → 3,362 | 2,513 → 2,490 | 128 → 114 |

**Boulders across the five live presets: 3 cells → 777.** `flat` is age 0 and
runs no erosion at all, so it proposes no sockets and is correctly unchanged.

And the interference matrix itself:

| | before | after |
|---|---|---|
| `without brows: boulders APPEARS (was zero)` | arid, canyon, rolling, terraced, wetland | **gone from all five** |
| `without pockets: vaults` | +14% arid, +8% canyon | still there — the fix is measured and **withdrawn**, §3 |
| boulder sockets refused because another pass had taken the air (seed 1, 5 presets) | **3 of 3** | **0 of 3** |
| talus deposit realised as scree, `canyon` seed 1 | **16** cells of 689 planned | **40** |
| a pass at zero fails a gate | nothing ran the matrix | `scripts/worldgencheck.sh`, gated in CI |

Cost: `brows` loses 1–7% of its cells — the losing side of an arbitration that
now has a winner, a large pass giving up a little to a rare one.

The brow number is a one-binary control rather than a comparison of two
builds: with `PIXEL_PHYSICS_BROW_YIELD` as the only difference on `canyon`
seed 1, the pass table moves in exactly two places and is byte-identical in
the other twelve —

```
YIELD=0  ... brows 2352 ... boulders   0 ... vaults 42388  pockets 348873 ...
YIELD=1  ... brows 2082 ... boulders 171 ... vaults 42388  pockets 348873 ...
```

— which also shows the talus rounding is independent of it. (Those `vaults`
and `pockets` figures were taken while the withdrawn reorder was in; the
shipped numbers are the "before" column above.)

---

## 2. `brows` was deleting every boulder — R4-1, nine days live

`Reports/pass-interference-2026-08.md` recorded this on 2026-08-20 and it was
still doing it on 2026-08-29. `wiki/the-world.md` gives boulders a paragraph
— *"an event, not a decoration"* — and **no world had one**.

**The mechanism.** `erosion.rs` marks a column where a *hard* surface has
shed past its threshold, and hard surfaces shed at faces. That is the same
place `cliff_edges` finds, so `brows` and `boulders` want the identical air —
and `brows` runs four passes earlier and wins every time. `boulders` proposes
its whole dome and writes nothing unless every cell is open air or loose
cover, so one lip cell refuses the entire boulder.

**The fix**: a lip yields at a column whose reach covers a boulder socket.
The boulder gets the site, and that is a judgement rather than a coin toss —
a socket is *caused*, computed by erosion from what the rock did, while a lip
is drawn at whatever edge the detector found and passed a chance roll. The
priority is stated as data (the marker), not inferred from shape. It is cheap
because a world holds a couple of markers against thousands of qualifying
edges.

**The paired measurement**, both arms in one binary with
`PIXEL_PHYSICS_BROW_YIELD` as the only difference, seed 1, shipped size:

| preset | markers | seated, lip wins | refused | seated, socket wins | refused |
|---|---|---|---|---|---|
| canyon | 3 | **0** | 2 *air already taken* | **2** | 0 |
| terraced | 1 | **0** | 1 *air already taken* | **1** | 0 |
| rolling | 0 | 0 | 0 | 0 | 0 |
| wetland | 0 | 0 | 0 | 0 | 0 |
| arid | 0 | 0 | 0 | 0 | 0 |

**Every refusal in the whole set was "air already taken", and every one is
gone.** That is the positive control and the fix in one table — and it needed
a counter that did not exist. `boulders_seated` reported `0` for nine days
and `0` is the same output whether erosion proposed nothing, the massif was
in the way, or a lip got there first. `Ctx::boulder_rejects` splits the three
and the `erosion detail` line prints them.

`rolling`, `wetland` and `arid` propose **no markers at all** on seed 1, which
is a different problem and not this one: boulders are rare by construction and
about half of all worlds shed none. Over six seeds they do seat — 157, 77 and
42 cells respectively — so the seed-1 zeros are the draw, not the mechanism.

`flat` is unchanged and must be: it ships `world_age: 0.0`, erosion no-ops, no
column ever sheds, and there is no socket to seat. A boulder appearing there
would be the bug.

---

## 3. A sand lens was eating whole cave systems — fixed, measured, and withdrawn

`pockets` buries lenses of sand and gravel in the massif; `vaults` carves a
cave and `erode_breaches` retracts the void away from anything in its envelope
that is not intact rock. So one lens inside a cave envelope ate the system —
`pass-interference-2026-08.md`'s first row, and the reason
`without pockets: vaults +112%` was measurable at all.

**Running `vaults` before `pockets` fixes it with no new rule at all.**
`pockets` already refuses to write unless every cell of the lens *and its
one-cell rind* is intact rock, so a lens proposed across a chamber, a passage
or a vault lining declines and lands somewhere else. The eater becomes the
eaten, which is the right way round: a lens is a texture, a cave system is a
place.

It works, and the numbers are worth keeping. Median over 6 seeds at the
shipped size: `vaults` **+11.5% on `arid`**, +6.3% on `canyon`, +0.3% to
+2.3% elsewhere — `arid` had the largest suppression and takes the largest
recovery, which is the matrix predicting its own repair — and the
`without pockets: vaults` row drops below the matrix's 5% reporting floor on
every preset. `pockets` pays 0.9–1.3% of its cells.

**And it is not in this branch.** It makes a latent, unrelated defect fire.
`vault_density` places each system at an independent column with an
independent waterline, and nothing stops two envelopes overlapping — two pools
at different levels touching is a head difference, which is exactly what
`every_pool_has_a_level_surface` exists to catch for ponds. The reorder does
not create that; it re-rolls which seeds hit it. With it in,
`a_forced_vault_world_is_sealed_and_arrives_at_rest` — four systems crammed
into a 2048-column world — reports **one water cell in motion**, `canyon` seed
1 at (1341, 560). Isolated exactly: the same binary, the same test, that row
moved back in `PASSES` and nothing else changed, passes.

The shipped presets are unaffected — `generated_terrain_is_already_at_rest` is
green either way — so the failure lives only in a forced configuration. That
is not a reason to land it. A gate that is red teaches everyone to ignore it,
and this lane's whole point is that a check nobody runs is a check that does
not exist; landing a fix that reddens one is the same mistake with the sign
flipped.

So the reorder goes to whoever fixes overlapping systems, which is the cave
rebuild (W3) — they get a fix that is already measured, a control that
isolates it in one run, and a defect of their own subsystem they did not know
they had. The reasoning is in `mod.rs`'s `PASSES` beside the row itself, in
`dead-ends.md` with the re-test condition, and in the test that caught it.

**What the test caught is the most reusable thing here.**
`a_forced_vault_world_is_sealed_and_arrives_at_rest` identifies vault cells by
differencing against a `vault_density: 0.0` build, on a stated premise —
*"nothing downstream of it reads a vault, so every difference between the two
worlds is a vault cell and no difference is anything else"*. The reorder makes
`pockets` the first pass downstream that *reads* one, so the control grows
lenses exactly where the cave is and the test fails with *"vault wrote (x, y),
which was not intact rock before"*, pointing at a cell `vaults` never touched.
Classifying the diff by direction does not rescue it: `rolling` seed 1 has a
cell that is **both** — gravel in the control, open cave in the world.
Generally: **a paired-build instrument is exactly as good as its premise about
what else can differ between the arms, and a pass reorder is the change that
invalidates that premise without touching a line of the test.**

---

## 4. The talus deposit: the rounding was one drain, the cover cap is a bigger one

`Reports/worldgen-architecture-ceilings-2026-08-29.md` measured erosion
computing a median talus volume of **244.5 cells per world** and the realise
side turning **3** of them into visible scree. That is round-4 finding R4-2,
and it turned out to be two separate losses stacked.

**The one that is fixed.** `soil_blanket` recoloured cover as gravel only
where `deposits.talus[x] >= 1.0`, per column, independently. A few hundred
cells spread over 8,192 columns is a fraction of a cell each, so nearly every
column failed the test. It now rounds **stochastically** on a draw keyed on
`(seed, x)`: `floor(v)` cells always, one more with probability `fract(v)`.
Still a pure per-column function — no carry swept along the row, which would
make the result depend on traversal order and break the decide phase's
contract — still deterministic, and it conserves the planned volume in
expectation instead of discarding it.

Paired, one binary, seed 1:

| preset | planned | old rule | new rule |
|---|---|---|---|
| canyon | 689.1 | 16 | **40** |
| terraced | 146.1 | 8 | **28** |
| rolling | 124.9 | 7 | **23** |
| arid | 24.9 | 4 | **13** |
| wetland | 5.5 | 2 | **7** |

**The one that is not, and it is the larger of the two.** On `canyon` the
deposit rounds to **636 cells** and **16** survive `.min(soil_depth)`. The cap
is doing 97% of the discarding, and it is not arithmetic: talus lands at the
foot of a face, `plan_from`'s slope gate gives a steep column no cover at all,
and `taper_cover` propagates that zero outward — so the columns holding the
most talus are exactly the ones with nothing to recolour.

That is **not fixed here, deliberately**, and the reasoning is worth keeping.
The recolouring can only paint cover the blanket was already going to place;
placing more would be new loose material with no at-rest proof, and the slope
gate and the repose taper are exactly what the at-rest guarantee rests on. The
right repair is to feed the deposit into the `talus` pass — which already
knows how to make an arbitrary heap stand up, by clamping its top profile with
a two-sweep repose taper — rather than into the blanket's recolouring. But
`extra_cover` already folds the same volume into `soil_depth`, so doing that
without also taking talus out of `extra_cover` double-counts it, and taking it
out moves cover depth across every world. **And W1 changes the slope
distribution**, which is the input that decides how much of this survives at
all, so a fix derived against today's cover gate would be re-derived away.
Sized, recorded, and left for the lane that owns relief.

`TALUS_DEBUG=1` prints planned / after-rounding / after-cap in one line, so
the next reader sizes it in one command rather than deriving it again.

---

## 5. Judge at the shipped world size

`filmstrip`'s `scene=worldgen` calls itself, in its own source, *"the thing
worldgen is judged on"*, and builds a **512x320** world. The game ships
**8192x2560** — **128 times the area** (16x wider and 8x deeper; the revamp
plan says 256x, which is wrong and is worth correcting because it is the
number the argument gets quoted with). At the small size the same generator
reports `boulders 0`, `vaults 0`, `springs 0`, `talus 2`: features that fire
at world scale read as dead, and the sheet cannot say so. Three review cards
asked the owner to judge a retune of `vaults`, which writes nothing there.

Two changes, and the second is a deliberate refusal:

- **`world_look mode=strip` is the worldgen review sheet.** A contact sheet
  of `views` player viewports spread across one **shipped-size** world — a
  filmstrip with *place* along the axis instead of time, which is what a world
  meant to arrive at rest wants — with **the pass table printed under it and
  any pass that wrote nothing named first**. `mode=shot` prints the table too.
- **`filmstrip scene=worldgen` says which world it is, every run**: the size,
  the ratio to the shipped world, which passes wrote nothing *here*, and the
  `world_look mode=strip` command that builds the real one.

**`filmstrip` was not grown to the shipped size, and that was a scope call
rather than an oversight.** `blastsweep.sh` steps that scene 5,000 frames, and
about thirty whole-world censuses in the file loop the frame's own
`WIDTH`/`HEIGHT` — which is the world's size today and would not be. Growing
the world without converting all of them leaves counters that are
arithmetically correct and about the top-left viewport, which is the failure
class `CLAUDE.md` names as this repo's worst-recurring. Converting them is a
30-site refactor of the second most contested file in the repo while two other
lanes hold it. The banner closes the "which world am I looking at" half now;
the refactor is a clean follow-up.

---

## 6. A pass at zero now fails something

`examples/pass_ablation.rs` found R4-1 on 2026-08-20 and **nothing ran it**,
so the feature was still missing nine days later with every test green. That
is `scripts/acceptance.sh`'s own origin repeating: a check nobody runs is a
check that does not exist.

`pass_ablation gate=1` asserts the two things the matrix can say that a pass
counter cannot, and exits 1:

1. **No pass APPEARS when another is switched off** — the R4-1 signature, and
   the one entry in the matrix that cannot be a matter of degree.
2. **Every pass writes cells on at least one preset** — pooled, because `flat`
   writes nothing from eight passes by design and `arid` stands no water.
   Skipped outright when `preset=` narrows the run, since that cannot answer
   it.

Magnitudes are printed and **not** barred. Several suppressions are the
generator working (§7), and a bar on those is how a gate becomes permanently
red and stops being read.

`scripts/worldgencheck.sh` wraps it, CI gates it at 2 seeds, and
`--selftest` puts R4-1 back with `PIXEL_PHYSICS_BROW_YIELD=0` and requires the
check to **name it** — not merely to exit non-zero. That distinction is the
whole value: narrowed to one preset and one seed the gate's *other* half fires
for an unrelated reason (`canyon` seed 1 stands no pond), so a selftest
satisfied by a non-zero exit would report the check as sighted having never
seen the defect. Blind injection, not blind check — the trap
`docscheck.sh --selftest` and `docbench.py` both record hitting.

---

## 7. Three "eaters" that are the generator working, and one the owner has ruled on

Named in the plan alongside the two real ones. Each was traced to the code
that produces it rather than left as a number.

- **`without talus: ponds +61%` (canyon).** `ponds` writes only into `EMPTY`
  cells above the plan surface, and a talus apron stands above the plan
  surface. So scree in a hollow displaces water and pokes out of the pool —
  which is what scree does. The pond's *level* is computed from the plan and
  is untouched. Correct by construction; leaving it.
- **`without ponds: springs +44%` (rolling).** Briefed as mine to fix; it is
  measured and **should not be fixed**, and the evidence is at the call site.
  `springs` refuses to cut a basin whose flanks hold standing water within a
  bowl-depth of the lip, because two pools at different levels touching is a
  head difference and the world stops being at rest. That guard is exactly
  what `every_pool_has_a_level_surface` exists to check, and it caught a real
  case on the way in (*"steps from 176 to 163 between x 406 and 407"*, in the
  check's own comment). Removing it trades a spring for a moving world, and
  the same defect has just cost this lane a withdrawn reorder (§3) — two
  pools at different levels is the failure mode of the day. Left alone.

  **It is a different change from the `life_scatter` half below**, and that
  matters because the two were briefed as one row. `springs` is gated by
  `clean`, a flank scan for `ctx.water` around the basin it is about to cut;
  `life_scatter` sites seeds against the finished ground. They share no code
  and no constant, so nothing here forces a choice between them.
- **`without soil_blanket: residuals −97%`.** Negative is *feeding*, not
  eating: a residual digs its socket through cover, so with no cover there are
  no socket cells to write.

### `without ponds: life_scatter +23%..+80%` — measured, and deliberately not fixed

**Owner ruling, 2026-08-30, before this lane finished:** *"Don't change
anything. Keep it how it is today. Your job is not to manage plant growth
rates. Right now the world starts with no plants, just seeds and they grow as
I play. Don't change that."* So this is a measurement and nothing else. No
file under `src/sim/plant.rs`, `assets/species/` or `life_scatter` is touched
by this branch — checked, the diff against the merge base is empty for all
three.

**What `ponds` is doing.** `life_scatter` runs last, so it sees the finished
world including standing water, and a column under water is not a column it
sows. Standing water is therefore a direct subtraction from the seed count,
and the size of it is a fact about how much of a preset is flooded rather than
about plants.

Median over 6 seeds at the shipped size, `life_scatter` cells with `ponds`
ablated against the baseline:

| preset | seeds sown | without `ponds` | `ponds` cells |
|---|---|---|---|
| wetland | 766 | **+80%** (~+613) | 45,186 |
| rolling | 342 | **+72%** (~+246) | 75,863 |
| terraced | 567 | **+23%** (~+130) | 23,163 |
| canyon | 163 | under the 5% floor | 487 |
| arid, flat | 0 | — | 0 |

So a change that let seeds sit where shallow water stands would add roughly
**989 seeds across the three wet presets**, and it would land hardest on
`wetland` — the preset whose whole identity is lushness, and which a world
review called *"a mud flat with a pond"*. That is what a fix would have been
worth. It is not made.

**And a note for whoever reads a review card next.** The world is sown, not
grown: at genesis it holds seeds and no plants, and they come up as the game
is played. A card rendered at settle therefore shows **no vegetation at all**,
and that is the design working rather than a bug. It has been filed as one
before.

---

## 8. Things that turned out not to be what they looked like

- **`springs` is not at zero.** The revamp plan's §3.4 lists *"springs: a
  causal source, built → 0 across 4 presets x 6 seeds"*. That number is a
  quotation from a source comment describing a **rejected mechanism** — the
  *found* basin, which placed zero and was replaced by a **cut** one. The
  shipped pass places: measured over 6 seeds at the shipped size, `canyon`
  129 cells, `terraced` 119, `rolling` 80, `wetland` 15, and
  `every_generation_pass_writes_cells` has been asserting it non-zero at the
  shipped size all along. Lane A's pixel audit agrees (109 cells, visible in
  three presets). Nothing to fix.
- **`viewshot boulder=1` was reporting boulders that were not there, and
  missing ones that were.** Its "is this seated" test recognised the boulder
  by *material*, and `boulders` changed material when the rock vocabulary
  landed: it paints `limestone` now and cap-rock-family `stone` only in the
  control arm. So it would have printed `NO SEATED BOULDER` for a world with
  two. Widening it to accept limestone as well is worse in the other
  direction — limestone is one of the six ordinary beds, and a `brows` lip
  continuing a limestone face is a rock cell above the plan surface too:
  measured on `canyon` seed 1 with the fix off, the widened test reported
  **1 seated** where the pass's own counter said **0**. It now asks by
  **ablation** — build the same world once more with `boulders` skipped and
  difference it — which no material can fool. That is the fourth instrument
  this programme has found measuring the wrong object.
- **The `>= 1.0` talus floor looked like the whole R4-2 loss and is the
  smaller half.** See §4: the `soil_depth` cap discards 97%, the rounding
  discarded most of the rest.
- **A test that models the rule instead of recomputing it reports its own
  error as the code's.** `erosion_talus_draws_as_buried_gravel_at_the_top_of_
  the_cover` recomputed the realise side's talus depth as `round(v)` while the
  code floored at `>= 1.0` — close enough to agree, for months. Under
  stochastic rounding they part: `round(1.6)` is 2 and the world may hold 1, so
  the second cell the test inspected was an ordinary stony-contact gravel and
  it reported that as a wrong-family talus cell (**1 of 25**). The test now
  recomputes the same draw, which costs it a dependency on `Purpose::Talus` and
  is the right dependency to have.
- **The shipped world is 128 times the small one, not 256.** 8192/512 is 16
  and 2560/320 is 8. The revamp plan says 256x in the sentence the whole
  judge-at-shipped-size argument is quoted with; the argument is unaffected
  and the number is not.

---

## 9. What a reader should not take from this

- **No boulder appeared on `rolling`, `wetland` or `arid` at seed 1**, because
  erosion proposed no sockets there. This lane made the mechanism work; it did
  not make boulders common, and the plan does not ask it to.
- **Nothing here is an appearance claim.** Whether a boulder *reads* as a
  boulder is with the owner (card
  `20260830T002417283Z-d50e39`). The open question on that card is real: the
  dome is drawn per column above each column's own ground, so on a slope it
  comes out slanted rather than round.
- **Nothing here is a performance claim.** The brow yield adds one `Vec<bool>`
  over the world's width, built once per generation, and no per-frame work at
  all. Nothing was timed, so that is an argument rather than a measurement.
- **The gate is at 2 seeds in CI and that is not a sweep.** Its two claims are
  structural rather than statistical — a pass APPEARS or it does not — so seeds
  buy coverage of *which* worlds, not confidence in a number. `SEEDS=6` is what
  this report quotes; anything comparing models still belongs in
  `seedsweep.sh`.
- **This branch is 14 commits behind `main`** (`behind x files = 490`, past the
  300 bar) because it was cut from the coordinating branch rather than from the
  trunk. Merging `main` in unilaterally would put fourteen commits of other
  lanes' work into this lane's diff, which is the coordinator's sequencing
  call, not this lane's.
