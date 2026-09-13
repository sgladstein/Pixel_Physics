# Rest with an end — round 33, lane F

*The owner marked three long ants as showing no movement; round 32 answered
that they were resting. He overruled that: **"How long do they go without
asking to move. If they never ask to move that is still stuck, it is just
because the rest mechanism needs fixing."** (2026-09-13.) This is what was
actually wrong, what it was replaced with, and what it cost.*

Register entry: [`../open-bugs-handoff.md`](../open-bugs-handoff.md) §Z13
(re-aimed) and §Z20 (a harness bug found on the way). Dead end recorded:
`../dead-ends.md`, the linear ramp.

## 1. What the defect is, in one paragraph

An animal's urge to walk is a weighted sum of reasons, squashed into `(-1, 1)`
and then **clamped to `[0, 1]`**. `brain::squash` returns a genuinely negative
number for a negative sum, so every degree of "would rather not" — the ant that
is mildly disinclined and the one that is emphatically so — lands on the **same
exact zero**, and `draw.unit_f32() < 0.0` is never true. An animal there is not
*unlikely* to move; it **cannot**, and nothing it does itself changes that.
Only the world moving one of its other inputs does. Walking or frozen, with
nothing in between — `CLAUDE.md`'s first law, failing.

## 2. Measuring it rather than naming it

§Z13 asked twice for `outputs[BrainOutput::Move]` printed beside the probe and
twice left it for whoever owns `src/sim/creature.rs`. Built as
`CreatureStats::p_move_hist` — one array index per creature decision tick,
bucket 0 being the exact zero. 120,000-frame beds, `RAYON_NUM_THREADS=1`:

| share of decision ticks at `p_move` **exactly 0.0** | |
|---|---|
| long ant, `played_bed_longant` seeds 1/2/3/7 | 48.7 / 64.0 / 54.6 / 62.5 % |
| shipped two-cell ant, `played_bed` seeds 1/2/3 | 77.5 / 66.3 / 54.4 % |

**The shipped ant is worse than the long one.** This was never a long-ant
defect; it is every animal in both games, and it went unreported for as long as
it did because two motionless pixels read as scenery while a seven-cell body
reads as broken.

**A note on the baseline, because the numbers here do not match §Z13's.** Its
table (idle 74–76%, streaks 33–83 stops) was taken on the tree of 2026-09-12.
Re-run on `main` at 2026-09-13 the same census reads **idle 26.9–49%, streaks
13–47 stops**. Rounds 30–32 moved the bed; the defect is unchanged and the
magnitudes are not, so every figure below is re-measured here rather than
inherited.

## 3. The fix

`brain::BrainInput::Stillness` — the one input read off the animal rather than
off the world. `OrganismState::still_ticks` counts decision ticks since the
body last moved (reset by a step or a launch, and by a flight frame);
`Stillness` is the **square** of `still_ticks / creature::STILL_SATURATION`
(192 ticks = 1,152 frames for an ant), wired `(Stillness, Move, 1.5)` in all
eleven shipped creature species. A rest therefore **ends**: the animal shifts,
the odometer resets, it settles again.

**This does not contradict the owner's 2026-09-09 ruling, it supplies it.**
Rest is still the absence of a reason to act. Having stood in one spot long
enough is now one of the reasons.

**Why a brain input rather than a floor in Rust.** The weight is the genome's,
so a lineage can breed itself more or less restless and one that mutates it to
zero gets the old pathology back and pays for it in the same currency as
anything else. It also makes the lever sweepable without a rebuild:
`labforage wire=Stillness:Move:<w>` overwrites it in the live genome, and
`w=0` is today's behaviour **as an arm of the same binary** — which is the only
honest control here, because a new input column changes `live_slots()` and
therefore every `random_genome` draw, so `origin/main` and this branch are
different animals and not comparable seed for seed.

**Why the ramp is squared.** `creature_tick`'s own comment: *"a laden ant
walking away from the nest scent computes a low `Move`, fails the roll, and
re-orients. That is the whole of the homing mechanism."* A low `Move` is two
states — re-orienting (tens of ticks) and resting (thousands of frames) — and a
**linear** ramp cannot tell them apart. Built that way first: `deliveries`
**4,908 → 54** on seed 1, the colony surviving and simply not provisioning.
Squaring puts the term under the noise a tumbling animal reads (+0.016 at 20
ticks, +0.10 at 50) and over the worst standing sum where it is needed (+0.41
at 100, +0.92 at 150, +1.5 at 192). Full account in `../dead-ends.md`.

## 4. What it moves — nine seeds, control is the same binary at weight 0

120,000 frames, `sample=900`, `RAYON_NUM_THREADS=1`, `labforage`.

| longest idle-with-room streak, stops of 900 frames | s1 | s2 | s3 | s4 | s5 | s6 | s7 | s8 | s9 |
|---|---|---|---|---|---|---|---|---|---|
| long bodies 3+ cells, control | 27 | 32 | 28 | 26 | 47 | 30 | 32 | 33 | 13 |
| ...with the fix | **3** | **2** | **2** | **1** | **4** | **1** | **2** | **1** | **1** |
| all bodies, control | 27 | 32 | 43 | 34 | 66 | 31 | 46 | 52 | 23 |
| ...with the fix | **4** | **6** | **4** | **4** | **4** | **6** | **6** | **4** | **3** |

11,700–59,400 frames → 900–5,400. Shipped two-cell ant, `played_bed`, all
bodies: **45 / 30 / 41 → 6 / 7 / 6**.

**The positive control `CLAUDE.md` demands**: the census reports the long
streaks on the unmodified tree before it is trusted to report their absence —
the control row above *is* that reading, taken with the same binary at weight
0, and it reproduces the unmodified `main` binary's streaks to within the
genome re-roll.

### The distribution keeps its middle, which is the claim that matters

`CreatureStats::rest_bout_hist` — power-of-two buckets of an animal's **own**
ticks, counted when a rest ends. `idle_with_room` cannot answer this: it
samples head position every 900 frames, so it cannot see a rest shorter than a
stop and it scores a walk-away-and-return as standing still.

| | control | with the fix |
|---|---|---|
| median rest | 2–4 ticks | 2–4 ticks — **unchanged** |
| p90 rest | 8–16 ticks | 8–32 ticks |
| rests longer than 1,024 ticks (6,100 frames) | 18–422 | **0–14** |
| longest animal still standing when the run stopped | 873–10,039 ticks | 29–2,226 |

The last row is the other half of the histogram and it has to be read with it:
a bout is counted when it *ends*, so an animal that stops and never starts
again contributes nothing — and that animal is the whole complaint.

**The short pause is untouched and only the tail is cut.** An animal still
declines to step on **83–94%** of its decisions (step rate 3.9–16.0% →
5.6–16.8%), so this is not "the ants now walk all the time". The
`idle_with_room` *rate* collapsing (27–49% → 3–6%) says nothing about that: at
900-frame sampling an animal that rests 600 frames and then moves reads as
moving, which is `CLAUDE.md`'s *ask what your number counts*.

## 5. What it costs

- **Foraging.** Deliveries per 1,000 ant decision ticks fall on **7 of 9**
  seeds, median **2.68 → 1.63**; steps per delivery roughly double (median
  32 → 76), which is the homing term being partly overridden and is the
  mechanism above, not a mystery. **But the two seeds where the control's
  foraging had collapsed outright recover** (0.02 → 0.71, 0.20 → 3.15): the
  arm is worse on a healthy colony and better on a failing one.
- **A gentler weight does not buy it back.** `wire=Stillness:Move:1.0` over the
  same nine seeds reads a *lower* deliveries median (1.48) while letting the
  tail return (`maxL` 4–12 stops). 1.5 is shipped because it is the only arm
  measured where bouts over 1,024 ticks are near zero on every seed, not
  because it is a fitted optimum.
- **Colony size is not moved.** `alive` is up on 4 seeds and down on 5. The
  medians (152 → 99) are the chaos §Z14 documents; do not read them as a
  finding.
- **Starvations fall on 7 of 9** seeds, median 125 → 94.
- **Frame time: no measurable cost.** `antcost ants=0,900 width=1024
  rounds=600`, four runs alternating main/arm/main/arm, minimum-over-reps:
  main **12.14** and **12.31** µs/ant at 658/650 standing; arm **11.22** and
  **12.68** at 610/634. The arm's own two runs bracket main's, so the
  difference is not separable from the box. Whole-frame **~8.0 ms/tick at ~650
  ants on both**. Round 33's parallel-creature lane is not being spent.
- **The mutable genome surface** — the constant this change is obliged to
  re-derive, and the reason it cannot be landed without touching every species
  file: `live_slots` **846 → 870**, so every `mutation_rate` is re-derived to
  `3.18 / 870 = 0.0036552`, and every breeding scene's numbers move from
  birth 1.

## 6. What broke on the way, and why each was the scene rather than the change

- **`a_lone_grazer_cannot_farm_a_moss_lawn_forever`.** Its "unlimited larder"
  was a 22-cell island of litter with an open corridor through it, and the
  arm's entire yield rested on the grazer being sessile *by accident of the
  clamp*. Give rest an end and the ant walks out into bare soil: `larder_intake`
  **4,201 J → 228 J**, below the moss lawn's 456 and failing the test's own
  ordering. Repaired by spanning the litter across the whole bank (the moss arm
  already spanned it, so the two larders were never the same size), and the
  repair was checked the way `CLAUDE.md` requires — moss's `food_energy` raised
  to 900 puts the test red again, so the guard still fires for the fault it is
  named for.
- **`labgif wire=` was a silent no-op** for every card it has ever produced.
  Filed as §Z20 and fixed here. Its own `assert!(moved > 0, "…the control
  wearing a label")` could not catch it: the override was written to a world
  `load_scenario` then rebuilt, so the assertion checked the genome it was
  about to discard, and the log printed `wire= set 1 of 1` either way. It
  surfaced only as **byte-identical PNGs at every sampled frame** across two
  arms that had to differ.

## 7. On the owner's queue

Card `20260913T172957391Z-dcab18`, board `lab`, **blind**, two 81-frame
sequences of `played_bed_longant` seed 3 at his own original crop and zoom
(`crop=160,120,224,56`, `up=4`), 4,800 ticks from frame 28,000, `rain=off`,
control against the fix in one binary. `meta` carries the rest-bout counts for
the window shown: control **15 rests longer than 1,024 ticks** and a stillest
animal that has held one cell for **2,796 ticks (16,776 frames — 3.5x the whole
window)**; the fix **0** and **147 ticks**.

**Shipped as default** per the round's standing instruction whatever the
verdict. If he wants it milder the lever is one weight per species file and
`wire=` sweeps it without a rebuild; if he wants it stronger, the same.

## 8. Left for whoever picks this up

- **The delivery cost is real and is not understood to the bottom.** The
  mechanism (a time-based term overriding run-and-tumble's steering) is
  established and the squared ramp already reduces it by an order of magnitude
  against the linear one, but the residual 40% has not been isolated from
  colony-composition drift. The clean way to separate them is a term that
  distinguishes *navigating* from *resting* as data rather than by timescale —
  `CLAUDE.md`'s *when a rule must tell apart two things that can look
  identical, state the difference as data* — and nothing in the brain currently
  says which an animal is doing.
- **`STILL_SATURATION` is a constant, not a species field.** Every creature
  therefore shares one timescale while the *weight* is per-species. If a
  species ever needs a genuinely different rest length rather than a different
  restlessness, that is the change; it was not made here because no measurement
  asked for it.
