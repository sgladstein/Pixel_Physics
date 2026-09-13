# Round 28 — the garden loop fires, and where it dies

*Lane C, `claude/lab-garden-loop-r28`. A finding, not a status: measured
`labforage scenario=played_bed`, seeds 1-3, 120,000 frames,
`RAYON_NUM_THREADS=4`, one binary, both hypotheses tested inside the same
runs.*

## What was asked

Over 360,000 frames across rounds 26-27, the fruit → animal → nest →
seedling chain (`Reports/evolution-lab-ecology-design-2026-09-10.md` §2)
fired a handful of times and `plants_from_pip` stayed at 0 in every arm.
Two named hypotheses: **(a)** the crop is set down somewhere a pip
structurally cannot germinate (light or soil water below the species'
`Germinate` threshold); **(b)** windfall lands where the colony's own foot
traffic never reaches, so the bite that starts the whole chain rarely
happens at all. Fix the dominant one, once, in the smallest change; if the
loop needs a new mechanism instead, write the finding and stop.

**The numbers choose (b), and (a) turns out to be untestable with the data
this round produced: no pip, in any of the three seeds, ever reached even
its first scheduled Germinate check.** A third, unasked-for finding sits
downstream of both: the one pip that completed the whole chain — bitten,
survived the gut roll, carried, delivered — was eaten again by the colony's
own traffic five frames after it was set down, before anything about light
or soil could matter. No mechanism is proposed this round; the reasoning for
that is in "Why nothing was built," below.

## The instrument

All of it is read-only bookkeeping — no `rng.chance()` call, no decision
branch — so it changes nothing about how the simulation behaves; the
numbers below are what round 27's own mechanism has been doing all along,
now visible.

- **`organism::PipCheck`** (`src/sim/organism.rs`) and `World::pip_checks`
  (`src/sim/world.rs`): one row per pip, taken at its *first* `Behavior::
  Germinate` evaluation only (`src/sim/plant.rs`'s Germinate arm), reading
  `light`/`soil_water` off the exact site the mechanism itself reads them
  from — not reconstructed after the fact. Carries `delivered` (A2 vs A1,
  via a new `OrganismState::pip_delivered` flag set in
  `plant::deliver_seed_passenger`) and `overburden` (`plant::
  overburden_depth`, a diagnostic-only "how buried" proxy — non-empty cells
  above before open sky, capped at 64).
- **`World::pip_rot_x` / `World::pip_eaten_x`**: the column of every pip
  that lost the viability race or was bitten a second time, at all four
  exit sites (`decay.rs`'s material channel, `plant.rs`'s two half-life
  rolls, `plant::seed_survives_bite`'s standing-pip branch).
- **`examples/labforage.rs`**: a `Garden` accumulator riding the census's
  existing per-stop grid sweep (not a second one) — `windfall_heat`/
  `ant_heat` per column, standing-windfall height/distance bands, and a
  light-based (not scenario-column-hardcoded) shaded/open split. Extended
  `census`/`mark_visited` in place; `control=selftest` carries a positive
  control over every new band (`garden0`/`garden1`/`garden2`/`garden3`).
  New SUMMARY fields are appended after `main`'s own, per the round's own
  environment note on that line's contention.
- **`examples/labgif.rs`**: `mark=1` rings the exact world cell `center=`
  named (composes with `crop=` for free, drawn before it); `png_dir=`
  writes every captured frame as its own PNG beside the GIF, because a
  one-cell event needs marking and scrubbing, not a still
  (`.claude/skills/review/SKILL.md`); `PIP_PROBE=1`/`PIP_TRACE=<id>` are
  free-when-unset debug hooks, same convention as `A2_DEBUG`/`WF_DEBUG`,
  that follow one pip's own organism handle rather than a fixed coordinate
  — needed because `pip` is a `Powder` and a freshly delivered one dropped
  into an elevated empty cell keeps falling for a few ticks before it
  rests, which a fixed-coordinate probe cannot tell apart from "eaten".

## Table 1 — the pip's own exits, three seeds

| seed | windfall_bitten | seeds_spilled | seeds_carried | seeds_delivered | plants_from_pip | pips_rotted | pips_eaten | pip_checks | windfall_bitten_ownerless |
|---|---|---|---|---|---|---|---|---|---|
| 1 | 2 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 1 |
| 2 | 9 | 9 | 9 | 6 | 0 | 0 | 6 | 0 | 0 |
| 3 | 1 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 4 |

**`plants_from_pip` is 0 on all three seeds, and `pip_checks` — the "did
this pip ever live long enough to be evaluated at all" counter — is 0 on
all three too.** Seed 1 and seed 3's bites both failed the gut-survival
roll (`seed_gut_survival` defaults to 0.6; 2 and 1 rolls respectively is not
enough to expect a pass), so no pip ever existed on those seeds to test
anything with. Seed 2 is the only seed where the whole A1→A2 chain ran to
completion, six times, and reconciles exactly: 9 spilled, all 9 taken as
passengers (0 left standing as pure A1), 6 delivered, all 6 of those
re-bitten (`pips_eaten`), the remaining 3 still riding in a live ant's crop
at frame 120,000 — not delivered, not rotted, not eaten, not germinated,
because they never left the mouth carrying them.

**Positive control, before trusting the zero**: `a_pip_germinates_through_
the_ordinary_seed_path` (`src/sim/plant.rs`) plants a pip on wet soil under
open sky with no ticking beforehand and confirms it germinates within
budget and `plants_from_pip` moves. The mechanism can fire; on the played
bed, across three seeds, it never got the chance to.

## Table 2 — hypothesis (a): first Germinate check, by seed

| seed | pip_checks | resting | light≥threshold | soil_water≥threshold | would-germinate-now |
|---|---|---|---|---|---|
| 1 | 0 | — | — | — | — |
| 2 | 0 | — | — | — | — |
| 3 | 0 | — | — | — | — |

**Hypothesis (a) is not confirmed and not refuted — it is untested.** Zero
pips ever reached a first check, on any seed, so there is no row to read
light or soil water off. Building a midden (setting the pip down at the
nest mouth, in the light) would be aimed at a failure mode nothing in this
round's data shows exists. The one pip whose fate is known in detail (seed
2, below) died five frames after settling — long before `ORGANISM_TICK_
INTERVAL` would have scheduled its first Germinate evaluation at all — so
even that single case cannot speak to (a) either.

## Table 3 — hypothesis (b): windfall landing vs the colony's own foot traffic

| seed | windfall column-stops | in a column ants never walked | % | standing-windfall bands (floor/low/aloft) | under canopy (<half open-sky light) |
|---|---|---|---|---|---|
| 1 | 12 | 11 | 92% | 4 / 6 / 2 | 8 of 12 |
| 2 | 3 | 2 | 67% | 0 / 1 / 2 | 1 of 3 |
| 3 | 4 | 4 | 100% | 1 / 3 / 0 | 4 of 4 |

**A majority of standing windfall sits in a column the colony's own body
never occupied for the whole 120,000-frame run, on all three seeds.** The
32-column heat-map bands make the shape unmistakable and identical across
all three seeds — fruit and ant traffic occupy disjoint bands:

| col band starts at | seed 1 windfall / ant-frames | seed 2 windfall / ant-frames | seed 3 windfall / ant-frames |
|---|---|---|---|
| 32 | 5 / 0 | 0 / 0 | 0 / 0 |
| 64 | 1 / 0 | 0 / 0 | 0 / 0 |
| 96 | 2 / 0 | 0 / 0 | 1 / 0 |
| 160-320 (the colony's own band) | 0 / **62,802 - 2,928,879** | 0 / — (see below) | 0 / **884,393 - 3,577,144** |
| 352 | 1 / 1,104,051 | 1 / — | 1 / 0 |

(Seed 2's full band table is in its own run log; the shape is the same —
windfall at columns 384-448, zero ant-frames there, and the colony's own
traffic peaked at columns 288-352 with over 2.2M and 3.7M body-frames.)

**This is not a new problem — it is `played_bed.ron`'s own, already-priced
trade.** The scene file's own comments record that the four scramblers
(the low fruiting thicket, at columns 15, 118, 395, 490) were deliberately
kept *out* of columns 180-330 because a scrambler there cost the colony
founders: 2 of 52 seated near a scrambler against 31 on the bare gap. The
owner took that trade for founder count. Moving fruit back into the
colony's home band to fix the bite rate would spend the same ground the
owner already spent on founding, in the other direction — not a smallest
change, a reopened decision.

## The one completed chain, frame by frame (seed 2)

`labgif`'s `PIP_TRACE=13116` (the delivered pip's own organism id, from
`A2_DEBUG`), every frame, `seed=2 rain=off`:

| frame | cell(s) owned |
|---|---|
| 55,900 – 55,931 | `[]` (riding in a crop, no cell in the grid) |
| 55,932 | `[(342, 154)]` — **delivered** |
| 55,933 | `[(342, 154)]` |
| 55,934 | `[(342, 155)]` — falling: `pip` is a `Powder` |
| 55,935 – 55,936 | `[(342, 156)]` — settled, briefly |
| 55,937 | `[]` — **gone**; `World::pips_eaten` moved this same frame, at column 342, matching `pip_rot_x`'s sibling list `[342, 349, 337, 338, 363, 364]` |
| 55,938 – 55,958 | `[]` |
| 55,959 | organism slot reclaimed |

Confirmed against the rendered pixels, not only the counters
(`examples/pixelcheck_scratch`-style spot check, thrown away after use):
the cell reads the `pip` palette (`[218,207,171]`, `[212,202,166]`,
`[189,180,148]` across the three rows it occupied — all inside `pip.ron`'s
declared range) while it stands, and the same dark background colour
(`[24,27,33]`) both before delivery and after it is eaten — the site was
open air the whole time, not soil, which is why nothing but the pip itself
was ever visible there.

**Five frames from set-down to gone.** `ORGANISM_TICK_INTERVAL`-scale
scheduling means this pip almost certainly never reached a Germinate
check before it died — consistent with `pip_checks` reading 0 across all
three seeds.

## Which hypothesis the numbers chose

**(b), with a caveat the brief did not anticipate.** The dominant,
measured cause of `plants_from_pip == 0` is that almost nothing bites
fruit at all (`windfall_bitten`: 2, 9, 1 across three seeds, matching
round 27's own "single digits" finding almost exactly), and the reason is
spatial: fruit stands where the colony does not walk, on 67-100% of
column-stops across all three seeds. That upstream cause is a real
mismatch between the fruiting understory's position and the colony's own
foraging/founding range — but the honest fix (move fruit into the colony's
home band) directly reopens `played_bed.ron`'s own prior, measured,
deliberate trade (founder count), so it is not a small scene change, it is
a re-litigation of a decision the owner already made on a different card.

The **third finding** — the one pip that got all the way to delivery was
consumed by the colony's own traffic five frames later, before hypothesis
(a)'s light/soil question could even be asked — means a midden (setting
the pip down at the nest mouth, in the light) is not warranted either: it
answers a failure mode (bad light or soil at the drop site) that has zero
supporting rows in `pip_checks`, and would do nothing about a pip being
re-bitten by a passing forager regardless of where it is set down.

## Why nothing was built

Per the cost fork: build the fix, or write the finding and stop — never a
half-built one. Both named remedies fail their own precondition here:

- **The midden (hypothesis a's fix)** is scoped for "a pip fails to
  germinate because its drop site is too dark or too dry." `pip_checks`
  is 0 on every seed — no pip has ever failed that test, because none has
  ever taken it. Building the fix would be aimed at a cause this round's
  data does not show exists.
- **A scene edit (hypothesis b's fix)** is licensed by the brief only "if
  a fruiting plant beside the colony is still the bed the owner plants" —
  and `played_bed.ron`'s own comments say the answer, already given on a
  separate card, is no: fruit was moved *out* of the colony's band on
  measured founder-count evidence. Undoing that trade to fix the bite rate
  is not this lane's smallest-change call to make.
- **The predation finding** (a delivered pip eaten by the colony's own
  traffic) has one data point. It is real — traced by organism handle,
  frame by frame, confirmed against rendered pixel colour — but a fix
  aimed at an n=1 observation is a guess wearing a mechanism, and CLAUDE.md
  is explicit that a clean, tidy-looking result from a single seed is
  reason for suspicion, not confidence.

So: the instrument is real and generalises (any future round asking "did a
pip survive to be checked" or "where does X land against where ants walk"
has it for free), the finding is precise and reproducible, and no
mechanism changes hands this round.

## Colony health (context, not a comparison)

No behaviour changed, so there is no before/after arm to report — the
numbers below are simply what these three runs looked like, for whoever
picks up hypothesis (b) next:

| seed | cols walked | eats | born | died | alive | deliveries | nest_visits |
|---|---|---|---|---|---|---|---|
| 1 | 258 | 3,399 | 80 | 70 | 48 | 68 | 675 |
| 2 | 284 | 13,923 | 157 | 119 | 69 | 3,958 | 4,643 |
| 3 | 202 | 15,800 | 133 | 51 | 97 | 5,167 | 7,929 |

## The card

`20260911T034404498Z-49ba0a` — a 21-frame scrubbable sequence, seed 2,
frames 55,925-55,945, the pip's own cell ringed in magenta. Question: does
the ringed cell reading pale-tan then vanishing five frames later read as
"eaten before it could grow." `meta` carries `plants_from_pip` (0, three
seeds), `seeds_delivered`/`pips_eaten` for this seed (6/6), `pip_checks`
(0), frames from set-down to gone (5), and the windfall dead-zone
percentages (92/67/100).

## What contradicts the brief

- Hypothesis (a)'s own framing ("the crop is set down INSIDE the nest,
  underground") does not match the code: `paint_nest_patch`
  (`src/sim/creature.rs`) lays `nest` as a *surface* material, replacing
  the topmost solid/powder cell per column — there is no underground
  chamber for a pip to be buried in. The functional concern the brief was
  reaching for (a pip delivered somewhere unsurvivable) is real, just not
  for the reason named — see the predation finding above.
- The card's suggested title ("A plant from a pip at the nest door") and
  its "from set-down to sprout" framing assume a sprout exists to show.
  None does, on any seed measured. The card posted instead shows the
  actual, honest outcome.

## What was left undone, and why

- **`src/sim/creature.rs`, the crop set-down site** — untouched. Hypothesis
  (a) is unsupported, so there was nothing to fix there this round.
- **`assets/lab_scenarios/played_bed.ron`** — untouched, per the cost
  fork's own condition (see "Why nothing was built").
- **`assets/materials/pip.ron`** — untouched; nothing measured this round
  implicates the material's own physics (`friction_angle`, `decay_chance_*`)
  over the delivery-site predation finding.
- **The predation finding is n=1.** A future round with a larger `seed_gut_
  survival` ablation (or more seeds) could turn it into a real rate rather
  than one traced organism — flagged here rather than acted on.
