# Colony fission — one odour per nest, and how a second nest starts. 2026-09-12

*Round twenty-nine. The owner: "I thought the code was set up that colonies
can diverge… What is most realistic? What do you recommend? Also ship it on."*

## The answer

Real colonies do not drift apart from the inside. Nestmates re-mix one odour
constantly — trophallaxis, grooming, the nest material itself — so the
difference accumulates **between nests that stop exchanging**, never inside
one. New colonies start by *budding*: a party of workers walks out of a
crowded nest and founds a satellite. While a thread of ants runs between them
they are one colony (polydomy); cut the thread and the two odours part, and
the frontier is where hungry strangers meet.

So: **a nest is a place that holds an odour**, an ant standing on it blends
with it, a party that leaves carries it a screen away, each patch's odour then
wanders on its own, and only the ants between hold the two together. Ships on:
drift **0.15**, blending **0.10**, the nest's wander **0.065 per 1,000
frames**, a `Leave` verb off the crowded nest — every one a dial.

## 0. What was measured, this session

`labstats`, played bed, seed 1, `RAYON_NUM_THREADS=4`, 120,000 frames.

| arm | alive | born | died | ant gens | shares | kills | sightings |
|---|---|---|---|---|---|---|---|
| `drift=0` (shipped) | 69 | 94 | 77 | **10** | 1,796 | 0 | 0 |
| `drift=0.15` | 69 | 94 | 77 | 10 | 1,796 | 0 | 0 |
| `drift=0.5` | **18** | 65 | 99 | 4 (ever 7) | 1,478 | 8 | 13,913 |

At `drift=0.5` the groups block reads

```
ANT 1      alive    0  starved 54  killed 22  | killed by ANT 1 x20
ANT 1b     alive   18  starved 8   killed 15  | killed by ANT 1b x15
```

**One nest, one home, and it eats itself** — the founding group wiped out, 20
of its 22 killings by its own name, the colony a quarter its size. That is the
case for cohesion, measured rather than argued.

`drift=0.15` is **byte-identical** to `drift=0`: the arithmetic being right,
not the knob disconnected. The harness echoes the value, the scent draw comes
from the birth's own stream (`creature.rs:2721`), and `E|Δ|² = 2nd²` gives
**0.45** at d = 0.15 against **5.0** at d = 0.5 on a radius² of 1.0 — one
formula predicting both arms, which is what makes it usable.

**Read §T2 before building either brief.** `deliveries 48` and `nest_visits
895` are *identical* at 9,000, 30,000 and 120,000 frames across both arms
while `pickups` scales 358 → 1,108 → 5,058: nothing comes home after the
founding cohort dies (`open-bugs-handoff.md` §T2, OPEN), and both mechanisms
below fire at the nest.

## 1. Cohesion — how a nest keeps one odour

**The nest holds the odour; the ant exchanges with it.** `World` gains a short
`Vec` of nest sites, one per `paint_nest_patch` call: `{x, y, scent:
[f32;3]}`. Per creature tick, for an ant whose `AtNest` input is already 1.0:

```
s_i += β (G − s_i)      // the ant takes the nest's odour
G   += γ (s_i − G)      // and leaves some of its own
```

**One ant against one nest site, and no scan is added:** `adjacent_nest`
(`creature.rs:5204`) is already called every tick for `AtNest`, so the blend
hangs off a branch already taken.

**Not trophallaxis as the load-bearing path, deliberately.** `Share` is a
brain output the genome evolves and may lose (round 25, never a rule), so
hanging cohesion on it puts cohesion hostage to an allele: a line that stops
sharing eats itself. Blending on `Share` rides along free — same contact — but
the *nest* is the floor, as it is in the gestalt model.

**One vector, not two.** `creature-signature-and-castes-2026-09-06.md` §1f
sketches a blended scent *beside* the heritable one; do not build two, because
the mouth, the eye and the kin sense read one scent through `scent_of`. The
`SCENT_SLOTS` are inherited *and* blended, so a child inherits its parent's
current scent.

**Why 0.15 is safe.** A newborn enters at the nest's odour plus ±d per slot,
so `|u| ≈ d`, decaying as `(1−β)^m` over m at-nest blends. Measured: **1,796
shares over 120,000 frames across 146 animals ≈ 25 contacts a lifetime**, a
strict lower bound on at-nest ticks. Over ages uniform in 0..25 at β = 0.1,
`rms = d · sqrt((1/25) Σ 0.81^m) = 0.458 d = 0.069` at d = 0.15, against a
radius of **1.0** — **7%**. The cloud reaches the radius only at d = 2.18, off
the ±1 axis: **no setting of `scent_drift` can make a cohered nest eat
itself**, which is why drift can finally ship non-zero.

## 2. Budding — a party leaves and founds

**The trigger is a verb, graded.** Append `BrainOutput::Leave` (slot 16 — a
lawful append under the 64-wide reserve, as `Fly` was on 2026-09-11) and wire
it in `ant.ron` off the crowded nest with the ±30 gate pair units 5/6 use for
digging: `(Bias, u, −30)`, `(AtNest, u, 30)`, `(Crowding, u, w)`, and its
mirror. `P(leave)` then rises with density at the nest and is flat elsewhere —
a distribution, not a threshold, and a line can lose it. **The rule evaluates
one ant, per tick.**

**Founding is a rule about a party.** A leaving ant sets
`OrganismState::leaving`, walks *down* channel A instead of up it, and keeps
what it carries. Each tick the engine asks of it: *am I `bud_distance` from
every nest site, on a column `colony_ant_site` accepts, with `bud_party − 1`
other leaving nestmates within a body length?* Those quantities are defined
for a party and already written. When true the party founds:
`paint_nest_patch` at the site, `claim_colony` mints the label,
`colony_parents.push((child, parent))` so the ANTS page names it `ANT 1b`
through machinery that exists, and a nest site is pushed carrying **the
party's mean scent plus `bud_scent_offset`**.

**Who goes: `bud_party = 8` workers** — 28 cells of band at
`COLONY_ANT_SPACING` 4, inside the 52-cell patch `COLONY_HALF_WIDTH` paints,
and 12–20% of the 40–69 ants the bed carries. **"With brood" has no referent
here and must not be invented:** there is no egg and no larva, and a party
carries what is in its stomachs, nothing else.

**Where: `bud_distance = 120` cells** from any nest site — the parent's patch
is 52 wide, its band 204, the box 512, so 120 puts the satellite outside that
band and visibly across the box, with room for more on a bed that grows (M10).
**What they take** is the parent's genes and its odour, their blended scents'
mean seeding the satellite. **What the player sees** is a second mound where
there was bare soil, and a thread of ants between the two for as long as any
keep crossing.

## 3. Divergence, and what it looks like

**Birth drift cannot separate two cohered nests, and this contradicts the
brief.** Once an odour is a colony-level quantity its centroid moves only by
births, each displacing it by `u/(N+w)`. With N ≈ 40 and the reservoir
weighing ~20 ants, `E|Δ|² = 2 n d² N/(N+w)²` = 0.0005 after ten generations
at d = 0.15 — **0.02** against a radius of 1.0, five hundred generations
against a session's ten. **The speciation speed cannot live in the birth
dial**, and the biology says the same: colonies smell different because they
were founded apart, not because they slowly mutated apart.

**So the odour a *place* acquires is the mechanism.** Each nest site steps ±σ
per slot every 1,000 frames; two nests no ant crosses separate as `E|Δ|² =
2nσ²`, so σ = 1/√(2n). At **σ = 0.065**, 120 steps put them at the radius —
**strangers one session after the thread breaks.**

**And one ant a minute holds them together.** A crossing ant arrives carrying
the odour it left with; over ~10 at-nest ticks at γ = 0.02 it moves the
receiving nest 18% of the gap, against the 0.09 the wander opens per 1,000
frames. **One crossing per 1,000 frames holds the gap at half a radius** —
kin, but visibly not identical — and two hold it at a quarter. Polydomy
becomes something the player can *see*.

**The frontier at play zoom.** Hungry ants bite strangers by the `Feed` path
and the alarm plane lights up. The `drift=0.5` arm says what that reads as:
attacks 1,990, alarm deposits 12,807, kills 8, and — the independent tell —
`sightings 13,913 | threat sightings 14,365` against **0 / 0** when everyone
is kin. Every one of those counters already prints.

**Trails: keep one shared set; do not key them per nest.** `pheromone.rs`
prices a plane at **~40 MB** in the shipped 8192×2560 world, allocated
eagerly. Per-nest keying is 40 MB × nests, unbounded, to buy a separation the
shared plane already produces: ants near each mound climb their own end of the
gradient and those between oscillate — which *is* the thread, and polydomous
ants genuinely share a trail network.

## 4. The economy, priced

**Budding costs the parent** 8 foragers and their delivery for the walk. No
energy is destroyed — the party carries its own banks and `paint_nest_patch`
converts surface material rather than buying it. **Founding costs the party
nothing today**: there is no construction price for a nest, and the gap stays
visible rather than invented away. **Constants calibrated against today's
single-nest bed, re-derived as part of the work rather than after it:**

- `brain::live_slots()` **846 → ~883** (one output × 29 inputs + 8 hidden).
  Every species' `mutation_rate` is derived from that count and must move with
  it — `Fly` paid exactly this on 2026-09-11, guarded by
  `the_live_slot_count_is_pinned_because_mutation_rate_is_derived_from_it` —
  and it shifts `brain::mutate`'s draw sequence, so **no world with a breeding
  animal is bit-identical across B2**.
- `COLONY_HALF_WIDTH = 26` against `COLONY_ANTS = 52` — the patch-to-band
  ratio the foraging scene measured 414 deliveries at. A party of 8 on a
  52-cell patch may have no gradient to climb; derive the patch width from
  the party.
- `reproduce_threshold = 1100`, grant `0.4 × 200 = 80`, stamp 960 — priced for
  a colony that never leaves one nest; a satellite of 8 has eight times fewer
  sharers. `CROWDING_SCALE = 8.0` / `CROWDING_RADIUS = 2` were fitted for
  digging, and `Leave` at digging's weight buds forever. `PHEROMONE_INTERVAL
  = 12` was fitted to a ~2,200-frame round trip at one nest; a 120-cell thread
  is longer than that fit saw.

## 5. The dials, and what ships on

| dial | default | what it is | the arithmetic |
|---|---|---|---|
| `scent_drift` | **0.15** | per-birth width (exists) | cohered cloud 0.458·d = **0.069** of a 1.0 radius (§1) |
| `nest_blend` β | **0.10** | an at-nest ant's step toward the nest | 25 contacts a life → 0.9²⁵ = **0.072** residual |
| `nest_uptake` γ | **0.02** | the nest's step toward an ant | ~10 ticks a visit moves a nest 18% of the gap |
| `nest_scent_drift` σ | **0.065** /1k frames | the odour a place acquires | 2nσ² = 1 at n = 120 → strangers **one session** after cut-off |
| `bud_scent_offset` | **0** | the founding jump | a bud keeps its parent's odour; the thread decides |
| `bud_party` | **8** | how many leave | 28-cell band in a 52-cell patch; 12–20% of the colony |
| `bud_distance` | **120** | how far out a party may found | patch 52 + band 204 in a 512 box |
| `Leave` wiring | `ant.ron` | the verb, off `AtNest × Crowding` | graded, never a threshold |

Each is a dial on the GENOME/ANTS page beside the four that exist, and a
`labstats` argument.

## 6. Guards and measurement

**Positive controls**, each watched going red with its mechanism removed:

1. `a_cohered_nest_never_splits_into_strangers` — one nest, `scent_drift =
   1.0`, 120,000 frames, blending on: `regroup_by_scent` mints nothing and
   `killed by ANT 1` stays 0. **The measured negative it replaces is §0.**
2. `two_cut_off_nests_read_as_strangers_within_one_session` — two nests, a
   wall between: mutual kin false by 120,000 frames. The control that makes it
   sensitive is those two with a corridor, which must stay kin.
3. `a_crowded_nest_buds_and_the_party_founds` — force `Leave`; assert a second
   patch, a second label with `colony_parents` naming its parent, and its
   odour within `bud_scent_offset` of the parent's.

**The bar.** The played bed's colony no worse off at 120,000 frames than
today, seed 1: **alive 69, born 94, died 77, generations 10, shares 1,796** —
a session census, never a minute's. Pair each "it fired" counter with an
effect counter from the far side of the call, and measure `ascii`'s
bit-identity rather than asserting it.

## 7. Two build briefs

### B1 — cohesion, and drift on by default

**Owns** `creature.rs` (the blend, on the `AtNest` branch), `world.rs` (the
nest-site list), `ant.ron` (`scent_drift: 0.15`), `lab/ui.rs` (the dials),
`examples/labstats.rs` (`blend= uptake= nestdrift=`, a per-nest odour
readout). **Not** `organism.rs`, **not** `brain.rs`, and none of the walk — so
it runs beside the long-ant lane. **Read first, capped:** this report;
`creature.rs` `scent_accepts` / `trait_width` / `adjacent_nest`; `world.rs`
`regroup_by_scent`; §1f.

**Build** the nest-site list, β/γ on the `AtNest` branch, the σ wander on the
census cadence, a blend on the `Share` path. **Counters:** blends applied,
per-nest odour, the gap between every pair. **Control:** guards 1 and 2.
**Measurement:** the §0 table at `drift=0.15` with blending on — colony no
worse off, and the arm must now *differ* from `drift=0`, which today it does
not. **Card:** `labgif` of one nest at `drift=1.0`, cohered, mint and kill
counts in `meta`. **Cost fork:** if the per-tick blend shows on `ascii`'s
worst frame, move it to the census cadence and re-derive β.

### B2 — budding, and the satellite nest

**Owns** `brain.rs` (`Leave` = 16), `organism.rs` (`leaving`), `creature.rs`
(the verb, the party rule, `found_satellite_of`), `ant.ron` (the wiring **and
the re-derived `mutation_rate`**), `lab/stats.rs` (a satellite's line naming
its parent). **After B1** (it needs the nest-site list) **and after the
long-ant lane**, since `found_satellite_of` calls `plant_creature_seed_in` and
so the founding walk that lane is inside. **Read first, capped:** this report
§2 and §4; `creature.rs` `found_colony_of` / `colony_stations` /
`paint_nest_patch`; `world.rs` `claim_colony` / `colony_parents`; `brain.rs`
at `BRAIN_OUTPUTS`.

**Counters:** `Leave` rolls, ants leaving, parties founded, satellites
standing, crossings per 1,000 frames. **Control:** guard 3, plus `live_slots`
pinned and every `mutation_rate` moved in the same commit. **Measurement:**
the §0 table with budding on — the honest risk being that **§T2** keeps
`AtNest` at zero after the founding cohort dies, in which case the finding is
*the trigger is correct and the bed cannot reach it*, not *the mechanism is
dead*; check the nest material still stands first. **Card:** `labgif` of a
crowded nest budding, the party walking, the second mound, the thread, with
parties founded and crossings in `meta`. **Cost fork:** gate the party scan on
`leaving` being non-empty.

## What this contradicts

- **The brief asks how many generations until two cut-off nests read as
  strangers. There is no such number** (§3): birth drift moves a colony-level
  odour as `d²/N`, so the speciation speed lives in a place dial and the
  brief's guard is restated in **frames**. Relatedly, **`bud_scent_offset`
  defaults to 0, not to a founding jump** — a bud keeps its parent's odour,
  which is why polydomy is the default outcome.
- **§1f sketches a worn scent *beside* the heritable one;** this recommends
  one vector carrying both (§1). **"Brood" is not modelled**, and §0's frozen
  `deliveries` (§T2) is upstream of both mechanisms.

## Rulings this depends on

- Trophallaxis is a brain output the genome evolves, shipped on, **never a
  rule** (round 25) — so cohesion may not hang on it (§1).
- A queen is three authored values over mechanisms that exist, **never a type
  the engine knows** (round 25) — the party is `bud_party` / `bud_distance` /
  a wiring, not a caste object. *Rest is the absence of a reason to act* —
  `Leave` is a reason.
- **Stop balancing, start exposing** (round 3); **ship new behaviours as
  default** (round 20); **movement, not stills** (round 25), so both cards are
  `labgif`. The old *colony rivalry* switch stays retired.
- An outcome is a distribution, never a binary; and a verb must deliver
  something visible — the mound and the thread are `Leave`'s.
