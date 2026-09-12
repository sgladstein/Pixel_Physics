# Lane: ants die of age (late-game brief 2)

Branch `claude/lab-lifespan-r29`. Design of record:
[`../evolution-lab-late-game-design-2026-09-12.md`](../evolution-lab-late-game-design-2026-09-12.md)
§1.2 and §2 brief 2.

## 2026-09-12 — the interval had to become an argument

`plant::old_age_chance` baked `ORGANISM_TICK_INTERVAL` (45) into the
per-tick chance; an ant rolls every 6. Called unchanged it would have put an
ant's median at **T/2.7** *and made `pace` a lifespan gene* — a heritable
trait silently deciding how long its lineage lives. The repair is
`plant::old_age_chance_over(age, T, interval)`, with the plant's own function
a wrapper on it, and
`the_hazard_is_a_property_of_the_half_life_not_the_interval` asserting the
three survivorship numbers at intervals 6, 12 and 45. **Do not re-derive
this**: the chance is per *frame* and the interval multiplies it, so any new
caller at a new cadence must pass its own. The interval used is the
*individual's* (`organism_tick_interval`), which already reads the `pace`
allele and the leg fraction.

**A corrected number came out of it.** The hazard's survival at `2.5T` was
documented as `0.4%` in `plant.rs`, in the design report §1.2 and in two test
comments. `exp(-ln2 · 2.5²)` is **1.31%**; `0.4%` is `2^-8`, the survival at
`2.83T`. No bar depended on it (every one is `< 0.02`), so the model is
unchanged and only the label is — but do not quote 0.4% again.

## The controls, quoted

**Positive control** — `latecensus scenario=played_bed seed=1 frames=24000
sample=1500`, `lifespan=6000` against `lifespan=0`, same binary. The bed
seats **36** founders on the timeline at frame 6,000, so a founder's age is
`frame − 6,000`.

| founder age | frame | L6000 ants / cumulative OLDAGE | L0 ants / OLDAGE |
|---|---|---|---|
| 0 | 6,000 | 36 / 0 | 36 / 0 |
| T/4 | 7,500 | 35 / **1** (97.2% survived; model 96%) | 36 / **0** |
| T | 12,000 | 7 / **17** (47% dead of age; model 50%) | 14 / **0** |
| 2.5T | 21,000 | 1 / **23** | 21 / **0** |

`SUMMARY` at 24,000: L6000 `oldage=24 starved=13 born=1`; L0 `oldage=0
starved=23 born=12`. **The fault put back is the L0 arm**: same seed, same
bed, `OLDAGE` 0 at every one of seventeen stops.

**Determinism arm** — `lifespan=0`, 60,000 frames, seed 1,
`RAYON_NUM_THREADS=1`: 21 / 43 / 47 ants and 158 / 232 / 214 plants at 20k /
40k / 60k, `oldage=0`. **Note the published census in the design report §0 is
NOT a valid baseline for this** — it was taken at `be2808de` and `main` has
moved a long way since (it reads 20 / 182 / 855 at 20k against this arm's 21
/ 158 / 672, and that gap is other lanes' landed work). The baseline has to
be built from `origin/main` itself.

**`ascii` is green and is NOT byte-identical**, and that is expected rather
than a surprise: four scenes found colonies from `ant.ron`, which now carries
`life_half_life: 40000`. At that half-life survival is 99.8% at 2,000 frames,
97.3% at 8,000 and **93.9% at 12,000**, so the long ant scenes lose a few
percent of their cohort to age. Every assertion still passes — `31 scenes
run, 0 skipped`, exit 0 — which is what CI gates.

## `labstats`, the paired arms

`labstats frames=120000 seed=1`, its own default bed (**`labstats` has no
`scenario=` knob** — passing one is silently ignored, so these are not
played-bed figures):

| | lifespan 0 | lifespan 40,000 |
|---|---|---|
| ants alive | 18 | **92** |
| starved / old age / killed | 137 / 0 / 2 | **63** / 50 / 1 |
| born / died | 107 / 139 | 164 / 114 |
| plants standing / seed bank | 34 / 85 | **101 / 166** |
| seeds borne / sprouted | 3,086 / 524 | **5,397 / 1,101** |
| animal generations | 5 | **15** |
| shares | 4,555 | 1,866 |

**The colony with a lifespan is bigger, not smaller**, and the bed under it
is three times greener. The mechanism is the one the design predicted read
from the other end: age deaths thin the colony *before* it eats the bed bare,
so the stand keeps producing and the crash never arrives. The bar this lane
was handed (alive 69 / born 94 / died 77 / generations 10 / shares 1,796)
does **not** reproduce on this binary's `lifespan=0` arm — that arm reads 18
/ 107 / 139 / 5 / 4,555 — so it was measured somewhere else and the paired
arm above is the baseline to use.

## The sweep

`latecensus scenario=played_bed frames=500000 sample=20000`,
`RAYON_NUM_THREADS=1`, lifespan 0 and 40,000 on seeds 3, 1, 2, then 20,000
and 80,000 on seed 3.

(pending — filled in when the runs land)

## What not to re-derive

- **`life_half_life` is `u32` frames on `CreatureDef`, 0 = immortal.** Every
  species file that does not name it is bit-identical, which is the whole
  reason the default is 0 rather than a large number.
- **It is not heritable, and that is a ruling** (design §4: priced before
  inherited). A trait slot widens the genome and shifts every seeded draw.
- **`DeathCause::OldAge` is appended to `DEATH_CAUSE_LIST`, not inserted.**
  `World::deaths_by_cause` and `GroupDeaths::by_cause` are positional arrays.
- **The dial is on the ANTS page under `tick_interval`**, not on GENOME —
  GENOME is titled "what a lineage inherits" and this is not inherited.
- **`lifespan=` is accepted by `latecensus`, `labstats`, `labforage` and
  `labgif`**, all four echoing the value whether or not it was passed.
- **The cohort test has to zero eight charges, not two.** `idle_cost_per_cell`
  and `move_cost_per_cell` are the obvious ones; the brain, eye, ground
  sense, jaw and shell are each levied per tick as a fraction of
  `start_energy`, and leaving them on puts starvation deaths in the column a
  survival curve reads.
