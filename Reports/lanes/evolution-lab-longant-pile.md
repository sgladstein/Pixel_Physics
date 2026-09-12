# The long ant pile — lane I, round twenty-nine (2026-09-11/12)

**The owner's playtest report, verbatim, 2026-09-11:** *"long ants getting
stuck. Not all of them but it happens regularly. It seems like they get
stuck in a big group/pile of long ants."*

Branch `claude/lab-longant-pile-r29`. Everything below is measured on the
owner's own scene — `assets/lab_scenarios/played_bed_longant.ron`, 3 seeds,
120,000 frames, `RAYON_NUM_THREADS=4` pinned so every counter is
load-independent — with both arms in **one binary**
(`PIXEL_PHYSICS_TRAFFIC_DEFER`), so no recompile sits between them.

## 1. The instrument the record lacked: a pile census

`moves_blocked` says an animal did not move. §13a's `boxed_ticks` says it
had nowhere to go. **Neither says whether what it had nowhere to go past
was rock or its own colony**, and those want opposite fixes — §13's flip
for the first, and something else entirely for the second.

`creature::head_block` (`src/sim/creature.rs`) splits one animal's eight
headings by what refused each, running `classify_step` and
`body_after_step` — the walk's own two calls, per §13a's rule that a
classifier which disagrees with the code it classifies is worse than none:

| column | what it counts |
|---|---|
| `open` | headings the animal could walk this tick |
| `by_creature` | refused, and the first landing cell that failed is another living creature's |
| `by_other` | refused by terrain, tissue, the world edge, its own flank, or no footing |
| `kin_would_open` | **the counterfactual**: refused today, and *legal* with the other animals' bodies removed (footing then asked with `kin: None`, because an animal standing *on* the pile has its foothold from the pile) |

`boxed` is `open == 0`; **`body_boxed` is `open == 0 && kin_would_open > 0`**
— it has nowhere to go, and at least one of the places it cannot go would
be walkable if a nestmate stepped aside. `creature::piles_of` groups those
into connected clumps (8-neighbour, the same neighbourhood every walking
and footing rule in the file uses), largest first.

`examples/labforage.rs` runs it at every sample stop: the split, the
largest clump, and a **stuck duration** — consecutive stops one animal
stays body-boxed — as a histogram. `pilefollow=1` names the
longest-stuck animal every stop with its head, tail, cargo and headings.

**Controls, both directions.**

- *Specificity*, the one the brief asked for: the same census on the
  **two-cell ant** colony (`played_bed`, 3 seeds, 120,000 frames) reads
  **body-boxed 0 (0.0%), 0 streaks, largest pile 0** on all three seeds.
  The owner does not report two-cell piles and the census agrees.
- *Sensitivity*: `six_long_bodies_nose_to_tail_read_as_one_pile` — six
  six-cell bodies hand-laid in a one-cell corridor with a blind end read
  as **one clump of six**, the five behind the leader `body_boxed` and the
  leader boxed by the rock it is facing. Watched red twice: with
  `kin_would_open` pinned at 0 the five read terrain-boxed, and with every
  non-own cell reading as a creature the lone-animal control goes red.
- `a_lone_long_body_on_open_ground_is_boxed_by_nothing` is the cheap
  always-zero check on `by_creature`/`kin_would_open`.

## 2. Why the pile holds

**A laden long ant that is boxed with a nestmate in the way defers its flip
— and the deferral never expires.** `boxed_by_traffic` (§13g) withholds the
flip from a laden animal on the premise that *"a jam clears on its own the
moment the other animal takes its own next step"*. §13g named the case
where that premise is false and left it open: **the other animal is boxed
too.** Then nothing in the world is going to move, the same question is
asked and answered the same way every tick, and the animal stands there for
the rest of its life. `pilefollow=1` on seed 3 shows it directly — through
the first 35,000 frames the longest-stuck animal is **laden on 24 of 25
sampled stops**, five to seven cells long, its head and tail in the
*identical cells* stop after stop with only its heading moving, because
`tumble` re-aims it every tick. That is the owner's *"stuck"*, and it is
§13e's own "stuck and just flashing" seen from the numbers rather than the
screen.

**And the second half, which the census found and nothing had looked for:
most of the pile is not long bodies at all.** Splitting `body_boxed` by
cell count, on the unchanged build:

| seed | body-boxed readings | of which 3+ cells | of which 1–2 cells | largest clump | ...3+-cell bodies alone |
|---|---|---|---|---|---|
| 1 | 2,822 | 304 | **2,518** | 18 | **3** |
| 2 | 406 | 113 | **293** | 8 | **3** |
| 3 | 8,696 | 986 | **7,710** | 108 | **4** |

A one-cell body cannot flip at all — reversing a list of one changes
nothing, so the delivery test refuses it every tick for ever, which is
where seed 3's **66,282 refusals against 2,927 traffic deferrals** come
from. These appear later in the run, as the colony passes thirty
generations (`gen=31` on seed 3) and a thousand animals in a 512x320 box.
The long-body clumps stay at 3–4 throughout; the big numbers are short
bodies. **Whether those short bodies are injured or an evolved morph of the
fates genome is not settled here and is worth a lane of its own** — the
census reports the split, and nothing in this branch touches what produces
it.

## 3. The fix: the deferral expires

`creature::deferral_still_applies(spine_len, waited, max)` --
`max` is `CreatureDef::traffic_defer_max`, **authored only on
`longant.ron`** (`Some(4)` of the animal's own ticks) and `None` everywhere
else, which is what makes every other species bit-identical.
`PIXEL_PHYSICS_TRAFFIC_DEFER` overrides it for the arms.
`OrganismState::traffic_deferred` counts consecutive deferrals and is
cleared by any tick that is not one, including a committed move.

`CLAUDE.md`'s *a size cap must bound work, never gate whether something
happens*, in the time axis: the gate used to decide **whether** the flip
ever happens, and now decides **how long the animal waits first**.

**It cannot reopen the two gates §13g rejected**: both of those *withheld*
the verb (a streak before flipping, 16.3% blocked on `tunnel` at N=2 against
7.6%; the traffic check asked of every animal, 77.5%), and this one only
ever returns it. **A spine of two cells or fewer is exempt whatever the
species authors** -- §13c's property, not a species check: such a body
reverses with an ordinary step onto its own vacating tail, so the flip is
not its only way out and a deferral that never ends costs it nothing.

## 4. The table

`played_bed_longant`, 3 seeds, 120,000 frames, `RAYON_NUM_THREADS=4`, one
binary. `before` is `PIXEL_PHYSICS_TRAFFIC_DEFER=65535` — the deferral that
never expires, i.e. `main`'s rule; it reproduces the unchanged build's own
numbers digit for digit on all three seeds, which is the ablation's
fidelity check and the stale-binary check at once.

| | seed 1 | seed 2 | seed 3 |
|---|---|---|---|
| **largest clump** | 18 → **16** | 8 → **7** | 108 → **41** |
| **body-boxed, % of readings** | 8.0 → **6.3** | 4.1 → 5.9 | 15.0 → **6.5** |
| **longest streak, all bodies** (stops) | 68 → **25** | 39 → 57 | 46 → **32** |
| **longest streak, 3+-cell bodies** | 64 → **15** | 30 → **29** | 36 → **24** |
| stops holding a clump of 3+ | 100 → **68** | 24 → 44 | 107 → 101 |
| deliveries | 59 → 56 | 1,700 → **1,970** | 2,601 → **3,065** |
| eats | 24,380 → 16,680 | 12,290 → **15,609** | 44,335 → **50,381** |
| **alive** | 673 → **182** | 194 → **144** | 1,006 → **274** |
| born / died | 858/203 → 379/216 | 195/15 → 159/29 | 1,371/397 → 1,132/890 |

**The stuck-duration tail for the bodies the fix is about falls on all
three seeds** (64 → 15, 30 → 29, 36 → 24), and the largest clump falls on
all three. **`alive` falls on all three, and it is not noise.** The cause is
§13g's own, one step further on: an animal that stops standing still starts
walking and burning — seed 3's burn goes 926,872 → 1,024,047 J on a quarter
of the animals, and every death in every arm is `STARVED`. The colony that
comes out is smaller, moves far more (834,562 → 1,621,888 moves on seed 3)
and delivers more on two seeds of three.

**So the brief's ship condition is not met as written** — it requires
`alive` not to fall — and this is recorded rather than argued away. What the
numbers say is that the two cannot both be had by this lever: the pile is
made of animals that are not moving, and the only way to stop them piling is
to let them move, which costs what moving costs. Re-deriving the economy
against a colony that does not jam is the next piece of work, and it is
`CLAUDE.md`'s standing warning in its usual costume: the food economy was
calibrated against a colony a fraction of which was standing still.

## 5. The owner's verdict on the card, and what it confirms

Card `20260912T041500164Z-018ccf` (blind, `blind_was [1, 0]`), verbatim:
*"You numbers show a huge difference, but I see a clear stuck clump in the
right of A and nothing so obvious in B. Before I read which was which I
thought A was the fix. Maybe it is just the screen shown and I am focusing
on one large clump but you fixed a bunch of tiny clumps... not sure"*.

**Translated through `blind_was`, his A is the *fix* arm** -- so he saw the
clear stuck clump in the changed build, and his own reading of it ("you
fixed a bunch of tiny clumps") is what the split table above says in
numbers. A largest clump of **41** is still a clear stuck clump on screen,
and 89% of it is short bodies the expiry cannot reach. **The expiry is right
for the long bodies and is not what the owner is looking at.** Re-posted as
movement rather than stills (`20260912T045951545Z-6931d4`), per the owner's
standing ruling that a judgement of this kind wants an animation.

## 6. What the short bodies are -- one counter, for round 30

`creature::authored_body_cells` reads how many cells an animal's own
`FateGenome` **would** unfold to, against how many it has. There are only
two ways a long ant is one cell long -- it was born that way (a short morph,
which a colony thirty generations deep can evolve, since the fates table is
heritable) or it was born long and lost cells -- and a standing count of
short bodies cannot tell them apart. `labforage`'s SUMMARY now splits them
(`pile_short_by_genome`, `pile_short_by_loss`, `pile_short_max_gen`).
**Nothing here fixes them**; the counter exists so round 30 starts from a
fact rather than from this note's guess.

## 7. What is not reopened

- The **streak-before-flipping** gate (dead ends; §13g: 16.3% / 18.8%
  blocked on `tunnel` at N=2 / N=3 against 7.6%).
- The **traffic check asked of every animal** (§13g: 77.5% blocked on
  `tunnel`).
- Passability through a nestmate. `climbs_over_kin` grants **footing only**,
  and it is already on `longant.ron`; two chains swapping through each other
  is a different and much harder change, and the census says it is not what
  the long bodies need -- their clumps are 3-4 animals. Filed in
  `Reports/dead-ends.md` with its numbers.

## 8. Mobility, unchanged

`creature_scale mode=walk`, seed 7, 4,000 frames, `RAYON_NUM_THREADS=4`.

**The shipped two-cell ant, this branch against `origin/main` built from the
same worktree in the same session**: `flat` 1.0%, `rolling` 4.4%, `tunnel`
5.4%, `chamber` 9.0% -- and the whole four-preset output is **identical in
every counter**, the only difference in the diff being two `ms` figures.
`examples/ascii` likewise: 31 scenes, and the only non-timing difference in
620 lines of output is a *ratio of timings*. The shipped ant authors no
expiry, so this is identity by construction as well as by measurement.

**The long ant, both arms of the expiry in one binary**
(`PIXEL_PHYSICS_TRAFFIC_DEFER`): `flat` 2.2 / 2.2, `rolling` **5.7 / 5.9**,
`tunnel` 8.4 / 8.4, `chamber` 13.6 / 13.6 (expiry on / off). One row moves
and it moves the *right* way. The mobility scenes carry almost no food, so
`boxed_by_traffic` is hardly ever called there -- `rolling` is the one
preset where an animal picks up dirt and gets a crop, and even there the
expiry costs nothing. **Neither rejected gate's cost is reopened**: those
measured 16.3% and 77.5% blocked on `tunnel` against 7.6%.

## 9. Gates

`cargo test --lib` **1,620 passed / 0 failed / 70 ignored**; `cargo clippy
--all-targets --release --locked -- -D warnings` clean; `cargo run --release
--example ascii` 31 scenes, 0 skipped, rc 0; `bash scripts/worldgencheck.sh`
clean; `bash scripts/docscheck.sh` clean.

**`bash scripts/acceptance.sh` fails two cases on both runs, and both
failures are its *wall-clock frame budget*, not its behaviour**: run 1
`lavadrop` 131.54 ms and `strike` 251.37 ms against a 60.0 ms budget, run 2
`rockdrop` **60.58** ms and `lavadrop` 160.56 ms -- a different pair each
time, which is the signature. The box was at load 16-20 with five lanes
compiling throughout. Neither scene contains a creature and this branch adds
no per-frame work to anything they touch. `CLAUDE.md`: *gate on counters,
never on wall clock*. Re-run on a quiet box before reading anything into it.
