# Why every creature crawls, and what the other gaits would cost

**2026-09-09.** Owner's question: *"explore creature movement more. we have
researched this before, but I have never seen any movement patterns different
than crawling. this is one way that we get from everything being ants to
having bees, birds, frogs, etc"*

**Status: diagnosis plus one landed instrument.** Nothing in `src/sim/` is
changed. `examples/food_height.rs` is new, `examples/filmstrip.rs` gains one
knob on an existing scene, and the rest is measurement.

---

## The answer in one line

**The engine has had a working jump since 2026-08-29 and not one creature in
the game is wired to use it.** It is not a missing mechanism, it is a missing
line in a `.ron` file — measured at **0 launches** across every real scene and
**275** in the same scene with one wire added.

The three other gaits the question names cost very different amounts, and only
one of them is a mechanism gap.

---

## 1. The hop is built, correct, and unreachable

`BrainOutput::Impulse` → `creature::launch` → `creature::step_flight` shipped
with `Reports/creature-motion-design.md`, five guards, and the owner's own
verdict on card `20260829T154736312Z-2536dc` (*"With the new jump is great. I
choose B"*). It has never appeared in the game.

**Nothing authors it.** `grep -ri impulse assets/species/` returns nothing, and
`creature.rs`'s own test says so out loud: *"ant.ron authors no Impulse weight,
so the output must be exactly 0.0"*. The only place the verb has ever been
rendered is `filmstrip`'s `scene=hop` — a hand-built shelf over a drop, which
wires the instinct itself precisely because no species does.

Measured three ways, and the instrument is sensitive in both directions:

| run | launches | airborne frames | flight moves |
|---|---|---|---|
| `forage_probe frames=8000`, 55 ants, 19,230 moves | **0** | 0 | 0 |
| `filmstrip scene=colony`, generated wetland, 18 ants | **0** | 0 | 0 |
| `filmstrip scene=hop` — the artificial shelf | 35 | 865 | 1,136 |
| **`scene=colony impulse=2.0` — one wire added** | **275** | 2,532 | 1,422 |

The last row is the positive control the diagnosis needs. A zero that stays
zero when the mechanism is switched on is a broken counter; this one moves by
275, so the zeros above it are the world and not the instrument.

**And the trade-off is real, which is what the design demanded.** Same seed,
same colony, the walking arm byte-identical to the shipped ant:

| | walk | hop |
|---|---|---|
| deepest forage trip | 9 cells | **18** |
| forage trips over the bar | 1 | **3** |
| walking moves | 541 | 115 |
| deaths | 3 | **10** |

A hopper covers more ground per decision and pays for it in bodies. That is
`creature-motion-design.md` §1's cost-and-benefit requirement satisfied by the
physics rather than by a tuned constant — nothing here was balanced.

The paired animation is card `20260909T034123716Z-e1b18f`.

## 2. The reason to leave the ground already exists, and nobody had measured it

`Reports/creature-behaviour-ceiling-2026-09-05.md` establishes that survival
correlates **+0.895 with how much an ant eats and with nothing else**, so a
verb that costs energy without feeding the animal is a debit selection
removes. That is the standing objection to authoring any gait: it would be
bred out. **It turns on a fact about the world that nothing had checked** —
whether there is food a walker cannot cheaply reach.

`examples/food_height.rs` bins every food-bearing cell by height above its own
column's ground and sums `creature::food_value`, the same function the eat verb
reads.

| preset | seeds | food worth 5+ cells up | 10+ cells up | highest |
|---|---|---|---|---|
| `wetland` | 4 | **95.8%** | 91.1% | 177 cells |
| `rolling` | 3 | **87.6%** | 76.1% | — |
| `arid` | 2 | 0.0% | 0.0% | — |

**Between 88% and 96% of every calorie in the world is in the canopy**, and
only 1–2% is within one cell of the ground. `rolling` is the control that says
this is not a wetland lake inflating the ground reference; `arid` is the
negative control and its zero is honest — that preset grows nothing at all, so
there is no elevated food to find. Leaf carries 96.7% of the total.

**But the gradient is in quantity, not in richness, and that cuts against the
bee reading.** Worth per food cell is **exactly 480 at every band**, ground to
crown — the canopy is not better food, it is simply where all the food is. A
flyer's advantage here would be *access*, never a bigger prize, which is a
weaker and more fragile kind of niche. The one material that would change that
is the one §2's second finding says is absent.

**So the vertical gradient is not something to build. It is already there, and
it is enormous.** What this does *not* settle is reachability: a crawler can
climb a trunk, and `organism::Crossing` lets it work round a bole, so "high" is
"expensive" rather than "impossible". How expensive is the next measurement,
and it is the one that decides whether flight pays.

**A second finding fell out of the same census.** `flower` is the richest food
in the world at `food_energy: 1440` — 3x a leaf, 1.5x fruit — and across two
worlds run to 30,000 frames there was **one flower cell**. The bee's food does
not exist in practice. Anything built for pollinators wants that looked at
first.

## 3. What the four gaits actually cost

| gait | status | what is missing |
|---|---|---|
| **crawl** | shipped, universal | — |
| **frog** (ballistic hop) | **mechanism done** | one instinct on a species file |
| **bee/bird** (sustained flight) | not built | a hop is an arc and gravity always wins; staying up needs a per-frame cost against gravity, which is a new verb with a new price |
| **fish** (swim) | **not built, and currently a defect** | `creature.rs:2415` — an ant that walks onto a pond *stands on it*. Drown, float or swim is recorded there as an open design question for the owner |
| **pace** (fast/slow) | mechanism shipped, gene unlocked | `tick_interval` varies per species and only the beetle uses it; `creature-motion-design.md` §8 calls this "an authoring gap, not a mechanism gap, and it is free to close today" |

Two of the five are free today. One is a bug that has been filed as a design
question and never answered. Only sustained flight is genuinely new work.

## 4. What this recommends, in order

1. **Author the hop onto a species of its own** rather than onto `ant.ron`.
   Every ant hopping degrades the colony (deaths 3 → 10) and ants do not hop;
   a separate animal that does is the "from ants to frogs" step, and it costs
   one file. Blocked only on the owner's verdict on the card above.
2. **Answer the water question.** It is a live defect, it is the whole of the
   fish niche, and it is the only gait whose absence is currently visible as
   something looking wrong.
3. **Measure the climb before building flight.** §2 says the calories are up
   there; it does not yet say a walker cannot get them. If a crawler reaches
   the canopy cheaply, sustained flight is decoration, and the S5 diet-gene
   outcome repeats.
4. **Vary `tick_interval` across the shipped species.** Free, and speed is a
   movement pattern the owner would see immediately.

## What would overturn this

- The flat 480 already weakens the flyer case on its own: if a climb census
  then shows the canopy is cheap to reach, there is no niche left at all.
- If a climb census shows crawlers already harvest the canopy at low cost,
  §2's gradient stops being a niche and recommendation 3 becomes "don't".
- If the owner's verdict on the card is that the hop reads as a bug rather
  than as an animal, recommendation 1 goes and the ballistics need work
  before any species carries them.
