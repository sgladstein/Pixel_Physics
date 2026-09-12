# The long ant pile — lane I, round twenty-nine (2026-09-11/12)

**The owner's playtest report, 2026-09-11:** *"long ants getting stuck. Not
all of them but it happens regularly. It seems like they get stuck in a big
group/pile of long ants."*

Branch `claude/lab-longant-pile-r29`. Measured on the owner's own scene
(`assets/lab_scenarios/played_bed_longant.ron`), 120,000 frames,
`RAYON_NUM_THREADS=4` pinned because the pile counters sit downstream of
`parallel.rs`'s checkerboard, both arms in **one binary**
(`PIXEL_PHYSICS_TRAFFIC_DEFER`) — and **on the tree after `origin/main`'s
`84dd9bfc` (nest odour) was merged in.** Every figure taken before that merge was on a tree nobody
else has; all were retaken and the pre-merge table is not reproduced.

## 1. Three findings, and only the first is fixed

| | what it is | filed |
|---|---|---|
| a laden long body waits for a nestmate that will never move | the deferral never expired | **fixed here**, §3 |
| most of what is wedged is **one-cell ants, bred that way** | a flip is a no-op for them | **§Z12**, OPEN |
| the long ants a player *points at* are **resting**, and resting looks identical to stuck | a look problem, not a walk bug | **§Z13**, OPEN |

The third was found by answering the owner's own markers, and it is the one
that matters for reading his verdicts: **what he pointed at was never what
this branch changes.**

## 2. The instrument

`creature::head_block`, `body_boxed`, `piles_of` and `labforage`'s `probe=`
carry their own doc comments. What is not in them:

- `moves_blocked` cannot tell rock from colony and those want opposite fixes,
  which is why `head_block` splits the eight headings through the walk's own
  `classify_step`/`body_after_step` — a classifier that disagrees with the
  code it classifies is worse than none.
- The largest clump is reported **with where and when**, so a card can be
  cropped on a measurement.
- `probe=` pairs `moves` with `moves_blocked` (*shoving and losing* against
  *not asking*) and `bites`/`digs`/`deliveries` beside them, because both
  counters flat is the finding **and** is exactly what a dead animal reads.
- Controls both ways. *Specificity*: the same census on the shipped two-cell
  ant reads body-boxed **0.0%**, largest pile 0. *Sensitivity*:
  `six_long_bodies_nose_to_tail_read_as_one_pile`, watched red twice.

## 3. The fix: the deferral expires

`creature::deferral_still_applies(spine_len, waited, max)`. `max` is
`CreatureDef::traffic_defer_max`, authored **only on `longant.ron`**
(`Some(4)`) and `None` everywhere else; `OrganismState::traffic_deferred`
counts consecutive deferrals and is cleared by any tick that is not one.

`CLAUDE.md`'s *a size cap must bound work, never gate whether something
happens*, in the time axis: the gate decided **whether** the flip ever
happens and now decides **how long the animal waits first**, so it cannot
reopen either gate §13g rejected — both of those *withheld* the verb and this
only returns it. **Two cells or fewer is exempt whatever a species authors**,
which is §13c's property rather than a species check.

**Every other species is identical, by construction and by measurement.**
Only `longant.ron` authors the field and `max: None` returns the old rule (a
unit test pins it). Measured on the shipped two-cell ant, `played_bed` 20,000
frames, `RAYON_NUM_THREADS=1`: the two arms **identical on every line of
output**, and since the env override forces the expiry *on* for that species
the run proves the two-cell exemption too, not just an unset field.

## 4. The owner's markers, answered — §Z13

Card `20260912T045951545Z-6931d4`, three markers on the **fix** arm: *"this
the most prominent thing that shows no movement in both images"*, *"also no
movement"*, *"no movement"*.

The card JSON records no capture parameters: they came out of the stored GIF
(uniform 4x4 blocks → `zoom=4`, a 1,660 ms delay → `every=100`) and the crop
offset from matching a full-frame render against frame 0. **Do this before
answering an annotated card — it also settles which arm is which without
trusting `blind_was`.**

His markers are world cells **(363,155)**, **(302,149)**, **(244,154)**, and
all three hold **full-length long ants** — 7, 7 and 6 cells against an
authored 7 — with **3, 1 and 1** of eight headings open, `moves` +0/+0/+1,
**`moves_blocked` +0 at all three** across 3,000 frames, `traffic_deferred` 0
at every reading, and `crossing`/`flight`/`senescent` clear on every line.
None is boxed, so **none appears in any column of §2's census, in either
arm.** In the unchanged arm those cells are bare ground, bare ground and bare
nest — the arms are different worlds by frame 28,000, so the same screen
position is not the same animal (occupied 151/151 frames in one arm against
6/151, 0/151, 0/151 in the other, read off the card's own images).

**This is the rest state, and the control says the duration is not the long
ant's either.** `p_move` collapsing is the owner's own ruling working as
shipped — *"rest is the absence of a reason to act"*, 2026-09-09. Two turns
of *ask what your number counts when nothing is wrong* cut the finding down
both times. The idle **rate** (a legal heading, head unmoved since the last
stop) reads **74–76%** for the long ant and **75% for the shipped two-cell
ant**. Then the idle **duration**, longest streak in stops of 900 frames:

| | s1 | s2 | s3 | s4 | s5 | s6 | s7 | s8 | s9 |
|---|---|---|---|---|---|---|---|---|---|
| long-ant colony, **3+ cells**, unchanged | 33 | 40 | 68 | 64 | 63 | 56 | 83 | 61 | 53 |
| ...with the expiry | 62 | 46 | 54 | 49 | 38 | 71 | 50 | 62 | 35 |
| **shipped two-cell ant**, all bodies | **56** | **68** | **62** | — | — | — | — | — | — |

p90 **11–16** stops for those long bodies against **13–20** for the shipped
ant. **The shipped ant rests as long as a full-length long body does** —
50,000-plus frames in one spot on every seed — **and nobody has ever reported
it**, because two motionless pixels read as scenery and a motionless
seven-cell body reads as stuck. Nothing about the long ant's behaviour is
anomalous. **So §Z13 is a look problem**: what should a resting ant *do* so
it reads as resting? Candidates that move nothing and touch no economy — a
head turn, antennating, a one-cell shuffle and back — each with a different
cost to the dirty-rect render skip, which is what to price first. **A card
for the owner, not a mechanic to pick.** (`idle_streak_*_any` exists because
the 3+-cell gate made the two-cell control vacuous, and an always-zero
control is not one.)

## 5. The table — nine seeds, and it is not the pre-merge story

120,000 frames per run, `sample=900`. **Read the wedged-long-body column as a
*rate*, not a count**: the two arms' colonies differ in size by up to 2x on
the same seed (`pile_animal_reads` 29,426 against 52,782 on seed 7), so a raw
reading count compares two different denominators. `CLAUDE.md`'s *ask what
your number counts*.

| unchanged → expiry | s1 | s2 | s3 | s4 | s5 | s6 | s7 | s8 | s9 | better |
|---|---|---|---|---|---|---|---|---|---|---|
| **wedged long bodies, % of readings** | 0.22→0.90 | 1.23→1.79 | 1.62→**0.69** | 1.93→**0.90** | 2.23→**1.52** | 0.85→0.83 | 1.23→**0.65** | 3.00→**1.08** | 5.05→**1.33** | **7 of 9** |
| longest long-body streak, stops | 16→**12** | 13→17 | 38→**28** | 30→39 | 34→**14** | 25→72 | 20→45 | 66→**14** | 59→**55** | 5 of 9 |
| largest clump, all bodies | 75→**14** | 6→10 | 19→56 | 25→27 | 10→**8** | 16→24 | 17→**191** | 15→**11** | 6→24 | 3 of 9 |
| ...3+-cell bodies alone | 3→5 | 3→4 | 3→3 | 6→5 | 3→4 | 4→3 | 3→5 | 7→3 | 5→3 | max **7** |
| **alive** | 1,895→259 | 63→**241** | 371→**400** | 385→**468** | 103→41 | 447→**517** | 149→**345** | 102→**162** | 65→**122** | **7 of 9** |
| deliveries | 382→193 | 2,589→**4,217** | 3,488→3,170 | 3,167→**4,268** | 3,905→3,631 | 267→222 | 4,228→2,926 | 2,976→**3,231** | 50→**130** | 4 of 9 |

**The column the change is about improves on 7 of 9 seeds**, median **−0.72
percentage points**, and it improves most where the problem was worst (seed
9, 5.05% → 1.33%). The longest long-body streak is mixed — 5 of 9, median −4
stops, with seed 6 going 25 → 72 the wrong way.

**`alive` rises on 7 of 9, median +60 — the pre-merge finding does not
reproduce.** That table had `alive` falling on all three of its seeds and the
lane read it as the price of letting animals move; on the merged tree the two
seeds that fall (s1 1,895 → 259, s5 103 → 41) are the two whose unchanged arm
runs a runaway one-cell population, and the expiry's colony sizes are far
*less* variable (41–517 against 63–1,895). `deliveries` are a wash, median
−45. Every death in every arm is still `STARVED`.

**What does get worse is the clump a player sees**: 3 of 9, with seed 7 going
17 → **191**. That is §Z12's population, not this change's — the 3+-cell
clump never exceeds **7** on any seed in either arm. The expiry's colony
breeds more, and what it breeds more of is one-cell bodies.

**So the ship condition as briefed is still not met** — the visible pile is
not reliably smaller — but the failure is not the one the pre-merge table
predicted, and the two quantities the change actually governs both move the
right way on 7 of 9 seeds. Whether that is enough is the coordinator's and
the owner's call, and it is recorded here rather than argued away.

## 6. §Z12's starting facts — settled, not guessed

`pile_short_by_loss` is **0 in all eight runs**: not one wedged short body
was born long and lost cells. Authored and held cell counts agree to within
29 of 10,154 on the worst seed, a mean of **1.0 against 1.0**, and
`pile_short_max_gen` is 6–31, **never 0**. They are a bred one-cell morph.
Full account, and the two candidate fixes that make each other unnecessary,
in **§Z12**.

## 7. What is not reopened

The **streak-before-flipping** gate and the **traffic check asked of every
animal** (§13g: 16.3% / 18.8% / 77.5% blocked on `tunnel` against 7.6%), and
**passability through a nestmate** — `climbs_over_kin` grants footing only
and is already authored, and the long bodies do not need it: their clumps
never exceed **7** on any seed in either arm. All three in
`Reports/dead-ends.md` with their numbers, with `traffic_defer_max` at 64
(vacuous, and the positive control on the ablation arm) and at 16.

## 8. Gates, on the merged tree

All six green: `cargo test --lib` **1,634 passed / 0 failed / 70 ignored**,
clippy clean, `ascii` 31 scenes / 0 skipped, `acceptance.sh` all cases met,
`worldgencheck.sh` and `docscheck.sh` clean. Both clippy findings were mine
and both 1.98-only lints (`manual_checked_ops`, `manual_is_multiple_of`) —
`CLAUDE.md`'s toolchain gotcha, caught by the pin rather than by CI.
`acceptance.sh` failed two cases pre-merge on its **wall-clock** budget with
five lanes compiling; on a quiet box it passes, which is the standing reading
of that gate.

Cards: `20260912T041500164Z-018ccf` (stills; the verdict §4 decodes),
`20260912T045951545Z-6931d4` (GIF, too slow, the three markers),
`20260912T082603394Z-3ede0a` (frame sequence — the review page plays these on
its own 90 ms timer, so `every=24` gives 4.4x real speed and a 9 s loop
without a delay knob anywhere; the late window where the crowding actually
is, asking whether what reads as stuck is the one-cell ants).
