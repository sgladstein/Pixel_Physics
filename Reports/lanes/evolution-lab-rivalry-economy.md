# Lane D — the economy a live rivalry now sits on

*Round 36, 2026-09-14. Branch `claude/evolution-lab-rivalry-economy`.
Brief: [`../evolution-lab-round-36-brief-2026-09-14.md`](../evolution-lab-round-36-brief-2026-09-14.md) §"Lane D".*

**The headline: the three constants #423 named do not need re-deriving, and
the reason is arithmetic rather than tuning — a colony cannot eat enough
rivals to matter. What does need attention is what the measurement was made
of.** Round 35's economy numbers were taken over a population that is
seven-eighths plants.

Everything below is **predation** — a stranger eaten by the ordinary mouth —
unless it says `Attack`. Nothing here is about a war.

---

## 1. What shipped

**§Z23 closed** (`src/sim/creature.rs`). `cry_alarm`'s two feeding call sites
are gated on the victim being an animal. The target rule is untouched, so
#417's argument stands: an animal cornered by something it cannot digest can
still hit it. One predicate, `is_animal_cell`, with three readers — the fight
site's assessment gate (which had an inlined copy) and the two feeding sites
(which had none).

Reproduced the register's own run byte-for-byte first (seed 1: 475 attacks,
79 cells, 629 deaths), then:

| | §Z23 open | §Z23 fixed |
|---|---|---|
| attacks, seed 1, `spread=0` | 475 | **0** |
| over 12 seeds at the shipped dial | — | every swing animal-directed: `xcol == killedA` on **all twelve rows** |

**`examples/rivalry` now reads the economy animal-only**, and `colony_ants=`
is a knob. New `SUMMARY` columns, appended after `main`'s:
`deathsA starvedA killedA starvA% raided_j raided_by_others_j`, plus a
`SUMMARY-causes-animal` line and an `economy` echo carrying the birth bar,
the grant, and what a rival is worth against it.

**§Z26 filed** — closing §Z23 doubled the moss lawn's yield and the moss pump
is now live. See §5.

---

## 2. The instrument finding, which reframes round 35's numbers

**`World::deaths_by_cause` is every organism in the bed, and a played lab box
is mostly plants.** Seed 1, off arm: **687 deaths, of which 95 are animals** —
the other 592 are vegetation, 390 of them `felled`. So `starv%`, which round
35 priced the switch on, has a denominator seven-eighths plants, and anything
that fells more plants lowers it without one ant starving less.

`World::group_deaths` is the animal-only tally and already existed — the
engine books a row there only `if creature`. The harness was reading the wrong
one of two censuses it already had.

**Re-measured, 12 seeds, paired, both arms post-§Z23-fix:**

| | whole-bed (what round 35 read) | animal-only |
|---|---|---|
| deaths, median delta | −3.0, up 5/12 | +0.5, up 6/12 |
| starvation share, median delta | **−4.4 pts**, down 8/12 | **−9.6 pts**, down 11/12, up on **none** |

The whole-bed figure reproduces round 35's −4.3 almost exactly, which is the
cross-check that says the two runs agree. **The animal-only figure is more
than twice as large**, so the original number understated the effect rather
than inventing one.

**And the mechanism round 35 proposed is confirmed, at the population where it
is a claim about ants:**

| | median delta | up | down |
|---|---|---|---|
| `starvedA` | **−8** | 1/12 | 10/12 |
| `killedA` | **+8** | 11/12 | **0/12** |
| `deathsA` | +0.5 | 6/12 | 5/12 |

**Displacement, one for one.** The same number of ants die; a rival's jaw gets
the ones the empty bank would have got. So the answer to the brief's second
question — *does that make the starvation balance right, or reveal it was
never doing the work?* — is that the balance is not being bypassed. Predation
substitutes for starvation at parity and adds no deaths.

**Seed 9 is the null and it is also the instrument's specificity control:**
every new column reads a delta of exactly 0, on the one bed round 35
identified as inseparable at any setting. The columns move on the eleven where
the mechanism fires and are silent on the one where it cannot.

---

## 3. The birth bar — no, and the reason is arithmetic

*Is a colony that can eat its neighbours meant to need the same birth bar?*

**No change needed, because predation income is under one child per bed per
run.** The arithmetic, now echoed by the harness on every line:

```
birth bar 1,100 J | start_energy 200 J | gut_bias 0.00
one cell of rival flesh yields 120 J  (ant food_energy 480 x diet_quality 0.25)
a whole 2-cell rival is 240 J = 22% of one child
a birth costs 4.6 rivals eaten whole
```

Measured, `ColonyBooks::raided` over 12 seeds at the shipped dial — the
engine's own books, not a counter I derived beside them:

| | median | range |
|---|---|---|
| `raided_j` (face) | 2,880 | 0 – 6,240 |
| digested, at the gut's 0.25 | **720 J** | 0 – 1,560 |
| **as children** | **0.65** | 0 – 1.42 |
| as a share of the colony's plant income | **~6%** | 2 – 10% |

**Why it is so small: the meat does not go to the killer.** Median `killedA` 8
against a median 6 cells of living flesh actually swallowed — most kills take
the deciding cell and leave the rest of the body to rot. `corpse_j` does not
rise to meet it either (median delta **−66**, up on only 5/12), so the
carcasses are not being scavenged back. **Predation destroys rivals far more
than it eats them.**

So the bar was never set against this income because this income is not there.
`reproduce_threshold` is already a live dial (`lab/params.rs`, span 0–4,000,
step 25), so the owner can move it in the game if he disagrees.

**One number did move where round 35 said it did not: births, median +1.5, up
8/12, down 2/12.** Round 35 reported "births unmoved, medians exactly 0". I am
not claiming this as a finding — the counts are 0–13, the sign test is
marginal (p ≈ 0.055), and my arms are both post-§Z23-fix where round 35's were
not. **Recorded as unresolved, not as a result.** If it is real it is far more
likely to be the larder surviving (§5) than rival meat, which §3 has just
priced at two-thirds of one child.

---

## 4. `colony_ants` — the dial is clamped, and the shipped default is already in the clamp

*`colony_ants` sets how many founders stand together — at a live dial that is
also how big a war party is.*

**It is a request, not a count.** Ants actually placed, deterministic across
seeds:

| requested, per colony | placed, per colony | arrives |
|---|---|---|
| 13 | 13 | 100% |
| 26 | 26 | 100% |
| **52 (shipped)** | **47** | 90% |
| 78 | 60 | 77% |
| 104 | **63** | 61% |
| 120 | **63** | 53% |

**Above 63 per colony nothing arrives at all — 104 and 120 place identically.**
The lab slider spans 1–120, so **roughly the top half of it does nothing**, on
the shipped 512-wide two-colony bed, with no feedback anywhere saying so.

**The cause is the bed's width, not a colony rule**, and the prediction was
tested rather than inferred. `colony_stations` lays a single row of `ants`
stations at `COLONY_ANT_SPACING` (4) apart, centred on the cursor; stations
that fall outside the world or off the ground are silently dropped by
`colony_ant_site`. So the ceiling is about `width / (colonies x spacing)`.
Positive control:

| colonies | requested | width | placed |
|---|---|---|---|
| 1 | 120 | 512 | **120 — the whole request** |
| 2 | 120 | 512 | 126 |
| 2 | 120 | 1024 | 207 |
| 4 | 60 | 512 | 149 |

One colony on the shipped width takes its full 120, which rules out any
colony-internal cap; doubling the width nearly doubles the ceiling. **This
matters beyond the dial: it is a silent placement loss that will move again
under M10 streaming, and any measurement quoting `colony_ants` as a population
is quoting a number the bed did not honour.**

**What it does to predation, 12 seeds each:**

| `colony_ants` | placed (both colonies) | median `killedA` | per head | seeds with a kill |
|---|---|---|---|---|
| 26 | 52 | **1** | 0.019 | 9/12 |
| 52 (shipped) | 94 | **8** | 0.085 | 11/12 |
| 104 | 126 | **7** | 0.056 | 11/12 |

**Predation is contact-limited, not war-party-limited.** Between 26 and 52 the
kill count rises eight-fold. Between 52 and 104 the population rises 34% and
the kill count does not move (8 against 7, well inside the per-seed spread:
`[8,6,10,12,7,11,7,12,0,9,8,6]` against `[9,10,8,7,11,7,6,5,0,5,7,9]`), so
**per head it falls**. Doubling a colony does not double its border.

**The shipped 52 is a good default and I am not proposing a change to it**: it
is the setting at which the mechanism is most reliably visible (11/12 seeds),
and it sits just under the point where the dial stops being honest.

---

## 5. What closing §Z23 exposed — §Z26, and it is the biggest thing here

**§Z23 was halving the yield of every renewable plant niche in the engine.**

`a_lone_grazer_cannot_farm_a_moss_lawn_forever` asserts that a renewable moss
lawn must not out-yield an inexhaustible litter larder — the sessile-
freeloading attractor P-20 names. It passed for as long as §Z23 was open, and
it passed **because** it was: the grazer's alarm aimed the fight verb at the
lawn it was eating, and that loss held the pump shut.

Paired, the two arms of that test, §Z23's gate the only change:

| arm | §Z23 open | §Z23 fixed |
|---|---|---|
| moss lawn intake | 456 J | **912 J** |
| litter larder intake | 684 J | **684 J** |
| lawn efficiency | 1.459 | **2.435** |
| larder efficiency | 1.757 | **1.757** |

**The larder arm is byte-identical, which is the specificity control** —
`litter` is a loose material carrying no organism id, so the gate provably
cannot reach that arm. The moss arm is live organisms and exactly doubles. One
quantity, moving for one reason.

**Not fixed here, on purpose.** The remedy is already written in the test's own
doc — a per-cell post-grazing cooldown, never shrinking `food_energy` — and it
reallocates the plant economy, so `moss.ron`'s `damp_chance: 0.35` at
`cost: 0.0` and everything calibrated against the grazing yield as it has
actually behaved want re-deriving with it. The guard is `#[ignore]`d with a
pointer, not weakened: **un-ignoring it is the acceptance test for that work.**

---

## 6. For whoever runs the next round

- **§Z26 is the one worth taking.** It is a live pump in the plant economy,
  it has a paired reproduction and a written remedy, and it is bigger than
  anything the rivalry dial did.
- **§Z23's second half is still open**: `CreatureStats::attacks` is not a
  count of animals fighting and there is no counter that is. It is now *more*
  misleading, not less — the number is small (0–71 against 344–475) and
  finally animal-directed, so a reader is likelier to trust it as a fighting
  figure. It still counts "the `Attack` branch reached a target".
- **The `colony_ants` clamp (§4) is not filed as a bug** — it is a silent
  placement loss rather than a defect with a wrong answer, and I did not want
  to spend a letter on something the owner may simply want the slider
  re-ranged for. If it recurs in another lane's measurement, file it.
- **Do not price a colony off `deaths`, `starved` or `starv%` from any lab
  harness without checking the denominator.** Those are whole-bed columns and
  a played box is mostly plants. `deathsA`/`starvedA`/`starvA%` exist in
  `rivalry` now; nothing else reads `group_deaths`.
- **Nothing was posted to the review queue.** Every finding here is a count or
  a joule, the rivalry-visibility question is closed by the owner's ruling,
  and the queue is for visual evaluations only.

## 7. Gates

`cargo clippy --all-targets --release --locked -- -D warnings`; `cargo test
--lib` (1,804 passed / 0 failed / 86 ignored); `cargo test --test worldgen
--test determinism` (44 passed); `cargo run --release --example ascii` (31
scenes, 0 skipped — the excavation scene still reads digs 354 / roofed void
42, unmoved by this branch); `bash scripts/docscheck.sh` clean. All measured
runs at `RAYON_NUM_THREADS=4` with a private `TMPDIR`.
