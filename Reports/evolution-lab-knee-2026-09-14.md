# What changes at the knee — round 34, lane A

*Round 34's one question, from the brief: an ant costs ~0.3–0.5 µs below ~400
of them and ~2.6–2.9 µs above ~600, in a cost that is otherwise diffuse.
**Something specific changes state at that threshold and nobody has looked.**
This lane was forbidden from shipping a fix; the deliverable is a diagnosis
good enough that round 35 knows what to build.*

*Status: measured. Everything here is `examples/antcost.rs` on a quiet
container at `RAYON_NUM_THREADS=1` with `par=off` — the shipped serial
creature pass — arms round-robin inside one run, figure the minimum over
reps. Branch `claude/evolution-lab-knee`. Background:
[`evolution-lab-round-32-2026-09-13.md`](evolution-lab-round-32-2026-09-13.md),
[`evolution-lab-creature-cost-2026-09-13.md`](evolution-lab-creature-cost-2026-09-13.md).*

---

## The headline, and it is a negative with a redirect under it

**There is no knee.** Once the two things that were moving underneath the ant
axis are held still, one straight line fits the whole range this container can
reach — **0 to 793 ants, on two bed widths, on two seeds.** The marginal cost
of an ant is **3.4–4.3 µs/tick across the 400–600 threshold**, which is the
same as it is at 20 ants and at 120.

**And the cheap sub-knee regime is an artifact this lane can produce on
demand.** Running the same harness on the bed round 32 used — planted, arms
left at their natural ages — reproduces the knee at the fifth try: the
marginal cost is **1.04 µs/ant from 108 to 230 ants and 5.22 µs/ant from 230
to 542**, a 5.0x step, from a harness whose own controlled arms say the ant
term never moved. Both halves are in §3.

**The redirect, which is the part round 35 wants.** Measured directly rather
than by subtraction: **about half of what an ant costs is not in the creature
pass at all.** The creature phase accounts for **45%** of the frame's growth
from 0 to 428 ants; the other 55% is the CA sweep over the **29.4 cells per
ant per frame** that an ant's movement leaves dirty. §4. That is not a
re-derivation of round 33's result — it is the explanation of it: round 33
measured its parallel read phase's hit rate at **0.35**, and 0.35 is this
share, arrived at from the other side.

---

## 1. Two things were moving under the ant axis, and both are larger than the ant term

### 1a. Bed age moves the empty bed 2.15x, with nothing else changed

An empty 512x512 bed, `founders=0`, three separate runs differing only in how
long the bed had been running before the clock started:

| bed age (frames) | µs/tick | `sw seen` |
|---|---|---|
| 1,000 | 429.8 | 3,174 |
| 5,000 | 924.3 | 16,355 |
| 9,000 | 712.2 | 10,392 |

**494 µs of spread on a bed with nothing in it**, tracking the soil-water
pass's own visit count. `CLAUDE.md`'s rule about dividing a designed
oscillator out of every number it reaches, arriving through the harness: the
water cycle is on the frame clock, and a 494 µs difference is the whole ant
term at 120 ants.

**This bites `antcost` specifically, because its stocking loop makes bed age a
function of ant count.** `stock()` alternates `found_colony_of` with `settle`
dispersal frames until it reaches `want`, so a 400-ant arm leaves the loop
thousands of frames older than the `ants=0` arm — measured at frame 4,440
against 1,000. Every arm is then a different *bed* as well as a different
population, and the difference is charged to the ants because they are the
x-axis.

**`age=N` is the fix and it is in the harness now** (`examples/antcost.rs`):
every arm is ticked forward to a common frame before the clock starts. Every
number below this line is age-matched.

### 1b. The `ants=0` arm is not a valid background

Three separate age-matched runs put the empty bed **above** beds with ants in
them:

| run | 0 ants | next arm |
|---|---|---|
| 512, seed 1 | 915.6 µs (`sw seen` 15,553) | 4 ants → 736.9 µs (9,038) |
| 512, seed 1 | 970.8 µs (14,098) | 8 ants → 883.1 µs (8,963) |
| 512, seed 2 | 675.5 µs (8,053) | 6 ants → 900.7 µs (10,022) |

The empty arm's soil-water state is simply a different state — no founding
ever disturbed it — and it sits at a different phase of §1a's oscillator. A
per-ant figure computed as `(cost − cost at zero) / ants` is therefore partly
a measurement of the soil-water pass, and at low ant counts it is *mostly*
that: the first row above gives a **negative** per-ant cost.

**So: do not subtract the empty arm.** Read the chord between two stocked
arms, or fit across the stocked arms only. Both are done below.

---

## 2. The controlled measurement: one line, 0 to 793 ants

All arms `founders=0` (a bare bed, so §3's plant term cannot enter),
age-matched, `frames=300 reps=3`, one process per table.

**512x512, `age=5000`, seed 1:**

| ants | 0 | 8 | 20 | 34 | 58 | 122 | 246 | 349 |
|---|---|---|---|---|---|---|---|---|
| µs/tick | 970.8 | 883.1 | 948.6 | 1030.9 | 1210.3 | 1483.3 | 1905.1 | 2263.1 |

`cost ≈ 925 + 3.94·ants`, residuals within **±78 µs** over the whole range.
Chords between adjacent stocked arms: **5.46, 5.88, 7.47, 4.27, 3.40, 3.48**
µs/ant. Adding awake chunks as a second term tightens the residuals to ±30 µs
and leaves the ant slope at 3.46.

**512x512, seed 2** — the same shape, so the negative is not one seed's luck:

| ants | 0 | 6 | 15 | 28 | 70 | 122 | 238 | 364 |
|---|---|---|---|---|---|---|---|---|
| µs/tick | 675.5 | 900.7 | 901.6 | 1095.8 | 1385.7 | 1687.8 | 2148.8 | 2683.3 |

Chords **0.10, 14.9, 6.90, 5.81, 3.97, 4.24**; end to end over the stocked
arms, **4.98 µs/ant**.

**2048x512, `age=5200`, seed 1 — the arm that crosses the threshold:**

| ants | 0 | 10 | 50 | 189 | 793 |
|---|---|---|---|---|---|
| µs/tick | 1729.1 | 1739.8 | 2186.0 | 2710.8 | 5335.2 |

The **189 → 793** chord runs straight through 400 and 600 and reads **4.34
µs/ant**, against **3.78** for the 50 → 189 chord below it. A knee would put
those at 0.5 and 2.7. They are the same number.

**Re-run after merging `main` (through #401/#403/#405), which added two root
materials.** The populations shift a little — the 300-arm stands 154 ants
instead of 122 — and the finding does not: 0/6/20/34/58/154/246/349 ants at
924.7/699.6/911.8/995.1/1273.5/1624.0/1985.3/2171.6 µs, **4.29 µs/ant end to
end** over the stocked arms against 4.05 before, with the three chords above
58 ants reading **3.65, 3.93, 1.81**. Nothing on `main` since this lane cut
touches `creature.rs`, `scheduler.rs`, `world.rs`, `update.rs` or
`parallel.rs`.

**The noise bar, so the flatness claim is checkable.** `min/med` over three
reps runs 0.89–1.00 and `spread` (worst rep over best) 1.03–1.31, so
differences below ~15% are not resolvable here. The claim being made is that
the chords are flat, and they are flat inside that bar; the claim being
refuted is a 6x step, which is far outside it.

---

## 3. Where the knee comes from: the plants the ants are eating

### 3a. A plant costs the same order as an ant

`plants=` is new on `antcost` and sets herb founders per arm. At **zero
ants**, age-matched, 512x512:

| herb founders | standing plants | µs/tick |
|---|---|---|
| 0 | 0 | 709.5 |
| 8 | 169 | 1795.6 |
| 24 | 596 | 2671.4 |

**6.43 µs per plant over the first 169, 2.05 over the next 427.** An ant is
~4. So roughly one plant cancels one to two ants.

### 3b. In a planted bed, adding ants makes the frame *cheaper*

Same bed, `founders=8`, age-matched at frame 9,000, with the new `pcells`
column — plant *cells*, which is the right ruler, because a bed whose ants
have eaten the canopy has the same number of plants and a fraction of the
tissue:

| ants | 0 | 4 | 36 | 83 | 122 |
|---|---|---|---|---|---|
| plants | 169 | 197 | 98 | 81 | 76 |
| plant cells | 4,147 | 5,302 | 4,272 | 3,736 | 3,508 |
| µs/tick | 1726.4 | 2124.1 | 2554.0 | 2375.4 | 2250.0 |

The marginal cost of an ant here is **−3.8 µs/ant** from 36 to 83 and **−3.2**
from 83 to 122. **Negative.** Nothing is wrong with the clock: the ants are
eating 764 plant cells' worth of work out of the frame while adding 47 ants'
worth to it, and the second is smaller.

### 3c. Which manufactures the knee, and here it is

The uncontrolled planted run — `founders=8`, arms left at their natural ages,
which is the shape a population sweep has if nobody stops it:

| ants | 0 | 6 | 38 | 108 | 230 | 542 |
|---|---|---|---|---|---|---|
| plants | 158 | 169 | 101 | 70 | 50 | 52 |
| µs/tick | 1735.4 | 2052.4 | 2353.1 | 2478.0 | 2604.9 | 4234.2 |

Chords: 52.8, 9.4, 1.79, **1.04**, **5.22**. A **5.0x step between adjacent
chords**, at 230 ants, in a run whose ant term is provably flat — and the
plant column says what happened at the step: the plants bottom out at ~50 and
stop paying the ants' bill.

**This is the positive control on the whole diagnosis, and it is the same
instrument in both directions.** The harness reports a knee when the confound
is present and reports none when it is removed. A harness that could not see a
knee at all would have reported none in both cases, and that is the reading
this rules out.

**What it does not establish.** The owner's played bed carries 264–409 plants
and *thousands* of ants, where the plant term is swamped; and this container
cannot stock past ~900 ants (§6), so nothing here measures his regime
directly. §3 explains how a headless population sweep manufactures a knee. It
is a sufficient cause for round 32's harness-side split at 378 ants; it is
**not** offered as the explanation of the split refitted from his own log.

---

## 4. The redirect: half of an ant is not in the creature pass

Measured directly with `SCHED_PASS`, which times the scheduler's dispatch
per site kind, rather than by subtracting a background. Age-matched,
`founders=0`, 512x512, medians over the timed window:

| ants | frame µs | creature phase ms | µs per creature decision | creature share of frame |
|---|---|---|---|---|
| 0 | 986 | 0.000 | — | 0% |
| 25 | 1,035 | 0.030 | 6.12 | 2.9% |
| 78 | 1,321 | 0.140 | 9.15 | 10.6% |
| 184 | 1,661 | 0.280 | 7.82 | 16.9% |
| 304 | 2,144 | 0.480 | 8.08 | 22.4% |
| 428 | 2,751 | 0.800 | 9.49 | 29.1% |

**The cost of one decision is flat** — 6.1 to 9.5 µs across a 17x range in
population, and most of that spread is between adjacent points rather than a
trend. Whatever the knee was supposed to be, it is not a decision getting more
expensive.

**The frame grows 1,765 µs for 428 ants and the creature phase accounts for
800 of it — 45%.** The other 55% is elsewhere, and `swept` (new column, behind
`swept=1`) names it: the cells inside every awake chunk's expanded dirty rect,
the same quantity `examples/labperf.rs` prints.

| ants | 0 | 10 | 38 | 122 | 304 | 428 |
|---|---|---|---|---|---|---|
| swept cells/frame | 8,443 | 8,821 | 11,567 | 18,114 | 20,296 | 21,011 |
| µs/tick | 955.4 | 954.4 | 1,338.7 | 1,800.2 | 2,201.8 | 2,599.9 |

**29.4 swept cells per ant per frame.** Two terms fit the whole series to
**±60 µs**:

```
cost ≈ 452 µs + 1.89 µs·ants + 0.0615 µs·swept
```

and **1.89 µs/ant is the creature phase measured independently at 1.87** —
two instruments, one from a clock inside the scheduler and one from a
regression, agreeing to 1%. The swept term is 29.4 × 0.0615 = **1.81 µs/ant**.
So an ant is **51% brain, 49% the hole it leaves in the sweep**.

**This contradicts round 32's 86%, and the disagreement is locatable.** That
figure is an honest extrapolation — 3,000 ants × 7.198 µs/ant against a 2,675
µs intercept — but it assumes the *whole* per-ant slope is creature-pass work
and therefore single-threaded. Measured, 45–51% of it is; the rest is the CA
sweep, which already runs on every core. Round 33 then measured the same thing
from the other side and called it a hit rate of 0.35, and got 3–4% of the
frame for a correct parallel creature pass. **The three numbers are one
number.**

---

## 5. Struck off, each by a measurement rather than an argument

- **The active-site scheduler.** `lag/tick` is **0.000** and `lag max` **0**
  in every arm of every run in this report, to 793 ants — no creature ever ran
  later than the frame it asked for. Standing heap depth peaks at **1,400**
  against caps of 2,000 background + 256 creature. Creature ticks per ant per
  frame is flat at **0.195–0.198** from 8 ants to 793: the population is not
  being throttled and it is not being over-served. `MAX_CREATURE_SITES_PER_FRAME`
  is 256 and `ant.ron`'s `tick_interval` puts the bind at ~1,300 simultaneous
  creatures, which this bed never reaches.
- **Density, and this was the brief's own first experiment.** Holding the
  animal count fixed and widening the bed 4x does not reduce the per-ant cost.
  At ~62 ants the marginal cost is **4.93 / 6.80 / 6.63** µs/ant at widths
  512 / 1024 / 2048; at ~380–460 ants, **3.98 / 4.89 / 4.60**. If anything it
  rises with bed size, which is the intercept coming with it (923 / 1,062 /
  1,676 µs at zero ants). **The knee does not track density.**
- **Jamming.** `blk%` — refused steps as a share of attempted ones, and a
  refused step costs *more* than a taken one because `step_chain` prices its
  alternatives first — runs **7–15% with no trend across the threshold**:
  10.4 at 58 ants, 9.4 at 122, 10.5 at 246, 13.1 at 349 on one bed. It is not
  what changes.
- **Contention and parallelism.** Everything here is `RAYON_NUM_THREADS=1`,
  `par=off`, which is what ships (`creature::par_mode_default`). There is no
  thread count to change state at 600 ants.
- **The soil-moisture pass.** `sw seen` is **flat in ant count** — 15,553 at
  0 ants against 15,820 at 428 on the same bed. It is a large, oscillating
  *intercept* (§1a) and not a per-ant term. This confirms round 32's §2 from
  a different bed.

**Not struck off, and not tested:** cache residency, which round 32 left as
its one surviving candidate. This lane did not measure it, because the shape
it predicts — a per-decision cost that steps up when the working set stops
fitting — is the shape §4 shows *absent*: the cost of a decision is flat from
25 to 428 ants. That is evidence against it, not a measurement of it.
`valgrind --cache-sim=yes` is on the container if round 35 wants it directly.

---

## 6. What the harness can and cannot reach, so nobody re-derives it

- **~900 ants is the ceiling**, and widening the bed does not lift it:
  `rounds=250 settle=20` reaches **892** ants on a 2048-wide bed and **916** on
  a 4096-wide one. `found_colony_of` lays one row at `colony_stations`'
  body-derived spacing, so throughput is per *round*, and the loss is
  starvation. **A bed with real standing food in it is what reaches three
  thousand**, exactly as the round-32 report said.
- **`founders=0` is the clean bed** for any cost question about animals, and
  it is a real choice rather than a convenience: with plants in it, §3b
  applies and the ant term is measured against a moving subtraction.
- **Every arm must be age-matched.** `age=N` above the oldest arm's
  post-stocking frame. A run without it is measuring §1a.
- **Do not quote `(cost − cost at zero ants)`.** §1b.
- **`SCHED_PASS`'s `[sched]` line prints `total` for the sum of its own phase
  timings — that is the scheduler phase, not the frame.** At 428 ants it reads
  0.80 ms beside a 2.75 ms frame, and the creature row is ~100% of it, because
  nothing else dispatches many sites in this bed. A share read off that line
  is a share of the wrong denominator.

## 7. What `antcost` gained this round

All in `examples/antcost.rs`; `--help` is its header comment.

| argument or column | what it is for |
|---|---|
| `age=N` | tick every arm to a common frame after stocking — §1a, and nothing else in the harness controls it |
| `widths=`, `heights=` | bed size as an arm axis, so the density discriminator is one process rather than two (§5) |
| `plants=N,N` | herb founders as an arm axis, which is the only way to ask what a plant costs in the same run that asks what an ant costs (§3a) |
| `swept=1` | cells inside every awake chunk's expanded dirty rect, per frame — §4. Off by default because it allocates inside the timed loop |
| `sites/f` | standing depth of both scheduler heaps, the step-change candidate (§5) |
| `lag/tick`, `lag max` | creature scheduling lateness, summed and worst — the starvation readout |
| `plants`, `pcells` | the other population in the bed, by count and by tissue |
| one fit per `(bed, founders, par)` | a fit pooled across beds fits a line through two backgrounds and calls the difference an ant |

---

## 8. What round 35 should build, and what it should not

1. **Do not build anything aimed at a knee.** There is no threshold to be on
   the far side of. An ant costs ~4 µs/tick at every population this container
   can hold, and the work is to make that number smaller everywhere.
2. **The largest untouched item is the sweep's dirty footprint, not the
   brain.** 29.4 swept cells per ant per frame at 0.06 µs each is half an
   ant's cost, and `pending_dirty` is one rect per chunk — so an ant at each
   end of a chunk dirties the whole chunk between them. `labperf`'s `est_rows`
   column already prices the obvious alternative (one x-span per row instead
   of one rect per chunk) against today's rule, on this bed, without anyone
   building it. **That is the cheapest next measurement in the programme and
   it needs no new harness.**
3. **The creature pass is worth ~51% of the ant term and it is already
   correct** — round 33 shipped `ParMode::Off` with the machinery behind it.
   If the sweep's half is taken first, the creature half's share rises and the
   parallel path's arithmetic changes; re-run round 33's numbers after, not
   before.
4. **Anything that measures a population sweep from here on inherits §6.**
   The four rules there cost this lane most of a session to find and each of
   them silently changes the answer.
