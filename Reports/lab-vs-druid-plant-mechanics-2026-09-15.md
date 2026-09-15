# Why the lab and the held world grow differently — and where the leaves come from

**Status: measured, no behaviour changed.** Owner, 2026-09-15: *"I thought
they were identical but I feel like they are different. The most obvious is
there seems to be way more leaves piling up on the ground in the druid
game."*

He is right, it is not subtle, and it is not the rules.

**The rules are genuinely identical.** Both binaries reload the same two asset
directories at startup — `bin/lab.rs:237-238` and `druid/mod.rs:833-834` — so
every species file, every material file, every growth rule, every creature
brain is one set of code shared by both games. Nothing in `plant.rs` or
`creature.rs` branches on which binary is running. The divergence is entirely
in **what each game does to the `World` before it starts**, and there are
three plant-rule switches, one time model, and a light model between them.

## The headline

`examples/held_litter -- profile=1 grow=8000 frames=36000 senescent=0`, one
bed, one seed, one species set, 8 founders grown 8,000 frames and then run
36,000 more under each game's settings:

| | standing leaf litter | peak | living plant cells |
|---|---|---|---|
| the lab's settings | **188** | 230 | 31,131 |
| the held world's settings | **1,296** | 2,480 | 22,079 |

**6.9x the floor, 10.8x at the peak, from the settings alone.** Same ground,
same seed, same trees. And the direction is not "the druid grows more": the
lab arm ends with *more* living tissue (31,131 against 22,079). The druid arm
is moving its stand onto the floor.

## The three switches

`World::new` ships one set of defaults; each game overrides a different
subset, and **no two of them agree on any of the three**:

| switch | engine default | the lab | the held world |
|---|---|---|---|
| `plant_size_cadence` — does a big plant tick less often than a seedling | `false` (`world.rs:5452`) | **`true`** (`lab/scene.rs:831`) | `false` — untouched |
| `plant_bending` — may a plant lean under load and wind | `true` (`world.rs:5451`) | **`false`** (`lab/scene.rs:832`) | `true` — untouched |
| `plant_load_failure` — may a plant be pulled apart by its own load | `true` (`world.rs:5449`) | `true` — untouched | **`false`** (`druid/mod.rs:848`) |

Each was set deliberately and each is recorded. The lab's two are an owner
ruling of 2026-09-04 — *"I kind of like the idea of the bigger plants get the
fewer ticks they have... we can turn off bending and stress by default"* — and
`lab/scene.rs`'s own comment is explicit that it is scoping them to the lab
and leaving the engine default alone. The held world's is an owner ruling of
2026-09-14 — *"the ability to turn off plant destruction or breaking due to
stress (which should be off by default)"*. **Neither decision is wrong. What
nobody wrote down is that between them the two games now run three different
plant rules**, and `plant_size_cadence` is the one with teeth.

### Which one it is: `plant_size_cadence`, and it is almost all of it

`examples/held_litter -- profile=ablate`, same bed, both directions — the lab
with one switch flipped to the held world's value, and the held world with one
flipped back. Everything the arms are not about cancels:

| arm | standing litter | peak | living plant cells |
|---|---|---|---|
| `lab` | 188 | 230 | 31,131 |
| `lab` + `size_cadence` **off** | **1,406** | 1,673 | 22,608 |
| `lab` + `bending` **on** | 104 | 377 | 28,591 |
| `lab` + `load_failure` **off** | 384 | 431 | 30,345 |
| `druid` | 1,296 | 2,480 | 22,079 |
| `druid` + `size_cadence` **on** | **488** | 523 | 30,118 |

**One switch carries it.** Turning `plant_size_cadence` off in the lab takes
its floor from 188 to 1,406 — 7.5x, and essentially onto the held world's
1,296. Turning it back on in the held world takes 1,296 down to 488. Of the
other two, bending moves the floor the *wrong* way slightly (188 -> 104) and
is not the cause; letting plants break under load is worth about 2x
(188 -> 384) and is the remainder.

Neither direction lands exactly on its target (1,406 against 1,296;
488 against 188), which is the shape of a real effect rather than an artifact
— three switches interacting, not one knob with a clean multiplier.

**The bands also say why the ablation is not a clean multiplier.** The switch
does nothing at all to a seedling and a great deal to a grown tree, so its
effect on a stand depends on the stand's size distribution at the moment you
look — which is itself moving, because the switch changes how fast plants
grow into the next band.

**And it is not only leaves moving to the floor: the stand itself is
smaller.** Every cadence-off arm ends around 22,000 living cells against
~30,000 for every cadence-on arm. A big tree running its economy four times as
often is not four times bigger; it is spending and shedding four times as
often.

**`plant_size_cadence` is not a performance flag.** `world.rs`'s own doc says
so in as many words: *"It is a behaviour change and not a hidden one. The tick
**is** the plant's economy — photosynthesis, transport, upkeep, and the budget
growth draws on — so a tree on a 4x interval does not merely update less, it
lives slower."* Leaf abscission is evaluated on that same tick: the shade roll
(`plant.rs:11027`, `shade_death * darkness^3`) and the drought roll
(`:11061`) fire once per organism tick. So the identical tree rolls to shed
far less often in the lab than in the held world — **how much less depends on
its size**, which is the point of the switch. `PLANT_SIZE_CADENCE`
(`plant.rs:7946`) bands it: up to 50 cells x1, 200 x2, 800 x3, 3,200 x4, and
anything larger **x5**. A seedling is unaffected in either game; a grown tree
in the lab runs its whole economy at a fifth the rate.

## The time model, which is a different effect than it looks

The held world sets `world.held = true` (`druid/mod.rs:914`), and
`World::time_runs_at` then answers `false` for every cell outside the druid's
carried circle or a placed quickening. That gate is asked at **one** place —
`scheduler::step:548` — which is where growth, creatures, decay and
evaporation are all dispatched from.

**Litter falls everywhere and only rots where she is standing.** Nothing in
`update.rs` or `parallel.rs` reads `held`, so the CA sweep carries a shed leaf
to the floor whether time runs there or not; `decay::tick` is a scheduled
site, so it does not run. `litter.ron`'s own note calls this shape out — *"the
same channel ash uses, which is what makes a forest floor a cycle rather than
an accumulator"* — and outside her circle the held world has the accumulator.

**But that is not what makes more litter, and reading it as though it were is
the trap.** Measured with the three arms `held_litter` runs by default:

```
                 litter at 36,000     peak     plant cells
running               1,299          1,577       23,199
quickened (control)   1,299          1,577       23,199
departed at 18,000      658            984       28,127   frozen since she left
```

The departed arm holds **less** litter than the running one, not more —
because the running arm went on growing and shedding for another 18,000
frames. What the gate does is **freeze the floor**, exactly, at whatever it
held the moment she left: 658 at frame 18,000 and 658 for ever. The
`quickened` arm is the specificity control — a circle covering every cell is
`time_runs_at` true everywhere, so it must come back bit-identical to
unheld, and it does.

So the held gate is not a multiplier on leaf fall. It is the removal of the
**sink**: per patch the floor is a ratchet that only ever goes up, and over a
session spent roaming 2560x960 the world's total is the integral of every
patch she has ever quickened, none of which ever clears. The lab's box has the
sink running on all 512x320 of itself, permanently.

**And the sink is slow relative to a circle's residence time.** The running
arm needed roughly 30,000 frames to bring a 2,114-cell peak back to a few
hundred. A carried circle is `CARRIED_RADIUS` 28 and it moves with her, so a
patch gets nothing like that. In practice almost every leaf grown in the held
world is still on the ground.

## Everything else that differs

None of these were measured here; they are read off the two setups and their
direction is not in doubt.

| | the lab | the held world |
|---|---|---|
| world | 512x320 sealed box (`lab/scene.rs:275`) | 2560x960 open (`druid/mod.rs:55`) — **15x the ground** |
| opens with | an **empty** bed: `founders: 0, colonies: 0` (`bin/lab.rs:120-132`, owner 2026-08-30 *"the game should start with no plants or creatures. I add them"*) | **bare** generated ground, `Start::Bare`, owner 2026-09-14 |
| light | declared **sunless** (`scene.rs:923`), lit by `growlamp`'s `glow` **2.4**, attenuating with distance and gapped between fixtures | full sky pinned at noon, `field::MAX_LIGHT` **4.0**, everywhere |
| weather | pinned clear — no gusts, no rain; water arrives from a mister at a rate (`lab/rain.rs`) | real `weather.rs`: rain, gusts and wind, gated per cell by `time_runs_at` |
| a stocked colony | `COLONY_ANTS` **52** (`creature.rs:2993`) | a founding is 4–24, **default 12** (`druid/founding.rs:56-58`), and only once she pays for it |
| speed | a tick multiplier on `Lab::advance` | the `Z`/`V` dial, 1–8, run as whole extra `frame::step` passes |

**The ants matter for the floor specifically.** Litter is food —
`food_energy: 480.0`, `food_class: -1.0` — so a colony grazes the forest floor
as well as the canopy. A stocked lab bed puts 52 mouths on it; the held world
has none until she founds, then a dozen, and they only eat while they are in
running time. *(Mechanism only — how much of the floor ants actually clear was
not measured here.)*

**The speed dial reaches this too, and another lane already priced it**: at
speed 8 the world inside a quickening runs eight `frame::step` passes per
update, so growth, shedding, rot and grazing all move eight times faster —
measured at **228 plant cells eaten at speed 1 against 2,217 at speed 8**
(`druid/mod.rs`'s `speed` doc). That is eight times the leaf fall per second
of play, landing on ground that then freezes.

## A doc comment that cost this session a hypothesis

`Start::Dead`'s doc opened **"The default."** It is not: `#[default]` sits on
`Start::Bare`, moved there by the owner's 2026-09-14 playtest (*"still
shipping full of plants... I thought we said bare"*), and `Bare`'s own doc
says so. Reading the variant docs rather than the derive produced a confident
wrong model of the shipped game — that the floor was a 4,095-organism standing
dead wood rotting into litter — in a world that actually ships with nothing in
it at all. Caught only by running `druid_garden`, which printed `start bare,
grown 0 frames`. The line is now a warning rather than a silent correction
(`druid/mod.rs`).

## What this does not say

- **Nothing here is a bug.** Every switch is an owner ruling with its reasoning
  recorded next to it. The finding is that the two games have drifted into
  three different plant rules without that ever being stated in one place.
- **It does not say the lab's setting is the right one**, only that it is the
  one making the difference. Which floor *looks* right is an owner call, and a
  judge-by-eye one: the held world's floor is the one he can see, and the
  question of whether 188 or 1,296 is the forest floor he wants is not a
  number this report can settle.
- **It does not touch creatures beyond the population numbers.** Brains,
  foraging, digging and breeding are one shared implementation and neither
  game overrides any of it.

## The instrument

`examples/held_litter.rs`. Three modes, all on one bed so every arm stands on
the same ground (`World` is `Clone` — `druid_garden`'s header says it is not,
which was true when that file was written):

```
cargo run --release --example held_litter -- selftest            # the controls
cargo run --release --example held_litter                        # the held gate alone
cargo run --release --example held_litter -- profile=1           # the two games as they ship
cargo run --release --example held_litter -- profile=ablate      # which switch it is
```

`selftest` is the positive control and it is not optional. It asserts three
things, and the middle one is the specificity check this repo keeps needing:
the running arm's floor must actually fall from its peak (there is a sink here
to be stopped); held-with-full-coverage must be bit-identical to unheld (the
arms differ by the gate and nothing else); and the departed arm's floor must
not move after the circle leaves.

**It asserts the freeze rather than an inequality, deliberately.** The first
version asserted `departed > running` and failed — at a shorter budget the
running arm is still filling its floor, so frozen reads *lower*. Whether a
frozen number beats a running one depends entirely on where in its own rise
and fall the run stopped, which is `CLAUDE.md`'s censused-before-it-settles
trap wearing a different hat. *"It did not move after she left"* is true at
every budget.
