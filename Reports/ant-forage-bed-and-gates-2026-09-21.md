# The foraging bed was measuring birth position, and two gates were asserting the old world

*2026-09-21, branch `claude/upbeat-shannon-cez0w4`. Follows
[`ant-return-leg-result-2026-09-20.md`](ant-return-leg-result-2026-09-20.md),
which landed the sensor projection, the graded crop and `PheroARise` and left
two gates red and four owner questions open.*

**Status: the gates are repaired and proven by injection; the bed defect is
measured and has two independent fixes, both behind selectors.**

---

## 1. Two gates, and neither was the code's fault

### 1a. The trail-sensor geometry test was asserting the bug it was written to catch

`the_trail_sample_is_taken_where_a_trail_could_be` asserted that **every**
heading whose nose lands off the ant's own row reads exactly `0.0` — and said
*"Off by default"* in its own comment. The projection shipped **on** in
`08b49906`, so the four headings with a horizontal component now deliberately
read the walker's row, and the test failed for being right.

Rewritten to the shipped contract: NE and SE must read **exactly** what E
reads, NW and SW what W reads, and N and S — which have no horizontal
component, keep the full offset, and are what lets a colony follow a trail up
a trunk — must still read `0.0`.

**Injected one half at a time, and they land on different assertions**, which
is why the two halves are now asserted apart:

| injection | red at | reads | wants |
|---|---|---|---|
| `sensor_projected` → false | assertion 3, on NE | `0.0` | E's `0.06666667` |
| `sensor_honest` → false | assertion 3b, on N | `-0.90909094` | `0.0` |

**And it corrects a figure that was being quoted wrongly.** Reverting the
projection *alone* gives `0.0`, not the `-0.48` the notes attribute to it —
that number is both halves off (`PIXEL_PHYSICS_SENSOR_PROJECT=none`). Quoting
it for the projection alone attributes one half's damage to the other.

### 1b. The per-colony books bar compared one colony against a cancelled sum

`the_books_close_for_every_colony` barred a colony's drift at `world_drift +
1e-3`, on the reasoning that a colony cannot drift more than the world. **The
world's drift is not an envelope around the colonies' — it is their signed
sum**, so two colonies erring opposite ways cancel and the whole reads tighter
than either part. Measured, `sum_of_colony_d == world_d` to every printed
digit:

| frames | colony 1 | colony 2 | world | old bar |
|---|---|---|---|---|
| 3,000 | +0.01219 | +0.00699 | +0.01918 | 0.02018 |
| 6,000 | +0.02288 | −0.00162 | +0.02127 | 0.02227 |
| **12,000** | +0.01157 | **−0.02809** | −0.01652 | **0.01752** |
| 24,000 | −0.01189 | −0.06327 | −0.07516 | 0.07616 |

12,000 frames is this test, and it is the row where the signs differ.

**It is rounding, not a leak, and the sign is what settles it.** Colony 1 runs
+0.0122, +0.0229, +0.0116, **−0.0119** — a walk that changes sign, which no
leak does. Against throughput the drift sits at **4e-7 to 1e-5**, `f32`
accumulation territory and four orders below the smallest joule this engine
books. A previous reading took absolute values, saw "monotonic growth", and
left a possible leak on the table.

The graded crop is what surfaced it: paying a meal out over ~291 chewing ticks
rather than one lump multiplies the `f32` additions by the same factor, so
every drift grew until the old 1e-3 slack stopped covering the cancellation.

**The new bar is scaled to the colony's own throughput**, and was
sensitivity-checked rather than argued: routing colony 2's `Metabolized` into
colony 1 takes it red by **1,234x** (1264.57 J against a bar of 1.02) while
`every_account_sums_over_the_colonies` stays green on the same tree — which is
the point, since a charge on the wrong colony leaves the world total right.

---

## 2. The bed was measuring birth position wearing a navigation label

`trailfollow` founds its colony with a cursor at `nest_x` and lays the
hand-laid food ramp `nest_x..=target_x`. **The colony is much wider than the
ramp's foot.** At `ants=20` the founders span `x 12..88` while the ramp starts
at 48, so everything west of the nest carries no channel B at all and reads it
as exactly `0.00000`. Split on that line, 240 ants:

| born | reached food | closed two laps | median life |
|---|---|---|---|
| west of `nest_x` | 40% | 1% | 979 ticks |
| on or east of it | 98% | 23% | 3,643 ticks |

That is a cliff, not a gradient, and it is the strongest single predictor of an
ant's life in this bed. `CLAUDE.md`: *a scene that contradicts the code will
look like a bug in the code.*

### 2a. The founding corridor is twice the body width, and one body width works

`colony_stations` spaces founders by `COLONY_ANT_SPACING.max(body_span * 2)`,
citing dead ends 775/829's **27,386 blocked ticks** from a gridlocked colony.
Measured over the setting, `ants=20`:

| spacing | band founded | founders placed | born on the comb |
|---|---|---|---|
| 1 | `39..57` (18 cells) | **10 — half the colony, silently** | 10 of 10 |
| **2** | `30..68` (38) | **20** | **20 of 20** |
| 3 | `21..78` (57) | 20 | 15 of 20 |
| 4 (shipped) | `12..88` (76) | 20 | 11 of 20 |

**Spacing 1 founds half the colony and nothing says so.** The corridor admits
the column and the *placement* then refuses it, because the ant is two cells
nose-to-tail and its second cell lands on the neighbour. `found_colony_of`
returns the count and the harness only checked `> 0`. That is the denominator
failure the `funnel` skill exists to prevent — a ten-ant arm compared against a
twenty-ant one — so `examples/trailfollow.rs` now asserts the full count.

**Spacing 2 is the floor that works**: full colony, band halved, and *every*
founder on the nest comb against 11 of 20 shipped.

### 2b. Stacking does not narrow a founding band, and it was worth checking

`PIXEL_PHYSICS_STACK_DEPTH` is true co-occupancy — animals of one colony share
a cell — so a deeper cap ought to allow a tighter founding. **It does not.**
At `ants=20`, `spacing=1` founds **10** at `STACK_DEPTH=1` and **10** at
`STACK_DEPTH=4`, identical bands at every spacing tried. `stack_cap` gates
*"may I enter a cell"* — the step — and `plant_creature_seed_in` never consults
it, so the founding walk sees the pre-stacking world whatever the cap says.

The two levers are orthogonal and answer different questions. What the deeper
cap buys is **flow on a working trail**, which is the case it was built for and
is section 3.

---

## 3. Five arms, 24 seeds, paired within seed

`gap=90`, `ants=20`, 24,000 frames, `stop=6000`, `RAYON_NUM_THREADS=2` pinned.
Arms are one binary and environment switches, so no recompile sits between them.

| arm | spacing | stack | ramp foot |
|---|---|---|---|
| A base | 4 (shipped) | 1 (shipped) | nest |
| B narrow | **2** | 1 | nest |
| C stack | 4 | **4** | nest |
| D narrow+stack | **2** | **4** | nest |
| E all | **2** | **4** | **founders** |

### 3a. The headline, on the measure that cannot be inflated

**`DELIVERED` says 4.04x and it is wrong to quote it.** `instruments.md` records
why: `CreatureStats::deliveries` increments on *any* drop while `at_nest`,
whatever was dropped and wherever it came from, so **any** change that drives
laden ants homeward inflates it by construction. `ate J` is the ledger measure.

Per-seed medians, paired within seed:

| arm | ate J (larder intake) | vs base | seeds up/down | starved | up/down |
|---|---|---|---|---|---|
| A base | 10,356 | — | — | 16.5 | — |
| B narrow | 11,936 | +15% | 17 / 7 | 8.0 | 9 / 15 |
| C stack | 10,655 | +3% | **12 / 12** | 8.0 | 5 / 18 |
| D narrow+stack | 13,832 | +34% | 19 / 5 | **5.0** | **0 / 22** |
| E all | **14,707** | **+42%** | **20 / 4** | 6.0 | 3 / 20 |

**Starvation is the cleanest signal in the table.** D halves the median and is
down in **22 seeds and up in none**.

### 3b. Stacking fires and, on its own, feeds nobody

This is `CLAUDE.md`'s counter rule playing out exactly as written — pair the
"it fired" counter with an effect counter from the far side of the call.

| arm | blocked moves | `ate J` paired |
|---|---|---|
| A base | 12,454 | — |
| B narrow | 12,289 (−1%) | 17 / 7 |
| **C stack** | **6,555 (−47%)** | **12 / 12** |
| D narrow+stack | 7,098 | 19 / 5 |

**Stacking demonstrably works** — it halves blocked moves, which is what it was
built to do — and **on its own it is a perfect coin flip on whether the colony
eats**. Had `blocked` been the only instrument, or `DELIVERED` (which stacking
alone raises 2.3x), this would have shipped as a large win.

**But it is not inert, because the levers interact**: narrow alone is +15% and
narrow+stack is +34%, so stacking roughly doubles what narrowing buys. The
reading that fits is that narrowing puts every founder on the nest and so puts
the traffic *on the trail*, and stacking is what lets traffic flow — a lever
that only pays once there is congestion to relieve. **That is a hypothesis the
`blocked` column does not establish**, since it counts the whole world rather
than the route, and a per-route jam census would be the test.

### 3c. What did not move

`carry->nest` (signed cells homeward) barely shifts — 1,074 base against 1,207,
1,014, 1,099, 1,092 — and the **homeward tumble share falls in every arm**
(6.6% → 4.5% in E, down in 22 of 24 seeds). So none of this is better homing.
What moves is how many animals are positioned to forage at all, and whether
they can get past each other once they are.

---

## 4. The trail over time — and the metric that had to be thrown away first

The question was *how the hand-laid trail compares to the ant-laid one, and how
it changes over time.* Two axes, so the answer is a picture:
`scripts/btrailchart.py` draws route across, time down, channel B as
brightness, with the `stop` handover marked in the gutter.

**Two instruments had to be corrected before the answer meant anything.**

### 4a. `b_profile` is a mean over the run, and `a_profile` beside it is not

Same name shape, two different quantities. With `stop=6000` in a 24,000-frame
bed, 60 of the 240 samples are taken while the hand-laid ramp is still being
refreshed at full `DEPOSIT` — far above anything the ants lay — so the column
**prints a healthy ramp for a run whose route is bare for its last 18,000
frames**, and was read that way. A healthy `b_profile` is not evidence the
trail survived.

### 4b. A peak cannot see a hole, and that is how a spike passed as a trail

The first reading of these runs quoted the **ant-laid peak as a share of the
hand-laid peak** — 18%, 27%, 32%, 78% across four seeds — and concluded the
outcome was a wide distribution with seed 4 nearly succeeding. The owner read
the panel and rejected it on sight: *"it is caused by a single bright spot not
a good trail from nest to food."*

Correct, and `max()` over cells is structurally incapable of seeing it. The
replacement is what an animal walking the route would meet — the **longest
unbroken dark run**, and whether it is shorter than the ant's own
`sensor_offset`, because a gap the animal can see across is not a break:

| seed | ant-laid cells lit | worst dark gap | route connected |
|---|---|---|---|
| hand-laid, every seed | **91/91** | **0** | **100%** of frames |
| 1 | 0/91 | 91 | 8% |
| 2 | 26/91 | 47 | 8% |
| 3 | 25/91 | 44 | 8% |
| 4 — the "78% peak" | 30/91 | **43** | 8% |

**Seed 4 has a 43-cell hole in a 91-cell route.** On peak it looked four times
better than seed 1; on connectivity all four seeds are the same, and the answer
is not a distribution at all:

> **The colony never builds a trail from the nest to the food. In any seed.**
> The hand-laid ramp is connected on 100% of frames with a worst gap of zero;
> what the ants maintain is connected on 8% of frames with a median worst gap
> of 43 to 91 cells.

What they *do* build is a blob near the nest and episodic streaks along the
route — visible directly in the panels, and consistent with
`pheromone-lifetime-and-wiring-2026-09-14.md`'s finding that a trail is a live
map of where ants are standing rather than a memory of where they went.

**This is `CLAUDE.md`'s *ask what your number counts* with a new instrument on
the list.** A peak is arithmetically correct, moves when the mechanism moves,
and answers a different question than the one asked — and it took an eye on
the picture to catch it, not another statistic. `continuity()` now carries a
positive control in `--selftest`: a single blazing cell must **win on peak** and
**fail as a trail**.

---

## 5. What shipped, and what is behind a switch

**Default behaviour is unchanged.** Every lever is a selector, because
`colony_stations` founds the real game's `Y` key and the evolution lab's beds
through the same function — a placement change reaches three games at once.

| switch | default | what it does |
|---|---|---|
| `PIXEL_PHYSICS_COLONY_SPACING=<n>` | off (`4`) | founding corridor; `2` is the floor that founds a full colony |
| `PIXEL_PHYSICS_STACK_DEPTH=<n>` | `1` | pre-existing; co-occupancy, and it is a movement rule |
| `trailfollow layfrom=founders` | `nest` | the hand-laid ramp's foot |
| `trailfollow btrail btrailevery=<n>` | off | the trail as a time series |

Three new instruments, each with a positive control:

- **`scripts/funnelpair.py`** — pairs `trailfollow`'s FUNNEL blocks within
  seed. The third of three pairing scripts and the one that was missing;
  `trailpair.py` pairs what the colony did and `tracepair.py` what the ant
  read, and the stages between them had to be read by eye. Reproduces the
  2026-09-20 sensor result off the stored logs and adds the per-seed direction
  it could not quote (**24/0/0** on three consecutive stages). Its `--selftest`
  is a constructed inversion: arm B wins the pooled total 25 → 76 on one
  oversized colony and loses the paired sign test 1 up / 4 down.
- **`scripts/btrailchart.py`** — the trail as a picture, plus `--stats`
  continuity. §4b is its story.
- **`scripts/loopchart.py`** — one ant's *whole* loop from the focal trace,
  answering review card `20260920T063732278Z-044d7d`: *"Why do both of these
  start at the food? Should we start away from food, move towards, become
  laden, go home."* That visual began at the larder because the loop did not
  close; it does now. Its `--selftest` asserts an ant that never carried
  anything **cannot** draw the laden colour, which is the old visual's failure
  mode.

## 6. What this does not settle

- **Gap 90 only.** The owner's standing instruction (2026-09-18) puts 90/140/200
  in every run, on the reasoning that survival at 140 and 200 *is* the success
  signal of recruitment. This ran 90 to establish the levers first. The sweep
  is now worth paying for, because for the first time there is an arm that might
  clear those floors.
- **`layfrom=founders` moves two things at once.** Renormalising `t` over the
  longer span also lifts the ramp's floor — the cell at the nest cursor goes
  from exactly **zero** to ~29% of `DEPOSIT` — so "the trail reaches further
  west" and "the trail is readable at the nest at all" are not separated here.
  A third arm holding the slope fixed would do it.
- **The stacking interaction is a reading, not a result.** `blocked` counts the
  whole world, not the route, so "narrowing creates the congestion stacking
  relieves" is the hypothesis that fits and is not established. A per-route jam
  census is the test.
- **Why no colony holds a route-length trail** is untouched here. §4b measures
  that it does not; `pheromone-lifetime-and-wiring-2026-09-14.md` already has
  the mechanism candidate — a trail is a live map of where ants are standing,
  unreinforced cells die in ~144 frames against a ~2,200-frame round trip, and
  `(CarryingFood, EmitB, 2.5)` is thirteen times weaker than the homing
  odometer's `(4, EmitA, 32.0)`.

---

## Data

- `Reports/data/forage-ladder-24seed-2026-09-21-{a-base,b-narrow,c-stack,d-narrow-stack,e-all}.log`
  — the five arms of §3, 24 seeds each. Read them with
  `python3 scripts/funnelpair.py <a> <b>` for the stages and
  `python3 scripts/trailledger.py <dir> <baseline>` for `ate J`, which is the
  one to quote.
- `Reports/data/btrail-4seed-gap90-2026-09-21.log` — the time series of §4.
  `python3 scripts/btrailchart.py <log> --grid` for the picture,
  `--stats` for the continuity table.

Review cards: `20260921T211003508Z-37b573` (the picture) and
`20260921T212556849Z-ecbaf8` (the correction that supersedes its metric).
