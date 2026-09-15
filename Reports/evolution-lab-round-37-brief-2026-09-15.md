# Round 37 brief — the last performance attempt, and the nest becomes a place

*Written by round 36's coordinator, 2026-09-15. Read
[`evolution-lab-round-36-2026-09-14.md`](evolution-lab-round-36-2026-09-14.md)
for what just landed and
[`lanes/evolution-lab-coordinator.md`](lanes/evolution-lab-coordinator.md) for
what binds. **Do not read the archive, `README.md` or `PLAN.md` whole.***

---

## Read this before anything else: two numbers you may have inherited are wrong

Both were retracted by the lanes that produced them, **after** they had been
quoted onward. If a document you are reading states either, that document is
stale.

- **"29.4 cells per ant per frame" is a PER-BED number, not an ant's.** It
  swings **1.75x across three seeds of one bed**, and the same seed reads
  **21.7 at `grow=6000` against 13.3 at `grow=2000`**. The round-36 brief led
  with it as an ant's cost. It is not one.
- **The plant fix did not raise plants standing on every seed.** Re-measured:
  **243→275, 161→97, 190→187**, median **−3**. The fix removes the jaw's pure
  loss completely; **what happens to the forest next is set by what the colony
  does with the freed energy.**

## Lane 1 — §E2, the narrow mark shape, and the END of the performance line

**This is the lead, and it ships with a kill condition the owner set.**

> **Three rounds have produced no shipped frame-rate improvement.** Round 33
> measured parallelising the creature pass at **3–4%** and shipped it off;
> round 34 deleted the knee; round 36 measured the single-rect reach narrowing
> at **1.9%** and deleted the 29.4 figure the line was aimed at.

**The owner's ruling, 2026-09-15: one more round, scoped hard, with a stated
kill condition — if the whole-frame paired figure comes in under about 5%, the
performance line CLOSES and we stop spending on it.** Do not widen this lane.
Do not open a fourth exploratory performance round.

**What is left, and why it is worth one attempt.** The saving is a **shape**
prize, not a reach prize: at reach 24 on a 64-wide chunk the dirty rect is
**already clipped to full width**, so no reach narrowing pays while the shape
stays one rect (measured: 19,261 → 18,904 cells/frame, 1.9%). The narrower
shape is worth 4.8–5.7x **of the swept region** — and **that has never been
priced at whole-frame**, which is exactly the number this lane owes.

**§E2 is the blocker and it is now small.** `examples/sweepgap.rs` steps both
arms in one process (the switch was a process-wide `OnceLock`, which is why it
had only ever been bisected to a frame): **first divergence at frame 237, four
cells, all `soil`, same material and organism id, differing only in `aux` — the
soil-moisture channel.** Thirty seconds to reproduce. **Located, not
diagnosed**: the moisture pass walks its region over *every* chunk rather than
the awake set, and its seed is identical in both arms, so the obvious route is
already ruled out.

**The two traps, both already paid for here.** *Removing work is not removing
cost* — a gate that removed 91% of the field's momentum work was bit-identical
and made the frame **slower in 7 of 8 paired runs**, because the arithmetic
went and the memory traffic only moved. And *a cost that vanishes may be work
that vanished* — "no loss of conservatism" is the load-bearing claim and no
timing can check it, so **write the guard, put the fault back, watch it go
red**. Determinism is required; `cargo test --lib` cannot reach
`tests/determinism.rs` at all.

## Lane 1b — three checks #450 owes, and they are the owner's own condition

**PR #450 widened the scent planes `u8 → u16` and was merged as-is on the
owner's call, 2026-09-15 — with these three checks moved here rather than
waived.** His condition was *"more memory is fine as long as speed is
unchanged"*, so the speed evidence is load-bearing and it is not yet at the bar
this repo sets.

1. **The timing is cross-run, not paired.** `pherocost` pins
   `RAYON_NUM_THREADS` and echoes it — correct — but the two arms are **two
   binaries in two separate runs**, and this container has produced a **2.42x
   swing on byte-identical deterministic work**. The reported 0.85x / 0.92x /
   0.92x are *better* than neutral, which is the direction motivated reasoning
   runs. **Re-take it paired and alternating with the order swapped each
   round**, or compare both arms inside one process.
2. **There is no whole-frame figure.** `pherocost` is a subsystem harness, and
   the standing rule is that those overstate — the same field change once read
   **−50% in a subsystem harness and −27% at whole-frame**. `examples/ascii`
   already reports worst-frame timing and CI runs it: **quote it for both
   arms.**
3. **The memory cost is never quantified.** Three world-sized planes at the
   shipped **8192x2560** is about **63 MB** more resident. He said more memory
   is fine — he was not told how much, and `pheromone.rs`'s own doc says
   world-sized planes are already **the wrong shape for M10 streaming**. State
   the number, and say what it becomes under streaming.

**None of this touches the mechanism finding, which stands**: channel A is a
**ramp**, so the far end of a trail is the faint end, and at a byte it read
**exactly zero past its own midpoint** — the trail was still there and had
stopped pointing. `DEPOSIT` could not buy it, because busy trails already peak
at 39–98 of 255 and raising it clips the loud end into saturation. **The
scale-free reader is scale-free in *ratio*, not in *resolution*.**

## Lane 2 — the nest becomes a site

**The research landed (#446,
[`nest-design-2026-09-14.md`](nest-design-2026-09-14.md)) and the owner asked
to be argued with rather than implemented. It argued.** Build what it
recommends:

- **A site, not a material — his instinct, confirmed.** `NestSite` already
  holds position and odour and has four readers; the gap is **one function**:
  `adjacent_nest` asking *"am I within the site's reach"* rather than *"is a
  nest cell 8-adjacent"*.
- **NOT a blob — refused with a measurement.** A footprint wider than the ant
  band **kills the colony monotonically on 3 of 3 seeds** (alive at 40k: 59 →
  38 → 19 → 2 over widths 36/72/144/288), and the case a blob defends does not
  arise — the patch loses **0/0/4 cells over 120,000 frames**.
- **The crust retires with it** (`nest.ron` `penetration_resistance` 6.0 →
  0.8). At 6.0 against every shipped `dig_force` of 1.0, **a colony cannot
  excavate its own doorstep** — the owner noticed this himself.

**Three held-world requirements, non-negotiable**: founding is a player verb at
an arbitrary cursor (`Druid::found_colony`); the shrunk gnome walks colony
galleries and `a_nest_still_stops_him` asserts today's behaviour, while
`rigid::is_tool_target` is true for any non-bedrock `Solid` — **so if nest
stops being `Solid`, both change**; and **tell the druid coordinator before it
lands**, not after.

**Do not quote "414 deliveries" as a constraint.** That scene places **15 of 55
ants and has no channel A by frame 6,000**, so it cannot carry a homing or
footprint measurement. Round 36's coordinator handed it over as a bar and was
wrong.

## Lane 3 — nothing steers a laden ant home, AND THAT NULL IS NOW SUSPECT

**READ THIS BEFORE RE-USING ANY NEST-RESEARCH NUMBER.** #446 merged
2026-09-14 19:18; #450 was opened 01:56 the next morning. **Every measurement
in the nest research was taken on the `u8` engine, where the far half of every
trail read exactly zero.** The widening moved the trail an order of magnitude:

| | `u8` (what the nest research measured on) | `u16` |
|---|---|---|
| unreinforced trail gone | 144 frames | **1,476** |
| stops steering | 36 frames | **1,080** |
| standing network, 52 ants | 208–342 cells | **1,405–2,057** |

**So the central null — *cutting the homing circuit out of the genome changes
deliveries not at all* — may be an artifact of the byte rather than a fact
about homing.** If the trail read zero past its own midpoint, the homing
circuit had nothing to read, and removing something that was already receiving
nothing would of course change nothing. That is this repo's *a mechanism
appears inert because the scene does not contain the situation* in a new
costume. **Re-run the genome-ablation null on `u16` before building or
abandoning anything on it.** Its channel-A standing figures (3,421 at frame
10,000, 0–9 from 20,000) are likewise pre-widening.

**And do not apply `DIFFUSE` and the widening together without re-measuring.**
`DIFFUSE` 0.25 → 0.02 was the lever found *against the byte*, lifting
laden-at-door 1.6% → 14.5%. #450 fixes a different half of the same problem —
**resolution, not spread** — without touching `DIFFUSE`. Stacking both may
over-correct, and `DIFFUSE` is explicitly not free: a full mean filter flattens
a shared trail's peak **153 → 63**, and that height *is* the path-selection
algorithm. **Measure whether `DIFFUSE` is still needed at all now.**

**What the widening does NOT touch, so these stand**: the nest-as-a-site
finding, the blob refusal (a footprint sweep and colony deaths, nothing to do
with trail resolution), and the crust-vs-`dig_force` finding. **Lane 2 is
unaffected and can proceed.**


**The owner's "nests never seemed to function" is right, and the measurement
picked his own second reading — the trail, not the nest** — but **on the byte**,
per the table above. As measured there: channel A stood at 3,421 at frame
10,000 and 0–9 from 20,000 on; cutting the homing circuit out of the genome
left deliveries **inside the noise** between two mechanically identical
no-homing arms; laden ants were **0–10% at the food**. **Treat all four as
pre-widening.**

`DIFFUSE` on channel A alone, 0.25 → 0.02, lifts laden-at-door on 3 of 3 seeds
(**1.6% → 14.5%** where the round trip fails) — and round 36 established
`DECAY_RHO` is **inert**, so `DIFFUSE` is the lever. **`DIFFUSE` is not free**:
a full mean filter flattens a shared trail's peak from 153 to 63, and the
*height* of a well-used trail against a lightly-used one **is** the
path-selection algorithm.

**The candidate mechanism from the research is path integration** — a per-ant
home vector accumulated from movement, attached to no cell, so digging cannot
break it, corrected at short range by the nest's own odour (which is what
`AtNest` already is). A `HomeBearing` brain input does not depend on the trail.
**Lane 2 and Lane 3 can run concurrently**; the trail gates homing only because
homing was built on the trail.

## Lane 4 — the birth bar, with seed 2 as the case

**Round 36 handed this a tuning case rather than a question.** The plant fix
removes the jaw's pure loss; where the colony stays its size the stand recovers
to the unhunted control (**seed 1: 275 against a no-ant 270**), and where it
explodes **grazing replaces the jaw** (**seed 2: ants 127 → 224, stand 161 →
97**). So the birth bar and the starvation balance — calibrated against a
colony paying a bill that has gone — are now measurable against a named case.

**Gate on an order statistic over seeds and keep an `off` arm. Six seeds is not
a sweep.** And `rivalry control=selftest` **fails on `main` today**,
byte-identically with and without the plant rule — its `shipped` arm asserts
about a default #423 replaced. That is this lane's to repair.

## Standing, and still unruled

- **THE ALARM SEMANTICS ARE THE OWNER'S AND HE HAS NOT RULED.** Round 36
  shipped *"a living animal bitten, whichever verb did it"* and it is **inert
  on his bed** (`alarm_eat_animal` is 0 on every run — a lone colony has no
  stranger to eat). **Do not treat that default as his decision.**
- **Read the open review queue before posting a card**, and the queue is for
  **visual evaluations only**.
- **There is no delivery signal for a poke — check the branch head.** Round 36
  cost itself a duplicated implementation because a poke crossed a landing by
  eleven minutes and the branch head was not read before writing it.
- `cargo run --release --example ascii` is a CI gate and is the one a lane's
  local list forgets. `cargo test --release` whole exceeds the 600 s Bash cap.
  `cargo build --release` does **not** rebuild examples.
- **`bugindex.py --branches` before filing a bug, never `--check`.** A new
  `dead-ends.md` entry needs a `screened.tsv` verdict and a
  `deadendindex.py` regeneration, or `docscheck` goes red and nothing else
  notices.
