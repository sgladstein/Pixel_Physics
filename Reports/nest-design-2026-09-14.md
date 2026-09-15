# What a nest should be — research, measurement, and a recommendation

*2026-09-14. Round 36's research lane, on the brief
[`evolution-lab-nest-question-2026-09-14.md`](evolution-lab-nest-question-2026-09-14.md)
(PR #441).
**Nothing is built here**: one harness (`examples/nesthome.rs`), no `src/`
change, no default moved. Every claim the recommendation rests on was
re-checked in the tree with the command beside it — §1.*

The owner's brief, in his words: *"it should be attached to a world location,
not a material and it should be a circle or blob so if the ground gets dug,
it can still be reached … but do more research first … I don't want you to
take my suggestions exactly."* Both proposals were treated as hypotheses.
One survives, one does not, and the measurement that decides them is the
same one that overturns the sequencing the brief proposed.

## 0. The answer

*Re-measured 2026-09-15 against PR #450 (the scent planes widened to
`u16`): §12. The homing numbers in §4–5 are the `u8` numbers and are now
dated; the site, blob, crust and footprint findings are not. Read §12
before quoting §5.1 or §5.3.*

**Attach the nest to a site, not to a material — his first instinct is right,
and the engine is already two-thirds of the way there.** `World::NestSite`
holds the position and the odour, four readers already find it through
`nearest_nest_site`, and every per-ant quantity a site test needs is already
in `OrganismState`. What is missing is one function: `adjacent_nest` asking
*"am I within the site's reach"* instead of *"is a nest cell 8-adjacent to
me"*. It is cheap (§8), it breaks nothing in the held world (§10), and it
is what lets the crust be fixed (§6).

**A blob is not the shape, and the case it defends does not arise today.**
The complaint is that the ground under the door gets dug away. Measured over
120,000 frames on three seeds, the patch loses **0, 0 and 4 cells** — because
the crust makes it undiggable by any shipped or evolvable jaw (6.0 against a
ceiling of 2.0). The blob defends against a failure the crust suppresses, and
**the moment the crust is dropped the failure will arrive** — which is the
real argument for the site, not the blob: a site is not in the ground, so it
cannot be dug out of it. Its footprint should stay the *strip* the colony
has now; a footprint wider than the band **kills the colony**, monotonically,
on 3 of 3 seeds (§5.2).

**"Nests have never seemed to function" is right, and of his two readings
the measurement picks the second.** On the played bed, cutting the homing
circuit out of the genome entirely — no ant lays channel A, or no ant reads
it — leaves deliveries **inside the noise between two mechanically identical
arms** across six seeds (§5.1). Nothing steers a laden ant home today. The
reason is not that ants prefer food: laden ants are almost never at the food
either (0–10% of laden ant-ticks); they are *near* the door and off it. The
reason is that the homing plane is gone before it can be read — channel A
stands at 3,421 units at frame 10,000 and **0–9** from 20,000 on, the same
144-frame lifetime Lane C measured for a trail, because the homing plane *is*
a trail that every ant lays and nobody re-lays far from the door. Give the
plane a longer life (`DIFFUSE` on channel A alone, 0.25 → 0.02) and the
laden-at-nest fraction rises on **3 of 3 seeds** (1.6% → 14.5% on the seed
where the round trip fails). So **the pheromone work is a homing fix**, and
it is the only one being made.

**The sequencing in the brief is half right and the half that matters is
wrong.** The trail's lifetime does gate homing — but it gates it because
homing was built *on* the trail, and a home bearing built on the per-ant
anchor the engine already keeps would not depend on the trail at all. The
nest-identity work (the site) does not depend on it either. Both can start
now; neither waits for Lane C.

**The crust (`penetration_resistance` 6.0 against every shipped jaw at 1.0,
ceiling 2.0) is a balance choice standing in for a design** — it is what
keeps a material door alive, and the site retires it (§6).

**On dials, per the owner's note today: values are picked where a
measurement picks them, and a dial is proposed only where none does.** §9
names each.

## 1. What was verified, and how

Each claim the brief made that this report rests on, with the command that
checked it. Where the brief was wrong, the correction is here and nowhere
else in the report is the wrong form repeated.

| claim | check | verdict |
|---|---|---|
| `AtNest` is an 8-neighbour material test | `sed -n 6629,6634p src/sim/creature.rs` | **true** — `NEIGHBOURS_8.iter().any(\|&(dx,dy)\| world.get(x+dx,y+dy).material == nest)` |
| "Since 2026-09-02 this decides nothing" (its own doc) | `grep -n "AtNest\|adjacent_nest" src/sim/creature.rs` | **true of the verbs, false of the circuit.** Nine readers: brain input 10; the cohesion blend (`:3639`); the room-gated `Crowding` input (`:4500`); `at_nest_ticks`; the delivery counter (`:7579`); the reversal counter (`:8816`); the `since_nest` reset and `forage_anchor` re-anchor (`:8998`); and in `ant.ron`, the odometer's charge wire `(AtNest, 4, 0.05)` and the chamber-dig gate `(AtNest, 5, 30.0)`/`(AtNest, 6, 30.0)` |
| `NestSite` holds `x, y, scent, seeded, drift_epoch` and **nothing reads it for location** | `grep -n "nearest_nest_site" src/sim/*.rs` | **half wrong.** `World::nearest_nest_site` (`world.rs:6873`) is called by `step_nest_room`, the `Crowding` input, `blend_with_nest` and `carry_nest_wander`. The site *is* read for location — for **attribution**. Nothing **navigates** by it. That is the true gap, and it is narrower than the brief says |
| Homing is "the channel-A gradient scaled by nest-touch recency" | `sed -n 1770p assets/species/ant.ron; grep -n "recurrence" -A 2 assets/species/ant.ron` | **true, and the scaling is three genome weights**, not Rust: `(AtNest, 4, 0.05)`, recurrence `(4, 0.99995)`, `(4, EmitA, 32.0)`. A laden ant follows A through units 0/1: `(Carrying, 0, 45.5)`, `(PheroAAlong, 0, 6.0)`, `(0, Move, 2.5)` and the mirror. `creature_tick` multiplies nothing (`creature.rs:3940-3990`) |
| The patch is a one-cell skin, ~53 columns, every third a drain | `sed -n 3234,3275p src/sim/creature.rs; grep -n "^const COLONY_HALF_WIDTH\|^const DRAIN_PERIOD" src/sim/creature.rs` | **true**: `COLONY_HALF_WIDTH = 26`, `DRAIN_PERIOD = 3`, one `colony_surface` cell per column |
| Crust 6.0, jaw 1.0, beetle 0.3, ceiling `1.0 + allele_bound` | `grep -rn "penetration_resistance" assets/materials/nest.ron; grep -rn "^\s*dig_force" assets/species/*.ron; sed -n 2265,2272p src/sim/creature.rs; grep -n "^pub const DIG_FORCE_SPAN" src/sim/creature.rs` | **true, and the ceiling has a number**: `dig_force_of = def.dig_force + trait.clamp(±bound) × DIG_FORCE_SPAN`, `bound = trait_reach` for arms-race slots, `trait_reach` defaults to 1.0, `DIG_FORCE_SPAN = 1.0` → **2.0 at the default reach**. `TRAIT_REACH_MAX = 8.0`, so a box dialled to maximum reach could reach 9.0. Every shipped species authors 1.0 (beetle 0.3, worm none) |
| The 414-delivery scene, and "the patch is narrower than the band" | `sed -n 1571,1730p examples/ascii.rs; grep -n "414" Reports/creature-direction.md` | **true, with the geometry**: 512×120, worldgen default preset seed 1, patch `for x in 16..90` (**74 columns**, no drains — hand-painted, not `paint_nest_patch`), 55 ants *attempted* at `x = 24 + 4i` (band 24..240) of which **15 place**, six trees from x=230, litter from x=200, 12,000 frames. 414 was 2026-08-23's reading; it read 348 after the tree-eating fix (§13g), and `instruments.md` prices the column at 154–980 over six seeds of an unchanged ant |
| Lane C: `DECAY_RHO` inert, `DIFFUSE` the lever, 144 frames vs 2,200 | `grep -n "144 frames\|2,200" Reports/pheromone-lifetime-and-wiring-2026-09-14.md` | **on `main`** (`e01dd1a1`), numbers as quoted. The 144 is an **unreinforced one-cell line**, and §4–5 measure that it transfers to the homing plane exactly: the plane is a trail |
| "3.5 s against a 37 s round trip" (druid lane) | `grep -rn "3\.5 s\|37 s" Reports/lanes/druid-*.md` | **not in the tree.** No lane note on `main` carries it; it is the same 144 frames (2.4 s at 60 Hz) restated. Treated as Lane C's number |
| `Druid::found_colony` places at the gnome's feet | `sed -n 1290,1300p src/druid/mod.rs` | **true**: `player.center()` → `found_colony_of(x, y, COLONY_SPECIES, COLONY_SIZE)` |
| `a_nest_still_stops_him` asserts nest is a wall; `is_tool_target` is any non-bedrock `Solid` | `sed -n 5595,5612p src/sim/player.rs; sed -n 428,431p src/sim/rigid.rs` | **true**; the test builds a **4×28 wall** of nest, a shape nothing in the game paints (the patch is one cell thick, horizontal) |
| Nothing in `dead-ends.md` has tried a site- or vector-based home | `grep -n -i "home vector\|path integration\|nearest_nest\|NestSite\|adjacent_nest\|AtNest\|forage_anchor\|beacon\|nest marker" Reports/dead-ends.md` | **two hits, both `nest.ron` water** (`water_capacity` on a `Solid`; making it a `Powder`). The mechanism is untried |
| A second nest can be founded | `grep -rn "paint_nest_patch\|found_colony_of" src/ \| grep -v "//"` | **only by a player verb**: `Druid::found_colony`, the lab's animal jar (`lab/mod.rs:3291`), scenario files and tests. No creature verb founds a nest; the fission design's satellite is design, not code |

## 2. How real ants find home

The owner guessed *"proximity to the queen"* and doubted it. He was right to
doubt it: **no ant homes on the queen.** Queen pheromone regulates
reproduction and is carried around the nest on workers; it is not a beacon
and a forager never smells it from the field. What a forager actually uses,
in the order that carries it, with what each costs an animal to hold:

1. **Path integration — the home vector.** The forager keeps a running
   estimate of the straight line back to the nest from its own turns and
   distances, using the sky's polarisation pattern as a compass and step
   counting as an odometer. *Cataglyphis fortis* runs almost entirely on it,
   will run the accumulated vector off when displaced, and when the vector
   reaches zero and there is no nest, starts a **systematic search** — an
   expanding spiral centred on where the nest *should* be
   ([Wehner & Srinivasan 1981](https://link.springer.com/article/10.1007/BF00199474);
   survey in [Myrmecological News 11](https://myrmecologicalnews.org/cms/index.php?option=com_download&view=download&filename=volume11/mn11_53-62_printable.pdf&format=raw)).
   **State: one 2-vector per ant**, accumulating error with distance, reset
   at the nest. It needs no marker in the world.
2. **The nest's own plume, at short range.** *Cataglyphis* pinpoints the
   entrance by following the nest's CO₂ plume — and only once path
   integration says it is near: while the vector still reads "not home",
   plume following is **inhibited**, which stops an ant walking into a
   foreign nest en route
   ([Buehlmann, Hansson & Knaden 2012, *Current Biology*](https://www.cell.com/current-biology/comments/S0960-9822(12)00178-9);
   [2025 follow-up](https://pmc.ncbi.nlm.nih.gov/articles/PMC13377868/)).
   **State: a gradient in the world**, effective over metres, not the
   foraging range. This is the real analogue of `AtNest` — a *local*
   confirmation.
3. **Home-range marking.** *Lasius niger* coats the nest's inner walls, the
   entrance and the foraging arena around it with its own colony-specific
   cuticular hydrocarbons, so the ground near home *smells like the colony*
   and workers behave differently on marked ground
   ([Lenoir et al. 2009, *J. Chem. Ecol.*](https://link.springer.com/article/10.1007/s10886-009-9669-6);
   [Devigne & Detrain, *Insectes Sociaux*](https://link.springer.com/article/10.1007/PL00012659)).
   **State: a slow, persistent field**, colony-keyed, decaying over days.
   This engine has the colony-keyed half already (`NestSite::scent`, the
   blend) and not the field.
4. **Trail pheromone** — a corridor laid by recruiting foragers, the
   Deneubourg double-bridge mechanism
   ([Deneubourg, Aron, Goss & Pasteels 1990](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC3400603/)).
   It is a *recruitment* channel, not a homing one: an ant that laid no trail
   still gets home by 1–3.
5. **Visual panorama** — the skyline as seen from the route, learned on
   outbound walks
   ([Graham & Cheng 2009](https://link.springer.com/article/10.1007/s00359-009-0443-6)).
   No analogue here and none proposed.

**The shape that follows is the standard one: a long-range egocentric
estimate that accumulates error, corrected at short range by something
local and colony-specific.** Three consequences for this engine:

- **Path integration is degenerate here, and that changes what it buys.** A
  real ant integrates because it has no absolute position. An ant in this
  engine *has* one: `OrganismState::forage_anchor` is already the world
  coordinate of its last nest touch (`organism.rs:5862`, re-anchored on every
  contact), so its home vector is `anchor − head`, exact, no drift, no
  search. That is cheaper than the biology and also *less* than it — the
  error and the spiral are what make a real return look like searching
  rather than teleporting. The doc on that field says *"the moment one
  [creature] reads this, the homing model has changed"*; §8 prices that
  change and says why not yet.
- **The engine's homing plane has item 3's shape and item 4's lifetime.**
  Channel A is laid by *every* ant, scaled by its own odometer, so it is
  meant as a colony-wide home-range field, freshest at the door. But it is
  laid on a plane that forgets in 144 frames (Lane C), and far from the door
  the odometer's deposit is single digits (§3.1, §4), so it stands only
  where ants are dense — the first few thousand frames, on the patch — and
  is **gone by frame 20,000 on the played bed** (§4). A home-range mark in
  the biology persists for days; this one persists for two seconds.
- **`AtNest` today is item 2 done as a contact test**, and the door's own
  plume — the site's odour, already in `NestSite::scent` — is the natural
  thing to widen it into.

## 3. What this engine does today

### 3.1 The circuit

```
AtNest (8-neighbour material test)
  └─ hidden 4: charge 0.05, recurrence 0.99995 ──▶ EmitA × 32   (the odometer)
                                                       │
                       every ant, every successful move: deposit A ∝ odometer
                                                       │
  Carrying ─▶ hidden 0/1: ±PheroAAlong × 6 ──▶ Move ±2.5   (the laden ant walks up A)
```

No ant ever asks where the nest is. The material's whole job is to recharge
the odometer on contact; the *field* answers "which way". Two things fall
out: **the nest could be anywhere the odometer can be recharged**, and **the
gradient's shape is set by where recharging happens** — which is what the
"narrower than the band" constraint is about.

### 3.2 The site

`NestSite { x, y, scent, seeded, drift_epoch }` is registered by
`paint_nest_patch` *before* any ground is painted and is found by
`nearest_nest_site` — a linear scan over the (tiny) site list — from four
places, none of them a navigation. It already models the fission design's
*"a nest is a place that holds an odour"*. The odour is exactly the
colony-specific home mark of §2 item 3, minus the field.

### 3.3 The per-ant state

`since_nest` (ticks since contact; nearly dead weight, its own doc says so),
`forage_anchor` (last contact position), `forage_max` (deepest excursion).
Every quantity a site-based or vector-based rule needs is already carried.

### 3.4 The door, and what actually removes it

The complaint the brief quotes — *"if the actual material or cell moves, if
the ground is dug up"* — is a claim that the patch gets destroyed. Measured
(`examples/nestdoor.rs`, §T2, 120,000 frames, played bed, three seeds):

| | seed 1 | seed 2 | seed 3 |
|---|---|---|---|
| patch cells destroyed, unbroken patch | **0** | 14 | 10 |
| patch cells destroyed, drained patch (shipped) | **0** | **0** | 4 |
| cover on the lost/covered cells | `water` 47–49, `packedsoil` **1** | | |

**The door is not being dug and is barely being buried.** It cannot be dug by
an ant (crust 6.0 vs ceiling 2.0, §6), spoil lands on it at one cell in
fifty, and what actually took it away on seed 1 was a **film of water** —
the material's impermeability, fixed by the drains and still 1% open. The
gnome's pick *can* cut it (`is_tool_target`), and structural collapse under
a slope can drop it (the 4–14 above), and nothing repaints it.

So the case the blob defends — "the ground under the nest is dug away" —
**does not arise from the colony today**, and arises from the player only
if he digs there on purpose. It *will* arise the moment the crust is
lowered so a colony can excavate its own doorstep, which is the fix §6
recommends. **The two proposals are one proposal**: fix the crust, and the
material-nest starts eroding; attach the nest to a site, and it cannot.

## 4. Does the nest "do nothing"? — measured

The owner's note, 2026-09-14: *"nests have never seemed to function. That
could be because 1) creatures just move to where food is, or 2) issues with
pheromones not lasting long enough."* Both readings were given a number.
Instrument: `examples/nesthome.rs`, `scene=bed` — the lab's `played_bed`
scenario through `Lab`, the colony arriving from the timeline at frame
6,000, run to 40,000, `RAYON_NUM_THREADS=1`, every ant's head binned every
250 frames as *nest* (on the patch's columns), *food* (more than 96 columns
from the patch centre) or *mid*.

**Reading 1 — "they move to where food is" — is not what the ants do.**

| seed | ant-ticks on the patch | ant-ticks > 96 cols away | laden ant-ticks on the patch | laden > 96 away |
|---|---|---|---|---|
| 1 | 6.7% | 2.7% | 1.6% | 3.4% |
| 2 | 40.2% | 3.7% | 30.0% | 4.8% |
| 3 | 21.7% | 7.0% | 20.2% | 7.2% |
| 5 | 42.5% | 0.0% | 34.9% | 0.0% |
| 6 | 39.1% | 3.8% | 46.0% | 3.8% |

Laden ants are not at the food and mostly not at the door: they are in
between, within a hundred cells of home and off the patch. That is #350's
*"the nest is not where the colony lives"* restated per individual, and it
is what a colony with no working homing looks like — foragers pick up, wander,
and drop wherever `Drop` fires. On the seed where the round trip fails
(seed 1) **1.6%** of laden ant-ticks are at the door.

**Reading 2 — the plane does not last — is the one the measurement picks.**
Channel A over the run, seed 1, summed over the 128 columns around the
patch (the *near* band) and beyond:

| frame | shipped `DIFFUSE` 0.25 | channel A at `DIFFUSE` 0.02 |
|---|---|---|
| 10,000 | 3,421 / 0 / 0 | 4,572 / 0 / 0 |
| 20,000 | **0** / 0 / 0 | 68 / 0 / 0 |
| 30,000 | 9 / 0 / 0 | 66 / 50 / 0 |

The homing plane exists for the first few thousand frames, while the founders
are still packed on the patch and re-laying it, and then it is gone. The
arithmetic says why (§3.1): the odometer's unit sits near 0.20 at the door
and decays hyperbolically once off it, so an ant a hundred ticks out deposits
**9 of 255** per move and one a thousand ticks out deposits **1**; a
one-cell line loses 16.7% per pass to the blend (Lane C), so a 9 is gone in
about twelve passes — 144 frames — unless another ant re-lays it. Far from
the door nobody does. **The homing plane is a trail, and it dies like one.**

## 5. The homing circuit, the footprint and the plane's life — measured

### 5.1 Cutting the circuit changes nothing detectable

Three arms, the genome patched before the colony is founded: `shipped`;
`noemit` (the odometer's output leg `(4, EmitA, 32.0)` → 0, so no ant lays
channel A); `nosteer` (the steer legs `(0, Move, 2.5)` and `(1, Move, −2.5)`
→ 0, so channel A is laid and never followed). With no plane the steer pair
cancels exactly (`+2.5·h₀ − 2.5·h₁` at equal activations), so **`noemit`
and `nosteer` are the same colony** and their spread is the noise floor.
Deliveries at 40,000 frames:

| seed | shipped | noemit | nosteer | laden at door, shipped / noemit / nosteer |
|---|---|---|---|---|
| 1 | 230 | 194 | 153 | 1.6% / 1.6% / 0.0% |
| 2 | 2,530 | 3,703 | 1,029 | 30.0% / 34.6% / 15.2% |
| 3 | 9,141 | 5,559 | 6,256 | 20.2% / 18.2% / 25.4% |
| 4 | 818 | 585 | 243 | 100% / 96.1% / 93.7% |
| 5 | 2,319 | 2,527 | 2,019 | 34.9% / 43.8% / 30.2% |
| 6 | 1,203 | 1,887 | 3,664 | 46.0% / 52.4% / 47.2% |
| **median** | **1,761** | **2,207** | **1,524** | |

The shipped colony beats `noemit` on 3 of 6 seeds and the two identical
no-homing arms differ by **3.6x** on seed 2. The homing circuit's
contribution to deliveries is not distinguishable from zero at six seeds,
and the laden-at-door column says the same thing the other way: it does not
move when the circuit is cut. **The round trip that does close on the
played bed closes by ants living near the door and `Drop` firing, not by
anything steering them.** That is the strong form of "the nest does not
function", and it is the form the owner saw.

(The two hand-built scenes could not carry this measurement, and the reason
is recorded so nobody re-runs them for it. `ascii`'s foraging scene — the
one the 414-delivery figure comes from — places **15 of its 55 ants**
(worldgen seed 1 puts water and slope under the other 40; the ones that
land are at `x = 24…236`, seven of them on the 74-column patch), its
channel A reads 1,253 at frame 3,000 and **0** from 6,000 on, and its
deliveries climb 321 → 670 regardless: a delivery there is an ant eating
beside its door. `forage_probe`'s bank at the shipped 87-cell gap kills 49
of 55 ants by 8,000 frames and nobody reaches the pile (deepest excursion
28); at `gap=30` its deliveries are corpses shuffled at the door.
`creature-direction.md` §13g already calls the flat floor degenerate.)

### 5.2 The footprint: too narrow starves the door, everywhere kills the colony

`width=` repaints the scenario's patch to the given width at the frame it is
painted, centred on its centre. Deliveries and **animals alive at 40,000
frames**, shipped genome:

| seed | 12 | shipped (31–36) | 72 | 144 | 288 (the whole bed) |
|---|---|---|---|---|---|
| 1 | 0 · 34 | 230 · 59 | 145 · 38 | 152 · 19 | 221 · **2** |
| 2 | 303 · 52 | 2,530 · 41 | 1,252 · 35 | 1,558 · 19 | 2,680 · **15** |
| 3 | 2,480 · 82 | 9,141 · 89 | 7,845 · 72 | 11,834 · 54 | 9,645 · **28** |

Two things are clean in a very noisy column:

- **Deliveries do not rank footprints.** Past the shipped width they wander
  (down on 3 of 3 at 72, up on 2 of 3 at 144) because a delivery is *any*
  drop 8-adjacent to nest — at 288 every drop is one.
- **The colony's survival falls monotonically with footprint on every seed**:
  59 → 38 → 19 → 2, 41 → 35 → 19 → 15, 89 → 72 → 54 → 28. A home that is
  everywhere is a colony that is packed at home everywhere — `AtNest` gates
  the chamber-dig pair and the room census, and `(AtNest, Drop, w)` unloads
  every forager where it stands. **This is the "home has to be a place"
  constraint, measured as a mortality gradient rather than as a gradient
  to walk up**, and it is real. At 12 the other edge shows: the seed where
  the round trip fails delivers **nothing at all**.

So a footprint at the colony's own half-width is as good as anything the
sweep can see, and a *disc* has no measured advantage over the strip it
would replace — its one difference, counting the galleries under the door
as home, is untested and is the lever to test next (§9).

### 5.3 A longer-lived plane is the one thing that moves homing

`diffuse=` sets channel A's blend alone, through `set_channel_diffuse`
(Lane C), B untouched. Shipped genome:

| seed | `DIFFUSE` A 0.25 (shipped) | 0.08 | 0.02 |
|---|---|---|---|
| 1 — deliveries · laden at door | 230 · 1.6% | 183 · 5.9% | **664 · 14.5%** |
| 2 | 2,530 · 30.0% | 1,244 · 32.0% | **4,649 · 41.4%** |
| 3 | 9,141 · 20.2% | 6,919 · 24.6% | 7,342 · 25.1% |

Laden-at-door rises on **3 of 3 seeds at both settings**, and by a factor of
nine on the seed where homing was dead; deliveries rise on 2 of 3 at 0.02.
This is the positive control for §5.1 — the instrument moves when homing
does — and it is the owner's reading 2 confirmed on the bed: **make the
plane live and the nest starts to function.** What 0.02 costs the *food*
trail is Lane C's number (`DIFFUSE` 0.10 scores 0.623 on-trail against
0.25's 0.817), which is why it is a per-channel setter and why the value for
A is Lane C's to set rather than this report's; the number it needs is
here.

**What the sequencing claim gets right and wrong, then.** Right: homing
today is the trail, so the trail's lifetime gates it — measured. Wrong: the
nest's *identity* (site or material) is orthogonal to all of this, and a
home bearing built on `forage_anchor` (§8, option C) would decouple homing
from the trail entirely. Neither waits.

## 6. The crust: bug, balance, or the design's confession

`nest.ron` authors `penetration_resistance: 6.0` with the doc *"a plain
`Solid` — deliberately the simplest thing that renders distinctly and that an
ant can stand next to"*. Nothing in the file says why 6.0. It is above
gravel and shard (3.5), ice (2.5), sand (1.4) — the hardest thing in the
world after bedrock — and every shipped jaw is 1.0 with an evolutionary
ceiling of 2.0 at the default reach. **A colony cannot excavate its own
doorstep**, and `ascii`'s chamber scene asserts that ants dig through soil
and stop at *stone* — the nest is harder than what that scene calls the
wall.

Three readings:

- **A bug.** The number has no derivation and no test names it. The only
  consequence anyone has measured is the one the owner saw: ants at the
  surface do not dig down through the door, so galleries start beside the
  patch, not under it. `nest_room`'s census attributes roofed void *by
  column* to the nearest site, so a colony that digs beside its door still
  gets credit — the room signal survives the crust by accident of the
  attribution rule.
- **A balance choice.** It is what keeps the patch intact: with a diggable
  door the material would go the way of the ground, `adjacent_nest` would
  go false cell by cell, and the homing odometer would lose its charging
  station. Under the material design the crust is load-bearing. **That is
  the confession**: a home that has to be undiggable to survive is a home
  attached to the wrong thing.
- **The thing that makes the material design wrong.** Under a site design
  the crust has no job. `nest` can be given soil's 0.8 — or the patch can
  simply be *paint* on whatever ground was there (a palette swap the lab
  already does in `earth_toned_nest`) — and the colony digs its own hall
  under its own door, which is what a nest *is* in the wiki's terms.

**Verdict: a balance choice made in place of a design, and the site fix
retires it.** File it as its own bug letter only if the site work is not
taken up; otherwise it lands with it.

## 7. What other simulations do

- **NetLogo *Ants*** (Wilensky 1997; the canonical minimal model, and the one
  most colony sims descend from): the nest is a **location**, and homing is a
  **static field** — `set nest-scent 200 - distancexy 0 0`, computed once —
  that a laden ant climbs with a three-way sniff. Food recruitment is the
  only dynamic pheromone
  ([model page](https://www.modelingcommons.org/browse/one_model/1408);
   [source](http://web.eecs.utk.edu/~bmaclenn/Classes/420-527-S12/NetLogo3.1.5/Ants.nlogo)).
  That is the owner's first proposal exactly, and it is the field form of
  path integration: a distance-to-site scalar every cell can read.
- **SimAnt** (Maxis 1991): the nest is a place the queen founded; ants
  return by trail, and the player's yellow ant lays trails others follow
  ([Wikipedia](https://en.wikipedia.org/wiki/SimAnt)). Nests are moved by
  *founding* a new hill in another sector, never by the ground changing.
- **Empires of the Undergrowth** (Slug Disco, 2017–): nest entrances are
  fixed objects that teleport between surface and underground; foragers path
  on a navmesh toward player-placed pheromone markers and *"go back to the
  nest entrance if on the surface or to the queen if underground"* — i.e. a
  known target and a pathfinder, no field at all
  ([wiki](https://empires-of-the-undergrowth.fandom.com/wiki/Basic_Mechanics)).
- **Ant colony optimisation** (Dorigo et al., after Deneubourg 1990): the
  nest is a graph node; pheromone is the *recruitment* channel; return is
  by the agent's own memory of its path — path integration in the discrete
  form.
- **Falling-sand games** (Powder Toy, Sandspiel, Noita): no colony model.
  Powder Toy's `ANT` is Langton's ant. There is no prior art for a nest
  *inside* a cellular ground that the ground itself can erode — which is
  precisely this engine's problem and why the material design got tried.

**The pattern across all of them:** the nest is an object or a coordinate;
where anything homes by field, the field is a *distance to that coordinate*
and not a thing an ant lays; and the recruiting trail is separate from
homing. Nobody attaches "home" to a block of material. This engine's version
— a laid, decaying, colony-repainted home field — is the most biological of
the set (§2 item 3), and the material test under it is the odd part out.

## 8. The options, priced

Costs are in the sweep, per ant per decision tick (`tick_interval` 6), and
in what each breaks. "Breaks" includes the three held-world facts of §10.

| | what it is | sweep cost | what it breaks | what it fixes |
|---|---|---|---|---|
| **A. Keep the material, drop the crust** | `nest.ron` 6.0 → 0.8 | none | the door: a diggable material patch erodes cell by cell, `adjacent_nest` goes false, the odometer loses its charge, deliveries stop. Not shippable alone | the colony digs its doorstep |
| **B. Site-based `AtNest`** | `adjacent_nest` becomes "head within `COLONY_HALF_WIDTH` columns of `nearest_nest_site` and within a few rows of its surface"; `NestSite` gains the founding surface row; the material stays as *paint* | `nearest_nest_site` is a linear scan over the site list (one to a handful) against today's eight `World::get`s — **cheaper**, and no per-frame scan over sites: it runs only inside `sense`, which already runs | nothing in the held world (§10); `deliveries` semantics unchanged at the shipped reach; `nest_room` unchanged (already keyed by site) | home survives digging, water, spoil and the pick; makes A safe; makes *move* and *lose* one-line verbs |
| **C. Home bearing** | a `BrainInput::HomeBearing` — the signed along-heading of `forage_anchor − head` (or of the nearest site), the shape `KinBearing`/`PreyBearing` already have; wired `Carrying × HomeBearing → Turn` through a hidden pair like units 0/1 | one subtraction and a sign per tick; the anchor is already kept | **reallocates the `Move`/`Turn` sum** — `CLAUDE.md`'s rule: the constants calibrated against today's behaviour want re-deriving on a seed sweep gating an order statistic before it ships. A positional-law slot addition (allowed; `Stillness` was one) | a laden ant knows the way home whatever the plane does — the biology's item 1, and the only option that decouples homing from the trail |
| **D. Static distance field** (NetLogo) | a per-cell scalar rebuilt when sites change; ants read it through a new slot or the existing A slots | a full-bed pass (163,840 cells) per founding or move, none per frame | straight-line distance through rock is not a route in a side-view world with galleries — a chamber ten rows under the door reads "home" through fifty cells of tunnel; and it replaces the one thing the laid plane does well, follow the walked route | same as C, worse |
| **E. Keep the plane, lengthen its life** | `DIFFUSE` on A alone → ~0.02 | none | the food trail's tracking, if applied to B too — hence per-channel | homing measurably (§5.3) |

**What B does to the 414 number: nothing, at the shipped reach**, because the
footprint it tests is the footprint the material paints, and §5.2 shows the
number cannot rank footprints anyway. What it does to the *right* number —
animals alive — is set by the reach, and the reach is the strip the colony
already has.

## 9. Recommendation

In the order it should land, with the value each ships at and whether it is
a dial — per the owner's note today, a value where a measurement picks one,
a dial only where none does.

1. **Build B: the nest is a site.** `adjacent_nest` tests the nearest
   `NestSite`: `|hx − site.x| ≤ COLONY_HALF_WIDTH` (26, the constant that
   already sizes the patch) and `site.surface − 2 ≤ hy ≤ site.surface + 2`,
   `surface` recorded at registration by `colony_surface`. **Value, not
   dial**: this reproduces today's footprint, which is the one configuration
   whose numbers exist (§5.1–5.2). The depth is the untested lever — a
   deeper reach counts the galleries under the door as home, which is what
   #350's "the colony lives beside its door" is asking for — and `nesthome
   width=` generalised to rows is the run that would pick it. Until it is
   run, 2 is the value. `paint_nest_patch` keeps painting; the paint is what
   the player sees and the gnome cuts. **Founding with no paintable ground
   now founds a home** (the brief's own author called the alternative *"a
   harder failure to see than an empty patch"*).
2. **Retire the crust with it.** `nest.ron` `penetration_resistance` 6.0 →
   **0.8**, soil's, because the door is worked soil. A value. The colony
   digs its own hall under its own door and the site does not notice.
3. **Two verbs the site makes one-liners, and they are the ones the ethos
   asks for.** *Move the nest* (set `site.x/y/surface`; repaint optional) and
   *lose the nest* (remove the site: the odometer decays hyperbolically, so
   the colony's homing fades rather than stops — a graded death, not a
   binary). Both are player verbs in the lab and the held world. The lab's
   animal jar already paints; it should register a site when it lands on
   ground it cannot paint.
4. **Homing, in this order.** Lane C's per-channel `DIFFUSE` is the dial that
   exists; the number for A on the bed is **0.02 lifts laden-at-door 3 of 3
   seeds** (§5.3), and whether that costs the B trail is Lane C's to weigh.
   Then **C, the home bearing**, as its own change with its own seed sweep —
   it is the biology, it is per-ant state the engine already keeps, it costs
   nothing in the sweep, and it is the only design under which a laden ant
   sixty cells out with nobody near finds home. Ship it **exact** (no noise
   dial): watch it, and add an error term only if it reads as teleporting —
   the ethos judges that by eye, not by a constant picked in advance.
5. **Not a blob.** A disc has no measured advantage over the strip; a
   footprint wider than the band kills the colony on 3 of 3 seeds; the
   "still reachable when dug" property comes from the site, not the shape.

**What I would say to the owner in one line:** you were right that home
should be a place and not a material, right that the nest has never worked,
and right about *why* — the plane dies; the blob is the one part to drop,
and the fix to the crust comes free with the site.

## 10. The three held-world answers

1. **Founding at an arbitrary cursor.** Unchanged in every option: `found_colony_of` already calls `register_nest_site` first and paints second, and `Druid::found_colony` goes through it. Under the site design the paint can fail (water, a plant) and the colony still has a home — today a founding on ground `paint_nest_patch` declines leaves an odourless patch, which the brief's own author called *"a harder failure to see than an empty patch"*.
2. **The gnome and the wall.** `a_nest_still_stops_him` tests a 4×28 wall no verb paints. Under the recommendation `nest` stays `kind: Solid` as paint, so `footing` stays `Hard`, the test stays green as written, and `is_tool_target` stays true — **he can cut the paint and the colony keeps its home**, which is the walk-the-galleries feature working *better*: he can bore through a doorstep without evicting anyone. If the crust drops to 0.8 the pick's behaviour does not change (the pick has no `dig_force`; `mine_swept` is unconditional on non-organism `Solid`).
3. **Tell the druid coordinator first.** This report is the notice; the lane note carries the pointer. Nothing lands from this lane.

## 11. Appendix — commands

Every number in §4–5 comes from one binary, `examples/nesthome.rs`, built
from this branch with `cargo build --release --examples` (`set -o pipefail`),
run with `RAYON_NUM_THREADS=1`, three processes at a time on a four-core
container (counters only; no timing is quoted).

```text
# §5.1, six seeds x three arms
for s in 1 2 3 4 5 6; do for a in shipped noemit nosteer; do
  RAYON_NUM_THREADS=1 ./target/release/examples/nesthome scene=bed arm=$a seed=$s frames=40000
done; done
# §5.2, footprint
for s in 1 2 3; do for w in 12 72 144 288; do
  RAYON_NUM_THREADS=1 ./target/release/examples/nesthome scene=bed width=$w seed=$s frames=40000
done; done
# §5.3, the plane's life
for s in 1 2 3; do for d in 0.02 0.08; do
  RAYON_NUM_THREADS=1 ./target/release/examples/nesthome scene=bed diffuse=$d seed=$s frames=40000
done; done
# the two scenes that could not carry it
RAYON_NUM_THREADS=1 ./target/release/examples/nesthome scene=loop seed=1
RAYON_NUM_THREADS=1 ./target/release/examples/nesthome scene=probe gap=30 seed=1
```

Two harness faults were caught by the tidiness rule and are recorded in
`Reports/instruments.md`: a genome patched into `Lab`'s world before
`load_scenario` is thrown away by `reset()` (three arms byte-identical), and
a surface scan from row 0 on the lidded bed paints the widened patch on the
ceiling (four widths byte-identical). Both results looked like findings.

## 12. Re-measured on PR #450 — the planes widened to `u16` (2026-09-15)

PR #450 widened both scent planes from a byte to `u16` with no constant
tuned, and its own numbers say an unreinforced trail now lives 1,476 frames
against 144. That is the lever §5.3 reached for with `DIFFUSE` on channel A,
obtained by resolution instead of blend, so it costs the food trail nothing.
The question is whether it changes what this report found. Same harness,
same scene (`scene=bed`, 40,000 frames, `RAYON_NUM_THREADS=1`), built
against `claude/pheromone-u16` at `9899a2d2`, seeds 1–3, channel A quoted
in the old 0–255 units (`u16` total ÷ 256):

| seed | deliveries shipped / noemit / nosteer | laden at door, shipped / noemit / nosteer | channel A near band at 30,000, `u8` → `u16` |
|---|---|---|---|
| 1 | 71 / 34 / 24 | **8.7%** / 1.9% / 1.2% | 9 → **42** |
| 2 | 2,293 / 1,664 / 1,456 | 51.5% / 51.7% / 53.6% | 0 → **313** |
| 3 | 6,506 / 6,608 / 8,095 | **30.7%** / 13.7% / 16.8% | — → **201** |

**What changes.**

- **§4's "the plane is gone by 20,000" is a `u8` fact and is overturned.**
  On `u16` the home plane stands for the whole run: 42–313 old units in the
  near band at frame 30,000 against 0–9 before, and a mid band that reads
  nonzero for the first time (573 and 1,906 `u16` units on seeds 1 and 3).
- **§5.1's "nothing steers a laden ant home" is no longer true as stated.**
  With the plane standing, the shipped circuit puts more laden ants at the
  door than either cut arm on 2 of 3 seeds — 8.7% against 1.9/1.2, and
  30.7% against 13.7/16.8 — and ties on the third, where every arm sits
  above 50% because that colony lives on its patch. Deliveries move the
  same way on seed 1 (2–3x) and not on the others. **Three seeds is not a
  sweep** (the same table on `u8` needed six to say "nothing"), so the
  claim this licenses is the weak one: the circuit is now *detectable*,
  and the owner's reading 2 was the right one — the plane's life was the
  fault.
- **§5.3 and §9 item 4 are superseded.** The per-channel `DIFFUSE` on A is
  no longer the lever to reach for; #450 bought the lifetime without the
  trade §5.3 priced. Withdraw the "0.02 on A" suggestion.
- **§9 item 4's "then build C, the home bearing"** drops from *next* to
  *conditional*: run the §5.1 sweep on `main` after #450 lands, six seeds,
  and build the bearing only if laden-at-door still reads as floor-level on
  the seeds where the round trip fails. On this evidence it may not be
  needed, which is the better outcome — the trail was the homing mechanism
  all along, and now it works.

**What does not change.** The site over the material (§0, §8 option B); the
blob (§3.4 — the patch is never dug because of the crust, not because of
the plane); the crust (§6); the footprint mortality gradient (§5.2 — its
mechanism is `AtNest` gating the drop and the dig, not the plane; not
re-run here); the 414-scene and probe-scene findings (§5.1's parenthesis —
placement and a flat floor, not the plane); and the three held-world
answers (§10). The recommendation's order is unchanged except that item 4
is now #450 itself, landed or landing.

## 13. Owner ruling, 2026-09-15 — no paint

> *"I don't like the paint, so it should stay gone. Otherwise sounds good."*

The recommendation stands with one change: §9 item 1's *"the paint is what
the player sees and the gnome cuts"* is withdrawn. **A nest is a site and
nothing else. No material is painted at founding.** What that removes, and
what it leaves:

- **`nest` retires as a material.** Nothing paints it, so `nest.ron`, its
  crust, `earth_toned_nest` (the lab's palette swap that already hid it),
  the drainage comb (`DRAIN_PERIOD`, `nest_drain_period`,
  `PIXEL_PHYSICS_NEST_DRAINS`), the two door guards
  (`a_film_on_the_door_drains_through_the_comb`,
  `the_nest_patch_is_still_continuous_enough_to_walk_home_to`) and
  `nestdoor`'s patch census all go with it. **§T2 closes outright**: the
  impermeable strip was the whole of the water problem, and its last 1%
  film goes with the strip.
- **The crust question (§6) is moot** rather than answered: there is
  nothing to dig through. The ground under the door is ground.
- **`paint_nest_patch` becomes `found_nest_site`**: it registers the site
  and records the founding surface row (`colony_surface` at `site.x`), which
  the reach is measured from now that no cell marks it. `AtNest` is
  `|hx − site.x| ≤ COLONY_HALF_WIDTH` and `|hy − site.surface| ≤ 2`.
- **`CreatureDef::nest` stops naming a material.** A species has a home iff
  it was founded with a site; `ancestor` (no nest, by design) keeps reading
  `false` because nothing founds one for it. The field becomes a flag or
  goes.
- **Held world (§10).** `a_nest_still_stops_him` tests a wall nothing
  builds and is deleted with the material; `is_tool_target` is untouched;
  `Druid::found_colony` loses its *"REFUSED — no nest material"* branch and
  can no longer fail for that reason.
- **What the player sees.** At rest, nothing — which is the ruling. The
  ethos still wants the founding verb to deliver something visible, and it
  does: the founders themselves standing at the door, and, once the crust
  is gone, the hall they dig under it. If a marker is ever wanted for
  *reading* the box rather than playing it, it is an overlay on the site
  list (an `F7`-class debug draw), never a world material.

Nothing here is built; it is the design of record for whoever picks the
site up.
