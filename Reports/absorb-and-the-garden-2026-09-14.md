# Absorbing, the speed dial, and what actually eats the garden

*2026-09-14. The stated cause is disproved by measurement; the real chain is
measured and named. The owner's **observation** is reproduced and is real.
Lane note: [`Reports/lanes/absorb-destroys-plants.md`](lanes/absorb-destroys-plants.md).*

Owner report, verbatim: *"Absorbing creature energy, destroys plants around
it. The energy particles need to be foreground and not interact with the
world."*

---

## 1. The short answer

**The `F` key does not touch the world.** Five paired arms — same world, same
seed, same elapsed time, differing only in whether `F` is pressed — came back
**equal in every column**: every plant material, every organism count, every
death cause, and every unit of `harvested_plant`. The only thing in the entire
world that differed was `Druid::power`.

**The proposed fix is therefore already true in both of its senses**, and
shipping it would be a no-op with the owner told his bug was addressed. The
motes are `Mote { x, y, bright }` in **screen pixels**, painted by
`render::put` straight into the finished RGBA frame (`hud.rs::motes`, and
`hc.put` at `hud.rs:396` onward) — they cannot touch the simulation, and being
painted after the world *is* foreground. `Druid::absorb` writes exactly three
things: `self.reserves` (the druid's own bookkeeping mirror, not the animal's
gut), `self.draws` (read only by the HUD and by the income readout), and
`self.power`.

**His observation is nonetheless correct, and it is worth saying that first.**
A circle with a colony in it really is chewed back to bare ground. What does
it is **ants grazing inside a quickening, multiplied by the speed dial** — and
absorbing is *upstream* of that rather than the mechanism of it. Measured on
the shipped `bare` start, 3,000 ticks, speed 8, one circle, the only
difference being whether a colony stands in it:

| | no colony | 33 ants |
|---|---|---|
| live plants at end | **352** | **177** |
| plant cells | 5,811 | 5,163 |
| plant energy eaten | 0 | **15,010** |

**The colony eats half the garden.** Card `20260914T043140244Z-b56729` is
those two frames side by side; the difference is obvious by eye.

---

## 2. Why absorbing still gets the blame, and it is not the player being silly

Three separate couplings tie the `F` key to the destruction without `F` ever
writing a cell. Any one of them is enough to make the two look like cause and
effect from the chair.

1. **You have to stand in the colony to press it.** `ABSORB_RADIUS` is 60
   cells, and an animal only accumulates charge where `time_runs_at` is true.
   So a productive absorb *requires* the configuration that eats the garden —
   a colony inside running time, next to plants.
2. **Absorbing pays for the circle to keep standing.** With the economy live,
   the same world and the same 2,000 ticks, the only difference being six
   presses of `F` worth 45 units of power:

   | | quiet | absorb |
   |---|---|---|
   | plant energy eaten | 1,026 | **1,260** |
   | plants felled | 122 | **126** |
   | animals at end | 1 | 2 |
   | power at end | 0 | 22 |

   Both arms ran out of power and closed their circle; the absorbing one
   closed it later. **That is a real effect of absorbing on the plants, and it
   is the whole of it** — 234 more units of plant eaten, bought with 45 units
   of power, entirely through how long time was allowed to run.
3. **The speed dial multiplies it about tenfold.** Same colony, same world,
   only the dial moved.

---

## 3. The instrument, and why a new one was needed

`examples/druid_garden.rs`, new on this branch, with its row in
`Reports/instruments.md`.

Nothing in the repo could take this measurement.
`PIXEL_PHYSICS_DRUID_CENSUS` counts tissue inside each standing circle and
exits — it has no *before*, and it is blind to a plant that was killed and
freed. `flora_census` builds its own world and never holds it. `latecensus`
reads a `LabBox`. So `CLAUDE.md`'s own instruction for this exact question —
*"if the question is 'how much did this eat', census the materials before and
after"* — could not be followed at all.

It reports, before and after one run of the same world: plant-kind **cells by
material**, world-wide, inside a 200-cell window on the colony, and inside the
60-cell absorb radius; live plant and animal **organisms**;
`World::deaths_by_cause`; and `energy_ledger.harvested_plant`.

**`harvested_plant` is the finding that made the diagnosis tractable**, and it
was already in the engine. A death count cannot answer "did the colony eat the
garden", because a grazed plant usually survives being grazed — eating and
dying are different events and only one of them is grazing. This counter is
the far side of the call and it is load-independent.

### The controls, both directions

`CLAUDE.md` asks for a case known to be fine *and* a case known to be broken,
and the nulls in §1 are worthless without both.

- **Positive control on the instrument.** `control=selftest` erases 100 plant
  cells from a grown world and asserts the census reports exactly 100. It does.
- **Positive control on the *question*.** The economy-live pair in §2.2 is the
  arm where absorbing *can* affect the world, and the same harness sees it:
  1,026 → 1,260 eaten. **So the nulls are not the harness being unable to see
  an absorb effect** — it sees one the moment there is one to see.
- **Sensitivity across the range.** The same census reports 228 units eaten
  (1 ant, speed 1), 2,217 (1 ant, speed 8), 15,010 (33 ants, speed 8) and 0
  (no colony). It is not stuck.

**The pairs being bit-identical is exactly the "tidy first result" this repo
warns about**, and it is the expected result here rather than a suspicious
one: the sim is deterministic same-build, absorb provably writes nothing the
sim reads, so identity is the prediction and the three controls above are what
license reading it as one.

---

## 4. The arms and their numbers

All on the `druid` preset, seed 1, `1024x512` (the full-scale confirmation
is §5). `unlimited=1` unless stated, so the economy cannot silently close a
circle and make "no absorb" secretly mean "less world running" — which is the
confound the whole measurement exists to remove.

### 4a. Arm 1 vs 2 — does `F` do anything at all

`START=grown`, 2,000 ticks, one circle, a colony at the player's feet.

| | speed 1 quiet | speed 1 absorb | speed 8 quiet | speed 8 absorb |
|---|---|---|---|---|
| plant cells (world) | 22,983 | 22,983 | 23,156 | 23,156 |
| within r60 | 6,416 | 6,416 | 6,900 | 6,900 |
| live plants | 1,021 | 1,021 | 1,181 | 1,181 |
| plant energy eaten | 228 | 228 | 2,217 | 2,217 |
| felled | 28 | 28 | 274 | 274 |
| starved | 0 | 0 | 59 | 59 |
| absorbs fired | 0 | **6** | 0 | **6** |
| energy drawn | 0 | **45** | 0 | **47** |
| power at end | 600 | **645** | 600 | **647** |

Repeated on `START=bare` with 32 ants and 10 absorbs drawing 708: identical
again in every column.

### 4b. Arm 3 — the speed dial, everything else fixed

Same world, same colony, same elapsed player time.

| | speed 1 | speed 8 | ratio |
|---|---|---|---|
| plant energy eaten | 228 | 2,217 | **9.7x** |
| felled | 28 | 274 | **9.8x** |
| starved | 0 | 59 | — |

On `bare` with 32 ants: eaten **1,112 → 15,010**, felled 42 → 335.

**This is the dial doing what it is documented to do.** `step_extra_ticks`
runs `frame::step` `speed - 1` extra times, and on a held world that is the
whole world's physics restricted to wherever time runs — so everything inside
a circle has eight times as much life happen to it, grazing included.
`Reports/dead-ends.md` already records the clock half of this (the withdrawn
per-circle rate, 2026-09-13: *"every organism's cadence is `frame + interval`
in the **global** counter"*). **The grazing half is new here.**

### 4c. Arm 4 — is it the ants, or the dial

`bare`, 3,000 ticks, speed 8.

| | no colony | 33 ants |
|---|---|---|
| live plants at end | 352 | **177** |
| plants near | 328 | **152** |
| plant cells | 5,811 | 5,163 |
| eaten | 0 | 15,010 |
| felled | 273 | 335 |

**Both.** The dial sets the rate; the colony is what converts that rate into
tissue leaving. Note that felling (273) is nearly as high *without* a colony:
most of what `deaths_by_cause` calls FELLED is seed-bank churn, not the
garden — which is why the cell and live-plant counts, not the death count, are
the ones to read here. (`FelledOrLost` is assigned at `free_organism` to any
plant arriving with no cells and no declared cause — `world.rs:6042`.)

### 4d. Arm 5 — `plant_bending`, ruled out by measurement

The brief flagged it as unchecked: `Druid::new` sets
`world.plant_load_failure = false` but leaves `plant_bending` at its `true`
default. Turned off, speed 8, everything else fixed: felled **269 against
274**, eaten 3,197 against 2,217 (the wrong direction, and inside this
scene's spread). **Bending is not it.**

---

## 5. Full scale

The numbers above are `1024x512`; the shipped world is `2560x960`. The
absorb null reproduces there — `START=grown`, 4,000 ticks, speed 1, 13
absorbs drawing 195 units: **every column equal** (plant cells
181,171 → 181,281 in both arms, felled 67 in both), with power 600 against
795 the only difference in the world. A `bare` full-scale sweep at speed 8 was
running when this was written; `scratchpad/arms-fullbare.txt` on the box, and
§8 records whether it landed.

---

## 6. Two things found on the way that are not this bug

- **A founding in a grown wood places 2 of its 12 ants; on bare ground it
  places 32 of 48.** `colony_stations` drops every station whose column has no
  `colony_ant_site` at the founder's own height, and a wood's ground is under
  its own litter and roots. Nothing reports this but the count in the log line,
  and *"founded 2 animals"* after a key meant to give you a colony is the same
  silent-shortfall shape `found_colony_of`'s own doc opens with. `creature.rs`
  / `druid/mod.rs` — not this lane's.
- **`step_extra_ticks` taking the player out is clean**, contrary to the
  brief's suspicion that it deserved a look in its own right. `frame::step`
  sets `world.carried = None` when there is no player and `player::step`
  returns immediately; no cell is written and no stale body is left behind.
  The only consequence is the documented one — the carried circle does not run
  during the extra passes.

---

## 7. The prescription

**There is no bug to patch in `absorb` and nothing to fix in the motes.** The
defect is that a real, large, ongoing cost is invisible and its apparent cause
is the wrong one. Two changes, neither in this lane's files.

### 7a. `src/druid/hud.rs` (Lane A) — say what the circle is costing in tissue

The readout says `CHARGE 266 IN 10 NEAR YOU` and names power, drain and
income. It says nothing about the garden. Add one line to the readout, beside
the drain:

```
EATEN 15010   PLANTS 352 -> 177
```

sourced from `World::energy_ledger.harvested_plant` and a live plant count —
both already on the world, neither needing a new pass. **This is the ethos
clause literally**: *a consequence with no visible cause is unfinished*. The
player currently has a number for what the circle costs him in power and no
number at all for what it costs him in plants, which is the thing he actually
minds.

### 7b. `src/druid/mod.rs` (Lane B) — the dial is a destruction multiplier and does not say so

`Druid::speed`'s own doc already records *"a plant inside a rate-4 circle is
having four times as much life happen to it"*, and `drain_for` prices that
honestly in power. What is not priced anywhere the player can see is that the
same multiplier applies to **grazing**. One line in the note raised when the
dial is changed — `self.note(format!("time runs x{} - and so does everything
eating"))` or similar — closes the gap for the cost of a string.

**No change is recommended to the dial itself, and the grazing is not a bug.**
*An outcome is a distribution, not a binary*: the colony eating the garden at a
rate the player set is graded, legible and reversible, and it is the game
working. What fails the ethos is only that the player cannot see it happening
or attribute it. Whether a colony *should* be able to halve a garden is a
balance question for the owner and is not mine to answer.

---

## 8. Where this brief was wrong, and where it was right

The coordinator asked to be told plainly.

**Right, and verified independently here:** `absorb` touches nothing but
`reserves`/`draws`/`power`; the motes are pure HUD in screen pixels; the
particles are *already* foreground and non-interacting; `plant_load_failure`
is false so stress-breaking is not it; and a new harness under `examples/` was
the right place — `bin/druid.rs` could not have taken a *paired* census
anyway, since it exits at the census.

**The leading hypothesis was half right.** *"Circles at speed > 1 run the
world N times → plants near the colony have N times as much life happen to
them, including being eaten"* — that is exactly what the numbers say, 9.7x.
But the brief's framing of it as **"absorbing funds the power that keeps
circles alive"** as the *chain* understates the case: with the economy on it
contributes (§2.2, +234 eaten), and with power abundant it contributes
**nothing at all**, yet the plants still die. So funding is the *smaller*
coupling. The larger one is that **you must stand in a running colony to
press `F`**, which the brief did not name.

**Wrong, but harmlessly:** `step_extra_ticks`'s `world.player.take()` was
flagged as "worth a look in its own right". It is clean — §6.

**Wrong, and worth correcting:** *"None of them census plants"* said of
`bin/druid.rs`'s hooks — `PIXEL_PHYSICS_DRUID_CENSUS` does census living plant
tissue per circle. It is still the wrong instrument, for a different reason
(no *before*, and blind to a freed organism), and the conclusion stands.

**Not answerable as posed:** *"If the destruction tracks the SPEED DIAL rather
than the `F` keypress, it is `step_extra_ticks` and/or grazing."* It tracks
the dial, and it is **both**, and they are not alternatives — the dial is the
rate and the grazing is the mechanism. Separating them needed the two
instruments to be read together, which is why `harvested_plant` is in the
harness beside the death counts.

---

## 9. What is not settled

- **My reproduction is not certainly his.** On a *grown* world nothing is
  destroyed at all on net — plant cells rise at every speed. The destruction
  is visible on the shipped `bare` start, where the garden is small enough for
  a colony to matter. If the owner was playing `grown`, or saw something else
  entirely, the card asks him directly and says so in as many words.
- **No arm sweeps seeds.** Everything here is seed 1. Outcomes in this engine
  are chaotic in the seed and the ratios above are single samples; the null is
  a determinism argument and does not need a sweep, but **the 2x on the
  garden does** before anything is tuned on it.
