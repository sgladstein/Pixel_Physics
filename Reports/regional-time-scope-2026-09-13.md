# Regional time in the held world — what a per-circle speed would take

**Status: scope. Nothing built, nothing recommended for immediate build.**
Written 2026-09-13 against `origin/main` at `f9dd3295`.

The owner asked that **each quickening have its own speed**, and asked for it
to be scoped. A per-circle dial was built earlier the same day, measured, and
withdrawn for one global `Druid::speed`; that work is on
`claude/determined-ramanujan-c9szc5` (commit `e27a4b15`) and **has not
landed**, so nothing described below as "shipped" is on `main` yet.

**What is measured here and what is reasoned.** Three things are measured:
the census of `World::frame` readers (§1), the behaviour of `rng::stream`
under a shortened or repeated key (§4b, with a positive control that
reproduces the shipped test), and the arithmetic of `old_age_chance_over`
under a rate change (§4a). Everything about cost, and every claim about what
a build would take, is **reasoned from the source** and labelled as such. No
held world was run for this report; §6 says why that is deliberate and what
the first run should be.

---

## 0. What this answers, and what already answers the rest

Two halves of "give a region its own clock" have been scoped separately, and
only one of them was done before.

- **The spatial half — *can a tick be restricted to a region?*** Answered, with
  numbers, in
  [`held-world-game-concept-2026-09-13.md`](held-world-game-concept-2026-09-13.md)
  §8. The CA sweep already iterates awake chunks; `field::step` already builds
  a per-frame tile subset and hands it to five of its eight passes (78.3% of
  field cost is per-tile-linear); the two sky passes are the floor and a held
  world removes them by premise rather than by gate. **Do not re-derive any of
  that.**
- **The temporal half — *what does time mean once two regions run at different
  rates?*** Nobody has written this down. It is what killed the per-circle
  dial, and it is this report.

The prior attempt is `Reports/dead-ends.md` **`other:124`** (on the branch
above, not on `main`). Its finding in one line: extra whole-world
`frame::step` passes were used to run circles faster, and **`World::frame` is
a single global counter in which every organism's cadence is expressed**, so
running the world eight times to speed one circle speeds the *scheduling* of
every circle. Its numbers, two circles on equivalent ground, 1,500 player
ticks, unlimited power:

| arm | circle @300 | circle @700 |
|---|---|---|
| control, both x1 | 58 | 75 |
| x1 beside x8 | **127** | 109 |

Its own closing warning is the one to carry: **the fast circle's number looked
right the whole way through** (143, then 109), so a measurement that reads
only the circle you sped up reports success.

One correction to how that table should be read, and it matters for §6: every
cell in it is **n = 1**. `CLAUDE.md` records that twelve identical trees from
one genome span 31 to 153 cells — a 4.9x spread — so 127-against-58 is a 2.2x
difference inside a distribution known to be wider than that. **The finding is
still right, but it is the mechanism that makes it right, not the number**:
the leak is visible in the source (`frame + interval` in a counter that
advanced eight times per update) and does not need a statistic. The number
alone would not survive this repo's own order-statistic rule.

---

## 1. Who reads `World::frame`, and what for

Census over `src/`, all `world.frame` / `w.frame` reads, split by whether the
line sits inside a `#[cfg(test)] mod` (brace-matched, not guessed):

| | sites |
|---|---|
| total | **303** |
| production | **163** |
| test / harness | **140** |

The test half is not noise to be discarded — it is a *class*, and the largest
single one. 63 of those 140 are **assignments** (`w.frame = frame`), and
`clock.rs`'s own doc already names the pattern: *"27 places in this codebase
assign `World::frame` directly, every one of them in order to select a time of
day or a weather window."* That is the reason `Clock::sky_frame` is derived
from `frame` rather than being an independent counter, and it is a standing
constraint on anything that replaces the clock: **a second counter that
`w.frame = 3600` does not move is a counter seven guards will stop seeing.**

### The seven production classes

| class | prod sites | needs a regional clock? |
|---|---|---|
| **Schedule** — `next_frame`, `frame + INTERVAL`, `due` | 24 | **Yes.** This is the whole mechanism |
| **RNG key** — `rng::stream(seed, …, frame, slot)` | 19 | **Care, not change.** See §4b |
| **Diagnostics** — log stamps, `timing.report`, HUD rows | 16 | **No** — but they want *player* time, not world time. Already learned once |
| **Modulo cadence** — `frame.is_multiple_of(N)` | 15 | **Split.** 11 are life or economy; 4 are physics parity, and those are a trap (§3c) |
| **Age / duration** — `frame - born_frame`, `frame - since` | 8 | **Yes, and this is the sharp one** (§4a) |
| **Phase clock** — `sky_frame`, `weather_frame`, `lightning_at` | 6 (+ `Clock`) | **No.** Legitimately global, and in a held world already stopped |
| **UI timeouts** — toast, shake-flash, druid message | 7 | **No.** Player time |

Two things the census settles that reading the code casually does not.

**The held gate has exactly two simulation callers.** `world.time_runs_at` is
read at `scheduler.rs:478` (per dispatched site) and
`plant.rs:6871` via `time_runs_for_organism` (per organism), plus one economy
read (`druid/mod.rs:465`) and one render read (`render.rs:7291`). **The CA
sweep, the field, liquids, rigid bodies and the player are not gated at all.**
So the shipped global dial at speed 8 runs *the whole world's physics* eight
times per player update; the held gate only means there is no *life* outside
the circles for those passes to advance. Weather and springs are the
exception — commit `1409bfb3` gated them per position on `time_runs_at` at six
sites, so those already follow the circles.

**The schedule seam already exists, it already distinguishes growth time
from physics time, and only two sites are genuinely outside it.**
`World::organism_due(base)` and `World::creature_due(base)` are the single
point where a life interval becomes an absolute due frame, and
`organism_due`'s doc claims *"every `world.frame + INTERVAL` in the plant
subsystem goes through here."* 23 callers do; eight do not, and **six of the
eight are deliberate**:

```
src/sim/plant.rs:2862    reschedule_organism(.., world.frame + SEED_TICK_INTERVAL)
src/sim/plant.rs:3802    reschedule_organism(.., world.frame + SEED_TICK_INTERVAL)
src/sim/plant.rs:4364    reschedule_organism(.., world.frame + SEED_TICK_INTERVAL)
src/sim/plant.rs:6397    world.frame + SEED_TICK_INTERVAL
src/sim/plant.rs:12898   reschedule_organism(.., world.frame + SEED_TICK_INTERVAL)
src/sim/rigid.rs:3823    next_frame: world.frame + super::plant::SEED_TICK_INTERVAL
```

`plant.rs:6388`, in place, with a reverted attempt recorded beside it:

> *"The seed cadence is deliberately **not** scaled by `growth_slowdown`… it
> is not a statement about how fast a seed grows, it is bookkeeping against
> how fast a seed **falls** — about a cell a frame, which is the physics rate
> and does not slow down when growth does. Scaling it was written and
> reverted: at `growth_slowdown: 8` a falling seed is 32 cells from where its
> `ActiveSite` says it is."*

`rigid.rs:3823` cites the same reason explicitly. **This is the most useful
thing in the census and it was very nearly written up as a bug.** The plant
subsystem has already drawn the exact line a regional clock needs — *is this
duration a fact about growth, or a fact about physics?* — and it drew it the
hard way. Anyone touching this seam must not "tidy" those six.

The two that are genuinely outside:

```
src/sim/plant.rs:2253    let due = world.frame + rebloom_after as u64;
src/sim/creature.rs:7766 let due = world.frame + thickness * organism_tick_interval(..)
```

`rebloom_after` is a plant-biology duration stored as an absolute frame in
`rebloom_pending` and compared against `world.frame` at `plant.rs:9592`, so a
plant at `growth_slowdown: 4` grows four times slower and reblooms at the
same wall-clock rate. `creature.rs:7766` sets a trunk-crossing deadline from
the individual's own `organism_tick_interval` but skips `creature_due`'s
clock scaling — while the *reschedule* it pairs with, at `:8501`, uses
`creature_due`. Both are small, both are pre-existing, and both are the same
one-line change.

The remaining raw sites are `evaporation` (5), `structural` (4),
`update::dissipation` (1) and `liquid`'s demote cooldown (2). All four kinds
are dispatched through `scheduler::step` and so are already held-gated, which
means they are already *regional in space*; making them regional in *rate* is
the same one-line change as the life sites and should be made at the same
time or deliberately not at all.

### What is legitimately global

The sky and the weather. Both are **phases** — pure functions of a frame
number modulo a period, read through `Clock::sky_frame` / `Clock::weather_frame`
— and a phase is the one thing that should *not* become regional, for the
reason `clock.rs` gives at length: rescaling a period silently re-derives the
quantisation and field-sleeping arguments that sit on top of it. A world with
two suns because two circles disagree about the hour is also just wrong.

In a held world this is already settled rather than merely arguable: the sky
is pinned (`clock.sky_hold`), which makes `sky_frame() == prev_sky_frame()`
permanently, which is half of `field::step`'s early-out. Keep them global.
**The one thing to check** is that a fine clock (§3d) does not make the sky
run `speed_max` times faster in an *unheld* world that uses the same
machinery — the sky must read player time, not fine time.

---

## 2. Where a region's identity would live

### Not the cell

`Cell` is 12 bytes and `aux` is kind-specific with no spare bits — the
widening from 8 bytes exists precisely because burn state and organism id
could not share it (`dead-ends.md`, `other`). There is no room for a region
tag and no argument for one: a region is a property of *place*, and the cell
already knows its place.

### Not the chunk, either — but the chunk is where the *gate* goes

`CHUNK_SIZE` is 64 and `CARRIED_RADIUS` is 28, so the player's own quickening
is smaller than one chunk and a placed one is a handful. Chunk-aligning the
region would make the carried circle a 64x64 square that snaps — visible, and
against the ring the interface already draws. **But the chunk is the right
granularity for the *sleep* gate** (§3d): "is any live circle overlapping this
chunk" is one test per awake chunk per pass, and a false positive costs a
sweep, not a wrong answer.

### The unit is the circle, and that is already true

`World::quickenings: Vec<Quickening>` with `Quickening { x, y, r }` and a
squared-distance `contains`. Adding `rate: u32` is the whole of the data
change; `time_runs_at` becomes `rate_at` returning `Option<u32>` and keeps its
`!self.held` fast path. Overlapping circles need a rule and the only sane one
is **fastest wins** — a slow circle inside a fast one is a hole the player
cannot see and cannot want.

### The organism that straddles a rim

**Already a known gap, already documented, and deliberately unsolved.**
`World::time_runs_for_organism` resolves at *one* cell — a creature at its
head, a plant at whichever cell its `PosMap` yields first — and its own doc
says: *"a plant straddling a rim is all-in or all-out. Whether a
half-quickened tree should grow on one side is a design question the concept
has not answered, and guessing at it in a resolver would bury the decision."*

That remains right, and a rate makes it *less* pressing rather than more: with
a binary gate, a rim through a tree is the difference between growing and not;
with a rate, it is the difference between 8x and 1x. **Recommend keeping the
one-cell resolution and letting the rate be read at the same cell.** The
alternative — per-cell rates within one organism — is not a boundary policy,
it is a different growth model, and it would have to answer what a plant means
whose root ticks eight times per shoot tick.

### The ant that walks across

This is the one that needs a decision, because a creature *will* cross, every
few seconds, and it carries state stamped in frames:

- **`born_frame`** (`organism.rs:6059`), read as `world.frame - born_frame` for
  age. Stamped in whatever clock was running when it was born.
- **`picked_up_frame`** on a `SeedPassenger`, read as `frames_carried`.
- **`crossing.due`** — an animal committed to working through a trunk.
- **`next_frame`** on its own scheduled site, already in the heap.

**The clean answer is to stop deriving duration from a global stamp.** An age
that is an accumulator the individual increments on its own tick — one `u32`
on `OrganismState`, incremented once per `creature::tick` — is correct under
every crossing, needs no reinterpretation, and is *cheaper* than the
subtraction it replaces. `born_frame` stays for diagnostics and lineage. This
is a small change with a large blast radius (§4a) and it is the single most
load-bearing item in this scope.

### The boundary of a fast circle and held ground

Nothing new. A site outside every circle is re-dated `HELD_RECHECK` (120)
frames out and skipped; `World::wake_region` pulls sites inside a new circle
forward to now so a placement lurches rather than trickling. Under a rate,
"outside" simply becomes "rate 0" and the same two mechanisms carry. The
**one** new case is a circle whose rate *drops*: sites inside it are then due
sooner than the new rate wants, and `wake_region`'s "only ever earlier"
invariant means it cannot push them back. Lowering a rate should therefore be
allowed to be *late by up to one interval* rather than corrected — the
alternative is a heap rebuild on a key the player can hold down.

---

## 3. The scheduler

`World::active_sites` and `World::creature_sites` are two `BinaryHeap`s keyed
on absolute `next_frame`; `scheduler::step` pops everything due, merges the
two ascending streams (not concatenates — dispatch order is a documented
determinism surface) and dispatches. `World::wake_region` already drains and
rebuilds both, and its doc prices that honestly: *"`O(n log n)` in the
scheduled-site count — fine when a player presses a key, ruinous every
frame."*

### 3a. One heap per region — reject

A region's heap must gain and lose sites whenever the region set changes, and
**the carried circle moves every time the player walks**. That turns
`wake_region`'s once-per-keypress rebuild into a per-frame one, which is the
exact cost its own doc rules out. It also multiplies the merge in
`scheduler::step` from two streams to two-per-region, and the merge is where
the determinism argument lives. The only thing it buys is not needing a common
key — which 3d provides for free.

### 3b. One heap with region-relative keys — reject, it does not typecheck

A min-heap needs a total order. Two regions' clocks are not comparable
quantities, so the keys must be converted to *something* common before they go
in, and once they are converted the scheme is 3c or 3d.

### 3c. Rescale the interval at push — cheap, and against the design of record

A site inside a rate-`r` circle pushes at `frame + interval / r`. One divide,
at a seam that already exists (`organism_due` / `creature_due`, plus the seven
leaks in §1). No heap change, no new counter, no rebuild. It is by some
distance the cheapest thing that produces a per-circle dial.

**And the concept report already ruled against it, with a measurement.**
§1d, quoting `frame::step`'s own doc: *"The tick is the unit of simulated time
and it is never scaled. A caller that wants the world to run faster calls this
more times per displayed frame; it does not speed anything up inside."* Under
it, `clock.rs`'s paired sweep: one tree for 4,000 frames at
`growth_slowdown: 1` against 16,000 at `4` — **the same number of organism
ticks either way** — gives a median **0.61x** final cells across eight seeds,
range 0.15x–1.34x, direction seed-dependent, spread about 9x. The concept's
conclusion: *"A quickening must be more ticks over a region, never faster
subsystems in a region."*

3c is that ruling run backwards. A plant ticking eight times per world frame
meets **one eighth** as much world per tick as it does today — the same
exchange asymmetry, mirrored. `clock.rs` is explicit that the mechanism is
unproven but the effect is not: *"every exchange a plant has with the world
outside its own tick is per real frame."*

**And the seed sites make it worse in a way that shows on screen.** Under 3c
the world's physics still runs once per player update, so a fast circle's
seeds keep falling at about a cell a frame while its plants grow eight times
as fast — `SEED_TICK_INTERVAL` is unscaled *because* it tracks the fall
(§1), and dividing it would reintroduce exactly the drift the revert recorded
there was withdrawn for. So germination lags growth by the rate: a rate-8
circle is a place where established plants race and new ones start at walking
pace. That is a legible defect rather than a subtle one, and it is 3c's, not
3d's — under 3d the region's physics is strided with everything else, so the
seed keeps its authored relationship to the fall.

Two honest points in 3c's favour, because it should not be dismissed without
them. First, **the slow circle stays bit-identical** — at rate 1 the interval
is unscaled and nothing about that region changes, so the failure that killed
the previous attempt cannot occur. Second, `CLAUDE.md` is emphatic that
exactness is not what this project optimises for, and "a fast bubble grows a
somewhat different tree than patience would have" may be a difference no
player can detect, since there is no side-by-side to detect it against. **That
is the owner's call, not an implementer's**, and it is worth putting in front
of them explicitly, because 3c is roughly a tenth of 3d's work.

### 3d. One heap, one fine clock, a per-region stride — the design of record's own answer

This is "more ticks over a region" made to work, and it needs **no per-region
counter** — a per-region *stride* on one global counter does the whole job.

Let `s = SPEED_MAX` (8 today). `World::frame` becomes the **fine** clock,
advancing `s` per player update; `Druid::ticks` is already the player clock
and that lesson is already paid for (the shipped commit records `frame % 30`
going 0, 8, 16, 24, 32 and never landing on 30, silently switching the whole
economy off). Restrict rates to divisors of `s` — 1, 2, 4, 8 — so a rate-`r`
circle has stride `k = s / r`. Then:

1. A circle is **live** on fine pass `f` iff `f % k == 0`.
2. `time_runs_at` answers for live circles only, so the two held gates keep a
   non-live circle's sites and organisms quiet on the passes it sits out —
   no new gate, and no narrowing of `World::quickenings` (which is what
   produced the withdrawn build's `3`: an excluded circle's cells read as
   *outside* and got re-dated `HELD_RECHECK` out).
3. Chunks overlapping no live circle are **forced asleep** for that pass, so
   the CA sweep and the field do no work there — §8's *"force everything
   outside the bubble asleep and this falls out"*, applied per pass instead of
   once.
4. Life intervals are pushed at `frame + interval * k`.

Steps 3 and 4 together are the point: a rate-`r` region is live on one fine
pass in `k`, and its intervals are `k` times longer in fine frames, so it gets
**one tick per `interval` frames of its own time and exactly `interval`
world-frames of physics between ticks** — bit-identical, in its own frame of
reference, to the world it lives in today. A rate-1 circle beside a rate-8 one
is unchanged; the rate-8 one genuinely runs eight times. That is what the
withdrawn build was reaching for and missed by leaving step 4 out.

Cost shape: `s` passes per player update, of which everything but the fastest
region sleeps through most. Roughly `8 x (fast region) + 1 x (everything
else)` — which is the same shape the shipped global dial already pays, minus
the seven-eighths of the world it currently sweeps for nothing.

---

## 4. What breaks

### 4a. Ageing — and it is neither of the two answers you would guess

The owner's question was whether a creature in an 8x bubble dies of old age
eight times sooner. **It does not, under any of the options, and the reason is
that the hazard is quadratic in age.** From `plant.rs:3990`, the shipped
function:

```rust
let per_frame = 2.0 * LN_2 * age_frames / (life_half_life * life_half_life);
(per_frame * interval.max(1) as f32).clamp(0.0, 1.0)
```

`age_frames` is `world.frame - born_frame` (`creature.rs:3452`). Simulating
that exact expression to the point where survival crosses one half, with `age`
kept in global frames while ticks arrive `m` times as often:

| rate `m` | dies at global age | in its own ticks |
|---|---|---|
| 1 | 1.000 x half-life | 1.00 x |
| 2 | 0.708 | 1.41 |
| 4 | 0.500 | 2.00 |
| 8 | **0.353** | **2.83** |

Exactly `1/sqrt(m)` and `sqrt(m)`. So a fast ant dies at a third of the age
and lives nearly three times as many ticks — *wrong in both directions at
once*, and it would read on screen as "fast colonies are somehow immortal and
somehow short-lived". This is arithmetic over the shipped closed form, not a
simulation of the engine; the model is self-consistent at `m = 1` (death at
exactly the half-life), which is the control that says the arithmetic is
right.

**The fix is §2's accumulator.** Age counted in the individual's own ticks
makes the hazard a function of the individual's own history, `m` cancels, and
a creature in an 8x bubble lives the same *life* while the world outside sees
it burn through that life eight times faster — which is almost certainly the
answer the fiction wants. **It is the owner's decision and should be put to
them as one**, because the alternative reading is defensible: a druid who
quickens a colony might reasonably be *spending* those animals' lives, and
"the fast bubble ages what is in it" is a price with a story.

Either way, **do not leave it as it is**: `sqrt(m)` is not a design, it is a
bug that happens to have a closed form.

Two neighbours behave better and are worth naming so nobody rewrites them:

- **`rot_remains`** uses `half_life_chance(remains_half_life, interval)`,
  which is `1 - 0.5^(interval/half_life)` — *exponential* in interval, so it
  commutes with a rate change exactly. A senescent tree in an 8x bubble is
  carried out eight times faster in world time and over the same number of its
  own ticks. Correct with no work.
- **`stale_ticks`** counts *ticks*, not frames (`ORGANISM_STALE_LIMIT = 4`), so
  it is already clock-relative and rescales for free.

`PLANT_SIZE_CADENCE` is a *multiplier on the interval*, so under 3d it
composes with the stride by multiplication and needs nothing; under 3c it
composes with the divisor and can floor to 1, which is the `organism_interval`
"a zero interval is an infinite loop inside one `step_active_sites`" hazard
arriving from the other side. Keep the `.max(1)`.

`pace` (`TRAIT_PACE`, heritable, via `tick_interval_of`) is likewise a factor
on the interval and composes cleanly. But note what a *region* rate does to
selection: under 3d a fast circle runs more generations, so it is a selection
accelerator, not merely a growth one. That is probably a feature and it is
certainly a thing to know before reading any lineage statistic out of a
quickened world.

### 4b. Determinism, and the RNG — measured

Determinism is required (same-build, `PLAN.md`). Nothing in any option makes
the world nondeterministic — a stride is data on the world like any other —
but two specific things need saying, and one of them is a live hazard for a
design nobody has written down yet.

**The hazard: a region that ticks `N` times inside one `world.frame`.** This
is the naive reading of "run the region eight times" — a sub-tick loop that
does not advance the frame counter. Every per-cell draw in the engine keys on
frame: `rng::stream(seed, organism, world.frame, SLOT)` at nine sites in
`creature.rs`, and `plant.rs`'s `growth_stream` on both its branches. Eight
ticks on one frame draw the **identical** number eight times.

Measured, replicating `rng::stream` and `Rng` exactly and reproducing the
shipped test `a_stream_stays_uniform_along_a_fixed_tick_stride` as the
positive control (stride 45 gives 0.3512 and 0.00210, inside that test's own
0.33–0.37 and 0.001–0.004 windows). 200 cells x 400 ticks = 80,000 rolls:

| p | stride | fires | cells that ever fired |
|---|---|---|---|
| 0.35 | 45 (today) | 28,098 | **200** / 200 |
| 0.35 | 0 (repeated key) | 27,600 | **69** / 200 |
| 0.02 | 45 | 1,610 | **200** / 200 |
| 0.02 | 0 | 1,600 | **4** / 200 |
| 0.002 | 45 | 168 | **113** / 200 |
| 0.002 | 0 | **0** | **0** / 200 |

**The total fire count is right and the distribution is gone.** Any counter
asking "did this happen, and how often" reads correct to within 2%; the number
of *cells it happened to* collapses from 200 to 69 to 4 to zero. That is
`CLAUDE.md`'s worst-recurring failure — an arithmetically correct number
answering a different question — and it is also a direct violation of the
ethos's first law: an outcome that was a distribution becomes a coin flipped
once per cell and then repeated. A rare event stops occurring entirely.

**Option 3d does not have this problem**, and that is the useful half of the
measurement: under 3d the fine clock advances every pass, so no organism ever
ticks twice on one `world.frame`. The same table clears the other worry too —
every stride from 1 to 45 is uniform (0.3465–0.3551 and 0.00194–0.00229), so
shortening the tick stride, which 3c does and 3d does not, costs nothing in
draw quality. **The measurement rules out an alternative and clears the
recommendation; it does not condemn either.**

The second determinism note is small: `growth_stream` under
`DevelopmentalKey::Plant` keys on `frame - germination_frame`, so under 3d
both terms are fine frames and the stride is `interval * k`. Covered by the
row above.

### 4c. Sweep direction — a real bias nobody would look for

`let rightward = world.frame.is_multiple_of(2);` is computed **once per
sweep, globally**, at `parallel.rs:110`, `update.rs:86` and `update.rs:67`
(the monolithic control). Its own comment: *"Sweeping right-to-left on
alternate frames cancels the directional bias that a fixed scan order would
otherwise bake into every pile and flow."*

Under 3d a rate-`r` region is live only on fine frames divisible by `k = s/r`,
and for every rate but the fastest, `k` is **even**. So every slow region
sweeps `rightward == true` on every pass it is awake for, for ever — a
permanent left/right bias in exactly the regions the player is *not*
quickening, in a subsystem whose own comment says that is what the alternation
exists to prevent. Nothing in the suite would catch it; it shows up as piles
leaning and flows preferring one direction, which reads as terrain.

The fix is that a gated chunk's parity must come from its region's own tick
count, `(frame / k) % 2`, not from the fine frame — which means the parity has
to be resolved per chunk rather than once per sweep. That is a real change at
a hot seam and it belongs in 3d's estimate. `CLAUDE.md`'s own note that
**chunk decomposition is a recurring root cause** applies directly.

### 4d. The gates that would stay green

Worth stating plainly, because it is the argument for §6. `bash
scripts/acceptance.sh` builds hand-placed structural geometry; `cargo test`
runs 1,324 lib tests almost none of which construct a held world with two
circles at different rates; `examples/ascii` reports frame timings. **None of
them can see a slow circle running at 2.2x real time.** The withdrawn build
was green throughout. The guard that would have caught it did not exist and
still does not: a paired two-circle bed whose *slow* arm is the assertion.

---

## 5. The three options, priced

| | (a) one global speed | (b) per-circle, interval divisor (3c) | (c) per-circle, fine clock + stride (3d) |
|---|---|---|---|
| **What the player gets** | one dial, 1–8, every circle at it | a rate per circle | a rate per circle |
| **Is it "more ticks"?** | yes | **no** — faster subsystems | yes |
| **Slow circle** | n/a | unchanged, bit-identical | unchanged, bit-identical |
| **Fast circle** | true 8x world | 8x cadence, ~1/8 the world per tick | true 8x |
| **Seeds** | correct | **germination lags growth by the rate** (§3c) | correct — the fall is strided too |
| **Build** | shipped | two real leaks closed, `rate` on `Quickening`, a divide at two seams, ageing accumulator | all of (b), plus fine clock, per-pass liveness, per-chunk sleep gate, per-chunk sweep parity, field subset gate |
| **Rough size** | 0 | one session | several, and it touches `parallel.rs` and `field.rs` |
| **Risk** | none | the 0.61x-median behaviour change, unquantified in this direction | the sweep-parity bias (§4c); the field's *"a region more generous than the global boundary goes inert and will look converged"* (§8) |
| **Against the design of record?** | no | **yes**, concept §1d | no — it *is* §8's answer |

### Recommendation

**Ship nothing per-circle yet. Do the two pieces of work that every option
needs and that stand on their own, then put (b)-versus-(c) to the owner as a
priced choice rather than an engineering one.**

The two pieces:

1. **Route `rebloom_after` and the trunk-crossing due through
   `organism_due`/`creature_due`** (§1), and **leave the six
   `SEED_TICK_INTERVAL` sites exactly as they are** — they are the growth /
   physics distinction already drawn, with a revert on record. Small, correct
   today, and a prerequisite for (b) and (c) alike. The larger value is the
   audit itself: the seam now has a stated rule — *a growth duration goes
   through `organism_due`, a physics duration does not* — and every site has
   been checked against it once.
2. **Replace `world.frame - born_frame` with a per-individual tick
   accumulator** (§2, §4a). Under *any* per-circle scheme, ageing is `sqrt(m)`
   without it. It is small, it is faster than what it replaces, and it is the
   only item here that is a bug in the shipped engine rather than a
   requirement of a feature.

Then the choice, which is a design question with a cost attached and not an
implementation detail:

- If the owner's reading of a quickening is *"this patch of ground is living
  faster"* — the fiction the concept report writes — it is **(c)**, and it is
  several sessions.
- If the reading is *"this circle grows things quickly and I do not care
  whether it grows the same things patience would have"*, it is **(b)** at
  roughly a tenth the cost, and the honest disclosure is that a fast bubble
  grows a somewhat different plant, median around 0.6x-in-reverse and
  seed-dependent, with no in-game way to notice.

**What would change this recommendation.** Two things, in order of how likely
they are to.

- **If §6's measurement finds the slow-circle spread is wide** — if the same
  1x circle over 12 seeds spans a factor of two or more — then the 127/58
  finding is inside noise as a *number* (it is not as a *mechanism*), and more
  importantly the acceptance test for (b) or (c) cannot be "the slow circle is
  unchanged", because nobody could tell. That would push toward (b): if the
  system cannot demonstrate the difference (c) buys, (c) is buying an argument.
- **If the owner rules that a quickened bubble should age what is in it** —
  which is a legitimate and quite good design — then §4a's accumulator gets a
  deliberate *inverse*, and that is worth knowing before it is written, not
  after.

One thing that would **not** change it: a cheap-looking way to make (b) exact.
Two attempts at per-circle rates have now failed in the same direction
(withdrawn build, and 3c's exchange asymmetry), and `CLAUDE.md`'s rule is that
two fixes failing the same way means the approach is wrong rather than the
tuning.

---

## 6. The first measurement, before anything is built

**The seed-to-seed spread of a single 1x circle.** Concretely: living plant
tissue inside one rate-1 circle after 1,500 player ticks, over at least twelve
world seeds, on a held world with one circle and unlimited power — the
`control` arm of the withdrawn build's own bed, repeated.

Why this one, ahead of anything about rates at all:

- **Every number in the record is n = 1.** The 58, the 75, the 127, the 109 are
  single runs. `CLAUDE.md` requires an order statistic over a sweep for
  anything governing procedural content, and records that six seeds is not a
  sweep (1.64x over the first six, 1.08x over the next twelve, pooling to a
  per-seed median of zero).
- **It is the acceptance bar for every option**, including doing nothing. The
  claim any per-circle build has to make is *"the slow circle is unchanged"*,
  and that claim is unevaluable until "unchanged" has a width.
- **It is a positive control on the instrument.** If the spread comes back
  suspiciously tight, suspect the harness before believing it — `CLAUDE.md`'s
  tell is tidiness, and this is a chaotic system where twelve identical trees
  span 31 to 153 cells.

**What it must not be.** Not the fast circle's own cell count: that is the
number the withdrawn build read all the way to the wrong conclusion (143, then
109, both "right"). Not `cells lost` or any single-frame world census: that
rides the water cycle at about ±1,700 cells, larger than most of the
quantities here, and a single-frame reading is that frame's phase plus the
effect with the two not separable. A per-circle living-tissue count at a fixed
*player-tick* number is phase-safe by construction — both arms are at the same
phase — which is exactly why the arms must be compared at equal player ticks
and never at equal world frames.

**The instrument mostly exists and is not on `main`.** The withdrawn commit
added `PIXEL_PHYSICS_DRUID_CIRCLES=x,y,r,rate`, `PIXEL_PHYSICS_DRUID_CENSUS=N`
(living tissue per circle, senescent excluded — *"or a dead wood reads as
thriving"*) and `PIXEL_PHYSICS_DRUID_UNLIMITED=1` to `src/bin/druid.rs`. What
it lacks is a world-seed knob: `Druid::new` takes `PIXEL_PHYSICS_DRUID_SIZE`,
`_GROW` and `_PRESET` but nothing to vary the seed, so **a twelve-seed sweep
cannot be run today at all** — and per `CLAUDE.md`'s harness gotcha, a knob
nobody can see the value of is a knob nobody can tell is disconnected, so the
census line should name its seed. Adding `PIXEL_PHYSICS_DRUID_SEED` is the
smallest useful piece of work in this whole scope. Check
`Reports/instruments.md` before building anything larger; the 26 existing
harnesses do not cover a held world, but `examples/divergence.rs` already
provides an exact-zero determinism control at whole-organism scale and is the
right shape for the "regional time at rate 1 everywhere is bit-identical to
today" null test that must pass before any of this is believed.

---

## 7. Build order, if it goes ahead

Stated so a session can pick this up. Each step is independently landable and
each has something that can go red.

1. **Route the two real leaks** (§1), leaving the six seed sites alone.
   Guard: a test that `growth_slowdown: 4` moves a rebloom's due frame — put
   the raw `world.frame + rebloom_after` back and watch it go red. And a
   *negative* guard beside it, because this is the direction the mistake goes:
   that the same setting does **not** move a seed's.
2. **`PIXEL_PHYSICS_DRUID_SEED`**, and make the census line name its seed.
3. **Take §6's measurement.** Twelve seeds minimum. Publish the spread.
4. **The ageing accumulator** (§4a), behind the owner's ruling on which way
   ageing should go. Guard: the `1/sqrt(m)` table above, as a test over
   `old_age_chance_over` — it is a pure function and the assertion is exact.
5. **`Quickening::rate`**, fastest-wins on overlap, `time_runs_at` →
   `rate_at`. Null test: every circle at rate 1 is bit-identical to today
   (`divergence`-shaped, exact zero).
6. Then **(b)** or **(c)** per the owner's choice. If (c): the fine clock and
   `Druid::ticks` separation first (already half-learned on the withdrawn
   branch), then per-pass liveness, then the chunk sleep gate, then the sweep
   parity fix (§4c) — and measure the whole frame paired and alternating, per
   §8's instruction to take `skip_momentum`'s verdict literally: *removing
   work is not the same as removing cost.*

The acceptance test for 6, in both cases, is the paired two-circle bed with
**the slow arm as the assertion** and the fast arm as the positive control.
Both halves: `slow ≈ control_slow` within §6's spread, *and* `fast >
control_fast`. The withdrawn build passed the second and failed the first by
2.2x.
