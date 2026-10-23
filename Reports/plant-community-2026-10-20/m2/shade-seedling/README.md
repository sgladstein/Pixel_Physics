# The shade seedling, 2026-10-22

Owner's choice on the shade lane's card (option A, "Fix the shade
seedling"): trace why a seedling in deep shade dies before its first leaf,
fix it behind a switch, then retrain shrub and creeper for the dark floor.
This picks up from `../shade-scald/README.md`, which found three walls in
order -- germination's absolute light threshold, a first growth step the tip
gives up on (`World::seedling_waits`), and a first leaf and root the
seedling cannot pay for.

**Result in one line.** Two switches (both off) get a seed past every wall
the shade lane named, and under a grown stand they make the creeper keep a
**seedling bank** -- alive at the end on 8 of 12 beds against 0 -- but the
floor itself is darker than any shade gene can live in, so almost none of
that bank grows, even when a gap opens over it. Retraining shrub and
creeper for this floor cannot work until the floor gets more light.

All runs: `plant_m0 mode=stand`, 192 wide, a stand of the other three roles
grown 60,000 frames (`grow=`), 4 seeds sown, `endow=0.25` (each sown seed
carries a bred seed's carbon, `Reproduce.seed_cost`; before this a sown
seed carried 0), 12 seeds per arm, one binary per table.

## 1. The first-leaf wall, traced: seed carbon nobody could spend

`plant_m0 mode=plastic` at 5% of open light (`dimlamp`), one shrub,
`PIXEL_PHYSICS_TRACE_CELLS=1` printing every cell's carbon:

- The seed's stake lands on the shoot cell (`germinate`) and diffusion
  levels it across the body at **0.15-0.18 a cell**, under the 0.27 a step
  costs. The frontier is funded only from **surplus income** (`income -
  maintenance`), and a plant with no leaves has none, so the stake is never
  moved to the tip. Same with a stake of 0.25, 1 or 4.
- When the tip does hold one step's carbon it extends at once, leaving
  ~0.06, against ~0.13 for a shade leaf cell: four leafless nodes, then the
  apex tries to flower (`OrganUnaffordable`).
- The starvation clock (`STARVATION_DEATH_TICKS`) then declares it dead
  **while it still holds ~2 carbon**.

**`World::seedling_reserves`** (off), three rules, all for a plant under
`SEEDLING_SHOOT_CELLS` (20) shoot cells:

1. With no foliage and no surplus, the frontier is funded from half the
   plant's own stock (`SEEDLING_MOBILISE`), nothing goes to seed, and roots
   wait while water is fine -- the heterotrophic phase of a real seedling.
2. A tip whose node is due a leaf, on a plant earning nothing, waits until
   it can pay the step *and* the leaf.
3. It is not starved while its stock covers `STARVATION_DEATH_TICKS` of its
   bill.

At 5% of open light: before, no leaf, dead by ~11,000 frames; after, one
leaf and a 5-7 cell seedling holding 1.4-2.4 carbon until ~37,500 frames.

## 2. Under the stand (`switch/`)

Paired, `waits=1 scald=1`, with and without the switch, and with the seed's
`Germinate.light_threshold` at its shipped 0.1 or lowered to 0.02 (a gene,
`plant_m0 genes=ROLE:germ:0.02`):

| arm | shrub alive /12 | creeper alive /12 | creeper end shoot (all beds) |
|---|---|---|---|
| switch off, threshold 0.1 | 0 | 0 | 0 |
| switch off, threshold 0.02 | 1 | 2 | 2 |
| switch on, threshold 0.1 | 1 | 1 | 3 |
| switch on, threshold 0.02 | 2 | 5 | 30 |

Where a shrub does sprout it now grows: one bed reached 267 shoot cells
before the canopy took it back down. But survivors are mostly 1-5 cells.

**The residents move too.** The rules apply to every seedling, and the
residents' own seedlings establish on them: the tree's mean shoot at 60,000
frames goes 5,059-6,029 -> 6,269-6,691 (+11-24%), and one bed (seed 1)
6,944 -> 9,385. This is a cost to the community's balance that has not been
judged at 450,000 frames.

## 3. Where the seeds actually were (`trace/`)

`trace=500`, every sown seed every 500 frames for 10,000 frames, 12 beds:
of 30 sown seeds, **6 sprouted, 11 lay on living tissue** (a trunk's base, a
root collar -- a seed falls through tissue only when there is air under it),
**7 on dead matter** (deadwood, litter, a dead leaf, a log -- cells that
hold no water, so `Germinate`'s water test can never pass), and **6 sat in
soil too dark to germinate**. The light at the seed was **0.01-0.04 absolute
against 2.40 open: 0.4-1.7% of open light**. The shade lane's 2-3% was read
three rows up; the herb and creeper layer takes most of the rest.

**`World::seed_settles`** (off): the parked `PIXEL_PHYSICS_SEED_SETTLE`
(commit `4d455bf9`, `Reports/dead-ends.md`), whose re-test condition --
"the bed's floor is flat and the low plants still fail to recruit from
seed" -- is met here. Narrowed: a waiting seed on litter, a dead leaf or
deadwood trades places with it at 1/32 a seed tick (never with other seed:
heaps are A6's), and a shoot tip heaves the dead matter or waiting seed
lying on it. The deeper water read was not ported. `settle/`, switch
reserves on, threshold 0.02:

| | settled a bed | heaved a bed | shrub alive /12 | creeper alive /12 |
|---|---|---|---|---|
| off | 0 | 0 | 2 | 5 |
| on | 396 / 311 | 89 / 46 | 3 | 7 |

It fires, and moves the count within the spread a 12-seed count has here
(three beds either way, `../stand/README.md`).

## 4. A light wall, not a gene wall (`maxgene/`)

Everything on, plus the most shade-tolerant leaf the genes allow
(`shade_tolerate` 1.0) and a 90% cheaper shade upkeep
(`PIXEL_PHYSICS_SHADE_UPKEEP_CUT=0.9`): shrub alive 4 of 12, creeper 7 of
12, **end shoot over all twelve beds 86 and 21 cells**. No gene in range
grows a plant on this floor; the seedlings live, and stay seedlings.

## 5. What the switches are for: a bank a gap can release (`release/`)

Sown under the closed stand; at 40,000 frames a 16-column gap is cut over
each sowing site (`gapat=40000`), every plant cell above the ground erased
**except the sown role's** -- a fallen crown opens the floor, it does not
clear the seedlings waiting on it. (A first pass cut them too and erased
the bank; not kept.) 120,000-frame window, `scald` off. Arm A is the
shipped engine and gene; arm C has `waits`, `reserves` and `settle` on and
the threshold at 0.02:

| | bank at the cut (beds) | alive at 120k (beds) | plants | end shoot | end leaf | beds over 50 shoot |
|---|---|---|---|---|---|---|
| creeper, A | 1 | 0 | 0 | 0 | 0 | 0 |
| creeper, C | 8 | **8** | 17 | 184 | 66 | 2 |
| shrub, A | 1 | 2 | 2 | 286 | 42 | 1 |
| shrub, C | 2 | 1 | 1 | 1 | 0 | 0 |

**The creeper keeps a seedling bank and the shrub does not.** But even for
the creeper, the gap releases it on 2 of 8 beds (96 and 61 shoot cells);
the other six stay at 1-8 cells for 80,000 frames under an open gap. Why
they do not grow there is the next trace and was not run.

## Where this leaves the understory

- The walls the shade lane named are crossed: a seed now germinates in the
  dark when its gene allows, spends its own carbon on its first leaf, and
  waits. That is engine work the retrain needs and is behind two switches.
- **The floor of a grown stand here is 0.4-1.7% of open light, darker than
  any shade gene in range can live in.** A retrain of shrub and creeper
  "for the dark floor" would be selecting in a place where nothing grows,
  which is the same condition that let the shade lane's training find the
  free photosynthesis rate first.
- So the next step is the light, not the seedling: a tree crown that lets
  more through, or a canopy that opens (trees dying and leaving gaps), or
  accepting that a closed tree stand excludes the understory and the
  understory lives at its edges and in its gaps. That is the owner's call.
- Neither switch ships on: each needs its 450,000-frame community run (the
  residents' +11-24% above), and neither has yet changed what the bed looks
  like.

## Instruments added

- `plant_m0 mode=stand`: `endow=X` (sown seeds carry X carbon), `trace=N`
  (every sown plant every N frames, then each one's death and cause from
  `World::death_log`), `gapat=F` (cut the gap F frames in, sparing the sown
  role), `reserves=`, `settle=`, and a `SWITCHES` line with each switch's
  did-it-fire count.
- `plant_m0 genes=ROLE:germ:X` sets `Germinate.light_threshold`.
- `PIXEL_PHYSICS_TRACE_CELLS=1` (plastic mode): every cell's carbon, and
  each growth visit with its price and why it stopped.

## 6. Leaves that let more light through (`leafdepth/`), owner's choice

Owner, 2026-10-22, on the card that followed: give the floor light by
letting leaves pass more of it, **with no change to how trees look**. In the
light field every `Plant` cell counted one cell of column depth (16 cells
pass 20%); `PIXEL_PHYSICS_LEAF_DEPTH=D` (`field::leaf_depth`, default 1.0)
counts a `leaf` cell as D of one. Wood, grass blades and reed are unchanged.

**The floor brightens at first** (`floor/`, 4 beds, stand grown 60,000
frames, light two rows above the ground):

| leaf depth | floor median, % of open (4 beds) | darkest tenth |
|---|---|---|
| 1.0 (shipped) | 0.6-6.4 | 0.3-1.1 |
| 0.5 | 2.1-10.7 | 1.3-3.9 |
| 0.25 | 5.1-14.6 | 3.6-4.3 |
| 0.1 | 5.0-14.6 | 3.3-6.2 |

**Sown seeds still do not get in** (`stand/`, 12 seeds, 4 sown at
`endow=0.25`, alive at 80,000): shrub 1 / 0 / 1 of 12 at depth 1.0 / 0.5 /
0.25, creeper 1 / 0 / 1; at 0.25 with `waits`, `reserves` and `settle` on,
shrub 0 and creeper 5 (tiny). Traced at 0.25 (`stand/trace_*`): the seeds
now read 3-15% of open, and die of other things -- landing on a seed heap
and leaving the slice (`Spread`, A6) within the first 1,000 frames, lying on
litter or deadwood, or sprouting a root and losing the shoot tip.

**And over a long run the trees take the light back** (`community/`, the
four-role bed, 8 seeds, 450,000 frames, one binary):

| | tree leaf | shrub beds (of 8, 100+ leaf) | herb beds | creeper beds | roles a bed | light under cover |
|---|---|---|---|---|---|---|
| depth 1.0 | 36,429 | 7 | 6 | 0 | 2.62 | 0.155 |
| depth 0.25 | 43,901 (+21%) | 4 | 8 | 1 | 2.62 | 0.152 |
| 0.25 + seedling switches | 50,961 (+40%) | 6 | 7 | 0 | 2.62 | 0.135 |

At 150,000 frames the floor under cover is brighter with thin leaves (0.25
against 0.20), and by 450,000 it is back where it was: **the trees grow
more leaf until the floor is as dark as before**. That is how a real canopy
behaves (leaf area grows to use the light), and it means leaf optics alone
cannot make room for an understory here. The roles that survive a bed do
not change (2.62 in every arm).

**Not shipped.** `leaf_depth` stays at 1.0 as an instrument. What the long
run points at instead is a limit on the tree itself -- a canopy that opens
as trees age and die, or a crown that stops growing -- rather than at how
much light one leaf passes.

## 7. A canopy that opens: trees aged on the wrong clock (`treegaps/`)

Owner, 2026-10-23, on the next card ("either suggestion or both"): the
lever is the tree. Chosen here: **gaps from tree death**, because §6 found
the trees fill any extra light, and a dead tree is the one thing that
takes leaf away for good.

**First, a census of what a death does.** `plant_m0 mode=community
gaps=N` prints, every N frames, one character per column for the light two
rows above the ground (digit = % of open light, `+` = 10% or more) and a
`TREE` line for every tree of 20+ shoot cells: span, cells, senescent, and
its age on three clocks. On today's engine (8 seeds, 450,000 frames) a big
tree (1,000+ cells) dies 0-3 times a bed. When one does, the floor under
its span goes from ~3% to 7-10% within 20-40k frames and stays open for
40-80k. So gaps work -- there are just almost none.

**Why there are almost none: old age reads the wrong clock.** The old-age
roll's age was `age_ticks * ORGANISM_TICK_INTERVAL`, rolled over 45 frames.
A big plant ticks every `45 * size_cadence` frames and the lab bed runs at
half pace, so both undercount: a tree 200,000 wall frames old read
**28,935** on that clock against **100,035** plant frames, and each roll
covered 45 frames of a ~155-frame tick. A tree authored to a 60,000 median
lived to roughly 415,000 frames. The annual hazard beside it already
reads `plant_frames` over the tick's real span (`review-3-2026-10-10.md`
§1 found the same trap for herbs). **`World::old_age_plant_time`** (off)
makes old age read the same clock. Guard:
`old_age_reads_plant_time_only_under_the_switch`.

**Every lifespan was tuned on the broken clock**, so the fix alone also
shortens the shrub (45,000) and the shrub loses. Re-derived: the shrub's
old real lifespan is ~45,000 × its cadence ≈ 135,000. Four arms, 8 paired
seeds, four-role bed (`mode=community mix=roles loadfail=0 rain=steady
spells=20000,40000 ground=patchy frames=450000 gaps=2000`), one binary:

| arm | trees died (all, big) | tree leaf | shrub leaf | herb leaf | shrub beds | herb beds | creeper beds | roles a bed | dark floor (under 2%) | floor mean | light under cover |
|---|---|---|---|---|---|---|---|---|---|---|---|
| A: today | 77, 16 | 36,429 | 1,925 | 5,104 | 7 | 6 | 0 | 2.62 | 32% | 5.5% | 0.155 |
| clock fixed only | 200, 70 | 17,061 | 693 | 13,627 | 2 | 8 | 0 | 2.12 | 9% | 8.0% | 0.237 |
| B: clock fixed, shrub 135k | 126, 54 | 9,789 | 5,612 | 10,681 | 8 | 8 | 1 | **3.00** | 5% | 8.6% | 0.273 |
| C: B + tree 120k | 109, 34 | 25,531 | 6,447 | 3,504 | 7 | 7 | 1 | 2.88 | 18% | 6.8% | 0.214 |

(Beds of 8 holding 100+ leaf of the role; "big" = a tree that had reached
1,000 cells, counted from the full runs; the committed files keep each
tree's first senescent line and the floor every 50,000 frames.
`treegaps/summary.py` rebuilds the rest of the table.)

- **The canopy opens and the shrub comes back.** B and C both double the
  big-tree deaths and halve the dark floor, and the shrub, the role a
  closed stand was squeezing, holds 3x its leaf on 7-8 beds.
- **B is the most mixed bed and the thinnest forest**: tree leaf falls to
  27% of today's and the herb takes the open ground. **C keeps 70% of the
  trees** and gives the extra ground to the shrub. Pictures:
  `tree-gaps-seed{1,4,7}-f450000.png` in the project files.
- **The creeper is still absent** (1 bed in B and C). It has no lifespan to
  re-derive; its trouble is §1-5's, not the canopy's.
- **The seven-species bed** (`treegaps/mix7/`, `mix=mix7`, same flags, 8
  paired seeds; arm on = clock fixed, tree and conifer 120,000, shrub
  135,000, grass unchanged): dark floor 41% -> 25%, herb beds 3 -> 7 and
  its leaf 1,763 -> 3,497, grass leaf +35%, conifer leaf -37% (25,518 ->
  16,142), tree -22%; species a bed 3.75 -> 3.88, inside the spread.
  Nothing collapses. Grass needs no re-derivation: the error is the
  plant's cadence, and a plant small enough to tick every 45 frames reads
  the same age on both clocks.
- **Not judged:** the lab game itself, with its colony eating plants;
  there, trees would die about 7x sooner than they do now.
  Rough frame cost (four runs sharing four cores): 329 s -> 348-359 s a run,
  the cost of more, smaller plants.

**Not switched on here.** It changes every world's plants, the lab's
included, and the lifespans that go with it (arm C: tree and conifer
120,000, shrub 135,000, grass by the same rule) are the owner's call.
