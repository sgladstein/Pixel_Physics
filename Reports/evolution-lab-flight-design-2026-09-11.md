# The flitter floats: sustained, steerable flight toward a bloom

*Design, round 29, lane DESIGN. Against `main` at `c51ae888`; no source changed.
Every number cites a code line or was measured here on one binary — `labforage
scenario=played_bed creature=flitter frames=12000`, seeds 1/2/3,
`RAYON_NUM_THREADS=4`.*

## The answer

The flitter cannot find a flower because **it is blind and rudderless for a third
to a half of its life, and the half of its brain meant to steer has no working
surface anywhere in the box.** `step_flight` runs no brain: an airborne animal
"does not read the world, does not evaluate its brain, and does not act"
(`creature.rs:3132`). On the ground, open bug **R4** has `Turn` nearly inert for a
walker on level footing. So `(BloomBearing, Turn, −2.5)` steers almost nothing,
almost never. Measured: **0.60 walking steps per launch**, 29–46% of all
animal-frames aloft. Switch the hop off with one weight and the animal walks
8–38x more, sees 1.5–13x more blooms, and on seed 3 visits **34 flowers against
0** — the only arm of six that breeds.

**So the float is not a prettier hop; it is where `Turn` finally works** — in the
air there are no candidate cells to be refused, only a velocity to rotate. B1: the
hop stays as the take-off the owner liked, a new `Fly` verb holds the animal up at
a price, the brain runs every tick aloft, `Turn` rotates the velocity,
`FoodAdjacent` sets it down on the flower. Nine cells then close in **34 frames
for 2.1 J against a 120 J meal**.

---

## 1. The mechanism of floating

**Reuse `Flight`.** `launch` is already the only thing that sets
`OrganismState::flight` (`creature.rs:7508`), and the owner liked the take-off, so
nothing new gets the animal off the ground; what changes is what happens next.
Append **`BrainOutput::Fly = 15`** (`BRAIN_OUTPUTS` 15 → 16), read raw and gated
strictly positive exactly as `Impulse` is, so `squash(0.0) == 0.0`: a species
authoring no weight takes no RNG draw and never flies. `Flight` carries the held
`fly: f32`, and `step_flight`'s gravity line becomes `g_eff = GRAVITY * (1 -
carried) * (1 - fly)`. **Not new physics, and already proved:** §Z9 is an animal at
`g_eff == 0` hanging in the air for ever — the float exists today as a *defect*,
and B1 gives it a price and a rudder.

**How it steers, and which object each rule evaluates.** Once per `tick_interval`
aloft: `sense`, `eval_brain`, rotate `(vx, vy)` by `−turn·π/4` — one octant a
tick, the granularity a walking step turns by, so the two verbs cannot disagree
about what a turn is — then thrust toward a species `flight_speed`. Lift and drag
evaluate **the whole body** (`body_drag` over the chain, as today); steering
evaluates **the velocity vector**, not a candidate cell, and that is the departure
— on the ground `Turn` biases three candidates and the world vetoes two (R4),
where in the air there is nothing to veto; `FoodAdjacent` evaluates **the head's
8-neighbourhood**, unchanged. Air has no horizontal drag today by deliberate
choice; the float needs one, **only while `Fly` is on**, so the ballistic hop stays
byte-identical.

**How it lands — three exits, none a new rule in Rust.** `Fly` falls to zero and
full gravity returns (`landed` fires as today); `(Energy, Fly, −w)` puts the floor
in the genome where selection can move it; `(FoodAdjacent, Fly, −2.0)` settles it
on the flower it is drinking at, while `(FoodAdjacent, Impulse, −2.0)` keeps its
own job on the ground. **And one line closes §Z9:** `landed` when `g_eff <= 0.0 &&
fly == 0.0` — weightless and not flying is standing on water, not flying over it.

**What it costs.** `fly_cost_in_moves: 0.25`, a species field in
`move_cost_per_cell` units like `LAUNCH_COST_IN_MOVES`: **0.0625 J per airborne
frame** for the two-cell flitter on top of pro-rated idle, so **0.0875 J/frame
aloft against 0.025 on the ground**. Derived, not chosen: *flying costs half of
walking, per cell covered* — at `flight_speed: 0.5`, 0.125 J/cell against walking's
0.25 and the ballistic hop's 0.037, **dearer than the hop or nothing lands, cheaper
than walking or nothing flies.**

**Two alternatives, priced and rejected.** *(a) a species field scaling gravity
while `Flight` is `Some`* — ~3 lines, no `brain.rs`, no `mutation_rate` move, every
species bit-identical, but it cannot land on purpose and **does not steer**, so it
fails the measured gap and not merely the ethos. *(b) `Impulse` re-read as the lift
while airborne* — ~10 lines, zero genome-shape change, recycling the launches now
thrown away as `impulses_refused`; rejected on `brain.rs`'s own precedent, made
twice (`Feed` out of `Dig`, `DropSpoil` out of `Drop`), that one output carrying
two probabilities is a gene selection cannot act on. **(b) is the fallback if §3's
blast radius cannot be afforded.**

## 2. Encounter — measured, and it is neither thing the brief guessed

Not a wrong bearing in the air, not `translated_if_free` stopping a cell short:
**there is no bearing in the air at all**, by three mechanisms already in the tree.
`creature.rs:3147` returns into `step_flight` *before* `sense` — no brain, no
`Turn`, no eye, on any airborne frame. `state.heading` is written in exactly four
places (`step_chain`'s chosen candidate `:7113`, `tumble` `:7853`, the flip
`:6999`, a trunk crossing `:7592`) and **`launch` is not one of them** — it takes
the heading the last *walk* left behind (`:3151`, `:3374`). And bug **R4** (OPEN)
has both outer candidates losing at every `Turn` value on level footing, so a
walker "cannot be steered; it can only be scattered."

Paired arms, one binary; `walk` is the same species with `wire=Bias:Impulse:-9.0`
— one weight, no rebuild:

| seeds 1 / 2 / 3 | hop (shipped) | walk (hop off) |
|---|---|---|
| walking steps `moves` | 1,338 / 979 / 301 | 10,488 / 13,074 / 11,593 |
| real launches | 1,903 / 1,641 / 454 | 0 / 0 / 0 |
| airborne frames | 114,671 / 157,373 / 175,611 | 0 / 0 / 0 |
| frames per launch | 60 / 96 / **387** | — |
| `bloom_seen` | 334 / 1,045 / 279 | 492 / 4,914 / 3,650 |
| `flower_visits` | 2 / 7 / 0 | 2 / 4 / **34** |
| born | 0 / 0 / 0 | 0 / 0 / **2** |
| head max rows | 28 / 23 / 31 | 23 / 28 / 34 |
| standing flowers | 43 / 11 / 36 | 39 / 13 / 32 |

At 0.60 walking steps per launch the animal turns about once per two hops and each
hop carries it ~27 uncontrolled cells — at a flower *nine* cells away, a 3x
overshoot rather than a near miss. Founders are 38 / 38 / 32 (alive + died; none
bred), so **29–46% of every animal-frame is aloft and blind** — a lower bound,
since deaths shrink the denominator.

**A second finding falls out of the same table.** A `Chain(2)` arc is ~22 frames
(launch v 2.0, `g_eff` 0.138 at `AIR_DENSITY` 0.08, terminal 1.66), so at 60–387
frames per launch **63–94% of airborne time is not an arc** — §Z9 at a scale the
register does not carry, seed 3 spending 175,611 frames aloft on 454 launches with
28 animals alive. *(The 22 is derived; confirming it is one water-free-bed run, in
B1.)*

**What floating changes, as arithmetic.** A bloom nine cells off is inside the
32-cell eye every tick. Worst-case bearing error 180° at π/4 a tick = **16 frames
to be pointed at it**; nine cells at 0.5 cells/frame = **18 frames**. So **34
frames and 2.1 J — 1.8% of one meal**, against 11–43 standing flowers over a
512-wide bed: one every 12–47 columns, inside a single sight line.

## 3. The economy, re-derived

Two-cell flitter, `tick_interval` 4: idle **0.025 J/frame**, walking step 0.25 J,
launch 1.0 J, float **+0.0625 → 0.0875 J/frame aloft**.

- **Unfed life: 8,000 frames on the ground, 2,286 flying** at `start_energy: 200`
  — at 0.5 cells/frame, **1,143 cells, two crossings of the 512-wide bed on the
  founding grant.** Deliberately generous; the owner asked for flowers to be
  easier to find.
- **Supply is not the constraint.** `nectar_refill: 0.25` per 45-frame tick
  refills a drained flower in 180 frames, so each renews **0.667 J/frame**. At the
  *worst* measured seed (11 flowers) that is **7.3 J/frame** against a flitter
  burning ~0.056 J/frame at half its life aloft — **a bed for ~130 flitters at the
  poor seed, ~500 at the rich one.**
- **The persistence bar, in visits.** Upkeep is 0.47 meals per 1,000 frames, and
  budding needs `reproduce_threshold` 1,100 / 120 = **9.2 flowers** on top inside
  an 8,000-frame life — **~1.6 visits per 1,000 frames per animal**. Today, 7
  visits / 12,000 frames / 38 animals = **0.015, a hundredfold short.**

**Constants this reallocates, and the budget for each.**

- **`mutation_rate`, 6 species files.** `live_slots()` 809 → **846**
  (`BRAIN_OUTPUTS` 15→16 adds `BRAIN_INPUTS` 29 + `BRAIN_HIDDEN` 8), so `3.18/846`
  = **0.0037589**. Mechanical, one line each.
- **Every breeding scene's numbers.** `brain::mutate` draws one `unit_f32` per
  live slot, so the stream diverges from birth 1: this **cannot be
  byte-identical**. Round 28's precedent is deliveries 584→526, births 8→5, and
  the remedy is a seed sweep, never a diff.
- **`sight_fraction` (flitter).** The brain now casts on airborne ticks — 29–46%
  more casts per life than its 2,000-idle-tick derivation assumed. **Re-derive
  after B1 on the measured share**, recording both so the gap stays visible.
- **`start_energy: 200`** becomes 2,286 frames under continuous flight; held for
  B1 so the arm is attributable, first to sweep if founders starve before eating.
- **`(Bias, Impulse, 2.0)`** narrows from *the whole flight* to *the take-off*;
  held, because the owner liked it, and B2 is where it is questioned.
- **`LAUNCH_COST_IN_MOVES`, `idle_cost_per_cell`, `move_cost_per_cell`** do not
  move: `fly_cost_in_moves` is authored in their units, so every ratio survives.

## 4. What the player sees

A pale dot leaves the ground on the hop the owner liked — and then **stops
falling**, slides across the canopy toward a flower head, turning as it goes, and
settles on it; when the bed stops flowering the dots come down and walk. **The card
is movement, not stills** (owner, 2026-09-09): `labgif follow=flitter` over 2,000+
frames, plus the bed with and without flitters at 120,000, `meta` carrying
`flower_visits`, frames-per-launch against the 22-frame arc, `starved_aloft` share
and `born` — a float that never drinks looks like a hop at contact-sheet zoom.

## 5. The guards

- **Every shipped animal bit-identical in behaviour.** No species but the flitter
  authors a `Fly` weight and the strictly-positive gate takes no RNG draw, so
  **the hopper's hop is untouched**; what the owner loses is §3's row 2, every
  *bred* lineage diverging. `ascii`'s non-breeding scenes stay byte-identical
  (1,096 non-timing lines); breeding ones are a seed sweep.
- **Positive control:** a floating flitter with a paying flower nine cells away
  reaches it. It **fails on `main` today** — round 28 followed that animal and it
  "never came closer than nine in 2,000 frames," so the fault is already back and
  already watched red.
- **`starved_aloft` falls from 57–87% to below 20%** — not to zero, since a flying
  animal *should* be able to starve flying and zero means the verb is unused; read
  it beside frames-per-launch against 22. **`flower_visits` ≥10x the hop arm's
  median**, paired on three seeds in one binary, and **`moves` per real launch off
  0.60**. **The eye-price control** (`creature_arena arm=ablate species=flitter`)
  becomes answerable once anything is alive at 24,000 frames; unsigned today, 0
  alive on both arms on all six seeds.

## 6. The owner's third option — a better bed

**The mechanism comes first, and the measurement says why.** §3's bed already
feeds 130–500 flitters, and the animal took **seven visits in 12,000 frames** — a
denser bed multiplies a near-zero encounter rate by a constant and leaves it near
zero, round 28's windfall finding again: more food where nobody goes is not more
food. **It becomes the right lever** after B1, if `flower_visits` rises while
`flower_visits_by` against `head_max_rows` leaves the upper canopy unvisited; the
change is then the thicket (`scrambler`), a **median 60 standing flowers against
`played_bed`'s 16**, in the band the float works in.

## 7. Two build briefs

**B1 — the float** *(creature cluster; do this one)*. **Owns** `creature.rs`,
`brain.rs`, `flitter.ron`, the `mutation_rate` line in five other species files.
**Read first, capped:** this report; `creature.rs` `launch` / `step_flight` /
`buoyant_share` / `:3367`; `brain.rs`'s `Impulse` and `Feed` docs; §Z9; R4.
**Build:** §1 entire, plus `mutation_rate` → 0.0037589
everywhere and **`flight_speed` as a three-way env selector (0.25 / 0.5 / 1.0,
default 0.5) naming the active one** — a does-it-look-right question, so a
selector rather than an argument. **Counters:** `fly_frames`, `fly_j`, `moves` per
real launch. **Positive control:** §5's nine-cell test. **Measurement:** §2's
table re-run, same seeds, one binary, paired against the hop arm, plus one
water-free bed for the §Z9 share. **Card:** `labgif follow=flitter`, 2,000+
frames, §4's `meta` — it must show a dot *crossing* to a flower head and stopping
there. **Cost fork:** if six files of `mutation_rate` and every breeding scene
diverging is too much this round, ship §1's fallback (b) as a knowingly-taken
shortcut.

**B2 — the take-off gate** *(`flitter.ron` only; after B1)*. The same measurement
says the hop is a **net negative for encounter today**: the walk arm out-visits it
on 2 of 3 seeds and is the only arm that bred, because `(Bias, Impulse, 2.0)` is
unconditional and fires on ~2/3 of move ticks — before the animal has walked far
enough to turn. **Build:** move the drive off `Bias` onto the sense — `(Bias,
Impulse, 0.5)`, `(BloomNear, Impulse, 2.0)` — so it takes off *toward* something;
**do not** gate it on `Energy`, `hopper.ron`'s recorded do-not-retry. **Control:**
`flower_visits` and `moves` per launch against B1's arm, three seeds, 18 s a run,
one binary. **Card:** the same follow, showing an animal that launches when it has
somewhere to go. **Cost fork:** none, but a `.ron` is `include_str!`-embedded, so
budget the 18-minute rebuild.

---

## What this contradicts

- **The brief's two candidate causes** — there is no bearing in the air, because
  there is no brain in the air. And **pollinator design §2.5**, *"the flitter can
  afford 2.0 because for it the air is not time away from food, it is the way to
  food"*: the air is where the animal is blind, and switching the hop off is the
  only arm of six that breeds.
- **Round 28's *"reach was the gap and is closed"*** is half right: the walk arm
  reaches 23–34 rows with zero launches, so the hop is not what buys the height.
- **The register's generated index reads `Z9 | closed`** while §Z9's heading says
  *REPRODUCED 2026-09-11, not fixed* — `bugindex.py` keys on `OPEN` in the heading
  and this one lacks it, so a lane reading the index skips a live bug.
- **Nothing in `dead-ends.md` covers a powered or floating flight state** — ten
  greps (`Impulse`, `flight`, `glide`, `hover`, `float`, `buoyan`, `density`,
  `Persist`, `BloomNear`, `sight_range`) return only the hopper's bloom-wiring
  entry and the flitter's `sight_range: 8` entry.

## Rulings this depends on

Ants are not the pollinators; the eye is heritable; hand-authoring is fine for
now; new behaviours ship as default. A mechanism at inherited constants is a
regression — §3 names all six. An outcome is a distribution, never a binary — lift,
speed and landing are graded. There must be a verb delivering something visible:
`Fly`, and the dot crossing the canopy. Stop balancing, start exposing —
`flight_speed` ships as a selector. Movement, not stills, is how animals are seen,
and a one-cell event cannot be judged on a card, which is why every gate here is a
counter beside the picture.
