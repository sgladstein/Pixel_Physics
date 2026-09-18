# The pheromone line: everything known, and what to build next

**Master report, 2026-09-17.** Supersedes nothing; it is the **entry point** to a
corpus of ~10,400 lines across thirteen reports and two lane notes. Read this
page, then read **one** source from §2 — never the directory.

**Who this is for:** a session picking the ant trail/homing line up cold. §7 is
an executable plan. §5 is the list of things that are wrong in the record and
must not be quoted. §8 is what has already cost days.

> ## ⚠️ RETRACTION, 2026-09-17 — read before §1, §3 or §7
>
> **The polarity metric does not measure which way the ramp points. It measures
> where the channel-A blob sits.** Two negative controls — a *flat*, ramp-free
> channel-A blob painted over the nest half and over the food half — read
> **+0.195 and −0.195**, larger than a perfect ramp's +0.115 and far larger than
> the −0.076 this document calls an inversion. Repaired and re-measured, the
> inversion falls **14x to −0.005**, goes the *other way* on 2 of 7 foraging
> seeds, and `r(AtNest, polarity)` goes from **+0.849 to −0.010**.
>
> **So §1 item 3, §3.3, §3.4, §3.5 and §5 item 5 are retracted**, and with them
> the two sweeps of §7.15–§7.18. Full account, with the controls and the root
> cause, in `pheromone-trail-direction-2026-09-16.md` **§7.19**.
>
> **What survives untouched**, because none of it is a polarity measurement: §3.2
> (a laid trail is decisive, the colony cannot build one, `self ≡ mute`), §3.7
> (`homeA` clips exploration), §3.8, the `u16` widening, and — stated by §3.7
> already — **that deliveries, never polarity, is the success criterion**.
>
> ## 🔑 AND THE CAUSE IS NOW LOCALISED — §7.22, 2026-09-17
>
> **The homing circuit is correct and switched off 98.16% of the time.**
> `Carrying` is not a boolean: it is `crop.worth() / crop_capacity`, and
> `ant.ron` gates the homing pair `Bias -45, Carrying +45.5`, so the pair only
> leaves saturation at **`Carrying >= 0.989`**. One cell of `fruit` is 960 J
> against `crop_capacity: 1440.0` = **0.667**. Over **570,660 traced laden
> decisions** the gate is OPEN on **10,509 (1.84%)**, and the modal laden ant
> sits at 0.7 — one food item.
>
> **When it is open the mechanism is excellent**: `P(move)` **0.8155**
> up-gradient against **0.1572** down, a swing of **+0.658** where the bare-slab
> harness reads 0.641 against 0.200 — and the displacement is **homeward**, so
> the colony's own channel-A ramp points at the nest after all.
>
> **The design that follows is §7.23**, and three things in it are load-bearing
> for whoever picks this up. **One food item is worth 5–17 round trips** (960 J
> against a 56 J round trip), so the correct threshold is *just above zero*, not
> 0.989 — this is not a tuning judgement, the margin is an order of magnitude.
> **The fix is a latch, not a slope**: a graded weight gives one uniformly
> half-hearted colony, where what is wanted is some ants committing to go home
> while others forage, which is per-ant persistent state. And **there is no
> hidden-to-hidden path** (`hh_slot` is one weight per unit, pure
> self-recurrence), so a latch unit *cannot gate* the homing pair — which
> dissolves the apparent contest for `ant.ron`'s last free hidden unit. The
> buildable latch is **recurrence on units 0/1 themselves: two weights, no new
> unit, no `mutation_rate` re-derivation.** ~~Order: rescale the gate first, then
> the latch; `crop_capacity` last if at all.~~ **That order is withdrawn — see
> the block below.**
>
> ## 🔑 AND THE ORDER IS WITHDRAWN — §7.25/§7.26, 2026-09-18
>
> **Crop fill is wired to stillness, not to direction, so neither the rescale nor
> the latch could have worked.** Measured per decision over 570,660 laden
> decisions: `P(home)` and `P(away)` are a **dead heat in every fill bin**
> (0.0177/0.0176 at a third of a crop, 0.0064/0.0064 at a full one), and
> `P(home)` **falls** with fill by 2.8x because the only thing fill does is lower
> `P(move)` — it brakes the ant equally in both directions. **Pooled, the entire
> gate-open population yields eleven net homeward cells** over six seeds and
> 24,000 frames, and opening the gate *lowers* `P(move)` by 0.38.
>
> **§7.24's null was this curve's direct prediction, not evidence for the latch.**
> The rescale widens a fill→direction channel that does not exist; the latch has
> 10.2% of open decisions to hold; `crop_capacity` moves a step along a flat
> curve.
>
> **`creature.rs:4156` has said so all along**, untouched: *"That is the whole of
> the homing mechanism — there is no steering toward the nest anywhere."* `Move`
> gates stepping along the heading the ant already has; only `tumble` changes
> direction, and it re-rolls **uniformly**.
>
> **The replacement is §7.26 — the owner's rule, fill-weighted, in `tumble`**: at
> probability `crop_fill`, re-roll toward the home bearing instead of uniformly.
> It is the only site that sets direction and the only one that works on flat
> ground, where **§R4 kills `Turn`** and with it `HomeBearing` as step 4 authored
> it. It costs **no `BRAIN_INPUTS` bump, no `live_slots` change, no
> `mutation_rate` re-derivation in six files** — `forage_anchor` already ships
> the home vector, re-anchored at every nest contact. Because `heading` is
> persistent, the graded draw gives the **two-group population** the latch was
> for, rather than one half-hearted cloud.
>
> **Keyed on `crop_fill`, never `Carrying`** — `Carrying` is
> `crop_fill.max(spoil ? 1.0 : 0.0)`, so keying on it sends ants home for dirt.
>
> **And `stop=` was never echoed in the harness header** (fixed 2026-09-18). It
> defaults to 0 and decides whether the hand-laid trail stands for the whole run
> or is seeded and released: the same command reports n 570,660 / 1.84% open
> against 639,100 / 1.25% under **byte-identical parameter lines**. The two
> scenes disagree on a published sign, so §7.22's *"positive means the ramp
> points at the NEST"* is a statement about one scene. **Check `stop=` on any
> archived log before comparing against it.**
>
> **And a second defect found on the way:** `SPOIL_IS_CARGO` is a measurement
> switch (default ON), not a split — an ant holding **spoil reads 1.0 and the
> gate opens**, while an ant holding food reads 0.667 and it stays shut. That is
> a live confound in the 1.84% above and is the first thing to check.
>
> This explains the 750x gap: `onetrail::hold_gate_laden` evaluates the circuit
> at `Carrying = 1.0`, a value the colony almost never reaches, so its
> +104-of-112 was never evidence about a colony. It also makes §7.21's
> "cutting the circuit is undetectable" the expected result rather than a
> puzzle. **§Z7's re-gate fixed the authored *open* value; the fault is the
> *input*.**
>
> **AND THE SYMPTOM IS UNTOUCHED — this retracts the explanation, not the
> problem.** Measured the same day with no polarity statistic anywhere in it
> (§7.20): on the shipped genome, `hand` arm, 12 seeds, **4,260,372 ant-ticks
> carrying larder, 2,177 of them inside the nest band (0.051%), 11 round trips,
> and +883 net homeward cells over those 4.26M ticks — +0.0002 per tick.** Seven
> of twelve seeds deliver exactly zero. A laden ant's motion toward home is not
> slow, it is nil. **The return leg is the thing to fix, and trail shape is
> downstream of a journey that is not happening** — so any mechanism whose
> subject is "the trail a homing ant lays" is untestable on this bed until that
> number moves.

---

## 1. The one-page answer

**The colony forages and does not bring food home.** That is the problem; it has
been the problem since §T2 was filed, and none of the pheromone work has moved it.

Three facts, each measured independently, that between them say why:

1. **A laid trail is decisive.** Hand-lay one and 91.4% of ants reach the food
   against 3.7% unaided; 7 of 12 colonies survive against 0. *(That is §7.13's
   **12**-seed isolated-larder run. §3.2's table is §7.11's **6**-seed run —
   92% / 3%, 4 of 6 — a different measurement, not a rounding of this one.)*
2. **The colony cannot lay one itself.** The `self` arm is indistinguishable from
   the `mute` arm (channel B zeroed): ~3.4 route cells of 90, in 2 of 12 seeds.
3. ~~**The homing ramp inverts in a foraging colony.**~~ **RETRACTED 2026-09-17
   — this was the instrument; see the banner above and §7.19.** What is true is
   the weaker and already-known statement that a foraging colony's channel-A mass
   sits nearer the food, which `occupancy/1k` reports directly. The text is kept
   because §7.18's sweeps were aimed at it. Channel A — the homing plane — is laid by
   every ant on every successful move, so it integrates where ants *are*. In a
   colony that forages hard, that is the food, and the ramp inverts.
   **Do not phrase this as "nothing steers a laden ant home"** — that sentence is
   `nest-design` §5.1's, taken on `u8`, and **§12 of the same report retracted it**
   on `u16`: with the plane standing, the shipped circuit puts more laden ants at
   the door than either cut arm on 2 of 3 seeds. See §5b.

**The structural statement.** Channel A and channel B have the **same laying
rule** and need opposite ones. `wiki/ants.md` says it without noticing:
*"Nothing decides where a trail goes: it is just the leftovers of where ants have
been."* Correct for a food trail — ants have been at the food, and that is where
you want to go. Exactly wrong for a home trail.

**The biology says the same thing.** Real ants home by **path integration** — a
private home vector, run straight — and use the trail as a *contextual modulator*
("you are on the route"), weighted by how long the vector is. Trails are
**isotropic**: they carry no direction. The engine asks a concentration gradient
to encode direction, which real trails do not do. See §6.

~~**So the fix is a direction sense, not a weight.**~~ **The evidence clause is
retracted.** The `r = +0.64 to +0.91` "correlation between where ants spend time
and which way the ramp points" is one quantity measured twice — under a metric
that passes its controls the same correlation is **−0.010** (§7.19). The two
sweeps did move colony survival and trail length, and that part stands; what they
did not do is fail to flip a ramp, because there was no ramp reading to flip.

---

## 2. The corpus — read one, not the directory

| Report | Lines | Status | Owns |
|---|---|---|---|
| **`pheromone-trail-direction-2026-09-16.md`** | 2,210 | live, this line's working record | §7.11–§7.18: the gap sweep, discovery vs homing, the inversion, both failed sweeps, the stale-binary incident. **§7.15/§7.18 carry errors — see §5** |
| `pheromone-lifetime-and-wiring-2026-09-14.md` | 746 | measurement, round 36 lane C | Trail life (144 frames vs a 2,200-frame round trip), `DECAY_RHO` inert, 4 of 7 reader slots unread, the `u8`→`u16` widening (§3c) |
| `nest-design-2026-09-14.md` | 648 | **design of record**, §13 owner ruling 2026-09-15, nothing built | What a nest is; **§12 supersedes §5.3 and §9 item 4 — read §12 BEFORE §5.3**; **§8 option C = the home bearing**; §13 retires `nest` as a material |
| `stigmergy-research.md` | 438 | research, **implemented** | Deposit → diffuse → decay → follow. The colony is built on it |
| `foraging-range-measurement.md` | 447 | measured, instrument landed | The 19-cell bubble, the 2-cell-spacing gridlock |
| `colony-starvation-separated-2026-09-08.md` | 516 | measured; pheromone clause corrected 2026-09-09 | The colony dies twice |
| `decaying-gradient-quantization-2026-09-15.md` | 280 | survey | Does the `u8` root cause generalise — decay-plus-gradient in narrow storage |
| `colony-economy-design-2026-09-09.md` | 350 | design | Foraging returns less than it costs |
| `colony-food-economy-design-2026-09-14.md` | 170 | design | Food economy successor |
| `evolution-lab-nest-question-2026-09-14.md` | 131 | **SUPERSEDED — do not read** | Round 37 overtakes its sequencing; its 414-delivery constraint is retracted; its path-integration content is §6 here, better stated |
| `evolution-lab-round-37-brief-2026-09-15.md` | 250 | current round | **Lane 3: the "nothing steers a laden ant home" null did NOT survive the `u16` re-measurement.** Every nest-research number is pre-widening; the home bearing is **conditional** on a named six-seed sweep |
| `creature-direction.md` | 1,772 | direction agreed 2026-08-17 | The origin document; most dead-end entries cite it |
| `creature-genome-flexibility-2026-09-02.md` | 2,471 | design, not built | Names homing-as-a-`since_nest`-odometer as one of four things "still spelling ant in Rust" |
| `lanes/evolution-lab-pheromones.md` | 170 | round 36 lane C | Nothing tuned; every default ships unchanged. The alarm fix |
| `lanes/evolution-lab-nest-research.md` | — | round 36 | Companion to `nest-design` |

**Open bugs — grep only these.** **§R4** (`BrainOutput::Turn` is nearly inert for
a surface walker on level ground) — **OPEN, and it governs step 4**. §Z7 (the
trail-following gate saturates the signal it gates) — **OPEN**, and note its
heading: the **homing** half (units 0/1, channel A) shipped 2026-09-09; the
**food** half (units 2/3) is deliberately still open, and its direct repair is a
`dead-ends.md` entry — *"correct … and it makes the animal decisively worse"*,
25.0% in a mirrored race against a zeroed-brain control's 15.4%. §Z6 (every shipped bed starves its colony inside one play session) —
**OPEN**. §T2 (1,651 pickups, 4 deliveries) — OPEN, but `nest-design` §13 says it
closes with the site ruling. §Z5 (every homing odometer dead, charge below
`W_EPS`) — **closed**.

---

## 3. The current observations, in full

> **§3.3, §3.4 and §3.5 are RETRACTED** — every polarity column in them is the
> artifact of §7.19. The `alive`, `foraging`, `ate J`, `cov` and `pk` columns in
> the same tables were not measured by that statistic and stand. §3.1, §3.2, §3.6,
> §3.7 and §3.8 are unaffected.

**All raw logs are archived in `Reports/data/`** — `homeA-positive-control-
2026-09-17.log`, `refill-{0,500,1000,2000,4000}.log`, and `sweep-*.log` for the
nine-arm sweep. Every table below is re-derivable from them; nothing here rests
on a scratchpad that no longer exists.

**Provenance caveat:** the `refill-*.log` headers pre-date the fix that made
`trailfollow` echo `refill=`, so they cannot state their own scene. The 2000 row
is corroborated — `refill-2000.log` and `sweep-shipped.log` re-derive identically
— the others are not. §8's stale-binary trap applies.

Everything in this section was measured 2026-09-16/17 on the **`u16`** engine.
Anything older is on `u8`, where the far half of every trail read exactly zero —
see §8.

### 3.1 The harness and what each arm isolates

`examples/trailfollow mode=gap` builds a bare bed: a nest band at the left, a
larder at `food=`, a gap between. Five arms, each cancelling one explanation:

| arm | hand-laid B trail | ants' own B silenced | hand-laid A homing ramp | isolates |
|---|---|---|---|---|
| `hand` | yes | no | no | does a laid trail move the colony |
| `hmute` | yes | **yes** | no | **the decay baseline** — our ramp laid, ants silenced. Without it, a dying hand-laid trail reads as ant maintenance |
| `self` | no | no | no | can the colony bootstrap a trail |
| `mute` | no | **yes** | no | **the no-trail control** — `self` minus the ants' own laying |
| `homeA` | no | no | **yes** | if laden ants only fail to route because they cannot get home, their own B appears here |

**Key readout columns.** `alive` = ants alive / colonies; `ate J` = joules off the
larder; `arrive@` = frame of first arrival; `carry` = ant-ticks holding larder;
`carry@nest` = those inside the ±26 nest band — **that pair, not a cell census, is
the answer to "is food being carried back"**; `reach 0-25-50-75-100` = how far
each ant ever got as a share of the gap, so a commuting population is a *shape*;
`route pk` = **route cells holding channel B**, of the gap length (the column
name says "peak"; it is used throughout as a count); `POLARITY` = channel A's nest-ward
gradient (positive = taller at the nest); `occupancy/1k` = ant-ticks per band,
nest→food; `other J` **must read 0** — it is the check that `onlyfood` held.

Critical knobs: `refill=` (re-places the larder every N frames — **defaults to 0
and the scene is unusable at 0**, see §3.3), `onlyfood=on larder=fruit` (isolate
the diet so intake is attributable), `gaps=`, `seeds=`, `arms=`, and the genome
riders `recur=`, `emita=`, `biasa=`.

### 3.2 A laid trail is decisive; the colony cannot build one

20 ants, 6 seeds, 24,000 frames, trail hand-laid to frame 6,000 then released:

| gap | arm | alive (med) | colonies alive | ate J (med) | ants reaching food | seeds arriving |
|---|---|---|---|---|---|---|
| **90** | **hand** | **102** | **4 of 6** | **317,486** | **1,299/1,415 (92%)** | **6 of 6** |
| 90 | self | 0 | 0 of 6 | 0 | 4/120 (3%) | 3 of 6 |
| 90 | mute | 0 | 0 of 6 | 0 | 3/120 (3%) | 2 of 6 |
| 150 | hand | 0 | 1 of 6 | 0 | 446/562 (79%) | 5 of 6 |
| 150 | self / mute | 0 | 0 of 6 | 0 | **0/120** | 0 of 6 |
| 220 | hand | 0 | 0 of 6 | 0 | 2/121 | 1 of 6 |
| 220 | self / mute | 0 | 0 of 6 | 0 | 0/120 | 0 of 6 |

**`hand` beats `hmute` on survival, and it is the one positive signal for a
pheromone mechanism anywhere in this work** — at gap 90, **7/12 against 3/12**
colonies alive and 1.8× on intake (§7.13). It vanishes at 150 and 220. Do not let
the next line stand unqualified.

**`self` ≡ `mute`.** The colony's own laying is worth nothing measurable: ~3.4
route cells of 90, in 2 of 12 seeds. **And nothing is maintained** — `hand` and
`hmute` both read `route pk` **88**, so ant maintenance of a laid trail is
**zero**. (An earlier reading missed this: the margin was sized against a single
`DEPOSIT` lifetime while `lay` deposits ~100 times, so a *dying* hand-laid trail
read as maintenance. `hmute` is the arm that catches it.)

**Discovery, not homing, is the binding constraint** (§7.13): 0–8% of ants reach
food without a trail, 62–91% with one. **Nothing provisions the nest** — 0.14% of
carrying ticks are inside the nest band.

### 3.3 `refill` decides whether the scene can pose the question at all

The larder defaults to **one-shot**, 48,000 J against a stated need near 46,800 —
1.03× subsistence. At that setting the colony starves and there is no foraging
behaviour to measure. Gap 90, 12 seeds, `arms=hand`, shipped ant:

| `refill` | seeds with ants alive | foraging colonies (`AtNest` < 5%) | of those, ramp points at the **food** | mean polarity, foraging | mean, stays home | r(`AtNest`, polarity) |
|---|---|---|---|---|---|---|
| **0** (one-shot) | 4/12 | 5 | 1 of 5 | +0.0082 | +0.0176 | **+0.19** |
| **500** | 5/12 | 4 | **4 of 4** | −0.0561 | +0.0266 | **+0.69** |
| **1000** | 3/12 | 2 | **2 of 2** | −0.0432 | +0.0392 | **+0.61** |
| **2000** | 7/12 | 7 | **7 of 7** | −0.0759 | +0.0518 | **+0.85** |
| **4000** | 3/12 | 3 | **3 of 3** | −0.0631 | +0.0441 | **+0.84** |

**Use `refill=2000`** — 7 of 12 alive and 7 of 7 foraging is the most informative
scene found. **But note the survival sequence is non-monotone** (4 → 5 → 3 → 7 →
3), so 2000 is the best of five sampled points, **not a located optimum**. The
inversion itself is robust across all four refilling settings; only the colony
size is lucky at 2000. At `refill=0` the inversion is absent *because no colony forages*,
not because it is not real.

### 3.4 The polarity metric, calibrated

`arms=homeA` paints a perfect nest-ward ramp (`lay_home`, `trailfollow.rs:264`)
and reads **+0.11493 … +0.11535 on 6 of 6 seeds** — an independent prediction
from `lay_home`'s own arithmetic said ≈ +0.12. Its profile nest→food is `[35678, 40062, 26502, 13536, 356]` — **not monotone;
the first two buckets rise** (an earlier draft called it monotone, which was
wrong), and the metric is a mean of per-cell gradients, not a shape test. So:

| | polarity |
|---|---|
| a perfect nest-ward ramp | **+0.115** |
| a colony that stays home | +0.018 … +0.052 |
| **a colony that forages** | **−0.076** |

**Caveat (§5 item 5):** the metric's admission gate appears **twice** and admits
blob-edge cells worth −0.97 against interior cells' ±0.03–0.07. The `homeA`
control has 91 of 91 route cells occupied so it has no interior edge; the `hand`
arms have 57–78 and therefore do. **The edge contribution is unquantified.**

**`:1121` is not a second polarity gate** — it reads `Channel::B` and feeds
`natural_along`. Same idiom, different metric. Report them as two figures.

**Archived** at `Reports/data/homeA-positive-control-2026-09-17.log`, regenerated
and reproduced exactly on 2026-09-17:

```
RAYON_NUM_THREADS=4 cargo build --release --example trailfollow && \
RAYON_NUM_THREADS=4 ./target/release/examples/trailfollow mode=gap gate=b2 \
  gaps=90 seeds=6 arms=homeA onlyfood=on larder=fruit food=200 \
  frames=24000 refill=2000
```

**And never pool `homeA` polarity with `hand` polarity.** `lay_home`
(`trailfollow.rs:264`) is an **external channel-A writer**: in `homeA` the plane
is partly ours, in `hand` it is entirely the ants'. They answer different
questions and their numbers are not comparable. `homeA` is the metric's control,
not a treatment arm.

### 3.5 Neither weight lever works

Gap 90, 12 seeds, `arms=hand`, `refill=2000`, `RAYON_NUM_THREADS=4`. `cov` is
route cells holding channel A, of 91. `pk min` is the minimum across seeds of the
running peak. `→food` counts foraging colonies whose ramp points at the food.

| arm | alive | foraging | →food | mean polarity | cov med | cov min | pk min | r |
|---|---|---|---|---|---|---|---|---|
| **shipped** | 7/12 | 7 | **7 of 7** | −0.0759 | 78 | 63 | 8,234 | +0.85 |
| `biasa` −0.05 | 6/12 | 5 | 4 of 5 | −0.0653 | 76 | 49 | 8,409 | +0.74 |
| `biasa` −0.10 | 7/12 | 6 | **6 of 6** | **−0.1015** | 68 | 44 | 9,111 | +0.91 |
| `biasa` −0.18 | 5/12 | 4 | **4 of 4** | −0.0845 | 57 | 44 | 8,231 | +0.84 |
| `biasa` −0.35 | 3/12 | 3 | **3 of 3** | −0.0628 | 52 | 41 | 8,184 | +0.81 |
| `recur` 0.999 | 5/12 | 5 | **5 of 5** | −0.0820 | 76 | 65 | 8,211 | +0.86 |
| `recur` 0.995 | 3/12 | 3 | 2 of 3 | −0.0439 | 76 | 43 | 8,265 | +0.65 |
| `recur` 0.99 | 3/12 | 2 | **2 of 2** | **−0.1291** | 65 | 40 | 8,254 | +0.64 |
| `recur` 0.98 | 6/12 | 6 | 5 of 6 | −0.0517 | 70 | 39 | 8,054 | +0.76 |

**31 of 34 foraging colonies across eight arms still point at the food.** Every
arm's mean is negative. Paired per-seed shifts go both ways with no trend
(floors: +0.048, −0.014, +0.037, −0.002; decays: +0.004, +0.054, −0.021, +0.035).
The only reliable effect of a floor is **shortening the trail**: coverage
78 → 76 → 68 → 57 → 52, with colonies dying alongside (7 → 6 → 7 → 5 → 3).

**A 400× change in the decay weight does not flip the ramp**, and `r` never
leaves +0.64…+0.91. That is the evidence that this is not a tuning problem.

### 3.6 The odometer grades steeply — the opposite of the first diagnosis

`brain.rs`'s own fitting comment: *"the decay is dominated by `squash`, not by
`w_rec`."* Simulating the real recurrence (`h ← squash(w_rec·h + w_in·AtNest)`,
`emit ← squash(w_out·h + bias).clamp(0,1)`):

| config | t=0 | t=70 | **t=141** | t=300 | t=2999 | fall over a 141-tick trip |
|---|---|---|---|---|---|---|
| **shipped** (`w_out 32`, no bias) | 0.819 | 0.293 | **0.177** | 0.094 | 0.010 | **78.4%** |
| `recur` 0.99 | 0.816 | 0.217 | 0.086 | 0.015 | 0.000 | 89.4% |
| `recur` 0.98 | 0.813 | 0.149 | 0.033 | 0.001 | 0.000 | 95.9% |

**Positive control:** the same simulator at the fitting test's chosen weights
(`w_in 0.05, w_rec 0.99995, w_out 900, bias −0.2`, 5-tick touch) returns
`0.992 → 0.072, t1000 = 0.402`, and the engine's own
`what_an_odometer_emits` prints `0.992 -> 0.072 (t=1000 0.402, t=2000 0.185)`.
Three decimals on three points.

### 3.7 Correct homing alone is not enough, and can hurt

*(Same archived log as §3.4. **Read it with §5b's fourth row**: `SPOIL_IS_CARGO`
defaults ON, so `Carrying` in this arm is 100% spoil, and §7.14 attributes the
exploration clip to that confound rather than to homing. The clip is real; its
**cause** is not settled.)*

`homeA` — a perfect homing ramp, no food trail — at gap 90, 6 seeds,
`refill=2000`:

**0 alive, 0 delivered, 0 trips, reach `12 8 0 0 0`, `AtNest` 27–50%.**

Not one ant gets past half the route, and they sit at the nest a third to half of
all ticks. **A strong homing signal suppresses exploration.** This is the
standing argument that **deliveries, never polarity, is the success criterion.**

### 3.8 Three mechanisms tested on the way, for the record

- **Reader re-gating** (§Z7): **9.6×** — and the control matters. `gate=b2`
  against **`gate=shipped`** (applies nothing) is the honest comparison and gives
  9.6×; the discredited `gate=saturated` comparison gave **8.6×**. Only the
  **homing** half (units 0/1, channel A) shipped, on 2026-09-09. The **food** half
  (units 2/3) is deliberately still open — see §5b.
- **Pass-through kin** (ants may swap places with a nestmate): cuts blocked moves
  **~90%** and *reduces* intake **35%**. Congestion is not the fault. Shipped as
  an option (`passes_through_kin` in `src/lab/params.rs`), default off.
- **Cargo sensing** (`Carrying` is true for dig spoil, so a spoil-hauling ant
  believes it is laden): a **real defect** — exploration up **4.2:1** at 50 seeds
  with the `mute` arm flat at 0.9:1 as the falsifier — but **no effect on
  transport**. Gated by `SPOIL_IS_CARGO` (default on = the bug, for A/B).

## 4. What is open

- **§Z7**: the trail-following gate saturates the signal it gates. The one direct
  repair is in `dead-ends.md` as rejected — but *on the bed*, and the rejection
  condition is *"what a food trail is worth in this bed, not the gate."*
- **Two unseparated causes for the same result.** §Z7's saturation, versus
  "channel B is laid only while laden so its gradient points at the **nest** and
  steers an empty ant home." `Reports/README.md` records: *"neither is
  established, and nobody has run the arm that separates them."*
- **The self-read** (§5, item 3) — never measured, only derived.
- **Which tree is right.** The reproducible tree grows a far sicker colony than
  the one §7.15's original numbers came from. Never resolved; see §8.

---

## 5. Errors in the record — do not quote these

**Mine, this line, all corrected in place but still readable in §7.15/§7.18:**

1. **§7.18's floor argument is self-refuting.** §7.16 *pre-registered* that
   `a_peak_amt` would not move under an emission floor ("a floor does not change
   what an ant fresh from the nest lays"); §7.18 read that confirmation as proof
   the floor never bit. It is a route-wide **max** quoted as a min across seeds,
   set by the nest-end deposits the floor deliberately does not touch. Predicted
   shift ~1.5%, below seed spread.
2. **§7.15's mechanism is wrong** (its *finding* survives). Step 1 derives the
   odometer's per-trip decay from `w_rec` alone and concludes the charge is flat.
   It falls 78% — see §3.
3. **"1,813" is derived, not measured.** It is `0.177 × DEPOSIT` off the odometer
   curve, presented as an observation. `ant.ron:1764-1783` is emphatic about not
   quoting a curve without naming the weights it belongs to.
4. ~~**"−418 net homeward cells over 2.1M carrying ticks"** could not be found in
   `Reports/`.~~ **THIS ITEM WAS ITSELF WRONG — retracted 2026-09-17.** The figure
   is at `pheromone-trail-direction-2026-09-16.md:1107`, in a table, and cited
   again at `:1484`, `:1553`, `:1623`. It is also a **live printed column** —
   `carry->nest`. Read it off the standing run; do not re-derive it. *(An errata
   section that flags a real measurement as missing is the most expensive kind of
   error in this document: it stops a future session using a good number.)*
5. **The polarity metric's admission gate** at `examples/trailfollow.rs:976`.
   **SUPERSEDED 2026-09-17 — this item was right that the metric is broken and
   wrong about why, and acting on the stated cause would have fixed nothing.**
   Tightening the gate to `&&` moves a ramp-free blob only from ±0.195 to ±0.181,
   7%. The defect is not edge cells admitted by `||`; it is that the scan window
   is fixed to the route while the trail diffuses past its ends, so only one of a
   blob's two shoulders is ever counted. §7.19.
   **(Corrected 2026-09-17: an earlier version of this item claimed a *second*
   gate at `:1121`. That line reads `Channel::B` and feeds `natural_along`, the
   food trail's own column — same predicate, different channel, different
   readout. The polarity metric has ONE gate.)** Its `here > 0.0 || ahead > 0.0`
   admits blob-edge cells worth **−0.97** against interior cells' ±0.03–0.07 —
   one edge cell is worth ~20 interior cells. The +0.115 control shows the metric
   is not *dominated* by this, but it is unquantified on the `hand` arms, where
   coverage is 57–78 of 91 cells and edges therefore exist.
6. **"Integral over occupancy" is the wrong noun.** Deposits happen **only on a
   successful move** (P-11, `creature.rs:4223`). The 21:1 food-end ratio is
   ant-*ticks*; moves-per-band has never been measured. A congested ant at the
   larder burns ticks without laying.

**Inherited, in source comments:**

7. **`creature.rs`'s odometer comment named the dead pre-fix weights** as "the
   weights `ant.ron` actually authors" — `w_in = 0.0005` is below `W_EPS` and
   `brain.rs`'s own readout labels that row `authored (dead)`. Corrected
   2026-09-17.
8. **`ant.ron` quoted a curve its own weights do not produce** — `0.992 → 0.072,
   saturated 85 ticks` is the fitting test's chosen fit at `w_out 900, bias −0.2`;
   the file ships `w_out 32.0` and **no bias**, which gives `0.819 → 0.010`,
   saturated 0. Corrected 2026-09-17.

---

## 5b. Claims this line has RETIRED — do not revive them

§5 lists errors this document committed. This lists claims the **corpus** retired
and an earlier draft of this document revived. Four of the six findings that a
cold-start review said would have cost it a lane were of this kind, so the
distinction earns its own section.

| retired claim | retired by | what is true now |
|---|---|---|
| **"Dial `DIFFUSE` on channel A to 0.02"** | `nest-design` §12; round-37 Lane 3 | *"Withdraw the 0.02 on A suggestion."* The `u16` widening bought the lifetime (144 → 1,476 frames) without the trade §5.3 priced |
| **"Nothing steers a laden ant home"** | `nest-design` §12 | *"no longer true as stated"* — on `u16` the shipped circuit beats both cut arms on 2 of 3 seeds |
| **"Build the home bearing next"** | `nest-design` §12 | drops from *next* to **conditional** on a six-seed `nesthome` sweep — §7 step 2 |
| **"Cargo sensing is not the fault"** | §7.14's 50-seed replication | it *is* a real defect on exploration (4.2:1, `mute` falsifier flat at 0.9:1); what it does not move is **transport** |
| **"The units-2/3 re-gating is the §Z7 repair"** | `dead-ends.md` | arithmetically correct and *"makes the animal decisively worse"*; only the units-0/1 homing half shipped |

## 6. The biology, and what to take from it

Established in the literature, and it corrects an earlier claim of mine that ants
do not use pheromones to home — **they use both**:

- **Path integration supplies direction.** A home vector, built from a sky
  compass and a stride integrator, run *straight* rather than retraced.
  Wittlinger 2006 proved the odometer directly: *Cataglyphis* on stilts overshot
  the nest, on stumps undershot, by the ratio of leg length.
- **Its weight scales with the vector's length.** A long path integrator
  *dominates* the trail and ants readily leave it; a short one makes them hesitate
  and retreat into the trail rather than overshoot the nest. In *Veromessor
  pergandei* the integrator wins outright in conflict.
- **Trails are isotropic** — "non-directional and axi-symmetric". The chemical
  trail does not encode which way the nest is. Direction comes from vision, path
  integration, and trail geometry.
- **So the pheromone is a contextual modulator**, not a compass: it says *you are
  on the route*, and adjusts how far the ant trusts its own vector.
- **The pioneer/recruit split is real**: a scout searches, homes by path
  integration, lays trail on the way back; recruits use that trail as a route and
  still resolve direction themselves.

**It does not have to match.** The owner's framing, 2026-09-17: *"It does not have
to perfectly match how real ants, but we should take inspiration when we can."*

---

## 7. The plan

> ⚠️ **STEPS 3, 4 AND 5 ARE RE-ORDERED BY §7.25/§7.26 (2026-09-18) — read the
> second banner at the top of this document before building any of them.** In
> short: the gate rescale and the latch are withdrawn as written (crop fill does
> not reach direction, so there is nothing for either to widen or hold), step 4's
> `HomeBearing` is blocked on flat ground by §R4, and the replacement is a
> **fill-weighted re-roll in `tumble`** at a fraction of step 4's price. The
> steps below are kept because their *evidence* stands and step 5 is untouched;
> only the order and the two gate repairs change.

Ordered so each step is attributable. **Steps 4 and 5 must not land together** —
both add a term to a shared weighted sum, and `CLAUDE.md`'s *"a correct mechanism
at inherited constants is a regression"* makes the joint result unreadable.

**The standing run.** Every step below uses this unless it says otherwise, and
the build is in the same command because a stale example binary already cost a
day (§8):

```
RAYON_NUM_THREADS=4 cargo build --release --examples && \
RAYON_NUM_THREADS=4 ./target/release/examples/trailfollow \
  mode=gap gate=b2 gaps=90 seeds=12 arms=hand onlyfood=on larder=fruit \
  food=200 frames=24000 refill=2000
```

**The success criterion for the whole plan is `carry@nest` and `trips`, not
polarity** (§3.7). Today both are ~0 at gap 90.

---

### Step 0 — baseline seed sweep at HEAD

**Why:** there is no control arm otherwise. `brain.rs:2046-2052` — once
`live_slots` changes, every breeding scene's numbers move from birth 1, and
*"the remedy is a seed sweep, not a diff."*

**Do:** the standing run, all five arms, 12 seeds. Archive the log.
**Accept:** nothing — this is the baseline everything else is diffed against.

### Step 1 — readouts only, no engine change — **DONE 2026-09-17, and it fired**

> Built, and the answer was the stop condition: see §7.19. The `&&` figure this
> step proposed as the decider **would not have fired** — it still reads −0.075
> on foraging colonies. What decided it was a pair of *negative* controls this
> step did not think to ask for. The metric is now `SPAN`, validated against a
> ramp-free field (exactly 0.000) and a real ramp (+0.026). **Do not re-run this
> step; read §7.19 and re-plan anything downstream of polarity.**

### Step 1 (as originally written, kept for the record)

**Why:** it fixes the meter everything else is judged on. Running a mechanism
before fixing the instrument is the §8 failure that cost a day.

**Do, in `examples/trailfollow.rs`:**
- **The polarity gate at `:976`** — add a boundary-cell count and a second
  figure restricted to `&&`. **Do not also change `:1121`**: it shares the
  predicate but reads `Channel::B` into `natural_along`, a different column.
- `probe_in[I::PheroAAlong]` and `[I::PheroBFront]` per ant, split laden/empty,
  beside the existing `AtNest` probe at `:1093`. `probe` is non-mutating by
  construction (`creature.rs:4505-4519`).
- Moves-per-band, not ticks-per-band. Plumb `deposits_a` (`pheromone.rs:1028`).

**Accept:** the `&&` figure and the `||` figure are reported side by side on
`arms=hand,homeA`. **`homeA` must still read ≈ +0.115** — it is the positive
control and if it moves, the edit broke the metric.
**Decides:** whether §3.5's magnitudes are real or edge artifacts. If the `&&`
figure for foraging colonies is **positive**, the inversion is an artifact and
§7.15 falls — stop and re-plan.

### Step 2 — the gating sweep — **DONE 2026-09-17: it does. Build it.**

> Run at six seeds on `u16` (§7.21). **Cutting the homing circuit out of the
> genome entirely is not detectable**: laden-at-door medians `shipped` 15.2%,
> `noemit` 16.5%, `nosteer` 16.9% — both ablations *above* shipped, `nosteer`
> with the most trips, every deliveries figure inside the instrument's own 3.6x
> noise floor, and the shipped circuit beating both cut arms on **2 of 6** seeds,
> which is chance. **This reverses the 2-of-3 reading below**, exactly as the
> lane that produced it warned it might: laden-at-door spans 0.1%–47.7% within
> one arm, so three seeds cannot separate medians 1.7 points apart. The
> pre-registered condition is met, so **step 4 is not cancelled.**
>
> Read it with §7.20's direct measurement on the other bed — 0.051% of carrying
> ticks reaching the nest band, **+0.0002 net homeward cells per carrying tick** —
> and with **§R4**, because the two beds disagree by two to three orders of
> magnitude and the difference is *slope*. The return leg is broken worst on flat
> ground, which is the geometry the loop needs and the one `Turn` cannot steer
> in. Design the bearing around that rather than discovering it afterwards.

### Step 2 (as originally written, kept for the record)

**Do:** `nesthome scene=bed arm=shipped|noemit|nosteer`, **six seeds**, on `u16`.

**Accept / decides:** if laden-at-door is **not** at floor level on the seeds
where the round trip fails, **step 4 does not get built.** That is the cheapest
possible outcome and the one to check for before spending a lane. On the three
`u16` seeds already measured the shipped homing circuit **beats both cut arms on
2 of 3** — laden-at-door 8.7 / 1.9 / 1.2 on seed 1, 30.7 / 13.7 / 16.8 on seed 3
— so this is a live possibility, not a formality. Clear `nesthome`'s own **3.6×**
noise floor between mechanically equivalent arms before calling anything a move.

This is not an invention of this document. `nest-design` §12 pre-registered it:

> **§9 item 4's "then build C, the home bearing"** drops from *next* to
> **conditional**: run the §5.1 sweep on `main` after #450 lands, **six seeds**,
> and build the bearing **only if laden-at-door still reads as floor-level on the
> seeds where the round trip fails.**

> ⚠️ **Do NOT dial `DIFFUSE` on channel A to 0.02.** An earlier draft of this
> document made that step 2, calling it *"the only lever with a positive homing
> result"*. It was **formally withdrawn by its own source** — `nest-design` §12:
> *"§5.3 and §9 item 4 are superseded … **Withdraw the '0.02 on A' suggestion**"*,
> restated in round-37 Lane 3. The `u16` widening bought the trail lifetime
> (**144 → 1,476 frames**; steering **36 → 1,080**) without the trade §5.3 priced.
> §5.3 is dated 2026-09-14, so §8's pre-widening bar applies to it. Three further
> reasons: its deliveries move on only **2 of 3** seeds (seed 3 *falls*
> 9,141 → 7,342), three seeds is not a sweep, and movement that size sits inside
> the 3.6× noise floor. `trailfollow` has no `diffuse=` argument at all, so the
> command that draft printed would have been **silently ignored** — §8's trap.

### Step 3 — deposit on the vacated cell

**Why:** within one tick `sense` reads `here` at the dispatch cell
(`creature.rs:4615`), `step_chain` moves (`:4209`), and the deposit lands on the
**new** head (`:4223` `if moved`, `:4284`). Next tick the ant is dispatched there,
so `here` holds its own deposit. With `ahead` empty that gives
`along = −1813/(1813+256) = −0.876` and **−4.20** on `Move`'s pre-squash sum — the
same magnitude as the entire swing that pair can produce (±4.29, derived).
**This repo already paid for this on channel B** (`brain.rs:1256`: *"the strongest
thing they could smell was the food trail they were themselves laying"*).

**Do:** the ant moves P→Q; deposit at **P**, not Q. `(x, y)` is in scope at
`creature.rs:4223`. No new state, P-11 untouched, trail shape unchanged but
registered one cell back.

**Not** *"subtract your own deposit"* — intractable: the amount is discarded, the
plane decays (`DECAY_RHO = 0.03`) and diffuses between write and read so a stored
figure over-subtracts, and a nestmate on the same cell breaks it.

**Two caveats.** It is **not** a standing homeward bias — only where `ahead` is
empty; walking back down its own trail both are high and `along ≈ 0`. So the real
effect is a **penalty on stepping onto virgin ground**, pushing `Tumble` over
`Move` (`:4218`) — which bears on **discovery**, not homing. And **1,813 is
derived, not measured** (§5) — measure it from step 1's probe before quoting.

**Accept:** determinism holds (P is a pure function of state) — verify
`sense_read_rects` (`:5422`) with `ParMode::Verify` (`:3877-3896`) rather than
assuming. `homeA` polarity unchanged at ≈ +0.115.
**Cost:** this shifts channel A's spatial registration for every ant, so **every
number in §3 and in §7.11–§7.18 must be re-taken.**
**Decides:** whether reach and `ate J` move. Expect discovery, not deliveries.

### Step 4 — `HomeBearing` / `HomeDistance`

**This is `nest-design` §8 option C, designed and priced 2026-09-14, never
built.** It is the only option under which a laden ant sixty cells out with
nobody near finds home.

**The state already ships.** `forage_anchor: (i32, i32)`
(`organism.rs:5888`, verified) and `forage_max` (`:5891`) are anchored at spawn
(`creature.rs:1509`), updated per move (`:9505-9512`) and **re-anchored at every
nest contact** (`:9516`). The inputs are `(anchor − head)` — **zero new state,
zero accumulation.**

**Do not accumulate displacement** at the deposit site: `step_crossing` and
`step_flight` return early at `:3863-3868` *before* it, so a flying or
trunk-crossing ant displaces without incrementing and the integrator drifts.
`step_chain` returns `bool`, not a delta. `since_nest` is not reusable —
`:9522-9532` records its visit guard firing exactly once per lifetime.

**Drive `Turn`**, as the four existing bearings do (`PreyBearing`,
`creature.rs:4901`, verified). That keeps it off `Move`'s crowded sum.

> **BUT READ §R4 FIRST — it is OPEN and it is about this exact wiring.**
> *"`BrainOutput::Turn` is nearly inert for a surface walker on level ground"*
> (`open-bugs-handoff.md:7423`). On a flat floor the downward diagonal fails
> `passable` and is zeroed; the upward one fails `body_has_foothold` and loses
> its footing bonus. **"Both outer candidates therefore lose, at every `Turn`
> value."** Reproduced with `(PreyBearing, Turn) = −2.5` wired and the eye
> reporting prey on 71% of casts: **byte-identical movement.**
>
> **`trailfollow mode=gap` builds a flat floor** — `LabBox { ground_y: 96,
> soil_depth: 48 }` (`trailfollow.rs:719`). So it is the degenerate case, and
> `nest-design` §8C was priced for the lab bed, which has slopes.
> **Either move step 4's measurement to `nesthome scene=bed`, or land §R4's own
> recommended counter first** — how often a `Turn` request is discarded because
> the side it asked for scored zero.
>
> **And state the sign.** `brain.rs:896-902`: positive `Turn` biases *left*, so
> an authored instinct toward a bearing is a **negative** weight. A sign error
> yields "no deliveries", which is indistinguishable from "the mechanism failed".

**`Carrying` is the wrong gate as it stands.** `creature.rs:4760` reads
`crop_fill.max(spoil ? 1.0 : 0.0)`, and in every arm that never finds food
`Carrying` is **100% spoil**. A `Carrying`-gated homing steer therefore
reproduces §7.14's `homeA` exploration clip *by construction*. Either run the
measurement arm at `SPOIL_IS_CARGO=0` (a non-shipped build — say so), or land the
food/spoil split input first; §7.14 names it and prices it at 24 live slots.
`Carrying` is also **graded**, not boolean, so a `Bias −45 / Carrying +45.5` gate
is fully open only at 1.0 — which is what a spoil pellet reads.
**`HomeDistance` is not decoration** — it is how the animal weights the two cues
(§6), and the term that makes trail and vector cooperate rather than compete.

| must change | site (verified 2026-09-17) |
|---|---|
| `BRAIN_INPUTS` 30 → 34 (both bearings + both `*Here` from step 5, **one bump** so `mutation_rate` is re-derived once) | `brain.rs:43` + doc `:35-42` |
| `INPUT_NAMES` — **compile error** if missed | `brain.rs:245` |
| `INPUTS` — **compile error** if missed | `brain.rs:1302` |
| `genome_manifest()` pin | `brain.rs:2184` |
| `live_slots()` pin **870 → 966** | `brain.rs:2055` |
| `mutation_rate` → **0.0032919** in **six** files | `ant.ron:718`, `ancestor.ron:620`, `beetle.ron:225`, `flitter.ron:809`, `hopper.ron:743`, `longant.ron:862` |
| the two *"measurement only"* docs — both say reading `forage_anchor` means the homing model changed and the doc is a lie | `organism.rs:5871`, `creature.rs:9504-9506` |

`live_slots = 16·I + 8·I + 8 + 16·8 + 14`, which reproduces the current **870** at
`I = 30`. **Both derived numbers rest on a derived numerator** — confirm `3.18`
against `brain.rs:2007`'s doc history and take the real count from the failing
pin. At `I = 32` (step 4 alone) the values are **918** and **0.0034641**.

Adding inputs **at the end shifts no existing slot** (`io_slot` is
`output · INPUT_SLOTS + input`, `INPUT_SLOTS = 64` fixed). Nothing in `assets/`
carries a `genome_manifest` stamp, so neither load site fires. Off-repo
`specimen.rs` jars with `layout: None` hard-fail `StaleGenome`.
`plainspeak.rs:1595` passes with **2 characters of slack** — any new input name
**≥ 15 chars fails it**.

**Wiring:** `Carrying × HomeBearing → Turn` through a gated pair, per
`nest-design` §8C. Ship it **exact**, no noise dial — watch it and add an error
term only if it reads as teleporting; the ethos judges that by eye.

**Accept:** `carry@nest` and `trips` rise on an order statistic over 12 seeds,
**and** a seed sweep shows no regression in reach. Guards expected red until
updated: `brain.rs:2007`, `:2058`, `:1652`.
**Decides:** this is the plan's main bet. If deliveries do not move here, homing
was never the constraint and §7.13's discovery finding is the whole story.

### Step 5 — a trail-concentration sense

**Recruitment, not homing.** Judge on reach and discovery, **never on delivery
counts**, and keep it out of any arm measuring step 4.

- `*Front` reads one cell at `sensor_offset: 6` **along the heading**
  (`creature.rs:4581-4593`) — directional, not the isotropic "am I standing on
  it" the theory wants, and its designed partner `*Lateral` is dead for surface
  walkers (the Jones/Physarum dead end). `here` is **already fetched** at
  `:4615`; expose it as `BrainInput::PheroAHere`/`PheroBHere` — ~2 lines, folded
  into step 4's bump.
- **Unsigned input → a single gated unit**, not the ± pair: `Bias −45,
  Carrying +45.5, <concentration> +w`. The ± idiom at `ant.ron:1793-1798` is
  antisymmetric because `along` is *signed*; a concentration is 0..1. `ant.ron`
  uses hidden units 0–6 with `BRAIN_HIDDEN = 8`, so **unit 7 fits exactly.**
- **Derive the weight.** Normalised by `Scent::MAX`, realistic trails read
  **0.059–0.216**, so `w ≈ 30`, not 6. Side effect: `GATE_DOMINANCE = 3.0`
  (`plainspeak.rs:493`) — the lab's cell page stops calling it a conditional.
  Cosmetic; note it in writing.
- **Write out what the unit computes** at `v ∈ {0, 0.06, 0.13, 0.22}` ×
  `Carrying ∈ {0,1}` **before running anything.**
- **In its favour:** `PheroAFront`/`PheroBFront` are written every tick and read
  by **no species** — the dead-weight half of this repo's writer/reader rule
  (`.claude/rules/src-sim-cells.md`).

**Accept:** reach histogram shifts right on an order statistic; `ate J` rises.
**Steps 4 and 5 must not land together** — both add a term to a shared weighted
sum, and `CLAUDE.md`'s *"a correct mechanism at inherited constants is a
regression"* makes the joint result unattributable. If both inputs arrive in one
`BRAIN_INPUTS` bump, land the bump with **step 5's weights at zero**, then wire
them in a second commit.

### Step 6 — docs

§7.19 in `pheromone-trail-direction-2026-09-16.md` correcting §5's items 1–6 in
place (**do not delete them**). `wiki/ants.md`: *"a colony paints its own map
outward from home"* is the premise being retired — with a real date, never "this
build".

### Things deliberately not in this plan

- **A nest-sourced `Spread::ActiveSpace` field on channel A.** The max-filter
  distance primitive already ships for `Channel::Alarm` (`pheromone.rs:334`);
  sourced at `Scent::MAX` with `fall ≈ 3 × SCALE` it spans ~21 cells, ~728 for 90.
  It cannot invert and is the cheapest thing to build — but it has no biological
  counterpart and moves homing out of the genome. **Held in reserve** if step 4
  proves too costly.
- **A static distance field** (`nest-design` §8 option D): straight-line distance
  through rock is not a route in a side-view world with galleries.
- **Re-tuning `DECAY_RHO`**: measured **inert** — a one-cell line loses 16.7% per
  pass to `DIFFUSE` against decay's 2.9%.
- **Halving `DEPOSIT`**: P-14's trigger has never fired.

## 7b. Where the code stands, 2026-09-18

**PR #464 MERGED** (`c40c1712`), so everything the 2026-09-17 version of this
section listed as "on the branch, not on `main`" **is now on `main`** —
`SPOIL_IS_CARGO`, `try_swap_with_kin` and its scheduler fix, the birth-path
attribution, the odometer doc corrections. Work since then is on
**`claude/laughing-davinci-f6lu1r`** (§7.19–§7.26), which is instrument and
report only: `probe_full` in `creature.rs`, an `#[ignore]`d readout in
`brain.rs`, and the `trailfollow` columns. **No ant behaves differently yet.**

The 2026-09-17 text follows, for the branch history it records.

Branch **`claude/vibrant-mayer-9o2dbx`**, PR **#464**, 41 ahead of `main`, 0
behind, CI green on `b1b3309e`. ~~Nothing here has landed on `main`.~~

**Engine changes already on the branch** — a new session inherits these:

| change | commit | state |
|---|---|---|
| `SPOIL_IS_CARGO` env gate on the `Carrying` fill | `45642414` | **default ON = the shipped bug**, so the repair is the `=0` arm. Cargo sensing is a real defect (§3.8) but does not move transport |
| `try_swap_with_kin` — ants may trade places with a nestmate | `5a180d99` | off by default; `passes_through_kin` in `src/lab/params.rs` so the owner can switch it in the lab |
| kin-swap scheduler fix (the swap orphaned the displaced ant's site) | `5824fd1d` | required by the above; without it displaced ants freeze |
| birth path booked every meal against `empty` | `6c184568` | **attribution only** — joules identical across the fix, verified byte-identical |
| odometer doc corrections in `creature.rs` and `ant.ron` | `fed1b76d` | §5 items 7–8 |

**Harness surface** (`examples/trailfollow.rs`): `mode=` (`gap`/`loop`/`arith`),
`gate=` (`b2`/`saturated`), `gaps=`, `seeds=`, `seed0=`, `arms=`, `ants=`,
`food=`, `frames=`, `relay=`, `near=`, `refill=`, `onlyfood=`, `larder=`,
`kinpass`, `dietdump`, `spec`, and the genome riders `recur=`, `emita=`,
`biasa=`. **Every rider asserts it is not a no-op** — passing the shipped value
is refused, which is what proves it is wired to the slot it names.

**Two harness faults fixed this session, both of which had invalidated runs:**
`refill` is now echoed in the header (it was not, and two different scenes printed
identical parameter lines), and `gaps=`/`seed0=` are now real knobs (they were
silently ignored).

## 8. Traps, each of which has cost real time here

- **The stale binary.** §7.15 and the odometer sweep measured a
  `target/release/examples/` binary matching no commit; a clean worktree build of
  the same SHA reproduced *different* numbers. **`cargo build --release
  --examples` in the same command as the run**, always.
- **`refill` was not echoed in the header.** A replenishing larder and a one-shot
  larder printed byte-identical parameter lines; six hypotheses (determinism, a
  six-value thread scan, contention, comment edits, `SPOIL_IS_CARGO`, corpse
  isolation) were spent before the *scene* was suspected. Fixed — the header
  prints it now. **A knob nobody can see the value of cuts both ways.**
- **A control that validates the knob does not validate the scene.** The rider
  controls before the odometer sweep all passed and were correct.
- **Unknown arguments are silently ignored** — `gaps=`, `seed0=` and `onlyfood=off`
  were each dropped at least once, and each time the run looked fine.
- **`cargo test --lib` reaches the brain and species guards** (they are in-lib);
  `tests/` holds only `determinism.rs` and `worldgen.rs`. Run full `cargo test`
  anyway for `determinism.rs:344`, which runs live creatures.
- **Every pre-2026-09-15 measurement is on the `u8` engine.** Round 37 Lane 3.
- **§R4: `Turn` is near-inert on flat ground**, and `trailfollow mode=gap`'s bed
  is flat. Governs step 4 — see there.
- **`deliveries` is not a transport metric.** `nesthome`'s own header: a delivery
  is any drop 8-adjacent to nest material, so *"a delivery there is an ant eating
  beside its door"*, and it *"cannot rank footprints on its own"*. Its noise floor
  is **3.6×** between two mechanically equivalent arms.
- **`AtNest` is about to change definition.** `nest-design` §13 (owner ruling,
  2026-09-15) retires `nest` as a material and makes `AtNest` a site test.
  `AtNest` charges the odometer that lays channel A, and `adjacent_nest`
  re-anchors `forage_anchor` — so **every channel-A number and step 4's own input
  source move under it.** Round 37 Lane 2 is building it concurrently.
- **`nesthome scene=bed` has two harness traps**: a genome patched into `Lab`'s
  world *before* `load_scenario` is discarded by `reset()` (three arms came back
  byte-identical), and a surface scan from row 0 finds the **lid**. Both produce
  the tidy-result tell.
- **A knob whose echoed name is not its accepted name is worse than an unknown
  one**, because the header reads as confirmation. `seed0=` was printed while the
  parser read `arg("seed")`.
- **`cargo test --release` in full exceeds the 600 s Bash cap.**

## 9. Instruments

`trailfollow` (can this animal read a trail, does a laid one move the colony —
`mode=gap`, arms `hand`/`hmute`/`self`/`mute`/`homeA`, `refill=`, `onlyfood=`,
`recur=`, `emita=`, `biasa=`) · `nesthome` (what homing rests on; `arm=noemit`,
`arm=nosteer`, `diffuse=`, `width=`) · `pherolife` (how long a trail lives) ·
`pherocost` (what one pass costs) · `pherowire` (which species read and write the
planes) · `onetrail` · `quantgrad` (does a decaying scalar still read as a
gradient) · `ant_ablation` (is the authored brain doing anything) · `colonybooks`
(what the colony lives on, in joules).

## 10. Commands

```
RAYON_NUM_THREADS=4 cargo build --release --examples && \
  RAYON_NUM_THREADS=4 ./target/release/examples/trailfollow \
  mode=gap gate=b2 gaps=90 seeds=12 arms=hand onlyfood=on larder=fruit \
  food=200 frames=24000 refill=2000
cargo clippy --all-targets --release --locked -- -D warnings   # unpiped, read the real exit code
cargo test                                                     # not --lib, for determinism.rs
bash scripts/docscheck.sh
python3 scripts/review.py serve --open                         # the owner judges deliveries by eye
```

**The bar is deliveries, judged by eye.** For creatures `filmstrip gif=1` is the
only instrument — an ant is two dark cells picked out by motion, so a grid of
stills cannot answer any question about it. Post with the delivery count in
`meta`.
