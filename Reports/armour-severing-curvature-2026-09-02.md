# Armour, severing, and a curvature channel — what shipped and what did not

**2026-09-02.** Executing `Reports/creature-genome-flexibility-2026-09-02.md`
§11 (armour and severing) and §5f (a curvature sense), with §14g–§14i's
lessons treated as preconditions rather than background.

Three of the five findings here are nulls or reversals, and they are the
useful part. The report is ordered by what a later session needs first.

---

## 0. The findings, stated once

| | |
|---|---|
| **The arena bed rewards the authored ant — but only above the endowment** | 12 of 12 seeds at `frames=18000`. At the shipped default of 9,000 the *random* genome wins 8 of 12, because the founding grant lasts 12,000 frames and an arm that does nothing cannot starve inside the run. The harness prints this warning itself. |
| **Armour ships and works** | `eats` 1008 → 1089, `harvested_corpse` non-zero, energy census still closes. The positive control passed, which is the only control that could have caught the trap. |
| **Severing ships and has never fired** | 0 severings in every shipped scene, against 20 injuries in `predation_probe`. The ant is a two-cell chain and a two-cell body has no middle. §11e predicted exactly this. |
| **The curvature sensor is dead in one bed and alive in the other** | Spread 0.000 over 18,720 samples in `LabBox`; 1.083 on a worked bank. |
| **The curvature lever works, at three times the weight §5f proposed** | At the authored 0.169 it is a null needing ~69 seeds. At 0.5, rounder in 10 of 12 and smaller in 12 of 12 against a **mean-matched** control. |

---

## 1. The rider that everything downstream leans on

*"This bed rewards the authored instinct over noise"* is the claim the whole
creature line is built on, and it had been checked at six seeds.

`creature_arena arm=random`, twelve seeds:

| horizon | arm B (random) share | seeds where random wins |
|---|---|---|
| **9,000 (the shipped default)** | median **60.5%** | **8 of 12** |
| 18,000 | median 0.0% | **0 of 12** |

The claim holds — and only above the endowment. The harness already knew:

> *THE HORIZON IS SHORTER THAN THE ENDOWMENT. An arm that does nothing cannot
> starve inside this run, so it cannot lose, and a negative control that
> cannot lose is not one.*

It prints that and then prints the table anyway. **Anyone who ran the default
got the opposite answer to the truth**, and the warning is one line above the
number that contradicts it. The general form: a guard that prints and does not
stop is a guard the reader has to remember to obey.

---

## 2. Armour (§11 R1)

`bite_force` on `CreatureDef` (`Option<f32>`, defaulting to `dig_force`), the
food table authored, and the gate — one atomic change, because either half
alone is a bug.

**The gate lives in `Gut`, not at the bite.** Gating at the bite site is the
obvious placement and builds a starvation trap: `adjacent_food` returns the
*best* mouthful in reach, so an ant between an armoured nestmate and a leaf
would be offered the nestmate every tick, bounce, and never take the leaf.
Filtering in the gut makes armoured flesh simply not food to a mouth that
cannot open it — the shape `eats_kin` and `gut_bias` already have.

**The table is derived from the shipped forces.** Weakest mouth in the world
is `beetle.ron`'s `dig_force: 0.3`; strongest is the ant family's 1.0.
Everything a shipped animal must eat is priced below 0.3, armour between 0.3
and 1.0, nothing food-bearing above 1.0.

```
0.05  deadleaf, litter, windfall      dead, half-rotted tissue
0.10  leaf, flower, moss, corpse      live soft tissue; carrion
0.20  seed, fruit                     coat and rind; level with snow
0.25  ant + variants, ancestor        live cuticle — what 0.3 was chosen against
0.50  chitin_pale                     armour: beetle-proof, ant-cuttable
0.70  chitin_mid                      armour, harder still
```

**§11b says sixteen food materials. It is seventeen** — `ancestor.ron` also
authors a `food_energy` and was missed. Left at the default it would have been
the same bug in a narrower costume: nothing can eat an ancestor.

### The acceptance bar, and why it had to be the positive control

At `penetration_resistance`'s 100.0 default the ant's 1.0 meets `1.0 >= 100.0`
on **every mouthful in the world**. Measured on `ascii`:

| | before | after |
|---|---|---|
| `eats` @12k | 1008 | **1089** |
| `harvested_corpse` | 2640 | **1680** |
| births | 6 | **9** |
| digs | 132 | **171** |
| standing meat | 7680 | **11040** |

Both legs non-zero. Energy census closes (live 7654.94 vs ledger 7655.05).

*(Measured on the armour commit against its parent, so the delta is armour and
nothing else. Running `ascii` on the branch head gives different absolutes —
the curvature channel and a `main` merge landed after — and the bar that has to
hold there is the same one: `eats` and `harvested_corpse` both non-zero.)*

### The dig branch takes a kind guard, and it holds behaviour fixed

No `Creature`- or `Plant`-kind material authored a resistance before, so the
dig gate could never reach one. Pricing flesh at 0.25 would have opened it
silently: a dug creature cell never goes through `reconcile_chain`, so the
victim runs on a stale chain and its `body_energy` stamp stands in the world
as a spoil pellet nobody owns. The guard states as data what force cannot —
*an animal is bitten, a plant is eaten, neither is excavated.*

### The shared field, measured on both sides

`seedbed_probe`, which owns this coupling: `litter`/`deadleaf`/`windfall`/
`seed`/`corpse` become root-penetrable for the first time.

| arm | germ before | germ after | estab | cells |
|---|---|---|---|---|
| bare soil (control) | 68 | 68 | 32 / 32 | 1763 / 1764 |
| litter | 41 | 41 | 20 / 20 | 1232 / 1232 |
| deadwood | 36 | 39 | 19 / 19 | 567 / 550 |

The litter arm is identical on every column; only the reported resistance
moved, 100.0 → 0.1. The deadwood shift is deterministic (roots now thread
fallen `seed`/`deadleaf` cells) and does not reach establishment. Controls
held. **The coupling is real in the data and null in the outcome**, which is
what `seedbed_probe` already predicted: the germination gate is water, not
material.

---

## 3. Severing (§11 R2) — built, guarded, and it has never fired

`reconcile_chain` walks 8-connected from the vital cell (8 because `Grow`
places body cells at 8). What stays attached lives on smaller; what detaches
is stamped as corpse where it stands.

The four things §11d left unspecified are specified:

- **(b) the ledger entry follows the matter.** A destroyed cell books
  `meat_lost`; a **severed** cell books nothing at all — it was already in
  `stamped` and is still standing in the world as meat worth exactly that
  stamp. Booking it would double-count; dropping `meat_lost` would lose every
  other loss route.
- **(c)** severed cells carry the stamp only, with no share of a bank the
  animal still holds — a limb is poorer eating than a fresh kill and renders
  darker for it.
- **(a)** `Rigid` keeps the old shortening rule until `body_after_step` stops
  re-deriving its template from the head; severing a body that regenerates
  would mint meat every tick.
- **(d)** mid-flight is defined by `step_flight` re-reading the chain each
  frame; severed cells are `Powder`, so they fall on their own.

### And the counter says it has never happened

| scene | injuries | severings |
|---|---|---|
| `ascii`, all colony scenes | 0 | **0** |
| `predation_probe`, 12 seeds | 20 | **0** |

Predation lands bites; nothing has a middle to sever. **The shipped ant is two
cells: bite the head and it dies, bite the tail and it shortens.** This is
§11e's own prediction arriving as a measurement — severing's payoff is the
articulated body of §13 and does not stand without it.

The counters were added *because* this would otherwise have read as a success:
standing meat rose 44% in the same run, which is the armour half.

---

## 4. Three preconditions for the curvature sense

### 4a. Does the placement predicate admit convex sites? — **yes**

`examples/spoil_curvature.rs`, 12 seeds, 17,040 surface cells:

| | convex | flat | concave |
|---|---|---|---|
| all empty-beside-ground | 14.6% | 84.5% | 0.8% |
| admitted by the predicate | **20.6%** | 78.6% | 0.8% |

The predicate admits 35.5% of surface cells and *enriches* convex ones 1.41x.
§5g's worry that the two-of-three footing clause is anti-pillar does not bite
at this disc scale.

### 4b. Does the sensor visit both ends? — **only in one of the two beds**

| bed | p10 | p50 | p90 | spread |
|---|---|---|---|---|
| `LabBox` | +0.000 | +0.000 | +0.000 | **0.000** |
| worked bank | −0.250 | +0.417 | +0.833 | **1.083** |

`LabBox` lays a level bed, so its surface is flat by construction. Reporting
the channel dead on that alone would have been §14i's own first retry
condition — *the scene may not contain the phenomenon* — committed by the
session quoting it.

**The first reading was −0.083 at every one of 18,720 samples.** That is
−2/24: flat ground plus exactly one extra solid cell, which is the ant's own
second body cell, adjacent to its head by construction. **The sense was
reading its own body.** A perfectly constant result across twelve chaotic
seeds is the tidiness tell. Flesh is excluded now, nestmates with it — an ant
in a crowd would otherwise read as standing in a hollow, and `Crowding`
already counts that.

### 4c. Is `moisture_gradient` calibrated for the lab? — **yes, and no re-derivation is needed**

The concern was that the weights were fitted on `ascii`'s bench scene at
m = 0.006 and m = 0.080 while the probe's bed reads ~0.17 — an order apart.
**The ~0.17 is the hand-built *control* bed, not the lab bed sampled where
ants stand.** Measured at ant head positions:

| frame / seed | where | ants | grad mean | fraction non-zero |
|---|---|---|---|---|
| 1200 s1 | surface | 51 | 0.0597 | 60.8% |
| 1200 s2 | surface | 52 | 0.0210 | 21.2% |
| 3000 s1 | surface | 44 | 0.0630 | 40.9% |
| 3000 s1 | **covered** | 8 | **0.0000** | **0.0%** |
| 3000 s2 | covered | 4 | 0.0009 | 25.0% |

Realised range 0.000–0.063; the bench fitting points **bracket** it. So the
weights are pointed at the right part of the curve, no separate change has to
land first, and the shared-budget rule is not engaged.

**The low tail is the finding.** 39–79% of surface readings are exactly zero
and covered ants are ~100% zero. That is `Crowding`'s failure in mirror image
— pinned at the bottom rather than the top. It does not break *dig once you
are inside, drop when you surface* (a zero **is** the inside signal), but the
term carries presence/absence there, not gradation.

### And the instrument was measuring an empty scene

`field_sense_probe mode=lab` at its own default `frames=9000` reports **zero
ants standing**: the bed places its colony with `founders: 0`, so there are no
plants and nothing to eat, and the colony starves out between 3,000 and 6,000.
Every column reads `0.0000` and looks exactly like *the sense returns nothing
underground*, which is the finding the harness exists to make. It asserts the
scene now.

`burrow_probe`'s colony arm had the same class of defect — §14g's founder
walk, still unfixed on `main` at the time — which landed independently in
PR #216 while this work was in flight.

---

## 5. The curvature lever: a confound, a null, and then a result

Judged with `burrow_probe`'s shape columns, 12 seeds, arms paired by seed
inside one binary via `PIXEL_PHYSICS_CURVATURE`.

**First reading, and it is §14g's confound exactly.** Wired against ablated:
`circ` 0.292 vs 0.213, higher in 9 of 12 — but **`digs` lower in 12 of 12**
(498 vs 594). The curvature weight feeds `Drop`, and `Drop` competes with
`Dig`, so the lever bought its shape result by digging less. Rate-matched at
`digbias=0.50` the effect collapses to 7/5: a coin flip. Raising `digbias`
alone moves `circ` 7/5, so the rate-match does not itself carry an effect.

**At the authored 0.169 this is a null, and its power is stated.** Paired
`circ` difference median +0.05 against a per-seed sd of 0.10, 8 of 12 seeds.
Twelve paired seeds detect **~0.09 in `circ`** at 80% power — a 45% lift on
the ablated median of 0.200. The observed +0.037 is 40% of that and would need
**~69 seeds** to resolve.

**§14i's other complaint was that one value of the slope was tested**, so
`burrow_probe` gains `curvdrop=` and the slope was swept, all rate-matched:

| slope | circ | cells | digs |
|---|---|---|---|
| 0.0 | 0.194 | 77.5 | 605 |
| 0.169 | 0.250 | 70.5 | 604 |
| **0.5** | **0.287** | **59.5** | 608 |
| 1.5 | 0.275 | 55.0 | 600 |

### The control that makes it readable, and it is new here

Curvature on a bank has a **positive median** (+0.417), so any positive weight
raises the drop urge *on average* — a constant offset that would move every
shape statistic in the world without the animal ever responding to a shape.
`PIXEL_PHYSICS_CURVATURE=flat` holds the input at that median and removes only
the spatial variation, so both arms carry the same offset.

| arm | circ | cells |
|---|---|---|
| slope 0.0, no term | 0.194 | 77.5 |
| slope 0.5, curvature **flat** (mean only) | 0.237 | **89.0** |
| slope 0.5, curvature **live** | **0.287** | **59.5** |

| comparison | circ | cells |
|---|---|---|
| LIVE vs FLAT — spatial information alone | **10 up, 2 down** | **0 up, 12 down** |
| FLAT vs 0.0 — the rate offset alone | 8 up, 4 down | 9 up, 3 down |
| LIVE vs 0.0 — both together | 11 up, 1 down | 2 up, 10 down |

**The offset alone makes the cavity bigger and does not move roundness; the
spatial information makes it smaller and rounder, in every seed.** They act in
opposite directions on size, so the naive live-vs-off comparison was
*understating* the spatial term rather than manufacturing it.

### And the weight stays at 0.169, because 0.5 breaks the foraging loop

The authored weight was moved to 0.5 and moved back, and **the reason is the
result**. At 0.5, three shipped guards over ordinary behaviour go red:

- `a_laden_animal_pays_more_to_move_than_an_empty_one`
- `a_predator_eats_a_creature_and_needs_no_predation_code_to_do_it`
- `an_ant_eats_a_living_worm_and_cannot_when_worm_flesh_is_worth_nothing`

All three pass with the term ablated (`PIXEL_PHYSICS_CURVATURE=off`), so it is
this weight and not the armour gate that moves them. The mechanism is obvious
once seen: **a stronger drop urge is a stronger drop urge everywhere.** The ant
sheds its load before it has carried it anywhere, and an animal that will not
stay laden fails every test about being laden.

**So the lever's working range and its visible range do not overlap.** At 0.169
it is measurable-in-principle and does nothing; at 0.5 it shapes the nest and
costs the foraging loop. That is `CLAUDE.md`'s shared-budget rule arriving as a
measurement rather than a warning — *a correct mechanism at inherited constants
is a regression* — and the constants it reallocates are the whole drop economy
(`(Bias, Drop)`, `(AtNest, Drop)`, `(Carrying, Drop)`). Re-deriving those is its
own piece of work with its own acceptance bar; doing it here would be two
changes to one outcome.

**The trade at 0.5, recorded so the next attempt starts from it**: a rounder
cavity, and a **smaller** one — 77.5 cells → 59.5 for the same digging, the
same direction §14g flagged as "the opposite of the goal" for crowding, though
there the shape statistic was a coin flip and here it is not. Whether a rounder,
smaller nest reads as *built* is not a question these numbers can answer; it is
card `20260902T221154020Z-140852` on the review queue.

**What ships is therefore the channel, its instrument and its controls — not a
behaviour change.** `SurfaceCurvature` is wired, opt-in, guarded, and authored
at a weight measured to be a null. That is deliberate: the alternative was
shipping a weight that trades a nest shape nobody has judged for a foraging
loop that is known to work.

---

## 6. What a later session should not re-derive

- **Do not read `creature_arena` at its default horizon.** Below the founding
  grant the bed cannot discriminate, and it says so in a line most readers
  will scroll past.
- **Do not expect severing to fire.** It cannot, until a body has a middle.
  §13's articulated body is the precondition, not a refinement.
- **Do not compare a curvature arm against `=off`.** The mean offset is a
  confound of the same size as the effect and points the other way on size.
  `=flat` is the control.
- **Do not measure a curvature channel in `LabBox`.** Its bed is level; the
  sensor is a constant there and a null means nothing.
- **`penetration_resistance` is now armour as well as ground.** Anything
  re-pricing it moves roots, digging and biting at once, and the ingest branch
  is one branch for all food — a food above every shipped bite force is
  inedible, not armoured.
