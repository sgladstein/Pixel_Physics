# Lane: ants die of age (late-game brief 2)

Branch `claude/lab-lifespan-r29`. Design of record:
[`../evolution-lab-late-game-design-2026-09-12.md`](../evolution-lab-late-game-design-2026-09-12.md)
§1.2 and §2 brief 2. Shipped on, authored per species, **not heritable**.

## The interval had to become an argument

`plant::old_age_chance` baked `ORGANISM_TICK_INTERVAL` (45) into the per-tick
chance; an ant rolls every 6. Called unchanged it would have put an ant's median
at **T/2.7** *and made `pace` a lifespan gene* — a heritable trait silently
deciding how long its lineage lives. The repair is
`plant::old_age_chance_over(age, T, interval)`, the plant's own function a
wrapper on it, and `the_hazard_is_a_property_of_the_half_life_not_the_interval`
asserting the three survivorship numbers at intervals 6, 12 and 45. **Do not
re-derive this**: the chance is per *frame* and the interval multiplies it, so
any new caller at a new cadence must pass its own — the *individual's*
(`organism_tick_interval`), which already reads `pace`.

**A corrected number came out of it.** Survival at `2.5T` was documented as
`0.4%` in `plant.rs`, in design §1.2 and in two test comments.
`exp(-ln2 · 2.5²)` is **1.31%**; `0.4%` is `2^-8`, the survival at `2.83T`. No
bar depended on it (all are `< 0.02`), so the model is unchanged and only the
label is. **The design report still says 0.4%** — not this lane's to edit.

## Scent drift moved the baseline out from under this whole sweep

**Read this before any number below it.** The sweep was measured against `main`
at `f3acaf76`. `main` then landed nest odour with `scent_drift` on at 0.15
(PR #347 and its neighbours). Re-running the **identical control arm** across
that merge — same binary, same seed, same scenario, only `main`'s landed code
between the two runs — gives a different bed:

```
before, life 0       73  205  339  354  752 1816 2013 1023  943 2079 3182 3099   16   25    6   46  458 1283  848  289  108
after,  life 0       73  236  334  254  274  420  676  705  726  760  126  208  129   94  125    2    0    0    0    0    0
after,  life 40,000  27  243  515  161   11    6    7   64  507 1265  733  653  799  579  459  367  141   73    0    0    0
```

The runaway is gone: the control peaks at **760** instead of 3,182 and is
**extinct by 420,000 frames** instead of holding 108.

**Only half the runs moved, and that is the useful half of this.** Of the six
paired runs, three are **byte-identical** across the merge — seed 2 at both
settings and seed 3 immortal — and three change completely: both seed-1 arms and
seed 3 with a lifespan. Main's change is inert on the small colonies and decisive
on the large ones, so a lane measuring a quiet bed will find nothing has moved
while a lane measuring a busy one finds everything has.

The changed arms are also *identical to 100,000 frames* (21/43/47/57/73 on both
trees), so this is slow-accumulating and not a broken build. The 60,000-frame
determinism control could not have caught it, and that is the lesson: **a
baseline control shorter than the mechanism's onset proves nothing about the run
you are about to sweep.**

## The sweep, re-measured at `main` = `c7ee0f40` (one trunk behind what ships)

**The branch ships over `39b31c7e`** — seed cargo, the drained door and the
long-ant pile all land after these rows were taken, and the round's queue had
no room for another two-hour sweep before landing. Ships under the owner's
ruling that the mechanism and the dial are the deliverable and the constant is
re-derivable from the ANTS page. Re-deriving it is six runs of `latecensus
scenario=played_bed frames=500000 sample=20000` at `RAYON_NUM_THREADS=1`.

| seed | life | peak | at frame | 500k a/p/bank | starved | killed | old age |
|---|---|---|---|---|---|---|---|
| 1 | 0 | 760 | 280,000 | 0 / 14 / 0 | 3,437 | 0 | 0 |
| 1 | 40,000 | **1,265** | 280,000 | 0 / **114 / 600** | 7,245 | 0 | **1,715** |
| 2 | 0 | 77 | 60,000 | 0 / 102 / 368 | 122 | 3 | 0 |
| 2 | 40,000 | 34 | 100,000 | 0 / 103 / 239 | 70 | 1 | 100 |
| 3 | 0 | 190 | 140,000 | 0 / 2 / 0 | 446 | 0 | 0 |
| 3 | 40,000 | 171 | 140,000 | 0 / 2 / 0 | 348 | 1 | 303 |

**`killed` is its own column and reads near zero here, which dates these runs.**
The seed-cargo build (#342) introduces a `Killed` channel that lane J measures
at 56/37/153 per 120,000 frames on this bed against 0/1/0 without it. These
numbers were taken on `c7ee0f40`, *before* #342 landed, so the near-zero killed
column is the tell that they predate it rather than evidence the channel is
quiet. On a post-#342 tree the lifespan's effect is measured with that channel
present and the two must stay separable.

**The lifespan does not bound the peak here — on the one seed with a big colony
it raises it, 760 to 1,265 — and every colony at every setting, 0 included, is
extinct by 500,000 frames.** The programme's bar is met by nothing.

What it does change is what the colony *leaves*: seed 1 ends with **114 plants
over a bank of 600** against 14 plants and an empty bank, the only arm to clear
the bar's bank half and the same direction `labstats` shows. And the deaths move
as designed — 1,715 of seed 1's 8,960 deaths are age, not hunger.

**The 20,000 and 80,000 arms have not been re-run** on this trunk; their rows
below are from `f3acaf76`. The cost fork's conclusion (halving flattened no
further) is therefore not re-established either.

## The sweep, as measured at `f3acaf76` (superseded)

`latecensus scenario=played_bed frames=500000 sample=20000`,
`RAYON_NUM_THREADS=1`. Peak ants and the frame it fell on, then
ants/plants/bank at 500,000, then old-age against starvation deaths:

| seed | life | peak | at frame | 500k a/p/bank | OLDAGE / starved |
|---|---|---|---|---|---|
| 1 | 0 | **3,182** | 300,000 | 108 / 58 / 39 | 0 / 13,897 |
| 1 | 40,000 | **483** | 320,000 | **256** / 36 / 73 | 1,379 / 4,575 |
| 2 | 0 | 77 | 60,000 | 0 / 102 / 368 | 0 / 122 |
| 2 | 40,000 | 34 | 100,000 | 0 / 103 / 239 | 100 / 70 |
| 3 | 0 | 190 | 140,000 | 0 / 2 / 0 | 0 / 446 |
| 3 | 20,000 | 155 | 140,000 | 0 / 2 / 0 | 612 / 250 |
| 3 | 40,000 | 171 | 140,000 | 0 / 3 / 5 | 299 / 364 |
| 3 | 80,000 | 332 | 160,000 | 0 / 50 / 107 | 192 / 749 |

**On this trunk the runaway was bounded 6.6x** (3,182 → 483) and the arm held a
152–483 band for 280,000 frames against a control swinging 6 to 3,182. **Both
claims are withdrawn** — neither reproduces after the merge; see above. The cost
fork was followed here — halved to 20,000, which flattened no further (155) and
merely killed more (612 of 864 deaths) — so **40,000 ships**, and that
conclusion has not been re-established on the trunk either.

**Card `20260912T092226496Z-07d3ab`** (blind, board `lab`) puts the two whole
sessions side by side as paired 41-frame sequences — `labgif … frames=500000
every=12500 rain=off`, peak/end/old-age counts in `meta`. Frame sequences and
not GIFs on the skill's own head-to-head evidence. Collect with
`review.py inbox`.

**The bar is not met at any setting, 0 included.** No arm ends with a live
colony over a bank above ~500. Seed 1 at 40,000 is the only arm alive at
500,000 frames at all (256 ants), and its bank is 73. The bank is what the mouth
takes first (design §1.1) — **brief 1's mechanism, not this one's**.

**The boom seed has moved, and design §0 is not a usable baseline.** §0 was
taken at `be2808de`; on today's `main` seed **1** is the runaway (3,182 ants
against §0's 116) and seed 3 is not (190 against §0's 495). Rebuild any baseline
from `origin/main` itself.

**Do not read the two arms at one frame — they are at different phases.** At
frame 200,000 on seed 1 the *control* bed is three times greener than the
lifespan bed (174 plants against 51), which reads as the mechanism making things
worse and is the opposite of what `labstats` says. Both are true and neither is
the outcome: the lifespan arm reaches its peak at ~120,000 frames and has
already grazed the stand, while the control is still climbing and has not
cliffed yet (it crashes to 58 plants by 500,000). This is `CLAUDE.md`'s *a
cascade censused before it settles* on the population rather than on rubble —
**compare at 500,000, or compare the whole curve, never at one stop.**

## The controls, quoted

**Positive control** — 36 founders at `life_half_life: 6000`, `latecensus
frames=24000 sample=1500`: 35 of 36 alive at `T/4` (97.2%, model 96%), 7 at `T`
with 17 dead of age (47%, model 50%), 1 at `2.5T`. The **fault put back** is the
paired `lifespan=0` arm: same seed, same bed, `OLDAGE` 0 at all seventeen stops.

**Determinism** — `lifespan=0` against a binary built from `origin/main` in a
separate checkout, seed 1, 60,000 frames: **every quantity both binaries report
is identical**. Literal byte-identity is impossible by construction — the build
*adds* `oldag` and `crpss` columns, an echo line and three SUMMARY fields — so
the comparison drops those and demands the rest match. It is not blind: fed the
`lifespan=40000` arm it refuses at the first non-zero `oldag`.

**`ascii` is green and NOT byte-identical**, and only **two** scenes changed
behaviour (an earlier note here said four): `ants: the foraging loop` — 22 → 15
creatures at 12,000 frames, deaths 7 → 9, births 11 → 6 — and `ants: deposition
follows the moisture gradient`, deaths 52 → 53. Expected age deaths among ~20
ants are ~1.2 and 2 are seen; the lost births are the knock-on of the lost
workers. A third scene (`M17`) differs only in a ratio of two wall-clock timings.

## `labstats`, the paired arms

`labstats frames=120000 seed=1`, full table in README's *Lifespan status*.
**`labstats` has no `scenario=` knob** — passing one is silently ignored, so
those are its own bed, not played_bed. Lifespan 0 → 40,000: ants alive **18 →
92**, starved 137 → 63, plants standing 34 → 101, bank 85 → 166, seeds borne
3,086 → 5,397, shares 4,555 → 1,866.

**The colony with a lifespan is bigger, not smaller**, and the bed under it is
three times greener: age deaths thin it *before* it eats the bed bare, so the
stand keeps producing and the crash never arrives. Read the generation row
exactly — the deepest line *standing* goes 5 → 15, the deepest ever **reached**
is 15 in both. The bar this lane was handed (69 / 94 / 77 / 10 / 1,796) does not
reproduce on this binary's `lifespan=0` arm; it was measured on an older tree.

## What not to re-derive

- **`life_half_life` is `u32` frames on `CreatureDef`, 0 = immortal.** A species
  file that does not name it is bit-identical; that is why the default is 0.
- **Not heritable, and that is a ruling** (design §4: priced before inherited).
- **`DeathCause::OldAge` is appended to `DEATH_CAUSE_LIST`, not inserted.**
  `World::deaths_by_cause` and `GroupDeaths::by_cause` are positional arrays.
- **The dial is on the ANTS page under `tick_interval`**, not GENOME — GENOME is
  "what a lineage inherits" and this is not. `src/lab/ui.rs` diff is zero lines.
- **`lifespan=` is accepted by `latecensus`, `labstats`, `labforage` and
  `labgif`**, all four echoing the value whether or not it was passed.
- **`labgif` gained `delay=<ms>`.** Its delay was `every * 1000 / 60` — real
  playback speed, right for the rain cards it was built for and useless for a
  session time-lapse, where `every≈1,700` means 28 s *per frame*. Unset
  reproduces every existing card byte-for-byte.
- **`labgif` defaults to `rain=steady` and `latecensus` has no rain at all, so
  a card paired against a census must pass `rain=off`.** Cost an afternoon's
  render: at the default the seed-1 colony peaks at **42 ants and is extinct by
  frame 250,000** (1,319 starved), where the census on the same scenario, seed
  and lifespan has it at 2,013 and climbing. With `rain=off` the two agree to
  the ant — 36 founders at frame 6,000, 21 at 20,000. The card was not showing a
  weaker version of the boom; it was showing a different world.
- **The cohort test has to zero eight charges, not two.** Idle and move are the
  obvious ones; brain, eye, ground sense, jaw and shell are each levied per tick
  as a fraction of `start_energy`, and leaving them on puts starvation deaths in
  the column a survival curve reads.
