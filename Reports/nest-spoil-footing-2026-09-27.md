# Where the heaps' soil comes from, traced per pellet

*Measurement, 2026-09-27. `engine` / `lab`. Two switches and an instrument;
nothing in any game changed. Lane note:
[`lanes/nest-mouth.md`](lanes/nest-mouth.md). Follows
[`nest-work-2026-09-27.md`](nest-work-2026-09-27.md) §4, whose guess about
the heaps this report corrects.*

## 0. The answer

**The spoil heaps refill the holes because two pellets in three are put down
where they cannot stay.**

- **Where they go:** on the carrying ant's own back, on a nestmate, or over a
  hole with ground on both diagonals.
- **The cause is one rule.** The drop site asks for filled cells under a
  pellet, and an ant counts as filled. The footing rule (§Z18) asks for ground
  straight beneath, and an ant is not ground.
- **What happens next:** each such pellet turns to loose soil within about ten
  frames and falls, usually into the pit or mouth it came out of.
- **They are most of the supply:** about 1,220 of the ~1,560 cells of worked
  ground that turn loose in a run.
- **The first write-up's guess was wrong.** It said ants dig out from under
  the heaps. That happens, but it comes second. It is also mostly hidden,
  because a cut packs the pellets beside it into lining (§1).

**Making the drop site ask for ground makes digging stick.**
`PIXEL_PHYSICS_SPOIL_FOOTING=ground`, 12 of 12 seeds:
- the share of cuts that build something roughly doubles (3.1% → 5.6%);
- the colony's room doubles (139 → 257 cells) on 31% fewer digs;
- dug cells that refill by falling and stay full fall to a quarter
  (1,148 → 294).

It also exposes two things the refill was hiding:
- **Hanging towers.** Pellets now stay as spoil. A cut beside a heap packs
  that spoil into tunnel lining, and lining needs no footing, so undermined
  heaps hang as blocks in the sky. Ground with no path to the floor rises
  from 16 cells to 124. It is the floating dirt the owner has reported three
  times. `PIXEL_PHYSICS_SPOIL_PACKS=off` stops the lining packing spoil, and
  removes it (16 → 0 on 12 of 12, and in the shipped game too).
- **Scattered digging.** Holes that used to refill now stay open. Openings
  to the surface rise from 31.5 to 41–46 on every footing arm, and the dug
  room is shallower and less roofed. **The refill was hiding how scattered
  the digging is.** A nest also needs something that concentrates digging;
  that is `nest-work-2026-09-27.md` §7's dig marker, and it is next.

## 1. The trace: why a pellet on the surface turns loose

`digbox`'s nest funnel now reads, for every pellet above the old surface that
turns to soil, **the cell straight beneath it** at that moment. That is the
cell `update_powder`'s footing test reads. It also reads how long the pellet
had stood. At every put-down it reads whether the pellet was set on ground at
all.

- The funnel reads the world between frames. All 12 seeds reproduce their
  census rows byte for byte with it on.
- Selftest case 4c drives one pellet per cause by hand. It was watched go red
  with the cut check disabled.

Default arm, 40 ants, energy 1,000, 12 seeds, frame 12,000, median (worst
seed):

| pellets above the old surface that turned loose | share |
|---|---:|
| on the carrying ant's own body | 46% (48%) |
| on another ant | 24% (26%) |
| over air (both diagonals filled, the centre empty) | 25% (27%) |
| cut out from under it in the last two frames | 0% |
| the ground under it fell away | 5% (6%) |
| **had stood 10 frames or less** | **95% (97%)** |

- 1,248 pellets a run (median) turn loose this way.
- **64% of all pellets put down have no footing** (66% worst).
- By route:
  - on the carrier's own body, about 700 a run; ~40 of them posted up the
    column;
  - on another ant, about 300; ~180 of them posted up the column onto an
    ant in the shaft;
  - over air, about 260; ~10 of them posted up the column.

**Why "cut out from under it" reads zero, and why that is not the whole
truth.** A cut runs `line_burrow`, which packs all eight neighbours into
lining, spoil included. So a pellet whose ground is cut from under it
becomes lining on the spot. It is never booked as a pellet turning loose.
With packing off (`SPOIL_PACKS=off`), the same count reads 145–181 a seed.
Undercutting is real, a sixth the size of the unfooted drops, and on the
shipped game it shows up as lining crumbling (13% of conversions) and as
hanging ground.

**Why the old "90% of pellets land above the old surface" was misleading**
(`nest-work-2026-09-27.md` §0 and §5). A pellet set on an ant's back is
above the old surface for the ten frames it lasts. The share of pellets that
stay out of the ground as spoil was never 90%. It is nearer a third.

## 2. The two switches

Both are off by default, and unset is bit-exact: the same reads in the same
order, and no draw either way. The 12 default seeds reproduce the pre-switch
logs line for line.

- **`PIXEL_PHYSICS_SPOIL_FOOTING=ground`** (`creature::spoil_footing_drop`).
  The drop site's cell test (`spoil_site_open`) asks what the footing rule
  asks: the cell straight beneath, and two of the three under it, must be
  ground (`is_footing`: not an animal, a `Powder` or a `Solid`).
  - It is still one predicate about the cell. Which cell is chosen, and when,
    is untouched. The owner's ruling on placement rules stands (`dead-ends.md`,
    the two hand-written rules, 2026-08-31).
  - Guard: `a_pellet_is_set_down_only_on_ground_under_the_footing_switch`,
    one arm per refused place and a control. It was watched go red with
    `is_footing` reverted to "any filled cell".
- **`PIXEL_PHYSICS_SPOIL_PACKS=off`** (`creature::spoil_packs`).
  `pack_neighbours` skips a cell whose material `needs_footing`, so placed
  ground stays placed ground. A heap undermined by a cut slumps as §Z18's
  footing rule intends, instead of hanging on one row of lining.
  - It meets `dead-ends.md`'s Khuong entry's condition (1): *a spoil that
    does not pack*.
  - Guard: `the_lining_leaves_spoil_unpacked_under_the_switch`, both halves.
    It was watched go red with the skip disabled.

`DropSpoil`'s own weights are brain wiring, and `digbox`'s `wire=` changes
them per run. So the drop-away flip is `wire=AtNest:DropSpoil:-0.9`: it
reproduces the variant binary measured earlier the same day to within a
point.

## 3. Ten arms in the dig box

`digbox`, 40 ants, energy 1,000, `w=200 soil=60`, stops at
0/3,000/6,000/12,000 frames, 12 seeds, frame 12,000, medians.

- **Cuts that built:** roofed new ground, pellet carried out, still open
  1,500 frames on.
- **Room:** the colony's dug room outside the founding cut.
- **Heap cuts:** the share of cuts above the old surface.
- **Hanging:** ground with no path to the floor.
- **Loose:** pellets above the old surface that turned loose.
- **Fills:** dug cells still ground 100 frames after they filled, by a fall
  and by a pellet.
- **Holding:** the share of ant-time with a pellet in the jaws.

| arm | cuts that built | room | digs | heap cuts | spoil standing | hanging | loose | fills: fall / pellet | pellets below the surface | holding |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| default | 3.1% | 139 | 1,981 | 36% | 40 | 16 | 1,248 | 1,148 / 60 | 10% | 20% |
| `SPOIL_PACKS=off` | 3.4% | 127 | 1,972 | 34% | 56 | **0** | 1,462 | 1,205 / 57 | 10% | 21% |
| drop-away | 3.5% | 130 | 1,298 | 28% | 28 | 3 | 988 | 887 / 22 | 4% | 23% |
| drop-away, no packing | 3.3% | 113 | 1,293 | 29% | 37 | 3 | 1,070 | 918 / 23 | 4% | 23% |
| **`SPOIL_FOOTING=ground`** | **5.6%** | **257** | 1,360 | 37% | 157 | **124** | 18 | 294 / 294 | 30% | 32% |
| ground, curvature 3 | 5.4% | 257 | 1,286 | 38% | 169 | 164 | 17 | 307 / 239 | 26% | 28% |
| ground, drop-away | **8.4%** | 262 | 901 | 34% | 170 | 117 | 10 | 199 / 162 | 23% | 41% |
| ground, no packing | 3.9% | 172 | 1,392 | 38% | 213 | 1 | 370 | 447 / 254 | 28% | 33% |
| all three | 4.0% | 167 | 1,291 | 39% | 190 | 8 | 406 | 471 / 176 | 21% | 30% |
| **ground, no packing, drop-away** | **7.1%** | 188 | 876 | 34% | 205 | 11 | 255 | 292 / 143 | 20% | 44% |

"Curvature 3" is `wire=SurfaceCurvature:DropSpoil:3`, meant to hold pellets
in pits and drop them on rises.

The shape of what was dug, from the scoreboard (`nestscore.py`). Each paired
column is seeds more nest-like / less than default:

| arm | mouths | fewer / more | roofed | more / less | depth90 | panel vs walkers |
|---|---:|---:|---:|---:|---:|---:|
| default | 31.5 | — | 0.53 | — | 6.5 | 0.00 |
| `SPOIL_PACKS=off` | **27.5** | **11 / 1** | 0.58 | 8 / 4 | 7 | 0.00 |
| drop-away | 32.5 | 4 / 5 | 0.46 | 5 / 7 | 6.5 | 0.00 |
| `SPOIL_FOOTING=ground` | 43 | 0 / 12 | 0.36 | 0 / 12 | 3 | 0.00 |
| ground, no packing | 41.5 | 1 / 11 | 0.39 | 2 / 10 | 5 | 0.00 |
| ground, no packing, drop-away | 44 | 0 / 12 | 0.35 | 0 / 12 | 3.5 | 0.00 |

What each row says:

- **The footing rule is the lever for building.** Every arm with it roughly
  doubles the building share. None without it moves.
- **Drop-away works only once pellets can stay where they land.** Alone it
  builds nothing (3.5%): 77% of its pellets are unfooted, more than the
  default's 64%. With the footing rule it adds a third on top (5.6% → 8.4%).
  The drop-away verdict in `dead-ends.md` was measured under a condition that
  made placement irrelevant.
- **Packing is what holds the hanging towers up**, and in the shipped game as
  well: 16 → 0 alone, 124 → 1 with the footing rule.
  - With the towers gone, undercut heaps slump back into the holes (loose
    18 → 370, of them cut from under or ground fell 99%). The building share
    falls back (5.6% → 3.9%).
  - So part of the footing rule's gain was heaps propped up on lining.
- **Curvature 3 did not keep pellets out of the pits.** Below-the-surface
  30% → 26% against a prediction of under 15%. Every row of it is within the
  footing arm's spread, and it hangs more ground.
- **Every footing arm opens more mouths and roofs less, on 10–12 of 12
  seeds.** Holes that stay open are counted as mouths. None of the arms beats
  random walkers from the door.

Pictures, seed 1, tinted (spoil orange, lining cyan, loose soil above the old
ground yellow), at 3,000, 6,000, 9,000 and 12,000 frames:
- default: a thin lined crust across the box, 3–4 shallow shafts, little
  standing spoil;
- footing rule: orange heaps stand, and blocks of spoil hang on one cyan row,
  2–3 rows up in open sky;
- footing rule, no packing, drop-away: orange heaps standing on the ground,
  and hanging ground at the default's level (11 cells, median);
- every arm: the dug room is a scrape across most of the 200 columns.

## 4. The colony bed and the lab

**Colony bed** (`trailfollow`, 24 seeds, 20 founders each; starved of 480):

| arm | starved | seeds fewer / more | sign p |
|---|---:|---:|---:|
| default | 201 | — | — |
| `SPOIL_PACKS=off` | 230 | 9 / 13 | 0.52 |
| ground, no packing | 184 | 14 / 10 | 0.54 |

Neither moves the bed beyond its seed-to-seed spread.

**Lab** (`labforage`, 12 seeds, 120,000 frames; medians; seeds lower
against default). The default's 12 summaries are identical to the pre-merge
default's, so the switches unset are bit-exact here too.

| arm | births | lower on | food eaten | lower on | alive at the end | colonies lost |
|---|---:|---:|---:|---:|---:|---:|
| default | 530 | — | 1.15 M | — | 80 | 0 |
| `SPOIL_PACKS=off` | 424 | **10 of 12 (p 0.04)** | 0.94 M | 8 of 12 | 50 | **3** |
| ground, no packing | 555 | 7 of 12 (p 0.77) | 1.00 M | 8 of 12 (p 0.39) | 128 | 0 |

- **Unpacked spoil costs the lab colony.** Births are lower on 10 of 12
  seeds, and three colonies die out; all three starved.
- Two of those three seeds were already near the edge on the default: it
  ends seeds 1 and 8 with 2 and 5 ants alive. The births count is the
  signal.
- **Why is not traced.** The lab's colony builds its mound out of spoil and
  food over its own mouth. A heap that slumps when it is tunnelled, instead
  of standing, is the obvious suspect. It is a hypothesis, and nothing in
  this report tests it.
- **Together, the two switches tie the default.**
  - Births 555 against 530, and alive at the end 128 against 80 (7 of 12
    higher).
  - Plants 78 against 38 (9 of 12 higher). No colony lost.
  - The footing rule removes whatever unpacked spoil alone costs; that is
    not traced either.
- **One number moves a lot, and it is the foraging lane's.** Food drops at
  the nest fall threefold on 12 of 12 seeds (7,053 -> 2,427), and pickups at
  the nest fall with them (6,270 -> 1,993). The ratio holds at about 0.85,
  so what fell is the pick-up-and-put-back churn at home, not food reaching
  the colony: food eaten ties. Why the churn fell is not traced.

## 5. Verdict, and what is next

**Both switches stay off in this PR.** The owner prefers options on by
default unless there is a good reason not to. Alone, each has one:

- **`SPOIL_PACKS=off`:** the lab colony has fewer births on 10 of 12
  seeds, and 3 of 12 colonies die out. That is despite the dig box's cleaner
  picture: no hanging ground, and fewer mouths on 11 of 12.
- **`SPOIL_FOOTING=ground`:**
  - More open mouths on 12 of 12 seeds (43 against 31.5), and a shallower,
    less roofed dig. That moves the dig box away from the spec's one mouth.
  - With packing on, it hangs spoil in the sky (16 → 124 cells).
  - With packing off as well, **both colony beds tie** (bed starved 184
    against 201; lab births 555 against 530, none lost). The hanging spoil
    goes (16 -> 1), and dug cells that refill and stay full fall from
    1,148 to 447.
  - What is left against the pair is shape, not colony outcomes: more
    mouths (41.5 against 31.5, 11 of 12) and less roofed (10 of 12) in the
    dig box.
  - **Whether that is a good enough reason is the owner's call**, and it is
    put to the owner with a recommendation: turn the pair on together, and build
    the dig marker on top of it. A default change also moves the foraging
    lane's baselines (the churn above), so it lands in its own PR after a
    poke.

**What the work did establish:**

- **The heaps' supply, and the mechanism, as numbers.** Two pellets in
  three are unfooted. `digbox` now reports both every run.
- **The shipped wiki claim that tailings no longer hang is only half
  true.** A cut packs undermined spoil into lining, and it hangs: 16 cells a
  run in the dig box.
- **Drop-away is a lever once pellets can stay put** (7.1% of cuts build,
  against 3.1%).

**The next step is not another spoil lever.** Every footing arm opened more
mouths, because holes that no longer refill show how scattered the digging
is. The refill was hiding scattered digging, not preventing a nest.

`nest-work-2026-09-27.md` §7's dig marker is the missing piece:
- digging drawn to where digging just happened, with a saturating response
  so one site wins (Toffin et al. 2009);
- only the density half of that has ever been tried here.

Built on top of the footing switches, it gets digging that sticks and
digging that concentrates. It must name its writer and its reader before it
is built. It must also not raise digging at home generally, which is how
`DIG_DOWN` took the foragers underground on the colony bed.

## 6. Predictions, written before each batch

Each row was written into the lane's scratch file before the batch ran
(`digbox`, 40 ants, 12 seeds, frame 12,000, medians), and scored after.

| # | arm | prediction | result | right? |
|---|---|---|---|---|
| 23 | footing | pellets above the surface turned loose ~1,250 -> < 300 | 18 (12 of 12) | right |
| 24 | footing | fall-fills 1,148 -> < 765 | 294; but fills by a pellet put straight in 60 -> 294 | right |
| 25 | footing | building share >= 4% | 5.6% (worst 5%) | right |
| 26 | footing | heap cuts 36% -> >= 40% | 37% | wrong: flat |
| 27 | footing | spoil standing ~40 -> >= 300 | ~160 | wrong |
| 28 | footing | digs within 20% | -31% (and the room doubled) | wrong |
| 29 | footing | panel vs walkers stays 0.00 | 0.00 | right; mouths 31.5 -> 43 not predicted |
| 30 | footing, no packing | hanging within +-15 of default; loose 100-400; fall-fills < 600; building >= 5% | hanging 1; loose 370; fall-fills 447; building 3.9% | right but for the building share |
| 31 | no packing | the default within a point | building 3.4%, room 127; hanging 16 -> 0 and mouths 27.5 not predicted | right on the funnel |
| 32 | footing, curvature 3 | below the surface < 15%; holding x2; digs < 1,200 | 26%; 28% (down); 1,286 | wrong on all three |
| 33 | every arm | panel vs walkers 0.00 | 0.00 | right |
| 34 | drop-away by `wire=` | the variant binary within a point | 3.5%, heap cuts 28%, below 3.5% | right |
| 35 | footing, no packing, drop-away | undercut < 150; fall-fills < 300; building >= 5%; hanging <= 20 | 252; 292; 7.1%; 11 | right but for the undercut |
| 36 | the same | room >= 200 | 188 | wrong |
| 37 | colony bed, no packing | starved within +-15 of default | 230 against 201 (9 / 13, p 0.52) | wrong on size, within the bed's spread |
| 38 | colony bed, footing + no packing | within +-20 | 184 against 201 (14 / 10) | right |
| 39 | lab, no packing | births, food eaten, colony-frames no worse than 4 / 8; no more colonies lost | births lower on 10 of 12 (p 0.04); 3 colonies lost | wrong |
| 40 | lab, footing + no packing | as 39 | births 5 / 7, food eaten 4 / 8, none lost | right |
