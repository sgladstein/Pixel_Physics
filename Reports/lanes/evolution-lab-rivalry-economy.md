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

## 1. What shipped — and what I handed back

**The §Z23 fix is NOT in this branch. Lane E's is better and it supersedes
mine; I reverted `src/sim/creature.rs`, `wiki/ants.md` and
`Reports/open-bugs-handoff.md` to `main` rather than make E resolve a conflict
against a subset of their own work.** §5a has the whole account.

What this branch carries:

- **`examples/rivalry` reads the economy animal-only**, and `colony_ants=` is
  a knob. New `SUMMARY` columns, appended after `main`'s:
  `deathsA starvedA killedA starvA% raided_j raided_by_others_j`, plus a
  `SUMMARY-causes-animal` line and an `economy` echo carrying the birth bar,
  the grant, and what a rival is worth against it.
- **The harness's own selftest, repaired** — it had been failing on `main`
  since #423 and nothing gated it. §5d.
- **`ancestor.ron` can hear an alarm**, routed here by Lane C. §5b.
- The findings below.

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

## 5. The moss lawn — withdrawn, see §5a

I filed this as §Z26 (*the moss pump is live*) and then withdrew it: Lane E's
`dead-ends.md` creatures:083 is the better disposition, and my claim does not
follow from a comparison whose two arms differ in more than the mechanism under
test. The paired numbers survive as evidence for **E's** entry. §5a.

---

## 5a. Lane E superseded this, and I stood down

**Resolved 2026-09-14, ~21:55Z.** The coordinator reassigned `creature.rs`'s
alarm/foe path to Lane E by a poke that fired at 19:58:44Z — *after* this PR
was opened and green at ~19:51Z, with E's branch not yet existing. That was a
poke crossing a landing, `CLAUDE.md`'s own case: *the only check that works is
the branch head.*

**Lane E has now pushed `claude/evolution-lab-forest-eaten`, and their fix is
strictly better than mine on every axis.** I reverted my `creature.rs`,
`wiki/ants.md` and `Reports/open-bugs-handoff.md` changes to `main`.

| | mine | Lane E's |
|---|---|---|
| eating a plant raises no alarm | yes | yes |
| creatures do not attack plants at all | **no** — I left the target rule intact | **yes**, both owner rulings |
| sized the problem first | no | **yes: the jaw takes 9 of every 10 cells a colony removes from a living plant** |
| counter split by victim kind | **no — I said this was missing and could not close it** | **yes** (`attacks_at_plants`, `attack_plant_cells`) |
| one-binary A/B | no | **yes** (`PIXEL_PHYSICS_PLANT_FOE=on`, reproduces pre-fix byte-identically) |

**E's counters settle the claim I struck.** I could not rule out plant-directed
swings surviving my gate, because `xcol`/`killedA` are kill counters. E
measured it: **100% of attacks and 100% of attack cells were at plants on
every seed**, going to 0 after their fix. So the gap I flagged was real and is
now closed — by them, with the instrument I said the question needed.

**E also out-diagnosed me on §Z26, and I withdrew it.** I filed the moss-lawn
result as a bug — *the moss pump is live*. E recorded it as `dead-ends.md`
creatures:083 instead, withdrawing **the guard's bar rather than the
mechanism**: `a_lone_grazer_cannot_farm_a_moss_lawn_forever` compares two
larders holding **different foods in joules**, and the wall arm is a ceiling on
*mouthfuls*. The two arms differ in more than the thing under test, so the
ordering was only ever correct while §Z23 propped it up. That is the better
reading, and it makes my "the pump is live" claim unsupported — the comparison
that would establish it was never valid.

**My paired numbers are the evidence for E's entry, not a separate bug**, and
they are exactly the tell E's general form names — *only one arm moves, and the
arm containing no instance of the mechanism is the control*:

| arm | §Z23 open | §Z23 fixed |
|---|---|---|
| moss lawn intake (contains the mechanism) | 456 J | **912 J** |
| litter larder intake (the control) | 684 J | **684 J — byte-identical** |

`litter` is a loose material carrying no organism id, so the gate provably
cannot reach that arm. Recorded here rather than re-filed.

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

## 5d. The harness's own selftest has been failing on `main` since #423

**Found because Lane E's note said "rivalry selftest failure is pre-existing".
`examples/rivalry.rs` is mine, so I checked rather than took it.** It is
pre-existing — byte-identical failures on unmodified `origin/main` — and the
cause is worth naming, because this harness's numbers are quoted all through
this note.

Two arms failed, and both are #423's live default landing inside a control
written before it existed:

1. **`Apart::No` set no `scent_spread` at all** — it inherited whatever
   `ant.ron` authored. That was 0 until #423, so the arm genuinely meant
   "everyone is kin"; from #423 it silently meant *the shipped stranger bed*,
   and the two claims resting on it (`shipped` reads `cross 0, attacks 0`;
   `wire-only` shows a swinging ant with no target) have been false ever since.
2. **The separated arms ADDED their offset to the scent an animal already
   carried** — precisely the trap round 35 recorded and repaired in the
   `spread=` path, and missed here because this function keeps its own copy.
   The tell was in the log: a founding gap of **2.170** against the **1.62**
   median round 35 measured for a true `spread=1`.

Repaired by pinning every arm's spread explicitly and re-deriving each standing
ant's signature from the **ancestral** point, so `requested` means "founded at
this spread" for any authored default, 0 included. Verified by the arms' own
numbers:

| arm | before | after |
|---|---|---|
| `shipped` gap / between | 1.744 / 100% — **FAIL** | **0.000 / 0%** — ok |
| `wire-only` attacks | 2 — **FAIL** | **0** — ok |
| `two-colonies` gap | 2.170 (doubled) | **1.710** — matches round 35 |

**Nothing gated this**: CI does not run `rivalry control=selftest`. A control
that inherits a default rather than naming it is a control that breaks silently
when the default moves, and `CLAUDE.md` already has the rule — *adding a member
to a set enrols it in every rule over that set*, with the set here being every
arm that did not name its own spread.

---

## 6. For whoever runs the next round

**FIRST, and it is routed to this lane by Lane E: E's fix grows the colony, so
every economy number in §2, §3 and §4 wants re-taking on the far side of it.**
E measured ants **66 → 123, 81 → 146, 156 → 190** across three seeds, because
the jaw work the colony was billed for bought nothing and that bill is now
gone. A bed with twice the ants is a different economy: predation opportunity,
plant intake and the starvation rate all move together, and **nothing in §3
survives that automatically.** My conclusion there — predation income is ~0.65
of one child per bed and cannot reach the birth bar — is a claim about the
pre-E bed, stamped at `768c1b59` in §6b. **Re-run the 12-seed paired sweep once
E lands.** The instrument is built and the columns are in place; it is one
command per arm.


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
