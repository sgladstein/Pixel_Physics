# Trophallaxis as a brain output — the colony's stomach, specified

*Design of record for the `Share` verb and the `KinNeed` sense, 2026-09-09.
Written the day the owner ruled that trophallaxis is a brain output the
genome can evolve, shipped on, never a rule; built the same day
(`README.md` "Trophallaxis status" carries what shipped and its first
measurement, which is a null with a warning sign — sharing on this bed at
this horizon cost survivors on two seeds of three, inside an eight-fold seed
spread, and undoes the founder stagger it shares down). The §5c composed
Move row is the joint target for this and the hunger wire; §11's sequencing
note was overtaken by events (the hunger lane landed first) and is kept for
the record. One correction to §5d: `hopper.ron` now exists.*


*Implementation spec, 2026-09-09. Written against `origin/claude/colony-economy-design`'s
`Reports/colony-economy-design-2026-09-09.md` §4a, which measures the disease
(52 ants, 52 economies, 46 of 52 dead inside one 500-frame window because every
founder empties the same 200 J grant on the same schedule) and names this as
the cure. The owner has ruled that it ships **as a brain output the genome can
evolve**, wired **on by default** in `ant.ron` — a rule-based always-on
transfer was rejected as hardcoding the colony.*

**Status: buildable as written. Every line number below was read on
`main` at 2026-09-09; re-check anchors after any merge — `src/sim/creature.rs`
is the third most contested file in the repo.**

---

## 0. The shape, in one paragraph

One new verb and one new sense. `Share` fires, the animal hands a graded
amount of its own energy to the neediest living nestmate touching its body,
and the flow only ever runs downhill. `KinNeed` is the largest energy deficit
among those nestmates, so a lineage can find *"give when a sister is hungry
and I am not"* without anybody authoring it — and, because the same number
also reaches `Move`, can find *"go out and look when the colony is hungry"*,
which is demand-driven foraging and is the only anticipation a colony has that
an individual's own stomach cannot supply. Nothing else in the engine changes:
the transfer is live-to-live, so the energy ledger needs no new term, and the
verb never runs under the parallel driver.

---

## 1. `dead-ends.md`, first

Grepped 2026-09-09, per `CLAUDE.md`: `trophallaxis`, `share food`,
`food sharing`, `regurgit`, `social stomach`, `transfer energy`,
`energy transfer`, `donat`, `KinNeed` — **zero hits, all nine.** Nothing here
has been tried and reverted.

One hit that matters and is *not* about this mechanism:

> `src/sim/creature.rs` guards that let their animals breed —
> `the_jaw_allele_decides_what_an_animal_can_cut` and
> `a_maximally_armoured_ant_is_graded_only_when_the_reach_allows_it`; both
> flipped by an unrelated change, 2026-09-06 … Appending `Provision` and `Made`
> took `live_slots` 637 → 706 and the rate with it, and both guards went red …

**That trap is already closed and this append will not re-open it.** Both
guards now set `def.mutation_rate = 0.0` on the test's own species copy
(`src/sim/creature.rs:8587` and `:9036`), so their animals breed clones and
the birth draw no longer moves with `live_slots`. Read both before trusting
this sentence; if either has lost its `mutation_rate = 0.0` line in a merge,
put it back before appending anything.

The *other* half of that entry is live and must be paid: see §3.

---

## 2. `BrainOutput::Share` — the verb

### 2a. The name is free

`grep -rn "\bShare\b" src/ assets/ examples/` returns five hits, every one the
English word "Share" inside a doc comment (`src/app.rs:133`,
`src/worldgen/passes.rs:488`, `src/worldgen/erosion.rs:67`,
`src/sim/update.rs:259`, `examples/creature_probe.rs:489`). No identifier, no
variant, no `.ron` key. Nothing collides.

### 2b. Energy, not crop — and the crop's own doc settles it

`OrganismState::Crop` (`src/sim/organism.rs:4231`) is **single-material and
discrete whole cells**, with `unit` taken as a `min` over everything ingested.
Handing crop contents across would need all four of: the two crops to agree on
material or the recipient's to be empty; a rule for what the `min`-over-`unit`
invariant means when a cell arrives from another stomach; a path for
`creature::carried_meat`'s `worth_in_aux` census that does not mint or hide
meat; and an answer for a 480 J leaf arriving in one lump. That last is the
one that decides it: **a whole cell is a binary, and the ethos' first law is
that an outcome is a distribution.** The report asks for a cliff to become a
draining reserve; a 480 J step function is another cliff.

Energy-direct is graded by construction, is exactly conservative, touches no
material path, and is what trophallaxis physically is — regurgitated liquid,
not a parcel. **Transfer `energy`.**

### 2c. The rule

Placed in `act` (`src/sim/creature.rs:4328`) **immediately after the fight
block and before the ingest block**, and it does **not** `return`. The reason
is the one `Feed`, `DropSpoil` and `Attack` each record when they were split
out: giving food away is not a meal, and forcing a tick to choose between them
re-creates exactly the confusion those splits ended. An animal that shares
still eats, drops and digs on the same tick.

```rust
let share_urge = outputs[O::Share as usize].clamp(0.0, 1.0);
...
// --- share ------------------------------------------------------------
// Gated on the urge *before* the scan, exactly as `Attack` is, and that
// gate is the whole cost of this verb for every species that does not use
// it: `squash(0)` is exactly 0.0 for an unauthored row, so `&&`
// short-circuits, NO RNG DRAW IS TAKEN, and a beetle is bit-identical to
// the tree before this slot existed.
if share_urge > 0.0 && draw.unit_f32() < share_urge {
    let gut = gut_of(world, organism, def);
    if let Some(kin) = neediest_kin(world, organism, (x, y), gut) {
        let mine = world.organism(organism).map_or(0.0, |s| s.energy);
        let theirs = world.organism(kin.id).map_or(0.0, |s| s.energy);
        // **Downhill only. When to give is the ant's; which way it runs is
        // the gradient's** -- the same division the spoil drop already
        // makes ("when to let go is the ant's, where it can lie is the
        // ground's"). Without it a pair can pump energy back and forth and
        // pay the jaw price both ways, which is an allele evolution finds
        // in an afternoon.
        if theirs < mine {
            let amount = SHARE_FRACTION * (mine - theirs);
            if let Some(s) = world.organism_mut(organism) { s.energy -= amount; s.last_share_frame = frame; }
            if let Some(s) = world.organism_mut(kin.id)   { s.energy += amount; s.last_share_frame = frame; }
            did.shares += 1;                       // billed by `creature_tick`
            world.creature_stats.shares += 1;
            world.creature_stats.shared_j += amount as f64;
        }
    }
}
```

`frame` is `world.frame`, read before the two `organism_mut` borrows.

**`SHARE_FRACTION = 0.25`, and it is the only constant this verb adds.** It is
derived rather than chosen, and it is doing three jobs at once:

- **It is the cap.** `0.25 < 0.5`, so after every share
  `donor − recipient = 0.5 × gap > 0`: **the donor is still the richer of the
  two.** No oscillation is possible, and no separate ceiling constant is
  needed.
- **It is the floor.** `donor_after ≥ 0.75 × donor_before > 0` for any live
  recipient (a live organism has `energy > 0`, `apply_creature_energy`
  :6579 kills at `<= 0`). **A share can never kill the donor**, with no
  authored floor to tune.
- **It is the grading.** A full ant beside an empty one hands over 50 J of a
  200 J grant; two ants a few joules apart exchange almost nothing. The
  outcome is a distribution, not a switch.

### 2d. The price — one jaw closure, through the machinery that already exists

`Did` (`:4303`) gains `shares: u32`, and `creature_tick` (`:2628`, the
`let Did { dug, gnaws } = act(...)` line) destructures it and bills it beside
the gnaw bill at `:2645`:

```rust
if shares > 0 && def.dig_cost_in_moves > 0.0 {
    let work = def.move_cost_per_cell * body_cells * def.dig_cost_in_moves * shares as f32;
    spent += work;
    world.energy_ledger.metabolized += work as f64;
    world.creature_stats.share_energy += work as f64;
}
```

**No new species field and no new ledger account.** Trophallaxis is
mandible-to-mandible; `Did::gnaws`' own doc already prices "one closure of the
jaw … the same muscular work whether it cuts or bounces", and the `Attack`
branch routes through the same apparatus for the same stated reason ("what
keeps one apparatus at one price"). A separate `share_energy` **counter** is
kept so `labstats` can attribute it, but the joules book to `metabolized`,
which is already a sink in `EnergyLedger::expected_live_total`
(`src/sim/world.rs:999`).

At `ant.ron`'s numbers this is `0.125 × 2 × 6.0 = **1.5 J** per executed
share`, and it buys a real property for free: **a pointless share is a net
loss.** Breakeven is `0.25 × gap > 1.5`, i.e. **a gap of 6 J, 3% of the
grant** — below that the pair loses more to handling than the transfer moves.
That is the pressure that makes `KinNeed` worth wiring rather than sharing on
`Bias`.

### 2e. The conservation invariant, stated so a test can assert it

Over one executed share, with `W` the world:

```
donor.energy + recipient.energy   is unchanged        (exactly; one f32 subtract, one f32 add)
W.energy_ledger.metabolized       rises by exactly the jaw bill
W.energy_ledger.expected_live_total() - sum(live energies)   is unchanged
```

The transfer is **live stock to live stock**, so it moves no term of the
identity at all. Only the price moves one, and it moves an existing one. This
is the whole reason energy-direct is cheap: `harvested_*`, `stamped`,
`stored_in_meat`, `meat_lost` and `max_standing_meat` are all untouched.

---

## 3. `BrainInput::KinNeed` — the sense

**Definition.** The largest energy deficit among living kin adjacent to any
cell of this animal's body, as `1 − energy / start_energy`, clamped to
`[0, 1]`. **`0.0` when there is no adjacent kin, and `0.0` when every adjacent
kin is full** — and those two zeros mean the same thing behaviourally (nothing
to give to), which is why one slot is honest here where `PreyNear` needed
`PreyBearing` beside it.

**Normalised by the *donor's* `def.start_energy`**, one field read, no species
lookup. Kin is same-species except when `CreatureDef::kin_crosses_kinds` is on
(`src/sim/organism.rs:3473`, off for everything shipped), so this is exact for
every animal in the tree today. Record in the doc comment that a cross-kind
kin would be normalised against the wrong scale, and that the fix if that ever
ships is `world.species.get(s.species)` at the cost of a lookup per kin
neighbour.

### 3a. Where it is computed — reuse the mouth's pass, do not add one

`sense` (`:2967`) currently fills `FoodAdjacent` at `:3065` by calling
`adjacent_food`, which is a one-line wrapper over `adjacent_food_counted`
(`:3701`). That function already walks the deduplicated body ring, already
reads every neighbour cell, and already calls `is_living_kin` on each. **The
kin-need reduction is free inside a branch that is already taken.**

1. `struct FoodScan` (`:3695`) gains a field:
   ```rust
   /// The neediest living nestmate this scan touched -- `BrainInput::KinNeed`
   /// and `BrainOutput::Share`'s recipient, found in the pass the mouth was
   /// making anyway. A sense is not an event: this books nothing, and cannot,
   /// because `sense` holds `&World`.
   kin_need: Option<NeedyKin>,
   ```
   with `struct NeedyKin { deficit: f32, id: u16, x: i32, y: i32 }`.

2. Inside the ring loop, the kin skip is restructured. It reads today:
   ```rust
   if !gut.eats_kin && is_living_kin(world, cell, gut) { continue; }
   ```
   and becomes:
   ```rust
   let owner = cell.organism_id();
   // **Somebody else's, never mine.** `is_living_kin` is true of this
   // animal's OWN body cells -- same species, identical scent -- and the
   // head's 8-neighbourhood always contains the next link of its own chain.
   // Without this test an ant reads its own hunger as `KinNeed` and shares
   // with itself, which conserves energy perfectly and would pass every
   // conservation guard ever written.
   if owner != organism && is_living_kin(world, cell, gut) {
       if let Some(st) = world.organism(owner) {
           let deficit = (1.0 - st.energy / def_start_energy.max(1.0)).clamp(0.0, 1.0);
           // Strictly greater, so ties go to the EARLIER ring position --
           // the same rule `adjacent_food_counted`'s own doc states, so the
           // choice is a function of the neighbourhood and not of the
           // iteration order (`CLAUDE.md`'s tie-order entry).
           if kin_need.is_none_or(|k| deficit > k.deficit) {
               kin_need = Some(NeedyKin { deficit, id: owner, x: nx, y: ny });
           }
       }
       if !gut.eats_kin { continue; }
   }
   ```
   `adjacent_food_counted` needs `start_energy` — pass it as an argument
   rather than threading `def`, so the function keeps taking only what it
   reads.

   **Cost check.** For `eats_kin: false` — every shipped species; `ant.ron`
   does not author the field and it defaults false — `is_living_kin` was
   already called on every neighbour, so the call count is *identical*. Only
   an `eats_kin: true` species pays one extra `is_living_kin` per neighbour,
   and there is none in the tree. Say this in the comment.

3. `sense` at `:3065` switches from `adjacent_food` to
   `adjacent_food_counted`, uses `.best.is_some()` for `FoodAdjacent`
   (unchanged behaviour), discards `.refused` and `.damage` (as it must —
   a sense books nothing), and fills
   `inputs[I::KinNeed] = scan.kin_need.map_or(0.0, |k| k.deficit)`.
   **Net added cost per creature tick: zero reads, one float move.**

4. `fn neediest_kin(world, organism, head, gut) -> Option<NeedyKin>` goes
   beside `nearest_foe` (`:3415`), walking the same deduplicated ring with the
   same skip and the same `owner != organism` test. It is a second walk, paid
   **only on ticks the verb actually fires** — the `Attack`/`nearest_foe`
   pattern exactly, and `nearest_foe`'s own doc already argues why a verb gets
   its own targeted walk rather than reusing the food ranking.

   To keep the eye and the mouth from ever disagreeing about who is needy —
   the standing failure this file records four times — extract the per-cell
   reduction as one `#[inline] fn kin_deficit(world, cell, organism, gut,
   start_energy) -> Option<(f32, u16)>` and call it from **both** sites. One
   predicate, two walks.

### 3b. Should `KinNeed` be gated on being at the nest? **No.**

Four reasons, in order of weight:

1. **It would hardcode the nest into the one mechanism that does not need
   it.** `BrainInput::AtNest` is, in `KinNear`'s own words, "the only sense in
   this suite that *pre-categorises* what it senses — a contact scan for a
   material literally named `nest`, which nothing creature-side can ever
   make". The owner's objection, 2026-09-02, verbatim from
   `assets/species/ancestor.ron`: *"I don't like that we have directly encoded
   there being a nest and dropping food at the nest. It seems designed like we
   were intentionally trying to create that behavior and intentionally trying
   to create ants."* An at-nest gate on trophallaxis is that objection
   arriving on a new verb.
2. **Adjacency already delivers the nest case for free.** Ants aggregate; a
   returning forager meets hungry sisters wherever the colony actually is. The
   generalisation `KinNear`'s doc states — *aggregation makes a place, a place
   makes a gradient* — is exactly what an adjacency-gated sense measures, and
   an at-nest gate would replace an emergent place with a painted one.
3. **`ancestor.ron` has no nest at all** and is `ant.ron` with five
   differences. An at-nest gate would make trophallaxis structurally
   impossible for the species that exists to prove home can be found rather
   than painted.
4. **If a lineage wants the nest gate, it is one weight away.**
   `(AtNest, Share, +w)` is expressible, mutable and unauthored today. Leaving
   it to the genome is the "stop balancing, start exposing" ruling.

---

## 4. The genome append, and what it costs

Both slots are pure appends into reserves that are already 64 wide.
`GENOME_LEN` **does not change** (12,416; both `INPUT_SLOTS` and
`OUTPUT_SLOTS` are 64 against live counts of 26 and 14), not one existing
weight moves, and `GenomeLayout::accepts` (`src/sim/brain.rs:312`) is a name
*prefix* check with `<=` on every dimension — so **every jar on the shelf
still loads.**

| file:line | edit |
|---|---|
| `brain.rs:35` | `BRAIN_INPUTS: 26 -> 27` |
| `brain.rs:55` | `BRAIN_OUTPUTS: 14 -> 15` |
| `brain.rs:251` | `INPUT_NAMES`: append `"KinNeed"` after `"Made"` |
| `brain.rs:254` | `OUTPUT_NAMES`: append `"Share"` after `"Provision"` |
| `brain.rs:761` | `BrainInput`: `KinNeed = 26,` after `Made = 25,` + doc |
| `brain.rs:932` | `BrainOutput`: `Share = 14,` after `Provision = 13,` + doc |
| `brain.rs:1065` | `INPUTS`: append `BrainInput::KinNeed` |
| `brain.rs:1081` | `OUTPUTS`: append `BrainOutput::Share` |

**The mutable surface, which is the part that is not free.**
`live_slots = O·I + H·I + H + O·H + CREATURE_TRAITS`:

```
today   14·26 + 8·26 + 8 + 14·8 + 14 = 706
+input  14·27 + 8·27 + 8 + 14·8 + 14 = 728   (+22 = 14 outputs + 8 hidden)
+output 15·27 + 8·27 + 8 + 15·8 + 14 = 763   (+35 = 27 inputs + 8 hidden)
```

**706 → 763, +8.1%.** So, in the same change, per this file's standing rule
that `mutation_rate` is authored per live slot:

- `brain.rs:1760` — `assert_eq!(live, 706, …)` → **763**, and add a paragraph
  to the append log at `:1793-1846` in the same voice as the six before it.
- `brain.rs:1847` — `assert_eq!(genome_manifest(), 2_611_525_623)` → recompute
  and paste. The manifest hashes the dimensions and the ordered names, so it
  moves on any append by design; that is the backstop working, not a failure.
- **`mutation_rate` re-derived to `3.18 / 763 = 0.0041678`** in
  `assets/species/ant.ron:664`, `ancestor.ron:595` and `beetle.ron:191`. Those
  are the only three files carrying the field; the four ant variant forks
  leave it at serde's `0.0` default and stay there.

**Two honest costs to state in the PR body rather than bury:**

- **Every ant's RNG stream shifts.** Ants get a wired `Share` row, so
  `draw.unit_f32()` is consumed on ticks it never was. Every seed-level
  creature baseline taken before this is void, exactly as `Impulse` and
  `Attack` each were — *the reserve is free, the wiring is not*. Species with
  no `Share` weight take no draw and are bit-identical; §6's ablation test is
  what proves that rather than asserting it.
- **The synapse bill.** `synapse_fraction 0.0000022222222 × start_energy 200 =
  4.44e-4 J per active connection per tick`. Four new wired connections
  (three into `Share`, one into `Move`) cost **1.78e-3 J/tick ≈ 3.6 J over a
  2,000-tick life, 1.8% of the grant**, paid whether or not the ant ever
  shares.

---

## 5. The default wiring for `ant.ron`

Appended to the `instincts:` list (`assets/species/ant.ron:826-1160`), after
the `-- the fight --` block, under a new `-- the colony's stomach --` heading.
`squash(x) = x / (1 + |x|)`, and `Share` is read raw and clamped like `Move`,
`Dig`, `Feed` and `Attack` — never through `unit_scale`.

```ron
(Energy,  Share,  2.5),
(KinNeed, Share,  1.9),
(Bias,    Share, -2.5),
(KinNeed, Move,   1.25),
```

### 5a. Why these three numbers, from `squash`

`s = 2.5·E + 1.9·N − 2.5`, where `E` is `Energy` (own bank / start_energy) and
`N` is `KinNeed`.

| donor `E` | kin `N` | `s` | P(share) | reads as |
|---|---|---|---|---|
| 1.00 | 1.00 | +1.90 | **0.655** | full ant, starving sister — shares on most ticks |
| 1.00 | 0.50 | +0.95 | 0.487 | |
| 1.00 | 0.25 | +0.48 | 0.322 | |
| **1.00** | **0.00** | **0.00** | **0.000** | **nobody hungry beside it — no roll, no walk, no draw** |
| 0.75 | 1.00 | +1.28 | 0.560 | |
| 0.75 | 0.25 | −0.15 | 0.000 | mostly-full ant, slightly-peckish sister — not worth 1.5 J |
| 0.50 | 1.00 | +0.65 | 0.394 | half-full ant still helps a starving one |
| 0.25 | 1.00 | +0.03 | 0.024 | |
| 0.10 | 1.00 | −0.35 | 0.000 | **a starving ant does not give away its last joules** |
| 0.10 | 0.00 | −2.25 | 0.000 | hungry ant beside a full one — never |

**`c = a` exactly, and that equality is derived rather than a taste.** With the
bias cancelling the full-energy term, `KinNeed` is the *only* input that can
open the verb: no hungry nestmate in reach means `s ≤ 0`, `share_urge > 0.0`
is false, and the animal pays one float comparison and never walks the ring.
That is `CLAUDE.md`'s "guard hot-path work at the call site that already has
the data", expressed in the weights instead of in Rust.

Record beside it, per the shared-budget rule: **the pair is calibrated against
each other.** A mutation to `(Energy, Share)` alone breaks the exact
cancellation and gives the lineage a standing share urge. That is evolution
working, not a bug — but the next reader must not "tidy" one of the two.

**What the shipped colony will actually do**, and it is worth predicting
before the run so the run can refute it: a cohort that is uniformly hungry
shares *nothing*, because `E` and `N` move together and `s` stays negative.
Sharing fires when one ant's `E` jumps — i.e. **whoever eats redistributes.**
That is the mechanism the report asks for: the cliff flattens because *income*
is pooled, not because a fixed stock is reshuffled.

### 5b. `(KinNeed, Move, +1.25)` — demand-driven foraging

The owner's addition: the ant must not become an animal that only acts when it
is itself hungry. `KinNeed` is a **colony-level** signal, and it is the only
anticipation available — an individual's stomach cannot say "the colony will
run short", and a full ant beside hungry sisters is precisely the animal that
should be out looking. So the same sense drives the legs.

**Ship `(KinNeed, Move, +1.25)` on top of today's `Move` row and nothing
else.** On the row as it stands (`Bias 2.0`, `FoodAdjacent −1.5`,
`Crowding −0.3`, `Alarm −1.0`, plus the four gated trail units at ±1.5):

| state | `Move` sum | P(move) |
|---|---|---|
| today, any energy, no kin need | 2.00 | 0.667 |
| full ant beside starving kin | 3.25 | **0.765** |
| full ant beside half-fed kin | 2.63 | 0.724 |

A colony in trouble gets measurably more restless, immediately, with **no
constant re-derived** and no existing behaviour disturbed.

### 5c. The composed `Move` row — the joint target with the rest lane

**Do not author `(Energy, Move, w)` in this lane.** Rest is §4b of the economy
report and belongs to the lane that is moving `Move`/`Dig` off `Bias`, and
that change reallocates the whole row: `(FoodAdjacent, Move, −1.5)` was
calibrated against `Bias 2.0` as the *universal* baseline, and the moment
`Bias` becomes only the hungry ant's rate, −1.5 stops a *full* ant dead on
food and it never carries anything home. That is `CLAUDE.md`'s shared-budget
rule arriving as a concrete regression, and it is unaffordable inside this
lane.

What the two lanes should jointly hit, stated here so the coordinator has one
target rather than two opinions:

```ron
(Bias,     Move,  2.0),     // unchanged -- now the STARVING ant's rate, not everyone's
(Energy,   Move, -1.75),    // the rest lane's line
(KinNeed,  Move,  1.25),    // this lane's line
(FoodAdjacent, Move, -1.16),// re-derived from -1.5 by the rest lane, in the same change
```

| state (`E`, `N`, food, alarm) | sum | P(move) | reads as |
|---|---|---|---|
| 0.0, 0 | 2.00 | 0.667 | starving ant forages at today's rate — nothing invented |
| 0.2, 0 | 1.65 | 0.623 | hungry ant forages |
| 1.0, 0 | 0.25 | **0.200** | full ant beside full kin — mostly rests, still wanders |
| 1.0, 1.0 | 1.50 | 0.600 | full ant beside hungry kin — shares, then goes out |
| 0.2, 1.0 | 2.90 | 0.744 | hungry ant *and* hungry colony — the most restless thing in the box |
| 1.0, 0, alarm | −0.75 | 0.000 | **stands its ground** |
| 0.2, 0, on food | 0.49 | 0.329 | lingers on a mouthful, as today |

`Bias 2.0` and `Energy −1.75` are fixed by two endpoints — today's 0.667 at
`E = 0` and a 0.20 resting rate at `E = 1` — so neither is chosen.
`(FoodAdjacent, Move)` is then re-derived to reproduce today's "beside food,
about half the run rate" at the *hungry* ant.

**One thing the coordinator should know rather than discover: the alarm term
is a shift and always was.** At `−1.0` it vetoes a resting ant (0.20 → 0.000)
and merely discounts a starving one (0.744 → 0.692). That allocation is the
right way round — the ants able to respond are the full ones near the nest —
but if the owner wants a true override it is `(Alarm, Move, −2.5)`, and the
price is that a starving forager freezes in a fight it cannot win. Not this
lane's weight to move.

### 5d. Which files get the lines

- **The four `Share`/`Move` lines:** `ant.ron`, `ancestor.ron`, and the four
  appearance forks `ant_block.ron`, `ant_block_shaded.ron`, `ant_long.ron`,
  `ant_wide.ron` — six files. The forks carry their own full `instincts:`
  lists and their headers promise they are "`ant.ron` unchanged" except in
  body plan; letting them drift turns `creature_look`'s appearance comparison
  into a behaviour comparison.
- **`beetle.ron` gets nothing.** A solitary predator that shared with its own
  kind would be a decision this lane has no evidence for, and an unwired row
  is `squash(0) = 0` at a cost of one comparison. That is the genome doing its
  job.
- **Correction to the brief:** there is no `hopper.ron` in this tree (the only
  "hopper" is a word in a comment at `creature.rs:5973`), and there are four
  ant variant forks, not five — `ancestor.ron` is the fifth ant-shaped file
  but is a distinct species with its own five documented differences.

### 5e. `plainspeak`

`src/lab/plainspeak.rs`, `phrasebook` (`:100-556`), new block:

```rust
// -- the colony's stomach.
(I::KinNeed, O::Share) => ("FEEDS HUNGRY NESTMATES", "IGNORES HUNGRY KIN"),
(I::Energy,  O::Share) => ("SHARES WHEN WELL FED",   "SHARES WHEN HUNGRY"),
(I::Bias,    O::Share) => ("SHARES WITH ANYONE",     "KEEPS FOOD TO ITSELF"),
(I::KinNeed, O::Move)  => ("GOES OUT WHEN KIN HUNGER","SITS WHILE KIN GO HUNGRY"),
```

Widths 22 / 18 / 20 / 18 / 24 / 24 / 24 / 24, all inside `PHRASE_COLUMNS = 26`.

**The two exhaustive width guards are safe and were checked rather than
assumed.** `every_generic_pair_fits_the_column` walks `INPUTS × OUTPUTS`
building `"{INPUT} > {OUTPUT}"`: the new worst cases are
`SURFACECURVATURE > SHARE` (16 + 3 + 5 = **24**) and
`KINNEED > DROPSPOIL` (7 + 3 + 9 = **19**), both under 26. The current worst
is already 26 exactly (`SURFACECURVATURE > DROPSPOIL`), so this append does
not move it.

---

## 6. Counters

`CreatureStats` (`src/sim/world.rs:390`), appended at the end of the struct:

```rust
/// **Executed transfers** -- the "did it fire at all" counter. A share that
/// was rolled and found nobody, and a share that moved joules, are the same
/// silence in every other readout.
pub shares: u64,
/// **Joules actually moved** -- the effect counter from the far side of the
/// call, and `CLAUDE.md` asks for it by name. `shares` can climb with
/// `shared_j` near zero if every gap is trivial, which is a colony grooming
/// itself rather than feeding itself, and only the pair separates them.
pub shared_j: f64,
/// What the handling cost, booked into `metabolized`. `shared_j /
/// share_energy` is whether the verb is paying for itself.
pub share_energy: f64,
```

**Do not add a per-individual counter to `LifeCounters`.**
`every_lifetime_counter_closes_against_its_world_total`
(`src/sim/creature.rs:7090`) requires every `state.life` field to close against
its world total across live *and* dead animals; a share credits a *second*
animal, so the obvious per-individual field would need a matching
`received` term and a rule for a recipient that dies. Out of scope, and the
test is right to demand it.

`examples/labstats.rs`, beside the `--- biting ---` line at `:602`:

```rust
println!(
    "--- the colony's stomach --- shares {} | shared {:.0} J | handling {:.1} J ({:.1}% of burn) | J per share {:.1}",
    st.shares, st.shared_j, st.share_energy, share(st.share_energy),
    if st.shares > 0 { st.shared_j / st.shares as f64 } else { 0.0 },
);
```

`examples/labforage.rs`: add `shares={} shared_j={:.0}` to the `SUMMARY` line
at `:337`. Without it the population arm in §7 is a picture with no body count
beside it, which is the exact failure `CLAUDE.md` opens with.

**No `LogKind`.** The run log is another lane's; every counter above is a
scalar on `CreatureStats` and nothing here touches `World::run_log`.

---

## 7. Visibility

**The ethos requires a mark.** A share moves an invisible scalar between two
invisible scalars — without one, the whole feature is unobservable on screen
forever, and *"if an event produces no visible consequence … it is not
finished regardless of what the simulation believes"*.

**The cheapest thing that works, carried by the marker pass another lane is
already adding to `Ui::draw` (`src/lab/ui.rs:6245`), not a new render path:**

- `OrganismState` gains `last_share_frame: u64` (0 = never), written on
  **both** parties at the moment of transfer. One `u64` on a struct with a
  4,095-organism ceiling is ~32 KB; nothing in the sim reads it.
- The per-animal marker pass draws **one full-replace pixel at the animal's
  own head** while `world.frame - last_share_frame < SHARE_FLASH_FRAMES`,
  through the same `renderer.world_rect_to_screen` + `render::put` the pinned
  marker uses at `:6297`. Two adjacent heads both lit is the pair meeting,
  which is what a player sees; a midpoint would read better and a *per-animal*
  pass does not have the pair, so the per-animal form is the right one.
- **A fixed colour, full replace, never a blend** — `ui.rs`'s own note at
  `:6270` and `CLAUDE.md`'s overlay rule. A warm amber, distinct from
  `MARKER`, so a flash is not mistaken for the pin.
- `SHARE_FLASH_FRAMES = 12` — two ant ticks at `tick_interval 6`, ~0.2 s at
  1x. **State the limit honestly:** it is measured in *simulated* frames, so
  above about 4x on the lab's speed dial the mark is gone before a frame is
  drawn. That is acceptable — above 4x the box is being watched in aggregate —
  and if the marker lane wants a minimum in *drawn* frames it owns that,
  because only it knows the draw cadence.
- **Frame cost:** one `render::put` per flashing animal per drawn frame,
  inside the lab's chrome pass, which repaints every frame regardless. No
  simulation work, no new pass, and it cannot cost the dirty-rect skip. That
  last clause is a claim, not a measurement — the marker lane should confirm
  it with `lab_cost` before the PR body repeats it.

---

## 8. Tests, each with the fault it catches

All in `src/sim/creature.rs`'s test module unless stated. `colony_bed()`
(`:7036`) is the existing two-ant-capable scene.

| test | fault it catches | how to watch it go red |
|---|---|---|
| `a_share_moves_energy_and_the_pair_keeps_its_total` | a transfer that mints or destroys joules — crediting the recipient without debiting the donor, or booking to `harvested_plant`. Assert `donor_after + recipient_after == before_sum` exactly, and that `expected_live_total() − sum(live energies)` is unchanged | credit `amount * 1.1` |
| `energy_only_flows_uphill_never` | a symmetric or uphill transfer. Recipient **richer** than the donor, maximal `Share` weight, run: assert `shares == 0` and both energies untouched by the verb | delete the `theirs < mine` test |
| `a_share_leaves_the_donor_the_richer_of_the_two` | a `SHARE_FRACTION > 0.5` that lets a pair oscillate and pay the jaw price both ways for ever | set `SHARE_FRACTION = 0.75` |
| `an_animal_cannot_share_with_itself` | the one **conservation cannot see**: without `owner != organism` an ant hands energy to itself, which balances perfectly. A lone ant, maximal weight, no neighbours → `shares == 0`, `KinNeed == 0.0` | drop the `owner != organism` test — `is_living_kin` is true of the animal's own chain |
| `nothing_is_shared_across_species` | a kin test that is really a liveness test. **Two arms in one test**: ant beside a beetle → `shares == 0`; the identical scene with the beetle replaced by an ant → `shares > 0`. The second arm is the positive control, and without it the first is a test of the scene being empty | see below |
| `a_share_across_a_chunk_seam_is_the_same_under_both_drivers` | any future move of the transfer into the CA sweep. Two ants astride `x = 63/64` (`CHUNK_SIZE = 64`), on a stone floor in still air so the sweep has nothing to move; run once with `update::step` + `step_active_sites` and once with `parallel::step` + `step_active_sites`; assert identical `shares` and `shared_j` | route the transfer through a queued `World::set` |
| `an_unwired_species_is_unaffected_by_the_ablation` (+ its sensitivity twin) | the missing `share_urge > 0.0` short-circuit, which would take an RNG draw for every animal in the world and silently change every existing behaviour. A worm/beetle bed hashed under `PIXEL_PHYSICS_TROPHALLAXIS` on and off must be **identical**; the ant bed must **differ** | delete the `> 0.0` guard so the draw is unconditional |
| `tests/determinism.rs` — extend `lab_hash` (`:287`) with `w.creature_stats.shares` | a share whose recipient is chosen by `HashMap` order rather than by ring order | reorder the ring |

**The `is_living_kin` fault, put back by hand.** The brief asks for
`is_living_kin` returning `true` for all, and that cannot live in the suite —
it is a one-line edit to a function four other call sites depend on. Do it
once, by hand, before the PR: make it `fn is_living_kin(..) -> bool { true }`,
run `nothing_is_shared_across_species`, confirm the beetle arm goes **red**,
revert, and put the result in the commit message. `CLAUDE.md` requires this
whenever a guard's green is going to be cited, and it also proves the
*specificity* half that the two-arm form cannot.

**Vacuity, and it needs measuring before it is asserted.** Do not write
`shares > 0` into `tests/determinism.rs` on faith — at the shipped weights a
*synchronised* cohort shares nothing (§5a), so the lab bed's `shares` at 2,400
frames depends on some ant eating and then standing next to a sister. Run
`labstats frames=2400` first, read the new `--- the colony's stomach ---`
line, and only assert a bar with headroom below the measured value; if it is
near zero, put the vacuity guard on the constructed pair instead and say so.

---

## 9. The population claim — the positive control, and how to run it

**Build the ablation switch as part of the change**, following
`spoil_kept` / `curvature_sense_enabled` (`creature.rs:4970-5000`):
`PIXEL_PHYSICS_TROPHALLAXIS=off` pins `share_urge` to `0.0` with the weights
still in the genome, read once through a `OnceLock`. `CLAUDE.md` is explicit
about why this and not two builds: two arms inside one binary are immune to
the stale-binary failure, and to load-dependent counter drift.

**The arm to run**, matching the report's own conditions
(`RAYON_NUM_THREADS=1`, default `LabBox`, seeds 1/2/3):

```
cargo build --release --examples          # with `set -o pipefail`; read ${PIPESTATUS[0]}
RAYON_NUM_THREADS=1 ./target/release/examples/labforage frames=6000 seed=1
RAYON_NUM_THREADS=1 PIXEL_PHYSICS_TROPHALLAXIS=off ./target/release/examples/labforage frames=6000 seed=1
```

**The number to read is not "ants alive".** It is the *shape* of the cliff:
`alive` at frame 3,500 against `alive` at 4,500. The baseline, from the
report's §1, is **50 → 6 — 88% of the colony inside 1,000 frames.** The claim
this feature makes is that the drop between those two samples shrinks, not
that the final count is larger; a colony that thins steadily to the same
number has done exactly what §4a asks for.

Report per seed, both arms: `alive@3500`, `alive@4500`, the ratio, and
**`shares` and `shared_j` beside them** — a flatter curve with `shares = 0` is
the ablation switch not working, and looks identical to a result.

Secondary, cheap, and worth having: `creature_arena` can race the wiring
directly with `arm=wire wire=KinNeed:Share:1.9,Energy:Share:2.5,Bias:Share:-2.5`
against `arm=same`, at **24,000 frames or more** — the arena's own recorded
finding is that a 9,000-frame window is shorter than the ant's 12,000-frame
founding grant, inside which not spending beats everything.

---

## 10. File-by-file, with sizes

| file | edits | ~lines |
|---|---|---|
| `src/sim/brain.rs` | 2 consts, 2 name tables, 2 enum variants + docs, 2 slot arrays, `live_slots` 706→763, manifest literal, append-log paragraph | 90 (mostly doc) |
| `src/sim/creature.rs` | `NeedyKin` + `kin_deficit` + `neediest_kin`; `FoodScan.kin_need`; the ring's kin branch; `sense` `KinNeed`; `Did.shares`; the `Share` branch in `act`; the jaw bill in `creature_tick`; `SHARE_FRACTION`; the env switch | 190 (about half doc) |
| `src/sim/world.rs` | 3 `CreatureStats` fields + docs | 20 |
| `src/sim/organism.rs` | `OrganismState.last_share_frame` | 8 |
| `src/lab/plainspeak.rs` | 4 phrasebook rows | 6 |
| `src/lab/ui.rs` | the flash, inside the marker lane's pass | 15 |
| `assets/species/*.ron` | 4 instinct lines × 6 files; `mutation_rate` × 3 files | 60 (mostly comment) |
| `examples/labstats.rs`, `examples/labforage.rs` | one print line each | 12 |
| tests | 7 new + 1 extension in `tests/determinism.rs` | 260 |
| `wiki/ants.md` | trophallaxis in plain language + a real dated freshness note | 20 |
| `Reports/README.md` | a line for the economy report if it lands with this | 1 |

**~680 lines, roughly 45% comment. One session.**

### Gates

```
cargo build --release --examples          # set -o pipefail; ${PIPESTATUS[0]}
cargo clippy --all-targets --release --locked -- -D warnings
cargo test --lib creature
cargo test --lib brain
cargo test --lib organism
cargo test --lib plainspeak
cargo test --test determinism
cargo run --release --example ascii       # worst-frame timing; CI runs it
bash scripts/docscheck.sh                 # after every merge, unconditionally
```

`bash scripts/acceptance.sh` is the **structural** acceptance suite and does
not cover creatures; run it anyway before the PR (it is cheap and CI gates it)
but do not cite it as evidence about this change.

`bash scripts/seedsweep.sh` is **not** required: this changes no model over
procedurally generated content. The seed variation that matters here is over
`labforage` seeds, and §9 is that sweep.

---

## 11. Concurrency note for the coordinator

**`src/sim/creature.rs` regions this lane writes:**

| region | what |
|---|---|
| ~2628–2650 | `let Did { dug, gnaws, shares }` and the jaw bill |
| ~3060–3070 | `sense`: `adjacent_food` → `adjacent_food_counted`, `KinNeed` fill |
| ~3346–3350 | `SHARE_FRACTION` beside `EAT_YIELD_THRESHOLD` |
| ~3415–3465 | `neediest_kin` + `kin_deficit`, beside `nearest_foe` |
| ~3695–3800 | `FoodScan.kin_need` and the ring's kin branch |
| ~4300–4310 | `Did.shares` |
| ~4415–4445 | the `Share` branch in `act` |
| ~4970–5000 | the env switch, beside `spoil_kept` |

**Against the other lane's declared regions:**

- **`:1270` — no overlap.** Not touched.
- **`:2001–:2140` (`try_bud`) — no overlap.** Not touched. Worth saying out
  loud: this lane deliberately does *not* let a share pay for a child
  differently than a mouthful does, so `try_bud` needs nothing.
- **`:2612` — one line of overlap risk.** This lane's nearest edit is
  `:2628`, sixteen lines below. The two are almost certainly separable, but
  they are inside the same `creature_tick` billing block and a merge will put
  them in the same hunk. **Land whichever is smaller first; the other rebases
  by hand rather than by `git`.**

**`assets/species/ant.ron` — the real collision.** The other lane rewrites the
`instincts:` list (moving `Move`/`Dig` onto `Energy`); this lane appends four
lines to the same list and changes `mutation_rate` at `:664`. That is a
textual conflict in a contested file by construction.

**Recommended sequence, and the reason is not tidiness:**

1. **Trophallaxis lands first.** It is the smaller ant.ron diff (append-only,
   plus one number), and it deliberately does *not* touch the `Move` row's
   existing weights — so the instinct-rewrite lane rebases onto a list with
   four extra lines at the bottom and nothing moved.
2. **The rest lane then lands `(Energy, Move, −1.75)` and the re-derived
   `(FoodAdjacent, Move, −1.16)` together**, hitting §5c's composed table.
   Its acceptance bar should include the four rows of that table, because
   `(KinNeed, Move, +1.25)` will already be sitting in the same sum.

**Semantic interaction, so neither lane is surprised:** `eval_brain` gives
every output its own row, so `Energy` feeding both `Move` and `Share` costs
nothing shared. What *is* shared is the synapse bill (`active` connections ×
`synapse_fraction × start_energy`), which both lanes raise. Between them they
add roughly 2% of an ant's grant in thinking; neither should re-derive
`synapse_fraction` for it, but the PR bodies should each state their share so
the two do not read as one unexplained regression.

**And re-list the branches before writing into either file** — the ownership
split above is a claim about 2026-09-09, not evidence about who is running
now.
