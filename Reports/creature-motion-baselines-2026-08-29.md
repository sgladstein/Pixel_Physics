# Lane C — creature motion baselines on today's `main` (2026-08-29)

**Status: measured.** Every figure below was taken on `main` at `3c4cc2b`
(worktree `agent-a76d4f129da25d3ef`, **0 ahead / 0 behind `origin/main`**, 0
changed files against it at session start), on one container shared with three
other agents. Release binaries rebuilt from source before any of it, exit code
read directly and never through a pipe.

**Everything gated below is a counter, not a wall clock.** The one timing
figure is quoted with its caveat and carries no conclusion.

---

## 0. HEADLINE — CORRECTED. The scheduler starves creatures once you dig;
this lane measured an undisturbed colony and correctly found that it does not.

**Read this before §0's original text below, which is left standing because its
arithmetic is right and only its scope was wrong.**

Every number in this section is reproducible and every control in it fires. What
it does not say — and was read as saying, by me as much as anyone — is anything
about a world with a **pick** in it. The scene here is an undisturbed 64-ant
colony: nothing generates structural checks, so nothing competes for the budget,
so of course the budget never binds.

Lane A subsequently reproduced the owner's complaint on demand by adding one
thing this scene has none of — **digging**. On `8192x2560 rolling seed 1`, a
64-ant colony, the pick swung every 20 frames, on stock `main`:

| frame | sites due | of which structural | of which creature | pending |
|---|---|---|---|---|
| 2,400 | 41 | 7 | **27** | 5,029 |
| 3,600 | **2,000 — the cap** | 1,974 | **11** | 20,467 |
| 4,800 | **2,000 — the cap** | 1,964 | **0** | 43,321 |
| 6,000 | **2,000 — the cap** | 1,926 | **0** | 62,658 |

The colony stops. The mechanism is the min-heap: a backlogged structural check
sits at a `next_frame` in the **past** while a creature reschedules to
`frame + 6`, in the **future**, and `ActiveSite`'s `Ord` is `next_frame` first —
so past the cap a creature is not behind in the queue, it is **unreachable**.

**The general lesson, which is this file's real contribution and is CLAUDE.md's
own rule turned on its author:** *ask what your number counts when nothing is
wrong.* A null measured in a scene that cannot contain the fault is not evidence
of absence. The controls below are genuine — they prove the instrument works —
but a positive control for "the queue drains" is not a positive control for
"the queue can starve a creature", and only the second was the question. The
blast control (`deferred` 291 → 3,222, then draining) was the closest this lane
came, and it drains precisely because a blast is one event and mining is
sustained.

The fix and the full reproduction are in PR #118.

---

### §0 as originally written, on the undisturbed-colony scene

## 0. HEADLINE — the scheduler is NOT starving creatures. Ruled out. (SCOPE: no digging)

The coordinator's prime suspect does not survive measurement, and the controls
say the instrument would have seen it if it were there.

**512x320 (the app's current world size), 64-ant colony,
`scale_probe phases=1 load=ants:64 warm=1500 frames=600`, `SCHED_PASS`:**

| quantity | reading | what starvation would need |
|---|---|---|
| `deferred` (whole heap after the batch) | **241 – 346**, flat | thousands, and rising |
| `sites` popped per frame | **0 – 46** | 2,000 (the cap) |
| `MAX_SITES_PER_FRAME` | 2,000 | — |
| share of the frame's site budget used | **≤ 2.3%** | 100% |

**Creature ticks are served on the frame they are due, every time.** Over a
full 6,000-frame colony run the scheduler executed **46,000 creature ticks =
46 ants x exactly 1,000 bursts = 6,000 / 6** — the theoretical maximum. Not
one tick was deferred by one frame. `produced` equals `sites` exactly on every
creature frame (38/38, 46/46), so each tick schedules exactly one successor:
the queue is not self-sustaining and there is no leak.

**`CLAUDE.md`'s `produced 7042 / deferred 61488` is not a colony reading.** It
is an 8192x2560 structural-blast measurement from `open-bugs-handoff.md` §S.
Nothing resembling it appears with a colony at play scale.

### Both controls fire

- **Negative (specificity).** Same world, no ants: the `creature` slot never
  appears at all. The counter goes silent when there are no creatures, so it
  is reading creatures and not something else.
- **Positive (sensitivity).** Same world, `load=ants:64,blast:600`: `deferred`
  jumps **291 -> 3,222** on the blast frame, the structural slot goes 0.01 ms
  -> 12.23 ms, and it drains back to 324 within ~500 frames. **The instrument
  can report a large backlog, and would have.**
- The detail inside the positive control matters most: creatures were **still
  served at 38 per burst straight through the spike** (frame 2400,
  `creature 0.27/38`, `deferred` still 502). Even a real backlog did not delay
  them — the heap is ordered by due-frame (`ActiveSite`'s `Ord` puts
  `next_frame` first), so a creature due now sorts ahead of a structural check
  due later. **Creatures cannot be starved by queue volume alone; they would
  have to be out-competed by other work due on the same frame**, and at ≤46
  sites against a 2,000 cap there is no competition.

### Frame cost is not the answer either, at this scale

Whole frame **mean 2.338 ms, p90 4.389 ms, worst 8.100 ms**, and **0 of 600
frames (0.0%) exceeded the 16.6 ms budget**. Field is 78.2% of it; the
scheduler phase is 2.1%.

*Timing caveat, stated because the box was not quiet — three other agents were
running.* Read these as "nowhere near the budget", not as a pinned cost.
`mean x frames = 1,403 ms` against an 8.100 ms worst **pins at nothing**, so
the worst-frame figure is an order statistic over many similar frames and is
noise wearing a number — do not quote it. The counter half (0 frames over
budget) is what carries the claim.

---

## 1. THE ONE CHECK WORTH DOING FIRST — `creature_slowdown`

**A hypothesis, not a measurement, and checkable by one glance at the
screen.** I cannot see the owner's working copy, so I cannot confirm it. It is
the only mechanism I found that reproduces the reported symptom *including*
the part that rules everything else out.

`Clock::creature_slowdown` multiplies the ant's tick interval directly:

```
world.creature_due(base)      = frame + clock.creature_interval(base)
clock.creature_interval(base) = base * creature_slowdown      // clock.rs:446
```

The ant's `tick_interval` is **6**; `MAX_SLOWDOWN` is **30**. At
`creature_slowdown: 4` an ant acts every 24 frames and, at the colony move
rate measured in §2, takes **a step about once a second**. At 30, once every
three seconds. That is "long pause, move a pixel, long pause, move one more
pixel", literally.

**It is the only knob that slows creatures and nothing else.** Its own doc
says it scales "creatures and the pheromone plane". Sand, water and fire are
the CA sweep (`parallel::step`) and never touch it; plants are on the separate
`growth_slowdown`. **The owner's "only the creatures are slow" is this knob's
exact signature** — no other mechanism I found produces that asymmetry by
construction.

**How to check, in one glance:** `src/app.rs` appends ` creatures Nx` to the
status line whenever `creature_slowdown != 1`, and stays silent at baseline.

- Status line says **`creatures Nx`** -> that is the whole complaint; set it
  back to 1 in the options panel (`O` -> WORLD).
- Status line says **nothing about creatures** -> ruled out; the answer is §2.

`assets/clock.ron` on `main` ships `creature_slowdown: 1` and is clean in my
worktree. But **the tunables panel rewrites that file wholesale on save**
(`O` -> WORLD, `S`), it is read by the app and by nothing else, and
`World::new` leaves the clock at baseline — so **every harness number in this
report is blind to it by construction.** A slowed app against a baseline
harness produces exactly the split we are looking at: a stutter the owner sees
and no measurement here can reproduce.

---

## 2. Moves per 1,000 frames, per creature — the number nobody had

Baseline clock throughout. **Every tick denominator below is counted from the
per-frame `SCHED_PASS=1` census, not inferred from ant count x frames** — see
the correction in §2.2, which caught a 2x error.

| | lone ant | colony, bare floor | colony, real terrain + trees |
|---|---|---|---|
| scene | `forage_probe` control | `forage_probe` forage | `ascii` foraging loop |
| creatures actually ticking | 1 | 46 | **27** |
| frames | 6,000 | 6,000 | 12,000 |
| ticks **scheduled** | 1,000 | 46,000 | 54,000 |
| ticks **executed** | **1,000** | **46,000** | **54,000** |
| **deferred** | **0** | **0** | **0** |
| **moves produced** | **681** | **18,851** | **8,812** |
| **moves per executed tick** | **0.681** | **0.410** | **0.163** |
| **moves per 1,000 frames, per creature** | **113.5** | **68.3** | **27.2** |
| blocked / moves | 0.000 | 0.034 | 0.036 |
| tumbles / moves | 0.231 | 0.620 | (not printed by `ascii`) |

Ceiling is **166.7 moves per 1,000 frames** (one tick per 6 frames, every tick
moving).

**The fork the coordinator named is settled: creatures are not starved, they
decline to move.** Scheduled == executed exactly in all three scenes, so the
scheduler contributes nothing. On the scene closest to the app an ant converts
**16% of its ticks into a move** — one step per ~37 frames, about **0.6 s at
60 Hz** — against 68% for a lone ant on bare ground.

### 2.1 The positive control that makes this trustworthy

The lone ant's **0.681 moves per tick matches the ant's own authored brain gain
to within 2%**: `ant.ron` has `(Bias, Move, 2.0)` and `brain.rs`'s
`squash(x) = x/(1+|x|)` gives `P(move) = 0.667`. An independently-derived model
value and a measured counter agreeing to 2% is the strongest control available
here — and `ant.ron`'s own comment states the same arithmetic for the rejected
0.7 setting ("P(move) = 0.35, an ant steps about once every 17 frames"), so the
relationship is documented rather than fitted by me.

The counters also have **proven zeroes**: the lone ant records `falls 0` and
`blocked 0` on flat ground in the same binary and run that records 4,370 falls
for the colony. They are not always-on.

### 2.2 The denominator error this caught — and a scene bug

I first computed the `ascii` figure as `8,812 / (55 ants x 2,000 bursts)` =
0.080 moves/tick. **The counted denominator is 54,000, not 110,000: only 27
creatures ever tick, constant across all 2,000 bursts, with `deaths 0`.** The
assumed denominator was 2x wrong and would have published an 0.080 that is
really 0.163.

**That is a scene bug worth someone's attention** (not mine to fix, and I did
not chase the cause):

- `ascii`'s foraging loop calls `world.plant_ant` 55 times and gets **27**
  creatures — it loses more than half. That scene is real generated terrain
  with a grown stand on it, so the handoff's colony-scene finding is the
  obvious suspect (on seed 1, 217 of 308 columns have a leaf on top), but I
  have not verified it here.
- `forage_probe` plants 55 and gets **46** on a *bare stone floor* — so
  whatever costs `ascii` its 28 ants cannot be the same thing, and the
  canopy explanation does not transfer. Its food pile sits inside the span
  the ants are placed along, which is a candidate and equally untested.

In both cases `scene()`/the caller returns the **requested** count, so
`forage_probe`'s printed "per ant" line divides by 55 when 46 exist,
**understating per-ant moves by 20%**. Any per-ant figure taken off either
scene is low by that factor — which is why every per-ant number in this
report uses the counted tick denominator instead.

### 2.3 Where the missing ticks go — for Lane A

Not blocking: `blocked` is 3.4–3.6% of moves in both colony scenes. **This
kills the "~60% of moves blocked" figure still quoted in the record** (§4).
It is the brain declining, and `ant.ron` authors two mechanisms that do
exactly that:

- **`(Crowding, Move, -0.3)`** — a standing negative on moving while crowded.
  A colony ant is crowded and a lone ant is not, which is most of the gap
  between 0.681 and 0.410.
- **`(FoodAdjacent, Move, -1.5)`** — its own comment says this "leaves
  P(move) at 0.33 beside food rather than 0.55". **This is my leading
  hypothesis for the further drop to 0.163 on the tree scene**, and it is
  circumstantial rather than proven: that scene reports **`food cells 7679`**
  with 3,157 pickups and 3,138 drops against only 8,812 moves — an ant is
  handling food more than once every three moves. A forest floor carpeted
  with litter, leaves and moss makes `FoodAdjacent` true almost everywhere,
  and the authored response to food underfoot is to stop. **Not tested; the
  clean test is a paired run with the food list emptied, which needs an
  engine-side arm I was not authorised to build.**
- **The hidden gate.** `hidden_outputs` are ±2.5 against a Bias of 2.0, and
  the file's comment says a laden ant pointed away from the nest "computes
  `squash(2.0 - 3.75) < 0`, clamps to **P(move) = 0** and does nothing but
  tumble until it is pointed somewhere better".

`tumbles` per move is **0.620 in the colony against 0.231 alone** — the colony
ant spends its declined ticks turning on the spot. **A stationary, turning ant
is a designed state, not a fault** — and it is also exactly what "long pause,
move a pixel" looks like from outside.

### 2.4 The move rate is NOT constant in time — it falls ~24% and then holds

**This is the closest thing I have to "they moved faster before", and it only
appeared because the gate sweep ran three budgets.** A single-budget
measurement cannot see it.

Seed +0, `spacing=4`, the same run read at three budgets, converted to moves
per 6,000-frame window (differences of cumulative counts, so each window is
disjoint):

| window | colony (46 ants) | lone ant (control, no food) |
|---|---|---|
| 0 – 6,000 | **18,851** | **681** |
| 6,000 – 12,000 | **14,253** (−24%) | 662 (−3%) |
| 12,000 – 24,000 | **13,488** per 6k (−5%) | **262** per 6k (−61%) |

**The colony loses about a quarter of its move rate over the first 12,000
frames and then holds.** So a fresh colony really is faster than a settled
one — but by 24%, which is not "long pause, move a pixel", and it stops
rather than continuing to decay.

**The control's late collapse is the scene, not the animal, and the
arithmetic says so.** `ant.ron` gives `start_energy: 900`, `idle_cost: 0.10`,
`move_cost: 0.25`. Over 24,000 frames the lone ant gets 4,000 ticks: idle
costs 4,000 x 0.10 = 400, and its 1,867 moves cost 1,867 x 0.25 = 467, for
**867 of its 900** budget. It runs out at almost exactly 24,000 frames, and
the control scene has **no food by construction**. So the −61% is a starving
ant in a deliberately foodless arm, and **must not be read as "ants slow down
over time"** — the colony, which has food, does not do this.

Two things follow for Lane A. The colony's −24% is real and worth explaining
(crowding rising as ants spread out and meet, or pheromone channels filling —
both untested by me). And **any creature measurement taken at a single frame
budget is quoting one point on a curve that moves by a quarter**, which is
its own reason to state the budget beside every number in this area.

### 2.5 A second amplifier: every creature in the world ticks on the same frame

`creature_due` is `frame + interval` with **no jitter**, and a colony is
planted in one call, so all ants share a phase for ever. Per-frame census:

```
frame 1506  creature 0.37/38      frame 1509  (none)
frame 1507  (none)                frame 1510  (none)
frame 1508  (none)                frame 1511  (none)
                                  frame 1512  creature 0.30/38
```

**Nothing creature-shaped happens on 5 frames out of every 6**, then all of
them move at once. At 60 Hz that is a 10 Hz strobe; the lower the frame rate,
the more it reads as hopping.

I am not proposing a change to it. But it means **creature motion degrades
visibly under load while the per-frame CA (sand, water, fire) degrades only
smoothly** — so a plain frame-rate drop would present as "only the creatures
are slow" without the queue being involved at all. That belongs on Lane A's
list beside the queue, because it means the owner's "only the creatures are
slow" does **not** by itself rule out a global frame-rate cause.

---

## 3. Falls per move, re-taken — the §7 gate

### 3.1 The exact re-take, on the identical scene §7 measured

`ascii` foraging loop, worldgen seed 1, 12,000 frames, baseline clock,
deterministic (my run reproduces yesterday's counters byte for byte):

```
after 12000 frames: 153 live organisms | moves 8812 blocked 313 falls 1217
  | eats 3 pickups 3157 digs 31 drops 3138 deliveries 1047 nest-visits 1207
```

**`1,217 / 8,812` = 13.8%, against §7's `1,629 / 11,031` = 14.8%.**

The *ratio* barely moved (−1.0 point) but **both operands did**: moves −20%,
falls −25%. A gate written on the ratio would have looked untouched while the
scene underneath it changed substantially.

### 3.2 Provenance correction — §7's warning is half right

§7 says 14.8% "predates `d007c156` and `4c95233`". The first is right; the
second is confused, and it matters because it sends the reader to the wrong
document.

- `4c95233` **is** PR #109 — lane C's re-baseline — and that report *contains*
  the `11,031 / 1,629` reading in its §1, measured at `ba6fc98` (2026-08-28,
  before `d007c156`).
- The **same report** also carries the post-`d007c156` reading in its
  "colony-scene fix" section: `moves 8812, pickups 3157, deliveries 1047,
  trips 23, deepest 28`. That is exactly what today's `main` produces.

So **the superseding number was already in the record**; it was filed as a
check that the colony-scene repair changed nothing, not as a falls baseline,
and it did not quote falls. Today's run supplies the missing `falls 1217`.
Two different `moves` figures for "the `ascii` scene" sit in one document
without a note saying which is which — worth a one-line fix when someone next
edits it.

### 3.3 Why 13.8% should still not be the gate

One seed, one scene, one budget — and **not settled at a short budget**. The
same scene reads:

| budget | moves | falls | falls/moves |
|---|---|---|---|
| 2,000 frames | 2,457 | 421 | **17.1%** |
| 12,000 frames | 8,812 | 1,217 | **13.8%** |

A gate set from the 2,000-frame reading would have sat 3.3 points high.

### 3.4 The gate figure: seeded order statistic across three budgets

`forage_probe seeds=12 spacing=4` (`COLONY_ANT_SPACING`, what a real colony is
founded at, not the historical jammed spacing 2). **12 seeds, min / median /
max, never a mean.**

| budget | day/night periods | falls/mv min | **median** | max | blocked/mv median | moves median |
|---|---|---|---|---|---|---|
| 6,000 | 1.67 | 0.218 | **0.239** | 0.278 | 0.036 | 18,080 |
| 12,000 | 3.33 | 0.208 | **0.225** | 0.334 | 0.034 | 33,021 |
| 24,000 | 6.67 | 0.201 | **0.215** | 0.351 | 0.033 | 59,919 |

**Settling evidence — and it does NOT fully settle. Stated plainly because
the honest answer is more useful than the tidy one.** The median runs
**0.239 -> 0.225 -> 0.215**: each doubling of the budget takes about one
point off it, and the steps are shrinking (−1.4 then −1.0) but have not
stopped. By the strict rule — a quantity that has stopped moving across two
consecutive samples — **this quantity has not settled at 24,000 frames.**

What makes it usable anyway is the ratio of the two spreads: the
between-budget drift is **1.0 point** while the within-budget seed spread at
24,000 frames is **15 points** (0.201 to 0.351). The budget dependence is an
order of magnitude smaller than the seed noise the gate already has to
tolerate.

**The operational consequence is a real one, so do not skip it: the gate must
fix its frame budget.** The same colony on the same build reads 0.239 or
0.215 depending only on how long it ran. A falls-per-move bar quoted without
its budget is not reproducible.

Most of the drift is the move-rate decline in §2.4, not the fall mechanism.

**Oscillator:** `falls/moves` is a ratio of two counters accumulated over the
same window, so day/night phase largely cancels rather than aliasing. The
budgets span **1.67, 3.33 and 6.67 full day/night periods** (baseline clock,
`DAY_NIGHT_PERIOD_FRAMES = 3600`) and the median moves ~1 point across them,
which is direct evidence the ratio is not riding the cycle. The ant is also
**not wired to `LightHere`** — that input exists but appears in neither
`instincts` nor `hidden_wiring` — so day/night reaches ant movement only
indirectly (`TempAboveAmb -> Turn` is the one authored weather-coupled input,
and it steers rather than gates movement).

**Positive control on the falls counter:** the lone-ant arm records **`falls
0`** across 6,000 frames while the colony arm records 4,370, same binary, same
run. Proven zero and proven non-zero.

### 3.5 The two scenes disagree, and a gate must name its scene

**13.8% on `ascii`'s real generated terrain, 22.5% on `forage_probe`'s flat
floor** — a 1.6x difference between two perfectly good instruments. Falls come
from ants losing footing while climbing over each other (`climbs_over_kin`),
and how often that happens is a property of the ground.

Both are far below the **59–80%** that §7 exists to catch, so either works as
a gate. **They must not be used interchangeably**, and the number the impulse
verb is judged against should be the seeded `forage_probe` median with its
scene, spacing, seed count and budget stated — not a single `ascii` run.

### 3.6 Recommended gate

**`forage_probe seeds=12 frames=12000 spacing=4`, `falls/mv` median, known-good
0.225 (min 0.208, max 0.334).** Set the bar with headroom above the measured
**max**, not on the median — the max is the seed that reshuffles on any
legitimate change. This replaces 14.8%, which should not be quoted again.

12,000 frames rather than 24,000 for two reasons, both measured: the colony's
move rate has stopped falling by then (§2.4), and it costs a quarter of the
runtime for a median 1.0 point away. **Quote the budget with the number every
time** — per §3.4 the figure is meaningless without it.

---

## 4. Creature-record numbers that should not be quoted

The handoff's rule — anything measured before the worldgen merge `39e6f36`
(2026-08-28 22:49) describes a world that is gone — plus what today's run
directly contradicts. **Flagged, not re-measured, except where I happened to
measure the replacement.**

| Where | Number | Why it should not be quoted |
|---|---|---|
| `foraging-range-measurement.md` §3 | `ascii` foraging loop: `forage_trips` **143**, `nest_visits` **6,014**, deepest **19**, profile `[6020, 787, 313, 143, 8, 0, 0, 0]` | **Directly contradicted today**: 23 trips, 1,207 nest-visits, deepest 28, `[1209, 176, 67, 23, 2, 0, 0, 0]`. Trips 6x high, nest-visits 5x high, deepest 32% low |
| `foraging-range-measurement.md` §3 | "**The colony works a 19-cell bubble**" and "*zero* excursions in any scene reached 32" | Today's `ascii` run reaches 28 with 2 excursions past 32; `forage_probe` medians deepest **57–71** with 4–8 excursions past 32 every seed. The headline finding of that report no longer holds |
| `foraging-range-measurement.md` §3 | 55-ant arm deepest **12** vs lone ant **42** | Pre-worldgen *and* pre-`climbs_over_kin`; that row is also the jammed spacing-2 scene its own 2026-08-23 correction warns about. Today at spacing 4: median deepest 57 |
| `creature-review-2026-08.md` §1 item 3 | "**~60% of moves blocked**" | **Measured today at 3.4–3.6%** in both colony scenes — a 17x error. Superseded twice: by `climbs_over_kin` (`ant.ron` records 0.311 -> 0.033) and again by the worldgen change. The most dangerous number in the record, because "the colony is jammed" is a live premise for traffic work |
| `creature-review-2026-08.md` §1 item 3 | "98 trips, deepest 18, reach `[3858, 475, 185, 98, 1, 0, 0, 0]`" | Pre-worldgen; measured 23 trips / deepest 28 today. The `98` is also already flagged superseded by the re-baseline |
| `creature-review-2026-08.md` §2 | "known-good today reads deepest 12–18 with buckets ≥32 at zero" — the WP-9 **decision rule** | This is a *bar*, not just a figure, and it is now trivially passed by the baseline: deepest medians 57–71 and the ≥32 bucket is non-empty on every seed. Any WP-9 judgement made against it would read as a large win with nothing having changed |
| `creature-review-2026-08.md` §1 item 2 | "deliveries +17%, moves −31%, nest visits −36%, digs −46%" (the floor-feeds-the-colony finding) | Pre-worldgen and pre-litter-merge; the report index already marks the S4 numbers superseded |
| `creature-evolution-plan.md` §S1–S4 "As built" | deliveries **238 / 313 / 408**, pickups 372/264, `blocked` 7,342 -> 3,292 | Measured 2026-08-19 to 08-23, pre-worldgen. `Reports/README.md` already says "every S4 number in them predates the litter merge and is superseded by it" — this adds the worldgen change on top |
| `creature-motion-design.md` §7 (lane B branch) | falls-per-move **14.8%** | Replaced by §3 above. Also see §3.2: its stated provenance is half wrong |
| `creature-rebaseline-2026-08-29.md` | **both** `moves 11031` (§1) and `moves 8812` (colony-scene-fix section) | Not stale — but the document gives two different `moves` figures for "the `ascii` scene" without saying the second supersedes the first. A reader gating on "moves" can pick either |
| Anything quoting `ascii`'s foraging scene **per ant** | any per-ant figure | The scene plants 55 ants and runs **27** (§2.2). Every per-ant number off it is ~2x low |

**Not flagged, and worth saying so:** `creature-rebaseline-2026-08-29.md`'s
own §2 guard re-baselines, `creature-appearance-design.md` and
`creature-export-design.md` were all measured on or after 2026-08-29 and are
not affected by the worldgen change.

---

## 5. What I could not measure, and why

- **Whether the app is actually slow.** Every number here is headless and at
  baseline clock. `World::new` leaves the clock at baseline by design, so a
  harness cannot see `assets/clock.ron`. §1 is the check that closes this and
  it needs the owner's screen, not a harness.
- **Whether `FoodAdjacent` is what drops the tree scene to 0.163 moves/tick.**
  The clean test is a paired run with the ant's food list emptied — an
  engine-side arm, and I was measurement-only.
- **Whether creature motion was faster before.** "Faster before" needs a
  paired A/B against an older commit on one instrument. I did not build it:
  the re-baseline already records that **every `ascii` creature counter was
  byte-identical across `d007c156`**, so the change the handoff points at did
  not move creature motion, and the remaining candidate (`39e6f36`, worldgen)
  would need two builds. **This is the highest-value thing left undone**, and
  it is cheap now that the instruments are pinned: build `forage_probe` at
  `39e6f36^` and at `main`, run `seeds=12 frames=12000 spacing=4` on both,
  compare `moves` and `falls/mv` medians.
- **Per-seed ant counts in the sweep.** The tick denominators in §2 are
  counted, but on one seed each (46 for `forage_probe` seed +0, 27 for
  `ascii` seed 1). Placement varies with terrain, so per-ant figures across
  the 12-seed sweep are approximate; the *ratios* (`falls/mv`, `blocked/mv`)
  are not affected, since both operands come from the same run.

---

## 6. What I changed

**No engine behaviour was changed. I added no counters to `src/`.**

One edit, to `examples/forage_probe.rs`: `Row` gains `falls` and `tumbles` —
both already on `CreatureStats` and simply not printed by this harness — plus
`falls/mv` and `tumbles/mv` order-statistic rows and a `falls`/`tumbles` line
in the single-seed report. Comments added in the house voice recording why
`falls` needed to be a seeded order statistic rather than one `ascii` run.

**Control on my own edit:** the same invocation before and after returns
`moves 681` and `moves 18851` — **byte-identical** — while the new `falls`
line appears. The binary is fresh (not the stale-binary tell) and the change
moved no simulation state.

**Landed on a branch, not left in a worktree:**
`claude/creature-motion-baselines-lane-c`, head **`b3f94f6`**, one file
(`examples/forage_probe.rs`, +45/−1), pushed. `cargo clippy --release
--example forage_probe -- -D warnings` is clean locally — but the container
ships an older clippy than CI's pinned 1.98.0, so that is not proof CI is
green, per `CLAUDE.md`. No PR opened; the coordinator owns the landing.

**Determinism check, which doubles as a control on the instrument:** two full
`ascii` runs — one plain, one under `SCHED_PASS=1` — return the foraging
scene byte-identically:

```
after 12000 frames: 153 live organisms | moves 8812 blocked 313 falls 1217
  | eats 3 pickups 3157 digs 31 drops 3138 deliveries 1047 nest-visits 1207 deaths 0
```

So **`SCHED_PASS` does not perturb the simulation**, and every counter in §2
and §3.1 reproduces. This also reproduces yesterday's re-baseline reading of
the same scene exactly.

Everything else came from instruments already in the tree: `SCHED_PASS`
(`scheduler.rs`), `scale_probe phases=1 load=ants:N`, `forage_probe`, `ascii`.
Per `Reports/instruments.md`, `frame_profile` has no creature scene and was
not used.
