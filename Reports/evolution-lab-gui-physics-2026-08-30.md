# The lab's interface, its soil, and how big a creature can be

**Status: one measurement, one correction to the design of record, and two
answers that reverse the question that was asked.** Downstream of
[evolution-lab-design-guide-2026-08-30.md](evolution-lab-design-guide-2026-08-30.md), which it corrects in one
place (§2b) and completes in another (§8.9). Where this says **measured**, a
harness in this repo produced the number and the harness is named. Where it
says **call**, it is a judgement open to being overturned.

Four questions were put by the owner, 2026-08-30. Two of them have answers
that are not the answer the question expected, and those two are the
valuable ones:

| asked | answered |
|---|---|
| **1.** The lab needs buttons, not just keys | Built. §1 |
| **2.** Soil collapses, so ants have never dug tunnels — how do we fix it | The flaw is real and worse than stated: **a gallery closes in 5 frames**. But the design guide already declined the wrong mechanism — §2 |
| **3.** How much can we raise resolution so creatures aren't 2x1 pixels | **Resolution is not what makes them 2x1.** A nine-cell ant is one line in a `.ron` file and already ships; it dies. The blocker is the birth economy, which is Gate 0 — §3 |
| **4.** Brainstorm | §4, ranked by what the machinery already supports |

---

## 1. The interface

*"There should be buttons at the bottom of the screen to play ants, plants,
pop up info panels, control the speed, what else should we have. It shouldn't
all be keyboard shortcuts."*

### 1a. What the lab had

Everything was a key, and there were thirteen of them: WASD pan, `Esc`, `/`
help, `Space` phase, arrows and `1`-`6` for speed, `Tab` stats, `R` reset,
`-`/`=` zoom. Mouse handling existed but did nothing — `CursorMoved` stored a
position that was read **only as a boolean** to force a full redraw, and
`WindowEvent::MouseInput` was not handled at all; `MouseButton` was not even
imported. The one on-screen readout was `ORGS 42` in the top-right corner.

There is no widget toolkit in this repo and there was no `fill_rect`. What
exists is `hud::draw_text` over a hand-authored 5x7 bitmap font, and
`render::put`/`blend` for single pixels. Everything above those is written
here.

**Two traps the font carries**, both already paid for: `draw_text` upper-cases
internally, and **any character outside its glyph set draws as a silent
blank** — a trap that has shipped three times, which is why `lab/mod.rs`
carries a standing test that every help line is drawable. There are no arrow
or play/pause glyphs.

### 1b. What else the lab should have

The design guide's §4 already lists the verbs and what each produces. Sorted
by what the interface can reach today, that gives three tiers — and the
ordering is itself the answer, because tier 1 is exposing things that already
run and tier 2 is where the engine work is.

**Tier 1 — a control for something that already runs.** Nothing to build but
the button.

| control | what it drives |
|---|---|
| **transport** — play/pause, faster, slower, presets | `TimeControl`, shipped. The readout must keep **requested against achieved**; that pairing is the honesty mechanism the dial was built around |
| **display rate** | 60/30/20/10 Hz. Lane D measures 20 Hz buying **2.6x** world-per-second on a loaded box, and the watchable crossover at **12 ticks per displayed frame** — so the rate is a player control, not a constant somebody picks |
| **info panels** — plants, ants, the box | Every number is already on `World`; nothing needs simulating |
| **inspector** — click a cell | `Renderer::screen_to_world` is built and the lab already holds a `Renderer`. This is the difference between an instrument and a screensaver |
| **overlays** — light, moisture, temperature, pheromone | `FieldOverlay`/`OrganismOverlay` exist and are keys in the sandbox. **Pheromone is the interesting one**: it is at full cell resolution, and it is the colony's own map |
| **export** — keep this specimen | `species_export` is built and round-trip verified |

**Tier 2 — the verbs with no engine support.** The guide names `cull` and
`partition` as the only two, *"and the two the premise most depends on"*.

| control | why it is the expensive tier |
|---|---|
| **cull** — click an individual or a lineage, remove it | §7b-i's decided progression puts **selection only** in the opening. So the opening's entire lever is this verb, and it does not exist |
| **partition / door** — drag a wall in, open a gap | Measured at **4.1x -> 7.6x** speed-up with the stand held to 0.2%, *and* evolutionary isolation, *and* the §5 score. One object, three payoffs |
| **plant / release** — sow a seed, drop founders | Placement exists (`plant_tree_species`, colony founding). What is missing is the **choice**: the guide's own note is that planting must show the individual's traits *"or planting is a slot machine"* |
| **equipment** — light, fan, heater, humidifier | The air simulation runs idle in a sealed box; equipment is what switches it on. §2c: the first fan costs, the rest are free, so the control is **"does this compartment have moving air"** and never "how many fans" |

**Tier 3 — instruments the Running phase needs.** Open question #9 is that a
fast-forward phase whose whole content is *watch evolution happen* has a
legibility problem, and the owner has confirmed it directly: shown a colony
that breeds beside one that cannot, same world, same moment, the verdict was
*"no, not without motion at least"*.

- **A run log** — births, deaths, first seed set, a lineage ending. A phase
  that fast-forwards 45,000 frames must be able to say what happened while
  the player was not looking.
- **Hover explanations on every quantity.** This is an explicit owner
  request, not an inference: *"the user should be able to mouse hover over
  some of the words and get an explanation of what it means and this could
  also be a way to access more details data"* — and the follow-up that built
  it was answered *"looks good"*.
- **A scrub bar.** Determinism is required same-build and fast-forward runs
  the identical tick sequence, so a run is exactly reproducible from a seed
  and a frame count. Rewinding is a replay, not a save format.

---

## 2. Soil: the flaw, measured

### 2a. What was already believed

`wiki/ants.md`, current:

> **They dig.** Ants chew through soil and stop at anything harder... **Turn a
> colony loose on a soil bank and it hollows it out**, leaving the stone
> beneath untouched.

The design guide §2b, on the owner's decision *"we can remove collapsing
tunnels"*:

> **What it buys**: the 16% structural purchase is declined... **Soil in the
> lab holds whatever shape is dug into it.**

Neither is true of the shipped engine, and nothing had measured it.

### 2b. What is true — `examples/burrow_probe.rs`

The harness carves what an ant would actually dig — a shaft down from the
surface, a gallery off its foot, a chamber at the end, all 3 cells tall — and
censuses each separately, because they fail for different reasons: a shaft is
a vertical face, a gallery has a roof, a chamber is both over a longer span.

| arm | frame 0 | frame 1 | frame 5 | frame 30 | frame 1,800 |
|---|---|---|---|---|---|
| **soil** — gallery | 180/180 **100%** | 121/180 67% | **0/180 0%** | 0/180 0% | 0/180 0% |
| **soil** — chamber | 128/128 **100%** | 105/128 82% | 30/128 23% | **0/128 0%** | 0/128 0% |
| **soil** — shaft | 300/300 **100%** | 190/300 63% | 20/300 7% | 18/300 6% | 18/300 6% |
| **sand** — all three | 100% | 67 / 82 / 63% | identical to soil to within 3 cells | | |
| **stone** — all three | **100%** | **100%** | **100%** | **100%** | **100%** |

**The gallery is gone in five frames. The chamber is gone in thirty.** The
shaft keeps 6% — the few rows at the top, above where the walls have slumped
to their repose angle — and that residue is the only trace an excavation
leaves.

`stone` is the **positive control** and holds 100% in all three voids at every
frame, so this is the physics and not the scene. The harness also asserts the
scene directly: every carved cell must read open at frame 0 or it panics
naming the geometry. That assertion is there because the first version of the
file did not have it and reported **0% galleries at frame 0** — `soil=140`
under `ground_y=200` puts the bed past the world's bottom edge, where
`World::set` silently drops it. A scene that contradicts the thing under test
looks exactly like a strong effect.

### 2c. The correction: §2b declined a mechanism that never applied to soil

This is the part that matters, and it is a correction to the design of record
rather than a new finding.

**What closes a soil tunnel is not the structural scheduler.** Powder never
enters it — `structural.rs:4816`'s `is_body_material` is `Solid | Plant`, so
a soil cell has no anchor distance, never breaks free, and is invisible to
`load.rs` except as *weight on something else*. Declining the scheduler's 16%
therefore changes nothing whatsoever about soil.

What closes it is three lines of the CA sweep:

| site | rule |
|---|---|
| `update.rs:631` | `if !hole_from_a_sideways_escape && try_move(surface, x, y, x, y + 1)` — the roof falls straight down into any empty cell, unconditional on support |
| `update.rs:634` | the two diagonals — the walls slump in |
| `update.rs:1906-1920` | `try_move` requires only that the destination `is_empty()` |

§2b names this itself, and then leaves it in place:

> Declining collapse is not the same as declining a repose angle — loose soil
> sliding to its angle of rest is **in the CA sweep, not the structural
> scheduler**, and costs nothing extra. **A dug wall that slumps a little is
> available and free**; a roof that falls in is what was declined.

"Slumps a little" is the reading the measurement overturns. In a powder there
is no representable state in which soil has a roof, so a repose angle is not a
gentler version of collapse — it is the whole of it, and it is total.

**And raising `friction_angle` cannot reach it.** `roll_along_slope`
(`update.rs:907`) is the only rule that reads the angle, and its own doc says
repose can only ever make a pile *flatter*. At 89° the roll reach goes to
zero, `roll_along_slope` returns immediately — and `try_move(x, y+1)` still
fires. The angle bounds a free surface's slope; it says nothing about a
ceiling.

### 2d. What the engine already has, and did not connect

One mechanism in the shipped engine makes soil immobile, and it is not
structural. `update.rs:567`, before any movement rule runs:

```rust
if holds_water && root_reinforced(surface, x, y) {
    return wet_changed;
}
```

A water-holding powder with a `reinforces_powder` neighbour never moves.
`reinforces_powder` is set on exactly two materials — `rootwood.ron:47` and
`grassroot.ron:33` — so **plant roots already hold soil up, and nothing else
does**. `Reports/dead-ends.md:311` names `update_powder` as the correct site
in as many words, having ruled out extending structural credit into powder.

The bug register already records the same fact from the far side
(`open-bugs-handoff.md:2363`):

> `reinforces_powder` does not stop digging, only avalanching, so ants can
> hollow a sod bank into a lattice that never collapses.

So the engine can already hold a tunnel open. What it cannot do is let a
*creature* produce the state that does it.

### 2e. The fix: ants line their tunnels

**(Call.)** Real ants compact tunnel walls as they dig. Modelled here, that
is a material change on an event that already fires, costing nothing per
frame:

1. **`packedsoil`** — a `Powder` that holds moisture like soil, is
   **self-supporting** (it does not fall), and has a `penetration_resistance`
   just under the ant's `dig_force`, so an ant can still dig through its own
   lining but pays more for it than for loose tilth.
2. **A material flag, tested at the dispatch site that already holds the
   `def`** — beside `holds_water` and `clings` at `update.rs:528-531`, in the
   same shape as the `root_reinforced` early-out. A `Vec` index, never an
   `id_of()` string hash in the sweep (`CLAUDE.md`: *guard hot-path work at
   the call site that already has the data*).
3. **The dig packs its own walls** — `creature.rs:2244` empties a cell; the
   loose soil in the ring around it becomes `packedsoil`, preserving moisture
   and shade.
4. **Water un-packs it.** Above `SOIL_FIELD_CAPACITY`, packed soil reverts to
   loose soil and the tunnel comes down.

Three things fall out that are worth more than the fix itself:

- **It is graded, not binary.** `CLAUDE.md`'s first law asks for a middle, and
  §2b names its own absence of one: *"a burrow becomes permanent the moment it
  is dug, so the excavation has no ongoing stake in it."* A lining that water
  softens gives the excavation an ongoing stake. This is a direct answer to
  the guide's open question **#10** — *what threatens a burrow, now that
  collapse is declined?* — and it is the cheapest of the three the guide
  offers, because the liquid rules already run.
- **The lining is deliberately NOT `reinforces_powder`.** A cell that
  stabilised its neighbours would progressively freeze the whole bed as
  tunnels spread. It holds itself up and nothing else.
- **It gives the Running phase something to watch**, which §8.9 records as an
  unsolved legibility problem. A nest that stands is the one thing on screen
  that visibly *accumulates* over 45,000 frames. Nothing else in the box
  does.


### 2f. What must NOT be fixed — the owner has already ruled on loose soil

**Loose soil slumping is wanted, and this is on the record.** Review card
2026-08-30T03:45, *"The pick digs soil now — but soil slumps"*, showing the
gnome's pick opening a hole in a soil bank that immediately falls in:

> *"this is fine"*

So the defect is not that soil is loose. It is that **there is no way for
anything in the world to produce soil that is not loose** — no verb, no
material, no state. Loose tilth collapsing into a pick-hole is correct
behaviour; a colony that can never build a nest is the flaw.

That is why the fix in §2e is a **second material laid by a verb**, and not a
change to how `soil` behaves. After it:

- The gnome's pick opens a hole in loose soil and it slumps, exactly as the
  owner approved.
- An ant's gallery stands, because the ant packed its walls.
- A flooded gallery comes down, because water un-packs them.

Three outcomes where there was one. `CLAUDE.md`'s first law asks for a middle;
this is the middle, and it arrives as a distribution over *who did the
digging* rather than as a tuning constant.

Full results in §2g.

---

## 3. Resolution: the question reverses

The owner's question was *"how much can we increase our resolution to fix our
pixel graphics, for the main goal of not having all our creatures just be 2x1
or 3x1 or 2x2 pixel creatures"*, with the hypothesis *"could we have a
different game resolution and field resolution — some of these things could be
a little more coarse."*

**The hypothesis is right, the split already exists, and it is not what makes
a creature two pixels.** Three separate things are tangled in the question and
they have three different answers.

### 3a. A creature is 2x1 because its species file says `Chain(2)`

`assets/species/ant.ron:27` — `body: Chain(2)`. Two cells. `BodyPlan`
(`organism.rs:1789`) is already fully parameterised: `Chain(u8)` or
`Rigid(Vec<(i8,i8)>)`, offsets up to ±127 cells. **Bigger bodies already ship
as files**: `ant_long.ron` is `Chain(6)`, `ant_wide.ron` and `ant_block.ron`
are 9-cell `Rigid` plans, `beetle.ron` is 4.

So a nine-cell ant is one line in a `.ron` file, today, at the current
resolution, with no engine change at all.

**Raising world resolution would make creatures *smaller*, not bigger.**
Nothing in `creature.rs`, `plant.rs` or `organism.rs` reads `world.cell_scale`
— the only readers are `player.rs`, `worldgen/region.rs` and `render.rs`. At
2x cell density the gnome would scale (`Player::at_scaled`) and every animal
and plant would stay at its authored cell count, i.e. **half its physical
size**. That is precisely the defect the owner already caught by eye once, on
the gnome: *"our gnome shouldn't have shrunk"*
(`Reports/resolution-step-2026-08-29.md`). It was fixed for the player and
never for anything alive.

### 3b. Why the nine-cell ant does not exist: it dies, and the arithmetic says why

`Reports/creature-body-extent-2026-08-30.md` §4b, `creature_probe terrain=world
seed=0xA17 frames=12000`:

| body | peak pop | deliveries | **live at 12,000** |
|---|---|---|---|
| `Chain(2)` — ships today | 45 | 733 | **29** |
| `Chain(3)`, priced | 34 | 339 | **0** |
| `Chain(4)`, priced | 30 | 624 | **0** |
| `Chain(6)`, priced | 29 | 352 | **0** |
| 9-cell, priced | 26 | 457 | **0** |

**No chain longer than two cells leaves a living colony** — and it is not the
terrain: on a hand-built flat slab, `body=2` gives 24 alive and `body=3` gives
**0**.

The arithmetic (§5b of that report, and §1a of the design guide) is the same
deadlock the lab already knows about, one factor worse per cell:

| body | birth cost (`body_energy` 480 x cells + grant) | bank ceiling | ratio |
|---|---|---|---|
| 2 cells | 1,040 | 460 | **0.44** |
| 6 cells | 2,960 | 460 | **0.16** |
| 9 cells | **4,400** | 460 | **0.10** |

Birth cost scales with the body. The bank ceiling — `hunger_fraction x
start_energy` plus one mouthful — **does not**, because the ant stops eating
above its hunger line and carries the rest home.

### 3c. So the good-looking-creature problem and Gate 0 are the same problem

This is the finding worth carrying.

§1a of the design guide proposes the lab's **first machine**: an incubator
that pays the ~580-unit shortfall the two-cell ant cannot pay, so that a first
generation exists at all. It is argued for as the opening's *content* — the
player's first real puzzle.

**That same machine, with a bigger dial, is what makes a nine-cell ant
possible.** The shortfall goes from ~580 to ~3,940; the mechanism is
identical. Which means:

- Body size becomes **an upgrade the player earns**, not a constant somebody
  picks — you can afford a bigger animal when your incubator can pay for one.
- It is **legible without a tutorial**, which §1a already wants: a bigger
  creature is visibly a bigger creature, where "the gut draws more per
  mouthful" is not.
- It gives §5's score a dimension that is *visible* rather than tabulated,
  which is open question **#5** (*is the score legible?*).
- And it does not fork anything: `creature-appearance-design.md` records that
  body plan is copied from the parent by `individual_as_species` and is not
  currently heritable, so making it an unlock is a smaller change than making
  it evolve.

**(Call.)** Prefer this to raising resolution. It is cheaper, it is already on
the critical path as Gate 0, and it turns a rendering complaint into a game
mechanic.

Two things it does not fix, both measured and both real: a `Chain(n)` hatches
as *n* cells in a straight horizontal line all of which must be empty
(`creature.rs:857`), so placement roughly halves at three cells; and a `Rigid`
body is blocked **41-43%** of the time against a chain's **4-5%**
(`creature-appearance-design.md` §5). A big creature wants to be a chain, not a
block.

### 3d. What resolution *does* buy — and the free half is the render

The owner's split is exactly right and **half of it is already built and
idle**.

**The framebuffer is not the cell grid.** `main.rs` and `bin/lab.rs` both open
a 1024x640 logical window against a `Pixels::new(512, 320, ..)` framebuffer, so
**every cell already occupies a 2x2 block of physical pixels and all four are
byte-identical**. The GPU is nearest-neighbour-replicating. Measured
(`subpixel_cost.rs`, `render_cost.rs`):

| px per cell | buffer | pixels | vs 1x |
|---|---|---|---|
| 1 | 512x320 | 163,840 | 1.00x |
| 2 | 1024x640 | 655,360 | **1.13x** |
| 3 | 1536x960 | 1,474,560 | **1.32x** |

**Four times the pixels for 13% more.** The mechanism is explained rather than
lucky: per-pixel work is under 10% of a redraw, and the rest — the sky-light
grid, the horizon rebuild, the glow-tile scan — is per-*draw* setup a finer
lattice does not repeat. And `Renderer` needs no new concept: `zoom` already
means "screen pixels per cell", `screen_to_world` and `sub_cell` already do
the mapping, and `cell_colour` already takes the sub-cell offset with exactly
one thing reading it today.

So **more pixels per creature is available now, costs ~13% of a redraw, and
touches no simulation.** `Reports/subpixel-rendering-2026-08-29.md` prototyped
the reconstruction half of this for plants and got a specific owner verdict
worth honouring: *"could it be more flat or cartoony"* and *"the smooth
circular shape/edges look fake"* — so the direction is flat fill with a drawn
edge, not shading, not smoothing.

### 3e. Raising the *simulation* resolution: expensive outdoors, plausible in the lab

Outdoors it does not fit, and this is settled rather than open. Holding the
same physical world at twice the cell density is 16384x5120 = **84M cells
against today's 21M** — `Reports/resolution-step-2026-08-29.md` says flatly
*"that does not fit"*, and generation time and peak RSS are both near-linear
in cells (5.8 s / 361 MiB at 21M today). The frame is not linear in cells,
which cuts the other way too: 8192x1024 has 60% fewer chunks than the shipped
size, solves **32% more tiles**, and costs **more**. There is no efficient way
to shrink the outdoor world for frame rate, because size is not the variable —
the awake set is, and outdoors the awake set is the sky-lit surface band.

**The lab is a different regime and this is the whole point of the concept.**
The box holds its sky (`set_sky_hold`), so `sky_drifted` does not wake every
surface tile every frame, and a 2048-wide box measured *cheaper* than a
512-wide one at fixed founders. The box is already a runtime parameter
(`LabBox { width, height }`, default 512x320). So the lab can plausibly
afford a cell density the outdoor game cannot.

**But not because cost follows biomass — that is the part PR #170
overturned, and it changes what a resolution budget should be built on.**
Measured in the lab bed: the frame's correlation with **biomass is +0.03**,
with the field's **solve set +0.92**, and with **awake chunks +0.93**. The
guide's *"cost follows living biomass"* and its Gate 3 corollary *"a mature
box is the expensive one"* are **backwards in this bed** — the multiplier
**rises** through a session, **9.0x fresh to 17.8x settled**, because the box
*quiets* as it fills.

The consequence for resolution is direct and it is the useful half: **more
cells cost what they cost by being awake, not by being alive.** A bigger bed
that mostly sleeps is cheap; the same bed with weather or a fan in it is not.
That is a much better budget to design against than a per-plant-cell figure,
and it puts the emphasis back on the sealed box and the held sky rather than
on the population.

### 3f. The field: the owner's hypothesis, and the pairing that makes it work

`FIELD_SCALE = 8` — one field cell per 8x8 world cells, six channels
(pressure, vx, vy, temperature, light, moisture), 135 references across 14
files. The field is **already the coarse half** the owner is asking for.

`Reports/resolution-step-2026-08-29.md` names the pairing:

> `FIELD_SCALE` 8 -> 16 keeps a field block covering the same *physical* area
> it does today, so light and shade look identical and the field's cost falls
> ~4x... **Do not do this without the content scaling: at unchanged content it
> coarsens the shade.**

That is exactly right and it is the correct way to think about it: at 2x cell
density, `FIELD_SCALE` 16 is not a coarsening, it is *holding the field still*
while the CA grid gets finer. Nobody had measured it; §3g reports the run.

Two second-order effects that are easy to miss and are not theoretical:

- Sky attenuation is `SKY_TRANSMISSION^(depth / FIELD_SCALE)` — optical depth
  is counted in **blocks**, so changing `FIELD_SCALE` changes how dark a given
  physical thickness of rock or canopy is.
- The ant's `sensor_offset` is **6**, already below `FIELD_SCALE` 8, which is
  why `field_at_bilinear` had to exist at all. At 16 it is 6/16 of a block, and
  every recorded dead end about coarse-field gradients
  (`dead-ends.md:981, 993, 743, 635`) carries a re-test condition keyed
  explicitly to `FIELD_SCALE` exceeding the sensor spacing.

### 3g. What must not move

`CHUNK_SIZE = 64` and `MAX_REACH = 32`. The equality `MAX_REACH ==
CHUNK_SIZE / 2` is a **proof obligation** for `parallel.rs`'s cross-chunk
write-disjointness, not a convention — and the design guide §7a names a
changed chunk size or `MAX_REACH` as one of exactly three things that would
make the lab *"a genuine second engine rather than a second game"*.

Note also that both are counted in **cells**, not physical distance, so at
higher density every material's physical sweep reach halves: the flattest
expressible sand pile (`atan(1/MAX_REACH)`) gets steeper, and a liquid's
`HORIZONTAL_TRANSFER_REACH` of 8 levels over half the distance. Raising cell
density is a physics change wearing a resolution costume.

---

## 4. Ideas, ranked by what the machinery already supports

Nothing here is a new system. Each names the shipped machinery it runs on and
the open question it closes, because an idea whose cost is unknown is not
comparable to one whose cost is zero.

### 4.1 — The nest is the thing to watch (free; falls out of §2)

The design guide's open question **#9** is the uncomfortable one: *"an ant is
two dark cells at play zoom, findable only because it moves — and a dead one
has stopped moving, so it is unfindable by the very channel that finds a live
one. A phase whose whole content is 'watch evolution happen' has a legibility
problem this repo has already measured and not solved."*

Standing tunnels solve it without solving it. The player does not have to
find an ant; they watch the **nest** — a differently-coloured, permanent,
growing record of every decision the colony has ever made. It is legible at
any zoom, it is legible when nothing is moving, and it is a *history* rather
than an instant. Cost: zero, it is §2's fix.

### 4.2 — Partitions, as a verb — but not for the speed-up

**Corrected 2026-08-30 by PR #170, and the correction matters.** §2c of the
guide measured one wall taking a fanned bed's speed-up from **4.1x to 7.6x**
and called it *"the strongest single design finding"*. Run in the lab's own
bed, **the containment reproduces and the speed-up does not**: `solved/f`
falls 39.4 -> 25.1 over 1 -> 16 compartments (**-36%**, exactly §2c's
mechanism), while the frame goes 1.69 -> 1.41 -> **1.92 ms** — non-monotone,
because the field is only 54% of a 1.5 ms tick at 512 wide. §2c's 7.6x was
measured on a **fanned 2048-wide** bed, and the lab is neither.

So partitions stay, for **evolutionary isolation** (asexual isolation is
where clusters come from) and for the **§5 score** (separation,
specialisation and persistence are all measured *across* compartments) —
**and no speed-up should be budgeted at lab scale.** Two payoffs, not three.
This is the guide's own trap about an isolated harness overstating what the
app will see, landing on the guide's own headline finding.

In interface terms it is a drag to place a wall and a click to open a door.
It is the single highest-value verb the lab does not have.

### 4.2a — The grow lamps do not light the crop, and that has to be said

Not an idea — a correction, filed here because every equipment idea below
inherits it. Measured PR #170: `labshot lamps=0` replaces every fixture with
stone and **the stand is byte-identical**. The glow decays over a handful of
field blocks and the bench is nineteen below it. **The room reads the
schedule; the shell passes the light.**

And the shell is the real knob: thickening the ceiling 4 -> 7 rows cost
**45% of the light on the bench and half the stand** — 468 plant cells to
286, **12 seeds to 0** — with **no gate going red**, because light travels a
column as `0.2^(depth / 8)` and optical depth is counted in *field blocks*.
One extra block between sky and bench is a factor of five.

Two consequences worth acting on. **A light schedule as a strategic lever
(§4.4 below, and the guide's open question #2) is a lever on the sky hold,
not on the fixtures** — building a lamp UI would be building a control for
something that does nothing. And **any change to the bed's geometry is a
change to the crop's light**, silently, with every gate green. That is the
mechanism to suspect first whenever a bed change reads as a biology result.

### 4.3 — A water table (cheap; closes two open questions at once)

A horizontal wet line in the bed. Roots grow toward it. Tunnels dug below it
flood, and flooding un-packs the lining (§2e). `PLAN.md`'s worldgen redesign
already calls the water table *"the single highest-value structure"* for the
outdoor world, for the same reason it works here: it is visible in any cut
face and it gives depth a meaning.

It answers open question **#7** — *what reaches the bottom of a deep bed?* —
which §2a raises as an obligation rather than a nicety: soil depth costs
**1.9x the frame** for 40→240 rows and the shipped herb roots to 40. Something
must use the depth being paid for. A water table makes the bottom of the bed
the place worth reaching, for roots and for burrows, immediately, with no
evolution required first.

Machinery: the liquid rules and soil moisture `aux`, both shipped.

### 4.4 — Heat, as the grow light's price (closes open question #2)

Open question **#2**: constant full light gives **1,037 seeds against 435** for
12% more cost, so it strictly beats a cycle — and *"a lever with no downside
is not a decision"*.

The field already carries `temperature`, with `sky_temperature` stored as a
decomposition of it. Let held light accumulate heat, and the **fan** — which
§2c measured as costing the same for eight as for one — becomes its counter.
Equipment then *interacts* instead of stacking, which is the difference
between a shop and a system. Cost: a term, not a subsystem.

### 4.5 — Cull, and pick-up (§4's two unsupported verbs)

The guide's own verb table names `cull` and `partition` as the only two with
no engine support, and *"the two the premise most depends on"*. Without cull
the player is not the selection pressure, and §7b-i's decided progression
puts **selection only** in the opening — i.e. the opening's entire lever is
the verb that does not exist.

### 4.6 — The specimen shelf (E8's keep, made into an object)

`examples/species_export.rs` already writes a genome and its traits out as
`assets/species/<name>.ron` and reads them back, round-trip verified. Make
that a **visible shelf** in the lab: every kept lineage is a jar on a rack,
clickable, re-releasable. It is the game's *keep*, it is the loop back to the
outdoor game (§0), and the file format half of it is built.

### 4.7 — Pheromone as a visible channel (an overlay, not a system)

`src/sim/pheromone.rs` is a double-buffered plane at **CA resolution** — 1
cell, deliberately not on the coarse field (`dead-ends.md:1029` records why).
Ants already paint their own map (`wiki/ants.md:87`). Drawing it faintly
during a Running phase lets the player *read the colony's foraging map form
and shift* — which is exactly the "interesting relationship" §5 wants to
score, made visible rather than tabulated. `render.rs` already has the
`FieldOverlay`/`OrganismOverlay` pattern to copy.

### 4.8 — Say what killed it (legibility for failure)

§6 argues from `CLAUDE.md`'s first law that a total wipe is the binary the
whole project is against, and §7b-ii records that how graded failure should be
is **deferred to playtesting**. Independent of that decision: when a lineage
ends, *say what ended it*. A failed experiment that reports its cause is
information; one that just empties is a reset. This is a readout, not a
mechanic, so it does not pre-empt the deferred decision.

### 4.9 — Hue, not size, for a findable creature (measured advice already in the repo)

`PLAN.md`'s M19 research is explicit: **distinguish adjacent materials by
hue, not just value, since value-only differences vanish at small pixel
sizes.** A dark-brown two-cell ant on brown soil is unfindable for a reason
that has nothing to do with how many cells it is. This is the cheap half of
§3 and it is advice this repo has already paid for once, on plants
(`plant-appearance-design.md`: three architectural levers fired 46–2,750
times each and moved nothing, because *"a lever that relabels a cell cannot
move a silhouette that texture and colour set"*).

---

## 5. What this report does not decide

- **Whether the lining should be an ant behaviour or a player machine.** It is
  built here as a creature behaviour because that is where the dig already is.
  A "shore up the tunnels" machine is the same mechanism with a different
  author, and §1a's argument — the player's first machine pays a cost the
  creature cannot — applies to it equally.
- **Anything about the outdoor game's soil.** The change is in shared code and
  the outdoor world would get it too. Whether ants hollowing out standing
  galleries in a hillside is *wanted* there is a judgement about the outdoor
  game, and it is not made here. (§7a's seam: the difference could be pushed
  into a species file — an outdoor ant with no lining behaviour — rather than
  into a constant.)
- **The score.** §5 of the guide holds it and §8.5 records that whether it is
  *legible* is open.
