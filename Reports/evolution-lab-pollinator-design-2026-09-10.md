# The animal whose living is flowers, and the pollen it carries

*Design lane, 2026-09-10, round twenty-seven. No engine code — a priced
specification, a build order, six Sonnet-ready briefs. It extends
[`evolution-lab-ecology-design-2026-09-10.md`](evolution-lab-ecology-design-2026-09-10.md),
the design of record, and does not re-derive that document's §1 measurements.
Numbers marked **(measured)** were taken on this branch on 2026-09-10 at
`RAYON_NUM_THREADS=1`, release, from `origin/main` = `fef35310`; numbers
marked **OWED** have not been taken and name the run that would take them.*

**Status: specification and prices. Nothing is built.** The decision this
document does *not* ask for is the one that started it — the owner ruled it
today, in chat, on the card *"Should a bee-line between two plants make the
seed a cross?"*:

> **"Animals carry it, but we will probably need to get creatures that are
> more pollination motivated then (like a bee/butterfly)."**

That **overturns the ecology design's own recommendation** (its §4.3: build
the player's BRUSH first, animals only on an owner reversal). Reading C-animal
is the direction and the brush is no longer a prerequisite. What the ecology
design said the animal reading *costs* is still true and is still owed; §4
pays it.

---

## 0. The answer

**The flowers are already there, the animals already walk past them, and what
is missing is a mouth that wants one and a grain that rides on it.** Pollen
with no dedicated carrier is an accident on an ant's back; a nectar-feeder
with no pollen is a second ant in a different colour — which is why these are
one piece of work.

**The headline to build toward is THE BED THAT BREEDS ITSELF** — a small quick
animal hopping stem to stem where the ants cannot go, flowers standing after
it leaves, and over a session plants coming up that are neither of the two
lines the player planted.

**Three measurements change what should be built.**

1. **The thicket is the pollinator's larder and the herb is not** *(measured)*.
   At frame 6,000, three seeds, standing flowers are **34 / 16 / 12** on
   `played_bed` and **81 / 60 / 35** on `played_bed_scrambler` — median **16
   against 60**, standing fruit **2 against 48**. The owner's ruling today that
   the thicket joins the default bed is this lane's precondition: on the
   herb-only bed a pollinator has a dozen flowers in a 512-cell box.
2. **The bed stops flowering on its own, and the colony is NOT the cause**
   *(measured — the control is the finding)*. Thicket bed, standing flower
   **81 → 25 → 8** at frames 6,000 / 20,000 / 40,000 with the colony, and
   **81 → 10 → 3** with the colony removed and nothing else changed. **Fewer
   flowers stand with no animals in the box at all.** Herb and scrambler are
   both **determinate** — an axis ends in a flower and stops — so the bed
   flowers, sets and is finished, while plant *count* rises (633 → 909 in the
   control). **B1 is therefore not "keep the ants off the flowers", it is
   "make eight flowers enough".**
3. **"Nectar is paid out of the reproductive budget" is a units error, and it
   is in the design of record.** `reproductive_budget` is plant carbon capped
   at `RESOURCE_SCALE` **4.0**, in which a fruit costs `Ripen(cost: 0.3)`;
   `nectar_yield: 120` is creature joules, in which a leaf is 480. Subtracting
   one from the other charges a flower four hundred fruit for a sip. **Nectar
   is two numbers — a cost in budget units, a yield in joules — and their ratio
   is an exchange rate the engine has never had to name** (§3.1).

**What (2) does to the design: renewal is not a nicety, it is the whole food
supply.** Sixty one-shot flowers are 7,200 J that arrive once. Eight flowers
refilling every 180 frames are **4.8 J a frame, for ever** — 192,000 J across
a 40,000-frame session, against a flitter that burns 0.025 J a frame standing
still. **A renewable flower turns the bed's endgame from a famine into a
living**, and that is the whole argument for building B1 first.

### The priced changes

| # | change | files | new lines | constants to re-derive | what the player sees | decided by |
|---|---|---|---|---|---|---|
| **B1′** | **Nectar, corrected** — a flower fed at pays a meal and stands; cost in *budget units*, credit in *joules through the gut filter*, own refill clock | `plant.rs`, `flower.ron`, `herb.ron`+`scrambler.ron`, one hook at `creature.rs:5007` | ~130 | `EAT_YIELD_THRESHOLD` **12.0**; `best_bite`'s doc example; `creature_probe`'s ceiling | ants working a flower head and leaving it standing | joules paid per 1,000 frames still > 0 at frame 40,000 |
| **B2** | **Pollination as a ripening discount** (ecology design §3.3, unchanged) | `plant.rs`, `assets/species/*.ron`, one `u8` on the organ cell | ~70 | `herb` `Ripen(cost:)` **0.3 / 0.35**, `seed_cost` **0.18**, `reproductive_allocation` **0.28**, `seed_maturity` **60** | fruit where the animals go and not where they don't | `fruit_dropped` split visited/unvisited |
| **P1** | **The bloom sense** — `BloomNear`/`BloomBearing`, appended into the genome's reserve, filled off rays `sight` already casts | `brain.rs`, `creature.rs` (`Sightings`, `sense`, `sight`) | ~90 | none — the reserve is what makes it free (§2.3) | nothing on its own | every shipped species bit-identical; `BloomNear > 0` at a flower, 0 at a leaf |
| **P2** | **The flitter** — the pollinator, `Chain(2)`, gut `−1.0`, no nest, the 2.0 hop with one new gate | `assets/species/flitter.ron` + `assets/materials/flitter.ron` (both new), two `include_str!` lines | ~40 asset + 2 | nothing of the hopper's — **that is the point** (§2.1) | a small pale animal hopping stem to stem where the ants are not | `refused` and `starved aloft` below the hopper's 2.0 arm; births > 0 |
| **C1** | **Animals carry pollen** — a grain on the animal, a paternity on the flower, a cross at fruit-set | `organism.rs` (`PollenGrain`, `cross`, `father_lineage`), `world.rs` (grain table, one `LogKind`), `plant.rs` (`bear_seed_at`), one hook in B1′'s site | ~230 | the clustering instruments (§4) | a bed that hybridises itself | crosses born > 0; a partitioned bed reads zero |
| **C2** | **The petal-colour locus** — so a cross has something loud to show | `organism.rs`, `plant.rs`, `plainspeak.rs` | ~70 | **every seeded plant figure in the lab** — one more draw per birth from a shared `Rng` (§3.5) | a hybrid visibly one parent's colour | the page tells two alleles apart; the seeded suite is re-baselined in the same commit |
| **I** | **The instruments** — lineage share, `selection_arena`, the allele census and the chronicle told a plant can have two parents | `stats.rs`, `ui.rs`, `selection_arena.rs` | ~120 | — | `LINES` counts what it says | pollen off reads digit-identical to today |

**Build order: B1′ → (P1 ∥ B2) → P2 → C1 → C2 → I.**

**The one thing the pollinator cannot do without is B1′ — a flower that
renews.** Not the sense, not the hop, not the pollen. A box holding three to
eight standing flowers at frame 40,000 *with or without animals* has no niche
for an animal whose living is flowers, unless each flower is a well rather
than a mouthful.

---

## 1. What the box has to offer, measured today

`labshot scenario=<bed> seed=<n>`, `RAYON_NUM_THREADS=1`, release, this branch
at `origin/main` = `fef35310`. The colony lands at frame 6,000 on both shipped
beds, so a 6,000 stop is the bed as a pollinator would first meet it.

### 1.1 The larder, three seeds

| seed | `played_bed` flower / fruit | `played_bed_scrambler` flower / fruit |
|---|---|---|
| 1 | 34 / 11 | **81 / 79** |
| 2 | 16 / 2 | **60 / 45** |
| 3 | 12 / 1 | **35 / 48** |
| median | **16 / 2** | **60 / 48** |

**The thicket is worth 3.8x the flowers and 24x the fruit**, and the spread
inside each column is wider than most effects this lab measures — which is why
this is three seeds and a median rather than one run and a number.

### 1.2 The collapse, and the control that reads it

Thicket bed, seed 1, three stops, with the shipped colony and then with the
`Colony` timeline entry removed and **nothing else changed**:

| frame | plants (colony / none) | ants | flower (colony / none) | fruit (colony / none) |
|---|---|---|---|---|
| 6,000 | 633 / 633 | 15 | **81 / 81** | 79 / 79 |
| 20,000 | 768 / 909 | 21 | **25 / 10** | 66 / 41 |
| 40,000 | 687 / 815 | 38 | **8 / 3** | 7 / 26 |

- **Grazing is ruled out as the cause of the flower collapse.** For the colony
  to be eating them the colony arm has to be the *lower* one; it is the higher
  one at both stops.
- **The cause is determinacy.** `wiki/plants.md` in the world's own words: an
  erect herb *"finishes in a single flower head"* and the growing tip is *used
  up* doing it. A bed of determinate plants flowers, sets and is finished —
  while plant count **rises** (633 → 909), so the bed is recruiting and the
  recruits are not reaching flower inside the horizon.
- **The colony eats fruit, not flowers**, which is `dead-ends.md` line 1636
  reproduced at a third bed: standing fruit at 40,000 is **7 with the colony
  against 26 without**, while flower runs the other way. Everything stands
  22–40 rows up; what reaches an ant is what falls.

**Two honest limits.** One seed, and the arms are **different worlds from frame
6,000 onward** — `CLAUDE.md` is explicit that two runs diverging on one frame
cannot be cleanly differenced. So the sizes are not effect sizes. What it
carries is a direction, and the direction is large and against the grazing
hypothesis, which is what a control is for. The paired three-seed version is
**OWED**.

### 1.3 One baseline moved under this round

The ecology design §1 measured **60** standing flowers on `played_bed` at frame
6,000; the same scenario at the same seed reads **34** today, with three
landings between (#300, #301, #302). Nothing here depends on which. The point
is `CLAUDE.md`'s: **re-measure a baseline in your own session before reporting
a delta against it.**

---

## 2. The pollinator: `flitter`

**A habit name, not a taxon**, following `scrambler.ron`'s own argument: a
habit name degrades gracefully (a flitter that has stopped flitting is a
mis-named habit) where a taxon name goes silently wrong (a `bee` with no hive
and no sting is a lie in the species list). `sipper` if the owner prefers the
mouth to the movement.

### 2.1 The body, and what is deliberately copied

**`Chain(2)`, with the whole energy budget copied unchanged from
`hopper.ron`**: `start_energy: 200.0`, `tick_interval: 4`,
`idle_cost_per_cell: 0.05`, `move_cost_per_cell: 0.125`,
`reproduce_threshold: 1100.0`, `body_energy: 480.0`.

**The copying is the discipline.** `hopper.ron`'s header works through what
happens when a species moves two axes at once — `tick_interval` ties directly
to the energy clock, so dropping it shortens the idle life for no behavioural
gain unless `start_energy` moves too. The flitter changes **the wiring and the
gut and nothing else**, so a difference between it and the hopper is
attributable: the rule `CLAUDE.md` demands of a sweep, applied to a species
file.

**`Chain(2)` rather than the hopper's `Rigid` L**, on the owner's parking of
the articulated bodies, and because two cells is the lightest body the engine
has: `launch` gives `speed = sqrt(2 · LAUNCH_WORK / mass)` with mass a cell
count, so **the lightest body sails furthest and the hop is the silhouette**.
`wiki/ants.md` promises exactly that — *a small, light creature sails*. If the
L reads better it is a one-line change and a card.

### 2.2 What it eats, and what a flower is worth to it

**Nectar, through B1′, at a gut authored to `−1.0`.** `diet_quality` is
`(1 − |gut − class|/2)²` and `flower.ron` carries `food_class: −1.0`:

| animal | gut | quality | a 120 J nectar meal pays |
|---|---|---|---|
| the shipped ant | 0.0 | 0.25 | **30 J** |
| the flitter | −1.0 | 1.00 | **120 J** |

**That 4x is the niche, as a number a player can move** — the specialisation
`ant.ron`'s own comment says S5 exists to create. Authoring it on a **new
species** costs the ant nothing: the *"an omnivore should be viable"* ruling
(card `20260823T104411499Z-963f8d`) and its guard
`a_starved_nestmates_corpse_is_still_dinner` are written against a **literal
`0.0` gut**, not a species table. What the flitter gives up is carrion — at
−1.0 a `food_class: +1` corpse pays exactly 0.

**The economy, as arithmetic and not a run:**

- a launch costs `move_cost_per_cell × body_cells × LAUNCH_COST_IN_MOVES` =
  `0.125 × 2 × 4` = **1.0 J**; a walking step **0.25 J**;
- idle is `0.05 × 2` per tick at one tick per 4 frames = **0.025 J/frame**, so
  an unfed flitter lives **8,000 frames**;
- **one nectar meal is 120 hops, or 480 steps, or 4,800 frames of idling**;
- `reproduce_threshold: 1100` is **about ten flowers above upkeep** to bud.

Against the bed's endgame of three to eight standing flowers (§1.2) that works
only because the flower **refills**: at `nectar_refill: 0.25` per 45-frame
organism tick a drained flower is full in 180 frames, so eight flowers are
4.8 J a frame shared out against 0.025 J a frame of upkeep each.

**The predicted failure mode, named before it is built.** `adjacent_food`
offers the *best* cell by `diet_yield`, and at gut −1.0 a flower's 1,440 face
beats everything in the box — so a flitter's mouth always chooses the flower,
and when the flower is **dry** the nectar hook declines and the ordinary bite
takes the cell. **A bed of poor plants gets stripped by its own pollinators.**
Graded rather than defective, and a runaway if the refill is slow; the counter
that sees it is `flowers_bitten` **split by species**, beside `flower_visits`.

### 2.3 How it finds flowers: the sense, and why it is nearly free

**Nothing in the engine points at a flower.**

| sense | what it reports | reaches a flower? |
|---|---|---|
| `FoodAdjacent` | `adjacent_food`'s best cell in the head's 8-neighbourhood | only once touching one |
| `PreyNear` / `PreyBearing` | `sight`'s nearest **living animal this gut would eat**; `is_visible_prey` requires `MaterialKind::Creature` | **no**, by construction |
| `LightHere` | the light channel at this cell | no |

`sight` is the right instrument and is already paid for: 16 rays, all round,
reach 64, **occluded by `Solid | Powder` only — `Plant` deliberately does not
block** — eye lifted a cell. A flower up a stem is already in line of sight of
an animal on the ground; there is simply no predicate that records it.

**So: `Sightings.bloom`, filled on the rays already being cast and never
breaking one.** Kin and threats are recorded that way and the code says why:
breaking a ray changes `reads`, which is what `sight_fraction` bills. A bloom
that breaks no ray keeps **every existing species bit-identical** — P1's whole
positive control. The predicate is two reads:
`materials.kind(cell.material) == MaterialKind::Plant` and
`organism::cell_type(cell.aux()) == Some(CellType::Flower)`. **Not
gut-filtered**, the same asymmetry `is_visible_kin` documents.

**Two new brain inputs, and the genome pays nothing.** `brain.rs`'s layout is
built from **reserved dimensions** precisely so this is lawful: *"Appending an
input, an output or a hidden unit lights up storage that already existed and
was already zero: not one existing weight moves and `GENOME_LEN` does not
change."* So `BloomNear = 27`, `BloomBearing = 28`, `BRAIN_INPUTS` 27 → 29,
rows in `INPUT_NAMES` and `INPUTS`, `genome_manifest` updated. Both normalised
exactly as `PreyNear`/`PreyBearing` are, so a genome that learned one has
learned the other. **The cheapest new sense this engine will ever add**, and
only because somebody built the reserve.

### 2.4 Where it lives: the niche by height

The ecology design §6 measured the gap and it has not moved: **95.8% of all
food stands five or more cells up**, organs reach **53 rows**, an ant's head
**38**, and every food cell is worth the same to every animal at every height —
which is why the box has no niches.

**The flitter's niche is three things at once and needs all three:** a food
only up there, a food that **renews** (B1′), and a way up that walking is worse
at (the hop). Remove one and it is an ant on a stem. **Not exclusion, and it
should not be** — ants climb to 22–23 rows, so the ant reaches the low flowers
and the flitter reaches all of them. Overlap with an advantage is a niche; a
wall is a rule.

### 2.5 The hop: 2.0, and the one wire that makes it survivable

The owner's verdict today is that **the 2.0 hop is the one they liked**. The
direction lane set `(Bias, Impulse, 0.5)` after measuring it, and re-read, its
numbers do not say what they are usually quoted as saying:

| `(Bias, Impulse, w)` | launches | refused | real launches | flight moves | cells/flight | starved aloft |
|---|---|---|---|---|---|---|
| 2.0 | 2,791 | 1,683 | **1,108** | 18,843 | 17.0 | 11.3 |
| 0.5 | 2,187 | 971 | **1,216** | 14,959 | 12.3 | 7.0 |

**The 0.5 arm makes *more* real launches than the 2.0 arm.** A "refused" launch
is the verb called while already in the air — firing into nothing — and at 2.0
**60% of every launch is one**. So whatever the owner liked about 2.0 is not
hop frequency; 0.5 already exceeds it there. What 2.0 buys is **26% more air
and 38% longer flights**; what it costs is `starved aloft`.

**The flitter can afford 2.0 because for it the air is not time away from food
— it is the way to food.** The branch's own finding agrees: *"a
four-times-fewer-jumps arm that dies faster says the jump is not what is
killing this animal."* So:

```
(Bias, Impulse, 2.0),           // the hop the owner liked, at its own rate
(BloomNear, Impulse, 1.0),      // hop harder when there is a flower to reach
(FoodAdjacent, Impulse, -2.0),  // and never off the flower you are drinking at
```

**The third line is the repair, and it is not the recorded dead end.**
`hopper.ron`'s do-not-retry is narrow: *"Do not retry a gate that suppresses
the jump when the animal is short of energy."* That gate was `(Energy,
Impulse, −1.0)`, and it throttled exactly the animal that needed to travel.
`(FoodAdjacent, Impulse, −2.0)` suppresses the jump when there is **food
here**, not when the animal is **poor**: a hungry flitter in open air hops as
hard as a full one, and only one *standing on its meal* sits still. Opposite
condition, opposite population silenced. **Write that at the line** or the next
session reads it as the retry.

**Prediction to falsify**, and P2's gate: against the hopper at 2.0, `refused`
and `starved aloft` fall while real launches hold or rise. If `refused` stays
near 60% the third wire is not reaching — the tell is `flower_visits` near zero
with launches healthy.

### 2.6 How it breeds, and what it has to bank

**Asexual budding, like every animal in the box, and no nest.** `nest: ""` is
supported and its own doc says what it buys: *"a species that declares no nest
has no home material, so `AtNest` reads a constant 0.0 for it and `deliveries`
reads 0 by construction."* `try_bud` never asks about a nest — it needs
`bank + reachable_provision ≥ bar`, `bar = max(reproduce_threshold ×
reproduce_at, birth_cost + 1)`. So **a solitary animal breeds by being well fed
in a good place**, and the visible consequence is that **flitters appear where
the bed is flowering and do not where it is not**.

The wiring follows. The ant's whole steering half — two pheromone channels,
four hidden gate units switching on `Carrying`, `(AtNest, Drop, w)`,
`emit_cost` — is machinery for getting a load home, and the flitter has no home
and carries no load. **Strip it and put the bloom pair in its place.** That is
what stops this being a recoloured ant in *behaviour*, the half `hopper.ron`
explicitly did not attempt. `birth_grant` stays at `−0.2` and `reproduce_at` at
0.0 — no measurement says a flitter should wait longer or less long.

### 2.7 What it looks like at play zoom

1. **The hop.** At 2.0 with the bloom gate it is the only thing in the box that
   leaves the ground on purpose. Movement is the signature, which is the
   owner's 2026-09-09 ruling exactly.
2. **The colour.** A **pale cool blue-white** — `(196,214,236) /
   (224,236,250) / (160,180,208)`. Soil is dark brown, plants green, ants
   near-black, the hopper took warm ochre; this is the hue the box's palette
   has no other user for, and a pale body is legible **in flight**, which is
   where this animal is looked at. Deliberately **not** a flower colour —
   `flower.ron`'s eight bands run yellow → white, and a pollinator that matched
   its food would vanish at the moment that matters.
3. **Where it is.** Ants are a line on the floor; flitters are dots in the
   canopy — §2.4 made visible, at play zoom, with no overlay.

**What is honestly missing:** at two cells, *size* cannot be a tell — the ant
is two cells too. If the silhouette must carry it, that is the articulated-body
programme the owner has parked, and this design deliberately does not wait.

---

## 3. Animals carry pollen

### 3.1 First, the correction B1 must absorb

**Two units, two numbers.** `reproductive_budget` is plant carbon capped at
**4.0**, out of which `herb` pays `Ripen(cost: 0.3)` for a fruit and
`seed_cost: 0.18` for a loose seed. `nectar_yield: 120` is creature joules.
The engine has never named the exchange rate because a **bite** does it
implicitly: a bitten leaf pays 480 J and costs the plant one cell of its own
carbon, and the two ledgers never meet. Nectar is the first transfer a plant
makes *deliberately*, so both halves must be authored:

- **`nectar_cost: 0.01`** (budget units) — a thirtieth of a fruit, so a flower
  pays thirty visits over its life without materially moving a pipeline already
  refused **18,867 times against 56 drops**. The competition the ecology design
  wanted — a plant that feeds animals sets fewer seeds — survives at 1/30 and
  would be an extinction at 1:1. **And a small charge is payable where a large
  one is not:** the budget accrues as a *flow* (`surplus ×
  reproductive_allocation`) while `Ripen` demands a 0.3 lump, which is why
  `organ_ripening_blocked` is the largest counter in the box.
- **`nectar_yield: 120.0`** (joules) credited **through `diet_quality`**, so
  the ant gets 30 and the flitter 120. Face value would delete the specialism
  the species is built on.

**And `ripeness` cannot be the nectar meter.** The ecology design gates refill
on *"the flower's own `ripeness` clock"*. `OrganismCell::ripeness` is a
**one-way clock toward fruit-set** — `Behavior::Ripen` adds `rate` each
organism tick and converts at 1.0 — so draining it would make a visit *delay*
fruiting, backwards from B2's discount, and not draining it says nothing about
nectar. **Nectar needs its own per-cell scalar:** `OrganismCell::nectar: f32`,
refilled per organism tick, capped 1.0, zeroed by a visit. A herb flower, whose
`Ripen(rate: 0.02)` gives it a 2,250-frame career, then pays about twelve
visits before it becomes a fruit.

**Why nectar lives on the cell and pollen does not.** Nectar changes every tick
for every flower, so the cell field is its home; pollen is rare, long-lived and
attached to a handful of cells, so a side table is right for that. The rule
underneath is `CLAUDE.md`'s on hot-path work: put the storage where the write
already is.

### 3.2 The mechanism, end to end

**One field on the animal, one entry in a table on the world, one call in
`bear_seed_at`.**

1. **Load.** At the nectar hook, after the plant has paid: the animal's
   `OrganismState::pollen` is **overwritten** with a `PollenGrain` copied from
   the flower's owning organism — `alleles`, `genotype_draws`, `fates`,
   `params`, `lineage`, `species`, and the frame it was taken. One grain, the
   most recent — the same graded asymmetry as A2's one-seed crop.
2. **Deposit.** At the same hook, *before* the load: if the animal carries a
   grain of the **same species** with a **different lineage**, it is written
   into the grain table against this flower's coordinates. **A self-visit
   writes nothing new** — and the test is the lineage, not the organism id, so
   two clonal siblings do not count as a cross either.
3. **Carry through the organ.** Fruit-set is a **relabel in place at the same
   (x, y)**, so a coordinate-keyed entry survives flower → fruit for free.
4. **Cross.** `plant::bear_seed_at` — the one place a seed's genome is written
   — looks the coordinates up. On a hit it calls `organism::cross(mother,
   &grain, rng)` instead of copying the mother whole; on a miss the seed is the
   clone it is today. The entry is removed on use.

**The grain table: `World::pollen: Vec<(i32, i32, u16, PollenGrain)>` — a
`Vec`, not a `HashMap`, deliberately.** Determinism is required (`PLAN.md`) and
a hash map's iteration order is the classic way to lose it; a `Vec` scanned
linearly is deterministic by construction, and the scan is over **tens of
entries** because the box holds tens of flowers and only *visited* ones are in
it. The `u16` is the organ's organism id, checked on read so a stale entry at a
coordinate that has become something else cannot father anything. Capped at
**256**, oldest evicted, `pollen_evicted` counted — a cap that **bounds work
and never gates whether the mechanism happens**, the shape `CLAUDE.md` insists
on: exhausting it costs a cross, not an answer.

**How long pollen lasts: two clocks.** On the animal, **`POLLEN_LIFE_FRAMES =
3,000`** — long enough to cross a bed whose plants sit 25–30 columns apart,
short enough that a grain is not still carried after its plant is dead. In the
table, until the organ drops or **12,000 frames** — twice the ~6,000 frames a
herb takes from visit to windfall at 45-frame ticks. **Neither is measured;
both are OWED**, and C1's first job is to print the medians and put the
constants beside them.

**`organism::cross(a, b, rng)` — uniform, over one shared scaffold.**
`evolution-lab-genetics-2026-08-31.md` §6.1 already argues the architecture was
built for it: *"positional draws, positional loci, and a fate table where the
honest operator is picking whole rules from one parent or the other."*

| block | granularity | why |
|---|---|---|
| `alleles: [u8; DISCRETE_LOCI]` | per locus, coin flip | positional and independent; the block that makes clusters |
| `genotype_draws: [f32; GENOTYPE_TRAITS]` | per slot, coin flip | positional; a *blend* would smear two morphs into one, the failure `DISCRETE_LOCI`'s own doc names |
| `fates: FateGenome` | **whole table from one parent** | order is load-bearing (first-match-wins) and a rule table *is* the plant; interleaving two makes a chimera nobody authored |
| `params: ParamGenome` | **whole table, same parent as `fates`** | an override names a number *inside* a rule; splitting them is the same chimera one layer down |
| `lineage`, `lineage_seed` | **always the mother's** | §4 — one line, one label, father recorded separately |

The payload is ~200 bytes (`FateGenome` 16 × `PackedFate` + a length,
`ParamGenome` 8 × `ParamOverride` + a length), so 256 entries is ~50 KB — which
is why the *whole* genome can cross rather than only the cheap half.

### 3.3 The ant is a pollinator by accident, and that is the right answer

Once B1′ exists an ant feeding at a flower loads and deposits through the
identical hook. There is no species test in §3.2 and there should not be.

- **Gene flow exists on day one, on the bed the owner already plays** — which
  is what makes C1 a change to the world rather than a feature of one animal.
- **The flitter's contribution is a rate, not a permission.** An ant gets 30 J
  against the flitter's 120, walks rather than hops, and works the ground and
  the low stems. So the flitter should show as *more crosses per animal* and
  *crosses between plants further apart* — a measurement, and the one that says
  whether the species earns its file.
- **It needs a counter that can tell them apart or the finding is
  unreadable:** `pollen_loaded` and `flowers_pollinated` **split by carrying
  species**, plus `cross_span`, a histogram of the column distance between
  parents. Without the split, a bed with 38 ants and 4 flitters reports the
  ants' work as the pollinator's.

**One risk.** At gut 0.0 a flower is still 360 against a leaf's 120, so B1′
pulls the colony *toward* the flowers. §1.2 says the colony is not currently
what removes them — but it measured a colony with **no reason to climb**, and
B1′ gives it one. **So B1′ is read on standing flower first, intake second.**

### 3.4 What the player sees — and the honest gap

What is heritable *and* visible on a plant is exactly two channels, both from
discrete loci: **foliage tone** from `alleles[LOCUS_LEAF_ECONOMY]` with
`LOCUS_ALLELES[0] = 2` — **two values** — and **bark tone**, three values and
mostly invisible on a herb. The other four loci change **which cell gets a
label**, and `Reports/plant-appearance-design.md` records what that is worth on
screen: three architectural levers ranked "very high" for silhouette, all three
demonstrably firing, and the owner's reading of the sheets was that nothing had
changed. **A cross whose only visible product is a binary leaf tone is the
sixth invisible lever in a row.**

**Petal colour is the loudest channel the plant has and it is not heritable.**
`plant.rs:2837` draws `flower_band` from `ORGAN_BAND_STREAM`, per individual,
and the comment beside it is explicit: *"There is no locus for petal or fruit
colour yet… Giving it a real locus is a genome change and belongs with the
heritability survey."* So a hybrid rolls its own petal colour and the crossing
verb produces something that looks random.

### 3.5 C2 — the petal-colour locus, and the cost that is not lines

`LOCUS_FLOWER_COLOUR = 6`, `DISCRETE_LOCI` 6 → 7, `LOCUS_ALLELES` gains a
seventh entry, and `flower_band` is derived the way `foliage_band` already is:
`flower_bands.first + alleles[LOCUS_FLOWER_COLOUR].min(flower_bands.count − 1)`.
The `ORGAN_BAND_STREAM` flower draw goes; the fruit draw stays.
`plainspeak.rs` needs a sentence per allele, and its own guard — *"two of its
alleles produced the same sentence"* — enforces that they differ.

**The cost that is not in the line count: it moves one draw out of a shared
`Rng`.** `jump_alleles` loops the loci and calls `rng.chance` once each, on the
**caller's** stream — the stream `bear_seed_at`'s caller goes on using. A
seventh locus is one more draw per plant birth, so every subsequent draw shifts
and **every seeded plant measurement in the lab moves**. That is exactly
`CLAUDE.md`'s gotcha: *"A genome widening shifted one draw out of a shared
`Rng` that the caller went on using, and both guards over it stayed green
through the regression."* So: **land it alone**, and **re-baseline the seeded
figures in the same commit**, saying the numbers moved because the stream did.

**And the ethos wants a middle.** At `LOCUS_ALLELES[6] = 2` a species has two
petal colours and a cross is a coin flip — the binary outcome the ethos section
is about. `flower.ron` carries **eight bands** and `herb` authors
`flower_bands: (first: 0, count: 2)`. Widening herb to `count: 3` with
`LOCUS_ALLELES[6] = 3` gives yellow / orange / red and a real spread in a
stand. That changes how the shipped herb looks, so **it is a card, not a lane
decision**.

---

## 4. What the world loses, and the instruments that must change

**"Asexual budding *is* the isolation" is the sentence this breaks**, and it is
load-bearing in three places: `evolution-lab-direction-2026-09-09.md` §3
(*"Mating in the world — hard dead end… Only the shelf verb"*),
`plant-evolution-design.md` §6, and the creature line's dead-end register. The
owner has moved it **for plants**; animals stay clonal.

**The operational definition of a plant species goes with it.**
`plant-evolution-design.md` §6 defines a species here as *"a persistent,
self-maintaining cluster in genotype space with a niche it holds"* precisely
*because* there is no gene flow, and builds three instruments on that: allele
multimodality against the clonal drift band, niche fidelity, and the reciprocal
transplant. **Gene flow across a shared bed is what dissolves clusters**, so
where pollen moves those three answer a different question than the one they
were built for. That is the cost the ecology design priced and the owner
accepted. The minimum that keeps the lab honest:

**4.1 Lineage: the mother's, with the father recorded.** `bear_seed_at` already
copies `state.lineage = parent_lineage`, with a comment saying it is *"what
makes a descendant attributable to an arm in `selection_arena`"*. That stays.
Added: `OrganismState::father_lineage: u32` — 0 for a clone, set once at birth,
inherited by nothing. **One line, one label, second parent findable.**

**4.2 `LINES` means something narrower and must say so.** `stats.rs` computes
`LINES n  BIGGEST x%` from `state.lineage` over the living, which with crosses
is **maternal descent** — a hybrid counts wholly to its mother's line. The
cheapest honest fix is the label: `LINES 7  BIGGEST 41%  CROSSED 12%`, the
third being the share of living plants with a non-zero `father_lineage`. **A
player reading `BIGGEST` at 90% in a hybridising bed is being told something
false today.**

**4.3 `selection_arena` breaks silently and must be told.** It stands two
genomes in one bed and attributes descendants **by lineage label**. With pollen
on, arm A's pollen fathers arm B's seeds and every hybrid is credited wholly to
its mother's arm — a clean winner reported while measuring *maternal* descent
in a merged pool. Two things: `pollen=off` as its bed's default (it builds its
own bed, so this is a scenario setting, not a code path), and allele-frequency
attribution when pollen is on. **Its existing control is already right:**
`arm=same` must still read ~50/50 with pollen on.

**4.4 The allele census reads a mixture and should say which.** Multimodality
with pollen on can mean two clusters *or* one segregating pool, and the two
look identical in a histogram. The minimum that separates them is **the crossed
share printed beside it**: 0% crossed is the old instrument unchanged; 40%
crossed is a warning label on the number under it. A linkage or heterozygosity
statistic is a research question and is not proposed here.

**4.5 The chronicle gets one event and one sentence.** `LogKind::Crossed`,
pushed at the cross, `lineage` = the mother's, **`other` = the father's lineage
truncated to `u16`**. The truncation is safe and worth stating: `claim_lineage`
is a monotone counter called once per *founder* and a lab box has tens, so a
`u16` holds it for any run made — and it avoids adding a field to `LogEvent`,
which has 27 construction sites. The sentence goes through
`names::line_name(seed, lineage)`, which is pure and works for a line that has
since died:

> `KESTREL x DOGWOOD — A CROSS`

That is `CLAUDE.md`'s second law satisfied for the player who was not watching:
**the event leaves a mark.**

---

## 5. Build order and briefs

**Order:** B1′ → (P1 ∥ B2) → P2 → C1 → C2 → I.

| lane | owns | disjoint from |
|---|---|---|
| **B1′** | `plant.rs`, `flower.ron`, `herb.ron`, `scrambler.ron`, one hook in `creature.rs` | lands first; everything downstream calls its hook |
| **P1** | `brain.rs`, `creature.rs` (`Sightings`, `sense`, `sight`) | **B2** entirely — run them together |
| **B2** | `plant.rs`, `assets/species/*.ron`, one `u8` on the organ cell | **P1** entirely |
| **P2** | `flitter.ron` (species + material), two `include_str!` lines | needs P1 and B1′ |
| **C1** | `organism.rs`, `world.rs`, `plant.rs`, one hook in B1′'s site | needs B1′; may run beside P2, which touches only assets and two lines |
| **C2** | `organism.rs`, `plant.rs`, `plainspeak.rs` | **must land alone** — §3.5, it moves the RNG stream |
| **I** | `stats.rs`, `ui.rs`, `selection_arena.rs` | needs C1 |

**B1′, B2 and C1 all touch `plant.rs`, a contested file.** Land each quickly
rather than holding a large diff.

**Every brief carries the same cost fork:** *build it, or write the finding and
stop; never a half-built fix.* **Every brief's animal card is a moving
sequence** — `labgif`, or `filmstrip gif=1` — never a still (owner,
2026-09-09). **Every brief's measurement is `scenario=played_bed` with the
thicket and the tree in it, 3+ seeds, 120,000 frames, `RAYON_NUM_THREADS`
pinned** — and see §6.2, because that bed does not exist yet.

### Brief B1′ — nectar, in two currencies

*Owns:* `src/sim/plant.rs`, `assets/materials/flower.ron`,
`assets/species/herb.ron` + `scrambler.ron`. *One hook handed on:* the swallow
block at `creature.rs:5007`, written as a one-line patch in your report.
*Read first:* §3.1 here and §3.2 of the ecology design — **its "pays 120 J out
of `reproductive_budget`" is a units error and you are the lane that fixes
it.** Also `dead-ends.md` line **911**: every organ is a *donor* in
`allocate_to_frontier`, so a flower charged against its own carbon is
permanently poor. Nectar is charged to the pocket for the same reason.
*Build:* `OrganismCell::nectar: f32`, refilled `nectar_refill` (0.25) per
organism tick, capped 1.0. `plant::nectar_offer(world, x, y) -> f32`: if the
cell is a `Flower` with `nectar >= 1.0` and its owner's `reproductive_budget >=
nectar_cost` (0.01, budget units), charge it, zero the cell's nectar, return
`nectar_yield` (120.0, joules). **The flower is not removed.** At the hook the
caller credits `yield × diet_quality(flower_material, gut_bias)` to
`state.energy` and books `energy_ledger.harvested_plant`, as the brood path at
`creature.rs:1384` already does. A dry flower returns 0.0 and the ordinary bite
proceeds — deliberate (§2.2).
*Constants to re-derive:* `EAT_YIELD_THRESHOLD` **12.0** — nectar is the first
sub-100 J meal and its own comment asks for this; `best_bite`'s doc example;
`creature_probe`'s ceiling. `flower.food_energy` **1,440 stays**.
*Counters:* `flower_visits`, `nectar_paid` (it fired); **joules paid per 1,000
frames at frame 40,000** and `organs_built` (the effect).
*Positive control:* an arm at `nectar_refill: 0.0` must take `nectar_paid` to
exactly zero while `flower_visits` still moves.
*Measurement:* `labforage scenario=<the new played bed>`, 3+ seeds, 120,000
frames, ablated by `nectar_yield: 0.0`. Ships if joules-per-frame from flowers
is still non-zero at 40,000 and colony intake does not fall.
*Card:* a `labgif` of ants working a flower head still standing afterwards,
`flower_visits` and standing flower in `meta`.
*Cost fork:* standing flower falling to single figures anyway is **expected
and is not this brief's failure** (§1.2). Nectar *paid* falling to zero with
it is — the box then needs an indeterminate flowering plant, not a nectar rule.
Write that and stop; it is the more valuable finding.

### Brief P1 — the bloom sense

*Owns:* `src/sim/brain.rs`, `src/sim/creature.rs` (`Sightings`, `sense`,
`sight`). No asset, no species.
*Read first:* §2.3, and `brain.rs`'s `GENOME_LEN` doc on reserved dimensions —
**appending an input is lawful and moves no existing weight.**
*Build:* `BloomNear = 27`, `BloomBearing = 28`; `BRAIN_INPUTS` 27 → 29; rows in
`INPUT_NAMES` and `INPUTS`; `Sightings.bloom`; `is_visible_bloom` (a `Plant`
cell whose `aux` unpacks to `CellType::Flower`, **not** gut-filtered); recorded
on the same rays and **never breaking one**; filled in `sense` exactly as
`PreyNear`/`PreyBearing` are. Update `genome_manifest`.
*The gate, and it is the whole brief:* **every shipped species bit-identical.**
Run `ascii` and one `labstats` arm before and after and compare digit for
digit. A moved counter means a ray broke or `reads` changed.
*Positive control:* one flower and one eyed animal must read `BloomNear > 0`;
the same scene with a leaf instead must read exactly 0. **Watch it go red both
ways before believing either.**
*Card:* none — nothing visible, and this is the brief where a picture would
lie.
*Cost fork:* if bit-identity cannot be held, stop and report which counter
moved. A sense that changes every other animal in the box is not landable.

### Brief B2 — pollination as a discount

Build to the ecology design §7's brief B2, unchanged. Two additions: **B1′ owns
`nectar` on the organ cell and you own `visits`** — both new fields on
`OrganismCell`, so sequence them or you conflict. And your visit count is **the
same record C1 reads**, so name it `pollination_visits`.

### Brief P2 — the flitter

*Owns:* `assets/species/flitter.ron` (new), `assets/materials/flitter.ron`
(new), and **two** `include_str!` lines — `organism.rs`'s `EMBEDDED` *and*
`material.rs`'s.
*Read first:* §2 here, and `hopper.ron`'s header, the file this is cut from.
**Both `include_str!` lines or the species does not exist.** A headless harness
reads only the embedded list, and `place_creature` resolves an animal's
material by its species' own name — so a missing material file makes the
species load, appear on the `COLONY` chip, and place **nothing**.
`hopper.ron` shipped one file short of exactly this once already.
*Build:* copy `hopper.ron` and change **only**: `body: Chain(2)`; `traits` slot
0 `0.0 → −1.0`; `nest: ""`; strip the pheromone steering, the four hidden gate
units and `(AtNest, Drop, w)`; add `(BloomNear, Turn, …)`, `(BloomBearing,
Turn, …)`, `(BloomNear, Move, …)`, `(Bias, Impulse, 2.0)`, `(BloomNear,
Impulse, 1.0)`, `(FoodAdjacent, Impulse, −2.0)`. The material is `hopper.ron`'s
with §2.7's pale blue-white band. **At the `(FoodAdjacent, Impulse, −2.0)`
line, write down why it is not the recorded dead end** (§2.5).
*Counters:* `impulses` and `impulses_refused`; `flower_visits` **by species**;
births, deaths, `starved aloft`.
*Measurement:* the shipped `hopper` at `(Bias, Impulse, 2.0)` as the control
arm, same bed, same seeds, 120,000 frames. `refused` and `starved aloft` must
fall while real launches hold. And: **flitter births > 0**.
*Card:* a `labgif` of one flitter crossing three flowers, real launches and
`flower_visits` in `meta`.
*Cost fork:* if it dies out at every setting of the hop, do **not** sweep the
hop further — `hopper.ron` records that the jump is not what kills this animal.
Report the death-cause histogram and stop.

### Brief C1 — animals carry pollen

*Owns:* `organism.rs` (`PollenGrain`, `cross`, `father_lineage`), `world.rs`
(the grain table, `LogKind::Crossed`), `plant.rs` (`bear_seed_at`). *One hook:*
two lines in B1′'s site, deposit then load.
*Read first:* §3.2 and §4 here, and `evolution-lab-genetics-2026-08-31.md`
§6.1. **Coordinate with the A2 lane before starting:** A2 adds
`Crop.passenger`, you add `OrganismState::pollen` — two fields, two lanes, one
struct.
*Build:* as §3.2 — uniform per slot on `alleles` and `genotype_draws`,
whole-table on `fates` and `params`, mother's `lineage`/`lineage_seed` always.
Deposit gated on **same species, different lineage**. The table is a `Vec`,
capped 256, oldest evicted, `pollen_evicted` counted — **it must bound work,
never gate whether a cross can happen.**
*Counters:* `pollen_loaded` and `flowers_pollinated` **split by carrying
species**; `crosses_born`; `cross_span` as a histogram.
*Positive control:* a two-compartment bed, one line each side, **no** partition
must produce crosses; **with** the partition, exactly zero. Any cross in the
partitioned arm means the lineage test is wrong.
*OWED constants to print, not assume:* `POLLEN_LIFE_FRAMES` **3,000** against
the measured median frames from load to deposit; the table lifetime **12,000**
against the measured median from deposit to fruit-set.
*Card:* the two parents and a crossed child at play zoom, `crosses_born` in
`meta` — **and say in the card that petal colour is not yet heritable**, so the
owner is not asked to judge a resemblance the engine cannot show (§3.4).
*Cost fork:* `crosses_born` zero with `flowers_pollinated` healthy is a finding
about where fruit-set actually happens. Write it and stop.

### Brief C2 — the petal-colour locus

*Owns:* `organism.rs`, `plant.rs`, `plainspeak.rs`. **Lands alone, in its own
PR, with nothing else in it.**
*Read first:* §3.5, and `CLAUDE.md`'s *"a green suite does not prove a test
could fail"* — the genome-widening case it describes **is** this change.
*Build:* `LOCUS_FLOWER_COLOUR = 6`, `DISCRETE_LOCI` 6 → 7, `LOCUS_ALLELES`
gains its seventh entry, `flower_band` from the allele the way `foliage_band`
is, the `ORGAN_BAND_STREAM` flower draw removed (the fruit draw stays), a
`plainspeak` sentence per allele.
*The cost, in the commit message:* **one more `rng.chance` per plant birth on
the caller's shared stream**, so every seeded plant figure in the lab moves.
Re-baseline them in the same commit and say that is why.
*Counter:* the allele's frequency in a standing bed — check a founder stand is
not all one value.
*Card:* one stand at three petal alleles side by side. **And a second question
in the same card:** should `herb` widen from two petal bands to three, so a
cross has a middle instead of a coin flip?
*Cost fork:* if the seeded suite cannot be re-baselined inside the lane, write
down which figures moved and stop.

### Brief I — the instruments

*Owns:* `stats.rs`, `ui.rs`, `examples/selection_arena.rs`. *Read first:* §4.
*Build:* the crossed share beside `LINES` and beside the allele histogram;
`pollen=off` as `selection_arena`'s bed default with allele-frequency
attribution when it is on; `LogKind::Crossed`'s sentence in `format_log_line`
and on the HISTORY page.
*The gate:* **a bed with pollen off reads digit-identical to today** on every
line these touch. That is the positive control and it is cheap.
*Card:* the HISTORY page with a cross in it.
*Cost fork:* if `selection_arena`'s attribution cannot be reworked inside the
lane, ship `pollen=off` alone and file the rest. A harness honest about what it
cannot measure beats one that is quietly wrong.

---

## 6. What is owed, what contradicts, and the rulings

### 6.1 The control that changed this document

The 81 → 25 → 8 collapse had two possible causes — the colony eating the
flowers, or the bed finishing flowering — and they prescribe different work.
The control (§1.2) points against grazing: **3 flowers standing at frame 40,000
with no animals in the box, and 8 with a colony of 38.**

**Method note, because it nearly went the other way.** The obvious reading was
written into a draft of this document as fact before the control was run. It
would have sent B1 at a problem B1 does not solve, and it would have read as a
strong finding — the collapse really happens and the number really is 81 → 8.
`CLAUDE.md`'s *"ask what your number counts when nothing is wrong"* is what
caught it, and the control cost one command.

### 6.2 The bed the owner ruled today does not exist yet

`assets/lab_scenarios/played_bed.ron` on `main` at `fef35310` is still the
thirteen-plant grass/herb/shrub mix, with **no scrambler and no tree**. The
thicket exists only as the comparison file `played_bed_scrambler.ron` (#302),
whose header records the tree's cost: **one tree founder becomes 79 plants and
shades the bench to 0.008 of lamp light by frame 30,000**, thirty times darker
than the herb bed, with the low plants that actually flower shaded out.

So every "3+ seeds on the played bed" measurement in §5 is of a bed nobody has
built, and **whoever builds it owes a paired figure first**: standing flower
and fruit at 6,000 / 20,000 / 40,000 with and without the tree. A pollinator's
larder is a *lit* bed's flowers. **That is the largest unpriced risk here.**

### 6.3 The finding this document cannot fix and should not pretend to

**The box has no indeterminate flowering plant**, and §1.2 is what that costs:
a bed that flowers hard for six thousand frames and then holds three to eight
flowers for the rest of the session. Nectar makes those few worth living on and
does not make more of them. Three routes, none this lane's:

1. **A flowering plant that does not terminate** — an axis that makes a lateral
   flower and *keeps growing*. `wiki/plants.md` says the shipped two "finish on
   purpose", so this is a third habit rather than a change to either.
2. **Faster recruitment** — the bed *is* recruiting (633 → 909 with no colony)
   and the recruits are not reaching flower inside 40,000 frames. Why is a
   question for the plant line: shading, `seed_maturity: 60`, or the budget
   block.
3. **B2's discount**, which makes the budget go further and is already in the
   order.

**Whichever it is, it is upstream of the pollinator**, and a flitter shipped
into a bed with three flowers at frame 40,000 will read as an animal that dies
— which will look like the species being wrong and will not be.

### 6.4 `dead-ends.md`, grepped before proposing anything

| grep | hits | what it says |
|---|---|---|
| `pollinat` | **0** | no pollination mechanism has ever been tried |
| `nectar` | **0** | nor a renewable flower meal |
| `hopper` | 4, **none about the animal** | all four are granular silo flow (Bazant's spot model) |
| `jump`, `impulse` | many, **none about the creature jump** | structural relaxation, gust dipoles, the gnome's climb key |
| `flower` | 2, lines **911**, **913** | 911: ripening must be charged to `reproductive_budget`, never the organ's own carbon — every organ is a donor, so a flower charged against itself is permanently poor (**35 flowers against 2 fruit**). Why B1′ charges the pocket and B2 acts on the cost. 913: a determinate axis's `after_metamers` must be checked against its turgor ceiling — bears on any change to how much a herb flowers |
| `windfall` | 1, line **1636** | *fruit can be **reached** is not the same as being sown* — everything stands 22–40 rows up. **A problem for the ant and the flitter's entire address**; §1.2 reproduces its fruit half at a third bed |

**Nothing here is a retry.** The nearest thing is the hop rate, and it is not
in `dead-ends.md` at all — it is `hopper.ron`'s own narrow do-not-retry, and
§2.5 gates on the opposite condition and says so at the line.

### 6.5 The owner rulings this depends on

- **"Animals carry it, but we will probably need to get creatures that are more
  pollination motivated then (like a bee/butterfly)"** — 2026-09-10; it
  **overturns** the ecology design §4.3's brush-first recommendation.
- **The 2.0 hop is the one they liked** — 2026-09-10; §2.5 is the reading that
  survives the direction lane's measurement.
- **The articulated bodies are parked** — so the pollinator is `Chain(2)`, and
  colour, height and the hop carry the silhouette instead of anatomy.
- **The thicket and a tree join the default played bed** — 2026-09-10; §1.1 is
  why that is this lane's precondition, §6.2 that it has not landed.
- **The seed rides home in an ant's crop (A2)** — a separate lane, and its
  field sits on the same struct as C1's.
- **"Give me the tools, data, access to the parameters"** (standing) — every
  number in §2 and §3 lands as a species or material field. Three exceptions,
  all bounds rather than settings: `POLLEN_LIFE_FRAMES`, the table lifetime,
  the 256-entry cap.
- **Ship new behaviours as default** (round twenty) — nothing here is behind an
  off switch; `pollen=off` exists only inside `selection_arena`, as a control.
- **An omnivore should be viable** (card `20260823T104411499Z-963f8d`) — §2.2
  puts the specialist gut on a **new species**, leaving the ant's guard alone.
- **Movement, not stills** (2026-09-09) — every animal card in §5 is a
  sequence.

### 6.6 What else is OWED

1. **All 120,000-frame figures**, every brief in §5. None exists.
2. **The paired three-seed version of §1.2**, colony against none.
3. **The median frames from pollen load to deposit, and deposit to fruit-set**
   — what decides whether the two pollen clocks are live knobs (§3.2).
4. **The nectar refill rate against a flower's actual career.** §3.1's "about
   twelve visits" is arithmetic on shipped constants, not a run.
5. **Whether a flitter can reach a flower at all.** The sight line is there on
   paper; nothing has ever tried to walk or hop an animal up a stem to a
   *specific cell*. P2's first honest failure mode is an animal that sees
   flowers and cannot arrive at one.
6. **What B1′ does to the colony**, which §1.2 measured with no reason to climb.

### 6.7 Two written claims this document moves

- **The ecology design §3.2's nectar payment is in the wrong units** (§3.1),
  and its §4.3 recommendation is **overturned by the owner's ruling** rather
  than by an argument here. Neither invalidates its measurements.
- **`flower.ron`'s comment** still calls nectar *"the richest thing a plant
  offers a walking animal"*. The ecology design noted that is true of the table
  and false of the world; **B1′ is what makes it true**, and the comment should
  carry the date it became so.

---

*Pollinator-design lane, round twenty-seven, 2026-09-10. Extends
`evolution-lab-ecology-design-2026-09-10.md` §3, §4 and §6; supersedes nothing.*
