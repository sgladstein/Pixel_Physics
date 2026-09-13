# The late game: a bed that goes on — design brainstorm, round 29

*Owner's ask, 2026-09-12, verbatim: "when there is available food / lots of
plants, the colony grows and breeds until they are so large. They decimate
all the plants, the food disappears and then the colony dies; when you get
to hundreds of creatures, they dig large chambers underground and piles of
dirt/chambers above the nest, which creates an area where plants don't
grow." Status: **design of record for the late game, with the census that
every recommendation below is measured against. Nothing here is built except
the instrument.***

## The answer

1. **Turn the harvester into a sower.** The colony's staple is already the
   seed bank — half the bed's edible joules, and the bed sets ~55 seeds per
   1,000 frames of which 84% never germinate. Today a bitten seed is destroyed;
   make it *cargo*: the bite pays the seed's provision and the seed rides
   home as the passenger the garden loop already carries, set down where the
   meal ends. Granivory becomes dispersal, the midden becomes the garden, and
   the colony's food scales with the plants' surplus instead of their bodies.
   With it, a live leaf becomes marginal food at the shipped gut — a dial —
   so grazing the stand is a niche a lineage can *evolve into*, not the
   default every colony ships with.
2. **Ants die of age.** No creature has a lifespan; every death in every
   run is starvation, so a population can only fall by famine. The plant
   hazard (`old_age_chance`, median at the half-life) applied to the ant
   bounds the colony at births × lifespan, spreads the decline over the
   whole run, and turns each death into 240 J of carrion for a nestmate.
3. **The nest stops digging when it has room, and its spoil weathers.** The
   dig gate reads a 5x5 density that hundreds of ants at one door pin at 1.0
   for ever, and nothing digging does can lower it; read *room per ant*
   instead. A dumped pellet is cemented for ever; let exposed spoil weather
   back to loose soil so the mound cones and roots take it, and the pips the
   sower drops make the anthill the bed's richest ground.

Why these and not the rest: budding relocates the load and the fission lane
owns it; birth gating already reads delivered stock and would only slow the
boom; refuges protect the canopy the colony never eats while it eats the
floor; seasons force a cycle the colony cannot yet store through; predators
add a second oscillator. The census says the crash is the seed bank going
first and the stand after it — so the fix is what the mouth does to a seed.

## 0. The census — `examples/latecensus`, built here, brief 0 delivered

`latecensus scenario=played_bed frames=500000 sample=20000 seed=N`,
`RAYON_NUM_THREADS=1`, shipped code at `be2808de`. One row per 20,000 frames:
ants, plants (**live organisms minus the waiting bank** — `labforage`'s
`plants` column counts both, so every earlier "plants" figure on this bed is
plants+bank), seed bank, edible cells and gut-priced joules at the founders'
gut **split by kind**, births and deaths by cause, digs, and the nest's
footprint: roofed void below the original surface, open pit, packed cells
above and below it, and the columns within ±64 of a nest with no plant at
all against the same outside. `control=selftest` carves a known chamber, pit
and mound and asserts each column. Cross-checked against `labforage` at
20,000 frames (ants, edible, worth identical). The unfed arm is `no_colony=1`, the paired control.

**The boom and the crash, every seed** (ants / plants / seed bank / edible
gut-priced kJ):

| frame | seed 1 | seed 2 | seed 3 | seed 2, no colony |
|---|---|---|---|---|
| 20k | 20 / 182 / 855 / 226 | 28 / 187 / 897 / 240 | 73 / 204 / 922 / 384 | — / 189 / 895 / 221 |
| 60k | 56 / 189 / 551 / 252 | 78 / 186 / 665 / 121 | 132 / 111 / 429 / 381 | — / 251 / 950 / 170 |
| 100k | 32 / 169 / 484 / 249 | 50 / 122 / 368 / 110 | 57 / 126 / 298 / 356 | — / 242 / 790 / 164 |
| 140k | 93 / 120 / 345 / 175 | 18 / 77 / 129 / 74 | 173 / 104 / 287 / 161 | — / 224 / 962 / 180 |
| 180k | 116 / 94 / 214 / 137 | 34 / 63 / 74 / 82 | 352 / 91 / 174 / 118 | — / 225 / 1,176 / 193 |
| 200k | 65 / 64 / 150 / 92 | 19 / 46 / 37 / 57 | **495** / 56 / 82 / 31 | — / 239 / 1,443 / 228 |
| 260k | 19 / 26 / 11 / 21 | 16 / 59 / 77 / 92 | 24 / 13 / 2 / 8 | — / 312 / 1,239 / 238 |
| 320k | 11 / 16 / 6 / 4 | 33 / 62 / 109 / 162 | 3 / 8 / 0 / 5 | — / 309 / 820 / 204 |
| 460k | 0 / 12 / 2 / 3 | 36 / 122 / 245 / 466 | 0 / 6 / 1 / 4 | — / 201 / 584 / 150 |
| 500k | **0 / 14 / 2 / 3** | **90** / 84 / 181 / 418 | **0 / 6 / 2 / 5** | — / 184 / 646 / 150 |

Three shapes from one binary. Seed 3 is the owner's report to the digit: 73
→ **495 ants** at 200,000 frames, the stand 204 → 56 plants and the bank 922
→ 82 under it, then 24 ants and **13 plants with two seeds in the ground** by
260,000 — a dead bed with a session still to run. Seed 1 booms later and
lower (116 at 180k) and ends the same way: 0 ants, 14 plants, 2 seeds. Seed 2
peaks at 78, crashes to 16 at 260k, and the bed comes back under the small
colony — 122 plants over a bank of 245 by 460k — whereupon the colony booms
again, 90 at 500k and climbing: the one bed that goes on goes on by
**cycling**, with a period near 400,000 frames. The unfed control swings
190–335 plants over a bank of 400–1,450 and never thins: the same bed,
without the mouth, has a gentle cycle of its own and no crash.

**What the mouth takes first is the bank.** Gut-priced larder by kind, seed
2 (leaf / litter / seed, kJ): 20k **40 / 83 / 111**; 100k 57 / 1 / 47; 200k
39 / 4 / 7. Leaf holds while the seed larder falls **16x** and the bank goes
897 → 37; the unfed bank sits at 900–1,200 throughout. Seed 3 eats both:
leaf 239 → 16 kJ, bank 922 → 82. A seed is 480 J at face — a whole leaf —
lying on the floor, the best food an ant can reach without climbing, and
eating it destroys the organism (`seed_survives_bite` is `false` for
anything but a windfall's passenger). **This is the overgrazing: it is
granivory of the bed's future before it is herbivory of its present.**

**What the bed produces, unfed** (seed 2, cumulative to 500k): **seeds
borne 26,768** (~54 per 1,000 frames), germinations 4,229 (16%), leaves shed
15,485 (~7 per 1,000 early, ~35 once the stand matures), fruit dropped 364;
2–23 flowers standing at any stop, up stems where no walker reaches. Per
1,000 frames, gut-priced at the shipped ant: seeds **~6,600 J**, litter
900–4,200 J, windfall ~200 J, nectar ~1,500 J if ten flowers were sipped
continuously. Against the measured burn of 0.316 J per creature-tick —
**~53 J per ant per 1,000 frames** — the seed rain alone could carry ~125
ants, and 84% of those seeds never germinate. That is the surplus, and it is
the one the colony already lives on.

**The dead zone is there from the first stop, and it is traffic, not
substrate.** Columns within ±64 of the nest with no plant cell, against
outside: seed 1 at 20k **68/129 (53%) vs 21/383 (5%)**, at 140k 101/129 vs
32/383, at 500k 104/129 vs 152/383. The unfed control's band goes 27/129 →
**0/129** by 160k — the founding clearance re-vegetates on its own. Seed 3
at the 200k peak reads 50/129 vs 153/383: by then the *whole bed* is the
dead zone. And seed 2 splits the cause: while its colony sat at 16–46 ants
(260k–440k) the band closed **84/129 → 0/129 with the cemented mound still
standing** (packed above the surface 386–456 cells throughout). A mound does
not stop plants; a colony at the door eating every seed and seedling does.

**The footprint** at 500k (roofed void / open pit / packed below / packed
above the original surface, cells): seed 1 306 / 116 / 694 / **711**; seed 3
302 / 40 / 1,281 / **1,099**; unfed 26–143 / 0–18 / 0 / **0** — the unfed
roofed void is roots eating soil, so the chambers net ~200–300 cells, and the
cemented spoil above the surface is the colony's alone. (The instrument's
`mound_high` column reads 24–40 rows on the *unfed* bed — litter rotting to
soil high on a drift — so it is not the anthill's height and is not quoted.)
Eats / digs / deliveries over the run: seed 1 28,281 / 1,969 / **34**, seed 3
59,532 / 6,109 / **14,997** — the boom is the round trip closing, and the
colony that never found its way home never boomed. The spoil above the old
surface is as big as the chambers under it, and every cell of it is cemented.

## 1. The brainstorm

Each candidate: what the player sees / frame cost / constants it reallocates
/ what already carries it. `dead-ends.md` grepped for every mechanism named
(`lifespan` 0, `senesc` plant-only, `spoil` 20, `chamber` 6, `graz` 3,
`palatab` 0, `nutrient` 1, `season` 0, `territory` 0, `gut_bias` 7,
`EAT_YIELD` 1); the entries that bind are cited where they do.

**1. What the colony eats.** The shipped ant is an omnivore at gut 0.0:
leaf, litter, seed 120 J a cell, fruit/windfall 240, corpse 120 (a 480-J
stamp at quarter quality), nectar 30, pip 10 (not food). The census says the
staple is the seed bank, then leaves, and that the stand is stripped as a
side effect of a colony that never stops breeding on seeds. Two moves, one
mechanism between them:
- *Seed as cargo (myrmecochory).* A bare `seed` bitten rolls the species'
  `seed_gut_survival` exactly as a windfall's passenger does (0.6 on herb
  and scrambler, **unset — i.e. certain death — on grass, shrub and tree**,
  so three species files gain a value); on survival the
  bite pays a **provision fraction** of the seed's 480 (a dial, ~25%) and the
  seed rides in `Crop::passenger`, released where the meal ends
  (`deliver_seed_passenger`, the round-29 rule). On failure it pays full
  value and the seed is digested, as now. Expected yield per seed falls to
  ~40% of a leaf's, so the mouth's ranking (`adjacent_food_counted` ranks by
  gain) stops preferring seeds over litter and fruit. Player sees: seeds
  carried home, the midden sprouting, `plants_from_pip` climbing from its
  measured 0–2 per run; the bank stops draining. Cost: nil — one roll and a
  passenger write per seed bite; no new state. Reallocates: `seed.ron
  food_energy`'s meaning, `EAT_YIELD_THRESHOLD`'s worked table, the A1/A2
  counters. Carried by:
  `seed_survives_bite`, `Crop::passenger`, `deliver_seed_passenger`, the
  midden.
- *Leaf marginal at the neutral gut.* `leaf.ron`/`grassblade.ron`
  `food_energy` 480 → a value that reads under the 12-J bar at gut 0.0 and
  over it at ≤ −0.6 (40 face: 10 at neutral, 25.6 at −0.6, 40 at −1.0).
  Litter stays 480: the floor is food, the plant is not. A grazer is then a
  lineage that walked its gut down and gave up carrion at −0.68 — the
  distribution law on the diet axis, and the histogram on the colony page
  shows it. Cost: nil. Reallocates: `ascii`'s foraging scene feeds a pile of
  `leaf` and gates `deliveries > 0` (make the pile litter); `crop_capacity:
  1440`'s "three leaves"; `digest_rate`'s bracket; `dead-ends.md` lines
  1593/1632/1645/1648's leaf arithmetic; `EAT_YIELD_THRESHOLD`'s table.
  Does not touch the flitter (`nectar_only` is a menu, not a price) or the
  beetle (gut +).
- *Z6 answered:* the colony starves because the right food runs out *after*
  it has eaten the wrong one. On seed 3 the larder is 384 kJ at 20k and 31 kJ
  at 200k with 495 mouths; at 53 J per ant per 1,000 frames that colony
  needed 26 kJ per 1,000 frames and the unfed bed makes ~8. There was never
  a bed for 495 ants; there is a bed for ~130 on seeds alone, ~150 with
  litter and windfall, if it stops eating the bank's future.

**2. Lifespan.** `DeathCause` has Starved, StarvedInFlight, Killed, Culled;
`OrganismState::born_frame` is the age and nothing reads it for a creature.
`plant::old_age_chance` is a Weibull hazard, median at `life_half_life`, 96%
survival at a quarter of it, **1.31% at 2.5x** (this line said 0.4% until
2026-09-12; lane M measured the hazard as shipped and the design's figure was
wrong) — already graded, already tested.
A `life_half_life` on `CreatureDef` (0 = immortal, every species'
default, bit-identical) rolled per creature tick with the plant's salted
stream. Authored for the ant at a measured value — the census's boom takes
~100,000 frames, a generation on this bed is ~12,000, so ~40,000 (three
generations, an ant outlives its founding grant 10x) is the first setting to
sweep, exposed as a scenario `Setting` and a lab dial. What it does to the
boom: N settles at births × L instead of running to the larder's edge, and
the fall is a slope not a cliff; what a corpse feeds: 240 J to a neutral
nestmate, ~23% of what the ant cost to make, and at L=40,000 a colony of 60
yields 1.5 corpses per 1,000 frames — ~7 ants' upkeep recycled. Also the
eusociality lane's clock: a session gets 5–10 generations instead of one
line that outlives it. Cost: one `rng::chance` per creature tick. Not
heritable in the first build (a trait slot widens the genome and shifts
every seeded draw — the `CLAUDE.md` gotcha); heritable later with a standing
cost like armour's, or it is a ratchet.

**3. Birth regulation before the crash.** `try_bud` needs `bank +
reachable_provision ≥ reproduce_at_of` (1,100 × the heritable
`TRAIT_REPRODUCE_AT`, floored at cost+1 = 1,041); `reachable_provision` is
gut-priced food in the head's eight neighbours, i.e. the larder at the
door. So births are already paid from *delivered stock*, the lag is one
larder's depth, and "foragers returning empty" is already the signal —
deliveries fall, the door empties, births stop. The overshoot is not a
breeding lag; it is that the stock keeps arriving while the bank is being
eaten. `KinNeed` (largest deficit among touching kin) could raise the bar
— `bar *= 1 + k·KinNeed` — an anticipatory brake that fires when nestmates
go hungry while food still stands. It is a rule on a gene's input, cheap
(the ring walk exists), and it slows the boom without changing what is
eaten; ranked below 1 and 2 because it moves nothing the player can see.
`queen`/`graded` (`PIXEL_PHYSICS_BREEDING`) regulate *who* breeds, not how
many are fed. Trophallaxis is shipped on and flattens reserves
(`ants.md`); nothing here changes it.

**4. Space.** Budding (fission lane, treated as available) spreads the load
across satellites; it *regulates* only if each nest's stripping is bounded —
by trail decay (`bdecay`) and the walk's diffusion it is, at roughly 260
columns of a 512 bed, so the played bed holds two nests and a third is
competition. Competition between nests and predators (the beetle, exogenous
today at `reproduce_threshold 0`) regulate in the Lotka–Volterra sense and
add a second oscillator; a beetle that breeds on ants is the box's next
relationship, not this fix. Territory as an authored range is the refusal already
given to authored spoil placement. **Relocates:** budding, territory. **Regulates:** diet,
lifespan, predation. Order: diet and lifespan first, or every satellite
booms and crashes on its own patch.

**5. The plants' side.** 95.8% of edible cells stand five or more rows up
and the colony still ate seeds 2 and 3 to 4–15% of the unfed larder: the
refuge protects the canopy and the regeneration runs through the floor,
where the bank is. So the plant-side lever that matters is that **the seed
bank is not the larder** — which is candidate 1. Burying seeds (a seed
worked one cell into soil by litter or rain) is the other form, cheaper in
mechanism and worse in feedback: it hides the bank instead of moving it.
Herbivory defence D (palatability from foliage tone) is priced and stays
binary until `LOCUS_ALLELES[0]` widens — after 1, not instead. Grazing that
leaves the plant alive already exists (a bite is one cell; a plant that can
pay regrows); what kills is the seedling, one cell, one bite. The right
plant variable is the pair the wiki already names: **plants over bank** —
seed 2 alive at 61 over 130 is a bed; seed 3 at 13 over 2 is not.

**6. The nest's footprint.** *Why it digs for ever:* `ant.ron` units 5/6
gate `Dig` on `AtNest` (an 8-neighbourhood test against the surface nest
patch) times `Crowding`, the fraction of a 5x5 that is flesh, clamped at
1.0. `dead-ends.md` line 1677 already measured the population statistic
pinned at 1.000 at the nest; with hundreds of ants at one door it cannot
leave saturation, and no chamber dug below relieves a density measured at
the patch. What would satisfy it is **room per ant**: ants / roofed void of
the colony, 1.0 packed, →0 spacious — a colony-level input like `KinNeed`
or `Alarm`, maintained by the dig and dump verbs (no grid scan), read in
the `Crowding` slot *when `AtNest`* so the genome is not widened. A colony
of 60 with a 300-cell nest reads 0.2 and stops; 495 in 360 reads 1.0 and
digs, which is Toffin's own transition. Cost: two counters per colony.
Reallocates `(Crowding, Move, -0.3)` at the nest only. *Spoil:* the dump
site is the first open cell with a footing beside the ant, else the first
open surface up the column (`SPOIL_LIFT` 160) — straight up over the nest,
tamped, `self_supporting`, never slumps; the waterlogging rule that undid it
is off by ruling. So the pile is permanent. **Spoil weathers:** `packedsoil`
gets `decays_into: soil` and the *dump* verb schedules the decay site (the
lining, laid by `line_burrow`, schedules nothing and never weathers) — a
pellet on the surface reverts to loose soil at litter's kind of rate, slumps
at the angle of repose, and a mound becomes a cone of ordinary rootable
ground with `water_capacity` 1,000. The dead-end on loose spoil (refills the
gallery) applies to a *fresh* pellet beside a gallery; a cone eroding into
a shaft mouth over thousands of frames is the anthill's crater and the
colony re-digs it. *What spoil is:* today a `packedsoil` cell with the water
it held; roots pay 1.2x to enter it, seeds germinate on it if wet. The
mound as the *richest* ground needs a nutrient the engine does not have — a
per-cell scalar fed by decay and drawn by roots is a moisture-pass-scale
cost (the perf line's second block) and a new economy for every plant
species to re-derive against: priced, declined here; "weathers back to
soil" plus the sower's pips at the door buys the visible half for nothing.
*Abandoned chambers caving in* contradicts the 2026-08-30 ruling (collapse
removed from the lab); the slump form — a lining cell with no traffic for N
frames loosens — is available and low-ranked.

**7. Time.** The lab pins noon and clear weather; `labstats steady`
confirms no oscillator. A lamp schedule or a dry season forces a periodic
resource, and a consumer with no store dies in every trough — it makes the
crash periodic, not survivable, until the colony can bank food (the nest
larder is cells at the door, eaten by whoever passes). A season is a player
verb for the box, in the owner's "give me the tools" sense, and a good one;
it is not the regulator and hides the problem in the runs it does not kill.

**8. The instrument** is built (§0) and is the gate for every build below:
a colony alive and a bank above ~500 at 500,000 frames on p90 of seeds,
with `bare in band` under 2x `bare outside`.

## 2. Ranked builds

**Brief 1 — the seed is cargo.** *Owns:* `src/sim/creature.rs` (the bite's
`seed_survives_bite` call and the passenger lift, ~40 lines), `src/sim/
plant.rs::seed_survives_bite` (accept a bare `seed` cell), `assets/species/
*.ron` (`seed_provision_fraction`, new, default 0.25), `assets/materials/
leaf.ron`, `grassblade.ron` (`food_energy`), `examples/ascii.rs` (the
foraging pile → litter). *Read first, capped:* `Reports/lanes/evolution-lab-
seed-where-eaten.md`; `creature.rs` 5780–5830 and 1585–1610; `EAT_YIELD_
THRESHOLD`'s doc. *Build:* bare seed rolls survival → pays fraction, rides;
leaf value dialled; `PIXEL_PHYSICS_SEED_CARGO=0` kill switch, one binary.
*Counters:* `seeds_carried` from bare bites (new source tag), `pips_
released_by_digestion`, `plants_from_pip`, bank. *Positive control:* fraction
1.0 and survival 1.0 must give `seeds_carried` = seed bites; leaf at 480 must
reproduce §0 byte for byte with the switch off. *Measurement:* `latecensus`
500k, seeds 1–3, both arms; the bar is §1.8. *Card:* the midden over 60,000
frames as a GIF, seedlings ringed, `plants_from_pip` in `meta`. *Cost fork:*
if the bank still drains with seeds riding, the drain is seedlings not
seeds — stop and measure `eats` by material.

**Brief 2 — the ant dies of age.** *Owns:* `src/sim/organism.rs`
(`CreatureDef::life_half_life`), `src/sim/creature.rs` (one roll in
`creature_tick`, `DeathCause::OldAge`), `assets/species/ant.ron`, `src/lab/`
(the dial). *Read first:* `plant.rs::old_age_chance` and its tests; `wiki/
ants.md` "Ants starve now". *Build:* the plant hazard per creature tick,
salted stream, default 0. *Counters:* `deaths_by_cause[OldAge]`, corpses
standing. *Positive control:* `life_half_life: 6000` must kill half the
founders by 6,000 frames. *Measurement:* `latecensus`, 20k/40k/80k swept,
p90 of 3 seeds. *Card:* the colony page's larder histogram and the population
strip over a session. *Cost fork:* if age deaths do not flatten the peak (N
still runs to the larder), L is above the boom's period — halve it once,
then stop.

**Brief 3 — room and weathering.** *Owns:* `src/sim/creature.rs` (`sense`'s
`Crowding` when `AtNest`; the dump verb schedules decay), `src/sim/world.rs`
(per-colony `roofed`/`ants` counters), `assets/materials/packedsoil.ron`
(`decays_into`, chances), `wiki/ants.md`. *Read first:* `dead-ends.md` 1015–
1025 and 1677; the spoil-drop comment in `act`. *Build:* room-per-ant in the
slot; `decays_into: soil` scheduled from the dump only. *Counters:* digs per
1,000 frames vs ants, `packed_above`, `mound_high`, `bare in band`.
*Positive control:* a bed with the room input pinned at 1.0 must dig as
today; a mound placed by scenario with no colony must cone within N frames.
*Measurement:* `latecensus`; the bar is the dead-zone ratio. *Card:* the
nest cross-section at 100k and 300k, before/after. *Cost fork:* the census already says
the band is traffic (seed 2 closed it under a standing mound), so this brief
is for the silhouette and the dig rate; if `bare in band` does not move,
that is expected, not a failure.

**After these:** the fission lane's budding; D's palatability; a beetle that
breeds; a season dial.

## 3. What this contradicts

- `labforage`'s `plants` column is plants **plus** the waiting bank; the
  bed-thinning bisect's 482→39 and round 28's 574→39 are that sum. The
  crossings stand; the labels do not.
- §Z6 *"the plants are not the casualty"* — already withdrawn by the
  separated report; the census adds that the **bank** is the first casualty.
- The ecology design's D-before-diet sequencing: the mouth's best reachable
  food is a seed, not a fruit it rarely reaches.
- `ants.md` "Ants starve now, and a colony settles at a size" — at session
  length it does not settle; it booms to 495 and dies.

## 4. Rulings this depends on

Stop balancing, start exposing (leaf value, provision fraction, lifespan,
room per ant are dials); ship new behaviours as default; an omnivore should
be viable (kept: the gut stays 0.0, carrion stays on the menu); a seed drops
where it is eaten (extended to bare seeds); collapse is removed from the lab
(weathering, not cave-ins); everything is priced (a lifespan gene needs a
cost before it is heritable).

## 5. Questions for the owner

1. Is a colony of **60–150** on this bed, living on seeds, litter, windfall
   and its own dead, the game — or must a bed carry hundreds, which means a
   bigger bed, more light, or a richer surplus (nectar, a fruiting stand)?
2. May a live leaf stop being ant food by default, with grazing an evolved
   niche? It reverses nothing ruled, but the ant has eaten leaves since day
   one and `ascii`'s scene is built on it.
3. Lifespan: authored per species now, heritable later — or wait for the
   priced gene?
