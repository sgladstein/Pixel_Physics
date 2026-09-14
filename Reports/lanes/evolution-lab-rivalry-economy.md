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
| over 12 seeds at the shipped dial | — | every cross-colony **kill** preserved: `xcol == killedA` on all twelve rows |

**Not claimed: that every remaining *swing* is animal-directed.** That is a
`kill` counter and it cannot say it — see §5a, where I strike the claim.
Ruling out plant-directed swings needs a counter split by victim kind, and the
owner's "creatures will not attack plants at all" is what closes the gap.

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

## 5a. ROUTING COLLISION — §Z23 was already fixed and pushed before the reassignment reached me

**Read this before Lane E starts.** The coordinator's poke reassigning
`creature.rs`'s `cry_alarm`/`nearest_foe`/attack-target path to Lane E fired at
**19:58:44Z**. My §Z23 fix was committed at **~19:47Z** and PR
[#436](https://github.com/sgladstein/Pixel_Physics/pull/436) opened at
**~19:51Z**, gates green. `origin/claude/evolution-lab-forest-eaten` did not
exist when I checked at 20:08Z. So this is not a lane writing into another's
file — it is a poke that crossed a landing, which is the exact failure
`CLAUDE.md` names: *there is no delivery signal for a poke; the only check that
works is the branch head.*

**I have stopped touching that path** and have made no edit to it since the
poke. What follows is the hand-off, not a claim on the file.

### What is already done, and what is still Lane E's

The owner's two rulings, against what #436 actually implements:

| owner's ruling | in #436? |
|---|---|
| **Eating a plant will not raise an alarm** | **yes — this is exactly the gate on `cry_alarm`'s two feeding sites** |
| **Creatures will not attack plants at all. No exception.** | **NO — and I deliberately did the opposite** |

**The second row is the important one and it is a correction to my own
judgement.** I left `nearest_foe`'s target rule intact on purpose, following
#417's argument and the §Z23 section's own designed repair — *an animal
cornered by something it cannot digest must still be able to hit it*. **The
owner has now ruled against that, with "no exception".** That argument is
overruled, the register section is out of date, and closing it is Lane E's.

### A claim of mine to strike, because it is wrong

My PR body and §1 above say every remaining swing is animal-directed, citing
`xcol == killedA` on all twelve seeds. **That does not follow.** `xcol` and
`killedA` are both *kill* counters; they say the kills were cross-colony. They
say nothing about swings that took no cell, and those are most of them — seed
10 post-fix reads **attacks 71, cells 13, killedA 9**.

**So I cannot rule out plant-directed swings surviving my fix**, and the
mechanism is still reachable: an animal bitten nearby raises an alarm, a
listener swings, and `nearest_foe` hands it whatever living non-kin organism is
nearest — which may be the plant it is standing on. Establishing that needs a
counter split by victim kind, which nothing has. **This is precisely the gap
ruling (a) closes**, so it is E's to close rather than mine to measure.

### What Lane E can take from #436

- **The guard** `eating_a_plant_raises_no_alarm_and_eating_an_animal_does`,
  with its animal arm as the positive control. **It was blind on its first
  writing** — the plant arm used a `leaf`, and `leaf.ron`'s `food_energy: 40.0`
  against a neutral gut's 0.25 is 10.0, under `EAT_YIELD_THRESHOLD`'s 12.0, so
  a shipped ant cannot eat a bare leaf and the call site was never reached.
  Use `fruit` or `moss`. Only the fault-reinstatement rule found this.
- **`is_animal_cell`**, one predicate with three readers, already in place.
- **§Z26** (§5 below) — closing this doubles the moss lawn's yield and makes
  the moss pump live. **E's fix will land the same consequence**, so the
  `#[ignore]`d guard and the filed bug apply to E's change as much as mine.
- **Wood.** The coordinator's note says the attack bite has no diet gate and
  takes wood, which carries no `food_energy` at all. That is consistent with
  what I measured and is the sharpest statement of why this was pure loss;
  nothing in #436 addresses it, because #436 does not touch the target rule.

**If E's fix supersedes mine at those two call sites, take E's.** Mine is a
strict subset of the owner's rulings.

---

## 5c. The coordinator's plant_j datum — checked, and it needs two corrections

*Handed to me as adjacent to the economy: "rivalry off vs on, same seed, plants
felled 161 -> 165 with `plantkill` 0 in both arms, but plants EATEN moved
5,333 -> 9,690 joules."*

**Correction 1: `plant_j` is not "plants eaten".** `EnergyLedger::
harvested_plant`'s own doc: *"Eating something whose worth comes from its
material: leaf, moss, seed, **a live animal's flesh**."* A mouthful of rival
books to the same account as a mouthful of leaf. That is why this lane had to
read `ColonyBooks::raided` to see predation income at all — and it means the
datum's rise is partly ants.

**Correction 2: one seed cannot carry it.** Over 12 paired seeds the sign is
not stable — `plant_j` **falls on 3** and rises on 8, median **+2,548 J** over a
range of **−5,899 to +9,158**. And of a *positive* delta, predation accounts
for a median **19%** (range 7–191%; on seed 8 predation exceeds the whole
delta, so genuine plant intake fell there while `plant_j` rose).

**So the effect may well be real, and it is not established by this datum.**
The honest statement is that rivalry raises plant intake on about two seeds in
three, by a highly variable amount, of which roughly a fifth is not plants.
Measured post-§Z23-fix on both arms (see SHAs below).

---

## 5b. Lane C's routed change, taken

**`ancestor.ron` now carries `(Alarm, Move, -1.0)` and `(Alarm, Attack, 2.0)`.**
Lane C found it the only species in the ant family reading the trail planes
and not the alarm — the word did not appear in the file at all. Verified here
before acting on it: every other ant-family species and the flitter carry five
`Alarm` mentions, `ancestor` carried zero.

It matters because round 35 shipped rivalry on and `ancestor` is a foundable
stock in the held world, so that is a real bed whose founding lineage heard
every fight and could not act on one.

**The same two weights, not tuned ones** — the claim is that this species was
missing what its siblings have, and a value picked for it would be a second,
unmeasured change wearing this one's justification.

**Guarded by `every_ant_family_species_can_hear_an_alarm`**, which sweeps the
family and asserts on the **genome past `W_EPS`**, not on the file. That is
Lane C's own warning turned into machinery: `eval_brain` drops a weight under
0.01, so a row present in the `.ron` and authored small is dead on arrival and
greps as wired. Watched going red for both faults — the row deleted, and the
row present at 0.001.

**Not sold as recruitment, per Lane C's §2d**: at the alarm plane's measured
two-cell reach these fire only for an animal already touching the fight.

**Lane C's item 2 — the flitter lays and reads no trail — is left alone.** It
is a design question about the one animal the lateral slots were justified
for, not a defect, and it is not mine to settle.

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

## 6b. Head SHAs for every measurement, per the coordinator's request

Both arms of every paired sweep were run on one binary, so each comparison is
internally consistent whatever moved around it.

| measurement | tree | side of E's fix |
|---|---|---|
| §Z23 reproduction, 475/79/629 at seed 1 | `f4e3b471` (`origin/main`) | **before** |
| pre-fix shipped arm, seeds 1–3 | `f4e3b471` | **before** |
| the 12-seed paired sweep (§2, §3), `colony_ants` sweep (§4), placement table, plant_j check (§5c) | `768c1b59` — my §Z23 gate, nothing else | **after mine, before E's** |
| the moss-lawn pair (§5) | `768c1b59` against the same tree with the gate reverted | both sides of **mine** |

**None of these sit on the far side of ruling (a)** — no arm here has
`nearest_foe` refusing plants, because nothing in #436 changes the target rule.
A number taken after E lands is not comparable to one here, and the columns
most likely to move are `attacks`, `cells` and anything reading `plant_j`.

## 7. Gates

`cargo clippy --all-targets --release --locked -- -D warnings`; `cargo test
--lib` (1,805 passed / 0 failed / 86 ignored); `cargo test --test worldgen
--test determinism` (44 passed); `cargo run --release --example ascii` (31
scenes, 0 skipped — the excavation scene still reads digs 354 / roofed void
42, unmoved by this branch); `bash scripts/docscheck.sh` clean. All measured
runs at `RAYON_NUM_THREADS=4` with a private `TMPDIR`.
