# The return leg works one lap, and one lap is longer than one ant's life

*Result, 2026-09-20. `engine`. Executes Step 1 of
[`ant-return-leg-plan-2026-09-20.md`](ant-return-leg-plan-2026-09-20.md);
diagnosis in `Reports/open-bugs-handoff.md` §Z29; handoff in
[`Reports/lanes/homing-return-arm.md`](lanes/homing-return-arm.md).*

**In one line:** `BrainInput::HomeAligned` gives a laden ant a sense that knows
where home is; the share of food-finding ants that complete the return goes
**6.8% → 28.1%**, better in 8 seeds of 8, and colonies where nobody ever
completes a lap go **4 of 8 → 0 of 8**. It ships **on**.

**And the loop still does not repeat.** One ant in 733 completed it twice,
because the homeward leg alone (1,924 ticks median) is longer than an ant's
whole life (1,491 ticks mean). That is the next problem on this line and this
wire does not touch it.

**Two things in it outlive the verdict.** The plan's wiring was unsound in a way
one run of the real evaluator catches and no amount of world-running would have
explained — recorded here and in `dead-ends.md` (`other:133`). And the
`P(move)` separation is a clean mechanism result on ~500k decisions, which
stands whatever is decided about shipping it.

## Contents

- [Reproduced first, bit-identically](#reproduced-first-bit-identically)
- [The plan's circuit does not work](#the-plans-circuit-does-not-work-and-eval_brain-says-so-in-one-run)
- [The mechanism bar: passed](#the-mechanism-bar-passed-decisively)
- [The outcome bar, read on the LOOP](#the-outcome-bar-read-on-the-loop--owners-ruling-2026-09-20)
- [It is NOT a repeating loop](#it-is-not-a-repeating-loop-and-that-is-the-finding-that-matters)
- [Yes, the ant eats its cargo on the way home](#yes-the-ant-eats-its-cargo-on-the-way-home--and-that-is-the-whole-economy)
- [Step 2: `vacated` is not attributable](#step-2-of-the-plan-deposit_atvacated-is-not-attributable-drop-it)
- [What shipped, and what is open](#what-shipped-and-what-is-open)
- [Reproducing the two arms](#reproducing-the-two-arms-which-changed-name-when-the-default-did)

### Reproduced first, bit-identically

`Reports/data/align-census-8seed-2026-09-20.log` came back to four decimals on
every row, on both the `homebias=0.5` and the `DEPOSIT_AT=vacated` arms.
Pointed at home −0.2003 / 5.3%; pointed away −0.2399 / 3.9%. The diagnosis
holds and this tree is the one it was measured on. The `vacated` arm doubles as
a sensitivity control on the census: it widens the home/away gap 0.0396 →
0.0651, so the instrument can register separation.

### The plan's circuit does not work, and `eval_brain` says so in one run

`squash` is `x / (1 + |x|)`, so a shut `(Bias, 7, -45)` unit reads **−0.978,
not 0**. The gated pair is neutral when shut only because it has two units at
−0.978 and subtracts them — **the mirror is the neutraliser, not decoration.**
Contribution to the `Move` sum (`brain.rs`'s ignored `what_the_home_wire_emits`,
`Reports/data/home-wire-response-curve-2026-09-20.log`):

| arm | away | across | home | **empty ant** |
|---|---|---|---|---|
| plan's single gated unit | −2.115 | **+0.833** | +2.167 | **−2.439** |
| the pair (needs two units) | −4.282 | 0.000 | +4.282 | 0.011 |
| **shipped** | −2.143 | 0.000 | +2.143 | **0.000** |

A walking ant's whole `Move` sum is about **+0.25**, so −2.439 is `squash`
clamped to `P(move) = 0`: **a colony that never leaves the nest.** The +0.833
is the subtler defect — a *laden ants move more* lever wearing a homing lever's
name, which would have lifted every alignment bin together.

**What shipped:** the gate moved upstream of `squash` into the sensor
(`creature::sense` reads `HomeAligned` as 0.0 for an empty ant) and the wire is
a direct `(HomeAligned, Move, w)` instinct with **no hidden unit at all**.
Costs the gate as a gene; buys that **unit 7 stays free**, so trap 1's
collision with the fold-change plan does not happen.

### The mechanism bar: passed, decisively

8 seeds, ~500k laden decisions, `homewire=0` against `homewire=3`. **The
control is this same genome with this one slot unread**, so `mutation_rate`,
`live_slots` and every dimension are identical across the pair.

| `P(move)`, laden | pointed **away** | pointed **home** | ratio |
|---|---|---|---|
| control | 0.0782 | 0.1152 | 1.5x |
| gain 3.0 | 0.0282 | **0.5089** | **18x** |
| gain 6.0 | 0.0021 | **0.7990** | 380x |

Pre-registered: *"`P(move)` in the pointed-at-home bin rises from 0.078 toward
the up-gradient figure of 0.59"*. It reads **0.509**. And the two bins move in
**opposite** directions, which is the thing the sensor-side gate was designed
to guarantee and is why this is steering rather than ladenness.

**Read `P(move)`, not `mean along`.** The plan's acceptance line *"the
alignment census must separate"* is loose: `mean along` is the `PheroAAlong`
reading and **this change cannot move it**, because it does not touch the
plane.

### The outcome bar, read on the LOOP — owner's ruling, 2026-09-20

**The axis is the loop and not starvation.** Owner, 2026-09-20: *"I don't care
about starvation. I care about the loop... are most of them following the trail
to the food, picking up food, turning around, going back to the nest, dropping
it off, repeating."* An earlier draft of this report gated the
recommendation on colony survival and intake, and that was the wrong weighting.

**The loop counter is `trips_laden`** — an ant that reached the food, carried
it, and got back to the nest. Not `DELIVERED`, which runs ~100x it here.

| | neither | **wire** | vacated | vacated + wire |
|---|---|---|---|---|
| ants that ever lived | 218 | 162 | 193 | 160 |
| reached the food | 133 | 64 | 106 | 66 |
| **completed the loop (distinct ants)** | **9** | **18** | 11 | 19 |
| **as a share of ants that reached food** | **6.8%** | **28.1%** | 10.4% | 28.8% |
| colonies where **nobody** ever completes it | **4 of 8** | **0 of 8** | 3 of 8 | 1 of 8 |
| completed it **twice** | 0 | **1** | 0 | 0 |

Paired within seed on the completion *rate*, the wire is better in **8 seeds of
8**; per-seed median 2.2% → 23.6%. The gain is not the lever: 1.5 / 3.0 / 6.0
give 26.2% / 29.7% / 31.8%.

**And the leg got shorter, which is the pre-registered branch point.** Median
laden leg **2,671 → 1,924 ticks**, shorter in all four seeds where both arms
completed one. The plan says *"if `P(move)` rises and the leg does not shorten,
the step choice (Step 5) is next"*. `P(move)` rose **and** the leg shortened, so
**Step 5 is not needed** and §R4 stays closed.

### It is NOT a repeating loop, and that is the finding that matters

**Across all four arms and 733 ants, exactly ONE ant ever completed the loop
twice.** `max loops by one ant` is 1 in every seed but one. The repeat step the
loop is named for does not happen.

**The reason is arithmetic rather than behaviour**, and it is the same standing
fact the handoff already carried, now measured on both sides:

| | neither | wire |
|---|---|---|
| mean decision ticks an ant **lives** | 1,721 | 1,491 |
| median ticks for the **homeward half alone** | 2,671 | 1,924 |

**The return leg alone is longer than the average ant's whole life.** A full lap
is well over two lifetimes, so a second lap is unavailable to almost every ant
no matter how well it steers. The ants that do complete one are the long-lived
tail.

So what this wire bought is precisely **one lap becoming reachable** — 6.8% →
28% of food-finders — and not a forage cycle. **A repeating loop needs the lap
to fit inside a life**, which is either a longer life (the colony is starving,
so lifespan is the binding constraint — starvation matters here as a
*mechanism*, not as a value) or a shorter lap (a nearer larder, or faster
travel). That is the next question on this line, and nothing in the current plan
addresses it.

### Yes, the ant eats its cargo on the way home — and that is the whole economy

Asked by the owner, 2026-09-20: *"Don't ants have food in their mouth for the
whole return arm? Do they not digest that?"* They do. `digest_rate_of` is
applied to the crop every tick the animal holds one, and the face value is
credited to that animal's own energy. **The crop is not freight, it is the
forager's packed lunch**, and the return leg is eaten out of it.

That makes delivering and surviving the *same* resource, and the two columns
show the trade directly:

| | neither | wire |
|---|---|---|
| face value the gut **absorbed** | 187,200 J | **83,520 J** |
| cells **put down at the nest** | 209 | **1,944** |
| chews **parked** mid-digestion by a drop | ~224 | **~1,670** |

**The control ant eats its cargo and never arrives. The wire ant arrives and
goes hungry.** The parked-chew counter is the mechanism caught in the act: it
fires when an animal puts its last crop cell down part-way through chewing it,
and it is **7.5x higher** with the wire — ants carrying food home, mid-meal, and
dropping the remainder.

**So the cost recorded above is not a side effect of homing, it is homing.** A
forager that delivers has given away the food it was living on, and nothing at
the nest gives it back (§7.28). This is also why the honest framing of the
lifespan limit is a loop through the economy rather than a bare constant:

> deliver → give up your own supply → shorter life → one lap is already longer
> than a life → no second lap.

**Both arms hit that wall**: mean lifetime is 1,721 (neither) and 1,491 (wire)
decision ticks, against a median homeward leg of 2,671 and 1,924. **In neither
arm does the average ant live long enough to complete the leg it is on.** The
ones that do are the tail.

### Step 2 of the plan: `DEPOSIT_AT=vacated` is not attributable, drop it

Crossed with the wire rather than measured beside it, which is what Step 2 asked
for. **Alone** it moves loop completion 6.8% → 10.4%, paired **3/4/1** — a coin
flip. **On top of the wire** it moves 28.1% → 28.8%, which is nothing. The wire
carries the whole result and `vacated` is not a component of it. It stays behind
its env switch, off.

### What shipped, and what is open

**The wire ships ON at `(HomeAligned, Move, 3.0)`.** It is the loop that this
line exists to fix, the loop is better in 8 seeds of 8, and it takes the number
of colonies where *nobody* ever completes a lap from 4 of 8 to **0 of 8**.

**Recorded, and explicitly not gating that:** larder intake falls 12,699 →
2,280 J (2/6/0) and colonies die sooner, because nothing at the nest banks a
delivered cell (§7.28's granary), so the walk home is energy the colony does not
get back. Owner's ruling: the loop is the axis, not starvation.

**What is open is no longer a ship/don't-ship question. It is this:** one lap is
longer than one ant's life, so the loop cannot repeat. Whichever way that is
attacked — a granary so the trip pays for itself and ants live longer, a nearer
larder, or a faster lap — it is a new piece of work and not a tuning of this
wire. **Do not re-test this by raising the gain**: 1.5 / 3.0 / 6.0 all land
within 6 points of each other on loop completion and none of them makes a
second lap fit.

### Reproducing the two arms, which changed name when the default did

The archived logs were taken against a binary whose `ant.ron` held **3.0**, so
they read `homewire=0` (control) against `homewire=shipped` (test). The file now
holds **0.0**, so the same two arms are **default** (control) against
**`homewire=3`** (test) — and `homewire=0` now trips the rider's own
*"already what ant.ron holds"* assertion, which is the assertion doing its job.

```
cargo build --release --example trailfollow          # set -o pipefail; ant.ron is include_str!'d
RAYON_NUM_THREADS=2 ./target/release/examples/trailfollow \
  mode=gap gate=shipped gaps=90 arms=hand seeds=8 seed0=1 ants=20 \
  frames=24000 relay=60 near=10 food=400 refill=400 stop=6000 trace homewire=3
```

### Data

- `Reports/data/homewire-loop-2x2-8seed-{a_none,b_wire,c_vac,d_vac_wire}-2026-09-20.log`
  — the 2x2 that carries the loop table and Step 2. It is the only one with the
  `LOOPERS ... repeat ... most loops by one ant` columns, which were added to
  `examples/trailfollow.rs` on 2026-09-20 to answer *"is it a repeating loop"*;
  `trips_laden` is a sum over ants and structurally cannot.
- `Reports/data/homewire-{24k,100k}-8seed-{ctrl0,gain3}-2026-09-20.log` — the
  earlier run length pair, taken before the wire's default changed, so their
  arms read `homewire=0` / `homewire=shipped`.
- `Reports/data/home-wire-response-curve-2026-09-20.log` — the four wiring
  forms through `eval_brain`.

### Also worth knowing

- **Only `ant.ron` is wired.** `longant.ron` and `ancestor.ron` carry the same
  homing pair on units 0/1 and the same defect; left alone until this is proven.
- **The cost was paid**: `BRAIN_INPUTS` 32 → 33, `live_slots` 918 → 942, every
  species' `mutation_rate` re-derived to `3.18 / 942 = 0.0033758`. Every
  breeding scene's numbers move from birth 1 — that is not a regression and
  cannot be checked by a diff.
- **Do not re-test by retuning the gain.** 1.5, 3.0 and 6.0 all give the same
  qualitative picture and the trend in colony survival is monotone against it.
- **Grading the sensor by crop fill will not help either**, and it is worth
  saying so before someone spends a cycle on it: this bed's larder is 960 J
  fruit against a 1,440 J crop, so a laden ant is at 0.667 fill and **cannot
  hold two cells**. Grading by fill is a 33% gain reduction wearing a
  distribution's clothes, and gain 1.5 already measured that region.
