# The return leg steers, and on this bed it starves the colony

*Result, 2026-09-20. `engine`. Executes Step 1 of
[`ant-return-leg-plan-2026-09-20.md`](ant-return-leg-plan-2026-09-20.md);
diagnosis in `Reports/open-bugs-handoff.md` §Z29; handoff in
[`Reports/lanes/homing-return-arm.md`](lanes/homing-return-arm.md).*

**In one line:** `BrainInput::HomeAligned` gives a laden ant a sense that knows
where home is, it steers exactly as pre-registered, and because nothing at the
nest banks what arrives it costs the colony its larder — so it ships authored
at **0.0**, as `CreatureDef::home_bias` does, and turning it on is the owner's.

**Two things in it outlive the verdict.** The plan's wiring was unsound in a way
one run of the real evaluator catches and no amount of world-running would have
explained — recorded here and in `dead-ends.md` (`other:133`). And the
`P(move)` separation is a clean mechanism result on ~500k decisions, which
stands whatever is decided about shipping it.

## Contents

- [Reproduced first, bit-identically](#reproduced-first-bit-identically)
- [The plan's circuit does not work](#the-plans-circuit-does-not-work-and-eval_brain-says-so-in-one-run)
- [The mechanism bar: passed](#the-mechanism-bar-passed-decisively)
- [The outcome bar: the leg improves and the colony dies](#the-outcome-bar-the-leg-improves-and-the-colony-dies)
- [Why, and it is §Z29's own prediction](#why-and-it-is-z29s-own-prediction-rather-than-a-new-finding)
- [What is open, and it is the owner's](#what-is-open-and-it-is-the-owners)
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

### The outcome bar: the leg improves and the colony dies

Paired within seed, at **both** 24,000 and 100,000 frames (Step 3's length):

| | 24k ctrl → gain 3 | sign | 100k ctrl → gain 3 | sign |
|---|---|---|---|---|
| `carry->nest` (cells home) | 152 → **329** | **7/1/0** | 159 → **329** | **7/1/0** |
| `CAME BACK` (round trips) | 1 → **3** | **6/2/0** | 1.5 → **3** | **6/2/0** |
| `ate J` (larder intake) | 12,699 → **2,280** | 2/6/0 | 14,730 → **2,280** | 2/6/0 |
| `starved` | 15 → 18.5 | 7/1/0 | 15.5 → 19 | 6/1/1 |
| `born` | 6 → **0** | 1/6/1 | 7 → **0** | 1/6/1 |
| colonies alive at end | 4 of 8 → **3 of 8** | | **0 of 8 → 0 of 8** | |

Colonies alive falls monotonically in the gain at 24k: **4 / 3 / 3 / 1** at
gains 0 / 1.5 / 3.0 / 6.0. **At 100,000 frames every colony in every arm is
dead**, so that column measures nothing at that length — the log prints the
warning itself. The longer run still *strengthens* the intake finding rather
than softening it: a healthy control colony compounds, and control seed 7 banks
**178,321 J** against the gain-3 arm's best seed at 7,356.

### Why, and it is §Z29's own prediction rather than a new finding

**`DELIVERED` is 0 in both arms.** Nothing this bed carries home is ever banked
— §Z29's own words, *"a delivered cell is left on the ground and nothing banks
it"*. So walking home is pure energy cost on a bed already near subsistence,
and **a better return leg buys nothing until the granary (§7.28) lands.** §Z29
predicted this shape for its own repair (*"intake falls 6,151,294 → 270,810 J
and births 5,173 → 137 while deliveries double"*) and it arrived unchanged.
`home_bias` produced the milder version of it (`dead-ends.md`: *"the return leg
navigates and does not provision"*).

**This is not a reason to disbelieve the mechanism.** The steering is measured
on 500k decisions and is not a small-n question. The colony numbers are 8 seeds
on one bed, which `CLAUDE.md` is explicit is not a sweep — but they point one
way at every gain and at both run lengths, and the causal story is already on
the record rather than inferred from them.

### What is open, and it is the owner's

**Turn the wire on, or leave it at 0.0 until the granary lands?** It is
`(HomeAligned, Move, 0.0)` in `ant.ron` and `homewire=` in `trailfollow`. The
case for on: the return leg is the thing this line exists to fix and it now
works. The case for off: on the only bed we have it costs six seeds of eight
their larder intake and takes births to zero. **Parked at 0.0 rather than
settled**, on the precedent that `home_bias` is parked for the same reason.

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
