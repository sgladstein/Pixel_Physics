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
carry their own doc comments; what is not in them is why each exists.
`moves_blocked` alone cannot tell rock from colony and those want opposite
fixes, so `head_block` splits the eight headings through the walk's own
`classify_step`/`body_after_step` — a classifier that disagrees with the code
it classifies is worse than none. The largest clump is reported **with where
and when** so a card can be cropped on a measurement rather than a guess.
`pilefollow=1` follows one animal, the only way to see *why* a pile holds.
`probe=` names what stands at a given cell stop after stop, and pairs `moves`
with `moves_blocked` (*shoving and losing* against *not asking*) —
`bites`/`digs`/`deliveries` beside them because both counters flat is the
finding and is also exactly what a dead animal reads.

**Both controls.** *Specificity*: the same census on the shipped two-cell ant
reads body-boxed **0.0%**, largest pile 0. *Sensitivity*:
`six_long_bodies_nose_to_tail_read_as_one_pile`, watched red twice.

## 3. The fix: the deferral expires

`creature::deferral_still_applies(spine_len, waited, max)`. `max` is
`CreatureDef::traffic_defer_max`, authored **only on `longant.ron`**
(`Some(4)`) and `None` everywhere else; `OrganismState::traffic_deferred`
counts consecutive deferrals and is cleared by any tick that is not one.

`CLAUDE.md`'s *a size cap must bound work, never gate whether something
happens*, in the time axis: the gate decided **whether** the flip ever
happens and now decides **how long the animal waits first**. It cannot reopen
either gate §13g rejected — both of those *withheld* the verb and this only
returns it. **A spine of two cells or fewer is exempt whatever the species
authors**: such a body reverses with an ordinary step onto its own vacating
tail, so an endless deferral costs it nothing.

**Every other species is identical, by construction and by measurement.** The
whole `src/`+`assets/` diff against `origin/main` is **684 insertions, 0
deletions**, only `longant.ron` authors the field, and `max: None` returns
the old rule (a unit test pins it). Measured on the shipped two-cell ant,
`played_bed` 20,000 frames, `RAYON_NUM_THREADS=1`: the two arms **identical
on every line of output** — and since the env override forces the expiry *on*
for that species, the run also proves the two-cell exemption holds.

## 4. The owner's markers, answered — §Z13

Card `20260912T045951545Z-6931d4`, three markers on the **fix** arm: *"this
the most prominent thing that shows no movement in both images"*, *"also no
movement"*, *"no movement"*.

The card JSON records no capture parameters: they came out of the stored GIF
(uniform 4x4 blocks → `zoom=4`, a 1,660 ms delay → `every=100`) and the crop
offset from matching a full-frame render against frame 0. **Do this before
answering an annotated card — it also settles which arm is which without
trusting `blind_was`.** His markers are world cells **(363,155)**, **(302,149)**, **(244,154)**, and
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

| | s1 | s2 | s3 | s4 | s5 | s6 |
|---|---|---|---|---|---|---|
| long ant, unchanged | 45 | 44 | 68 | 64 | 63 | 65 |
| long ant, expiry | 62 | 54 | 54 | 61 | 38 | — |
| **shipped two-cell ant** | **56** | **68** | **62** | — | — | — |

p90 **11–17** stops for the long ant against **13–20** for the shipped ant.
**The shipped ant rests just as long — 50,000-plus frames in one spot on
every seed — and nobody has ever reported it**, because two motionless pixels
read as scenery and a motionless seven-cell body reads as stuck. Nothing
about the long ant's behaviour is anomalous. **So §Z13 is a look problem**:
what should a resting ant *do* so it reads as resting? Candidates that move
nothing and touch no economy — a head turn, antennating, a one-cell shuffle
and back — each with a different cost to the dirty-rect render skip, which is
what to price first. **A card for the owner, not a mechanic to pick.**
(`idle_streak_*_any` exists because the 3+-cell gate made the two-cell
control vacuous, and an always-zero control is not one.)

## 5. The table — and it does not settle the pile

120,000 frames per run. **The sign flips across seeds on every pile
column**, so a handful of seeds cannot establish that the expiry improves the
pile, and this is recorded rather than argued away. Seeds 1–4 below; 5–9 were
still running when this was written and belong in this table before anyone
reads a direction into it.

| unchanged → expiry | s1 | s2 | s3 | s4 |
|---|---|---|---|---|
| largest clump | 75 → **14** | 6 → 10 | 19 → 56 | 25 → 27 |
| ...3+-cell bodies alone | 3 → 5 | 3 → 4 | 3 → 3 | 6 → **5** |
| body-boxed, % of readings | 13.6 → **5.9** | 2.9 → 5.1 | 5.7 → 7.9 | 6.8 → **5.0** |
| longest streak, 3+ cells | 16 → **12** | 13 → 17 | 38 → **28** | 30 → 39 |
| alive | 1,895 → 259 | 63 → **241** | 371 → **400** | 385 → **468** |
| deliveries | 382 → 193 | 2,589 → **4,217** | 3,488 → 3,170 | 3,167 → **4,268** |

What *is* consistent over these four is the **spread**: the unchanged arm's
colony runs 63 to 1,895 animals, a 30x span, and the expiry arm 241 to 468.
`deliveries` rise on two and fall on two. Every death in every arm is
`STARVED`. **The ship condition as briefed is not met**, and the case for
landing it is the defect being real on its face plus every other species
being bit-identical — a judgement for the coordinator and the owner, not for
this lane.

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
and is already authored, and the census says the long bodies do not need it:
their clumps reach 3–6 animals against 6–75 for the whole body-boxed
population. All three in `Reports/dead-ends.md` with their numbers, with
`traffic_defer_max` at 64 (vacuous, and the positive control on the ablation
arm) and at 16.

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
