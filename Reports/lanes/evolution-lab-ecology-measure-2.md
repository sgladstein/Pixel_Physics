# Evolution lab — ecology-measure 2, does a ground-level thicket close the loop?

Lane brief (M2): PR #301 built "the seed survives the mouth" and found the
bite-a-fallen-fruit event almost never fires on the owner's played bed
(grass, herb, shrub) — herb's fruit stands 22-40 rows up a stem. `scrambler`
(`assets/species/scrambler.ron`) is a sprawling determinate thicket that
carries flowers and fruit at ground level at every axis end, shipped and
never in the played bed. Question: does planting it make the loop fire, and
does it feed the colony? This is the finding.

---

## 2026-09-10 — fruit reaches the floor 9-10x more; bites go from rare to still-rare; the seedling half never fires at all

### The bed

`assets/lab_scenarios/played_bed_scrambler.ron` — the owner's played bed,
byte-identical except four `scrambler` founders at x=15, 195, 315, 495,
chosen to bracket the file's own nest-founding band (210..310) rather than
sit inside it (the brief's own sketch, x=90/215/300/490, put two of the four
inside the band, which would have cost founders the same way the played
bed's own header warns an explicit `Clear` would). Confirmed live before
measuring anything (`labshot scenario=played_bed_scrambler seed=1
frames=6000,30000`): scramblers grow and fruit at ground level in the box —
standing fruit 79 / flower 81 at frame 6,000 (before the colony lands),
21 / 11 at frame 30,000 (after 24,000 frames of ant grazing pressure). Cost
fork not taken: the thicket establishes and fruits, so the measurement
proceeded.

### The instrument: `windfall_bitten`, and the bug it was caught by, not by review

`plant::seed_survives_bite` had no counter for the bite itself — only for
its four *exits* (`seeds_spilled`, `plants_from_pip`, `pips_rotted`,
`pips_eaten`) and for the ownerless case. Without it "how many times did an
ant bite a fallen fruit" could only be estimated as
`seeds_spilled / seed_gut_survival` — silent at `seed_gut_survival: 0.0` and
rounded to a multiple of the reciprocal for every other species. Added
`World::windfall_bitten`, incremented in `plant.rs` immediately before the
survival roll.

**The naive placement was wrong, and the measurement caught it before
anything was reported.** Counting every call that reached the roll read
**384** bites in one 120,000-frame `played_bed` run against
`seeds_spilled=0` for that same run — arithmetically real, and about the
wrong thing (`CLAUDE.md`'s own "ask what your number counts when nothing is
wrong"). `windfall_material` defaults to the literal string `"seed"` for a
species that authors no fruit, so for grass and shrub `windfall_id` resolves
to the ordinary `seed` material and *every* bare-seed bite reached the same
line — grass and shrub's seed litter vastly outnumbers herb and scrambler's
fruit on this bed, so the naive counter was reporting bare-seed predation,
not fruit predation. Fixed by excluding the no-fruit sentinel at the
increment site, covered by a new test
(`windfall_bitten_counts_real_fruit_and_not_an_ordinary_bare_seed`) with both
arms as positive and negative controls, and watched red without the guard
(384 → the still-wrong 2 in the unit case) before landing it. Re-measuring
`played_bed` seed 1 after the fix: `windfall_bitten=0`, matching
`seeds_spilled=0` for that seed exactly.

### The measurement

Both beds, seeds 1/2/3, 120,000 frames, `sample=60000` (so the per-run
table lands exactly on 0/60,000/120,000), `RAYON_NUM_THREADS=4` pinned, a
private `TMPDIR`. Full logs in this worktree's scratch area (not committed —
measurement output, not source).

**Played bed (grass, herb, shrub — no thicket):**

| | seed 1 | seed 2 | seed 3 | mean/sum |
|---|---|---|---|---|
| plants @60k | 346 | 560 | 389 | 431.7 |
| plants @120k | 149 | 290 | 245 | 228.0 |
| fruit_dropped (cumulative) | 12 | 14 | 11 | 12.3 mean |
| `windfall_bitten` (real fruit, owned) | 0 | 2 | 2 | 4 sum |
| `windfall_bitten_ownerless` | 0 | 0 | 1 | 1 sum |
| **total real-fruit bites** | 0 | 2 | 3 | **5 sum** |
| seeds_spilled | 0 | 2 | 0 | 2 sum |
| plants_from_pip | 0 | 0 | 0 | **0 sum** |
| pips_rotted | 0 | 2 | 0 | 2 sum |
| pips_eaten | 0 | 0 | 0 | 0 sum |
| intake @120k (J) | 647,421 | 500,532 | 575,500 | 574,484 mean |
| born @120k | 271 | 224 | 245 | 246.7 mean |
| alive @120k | 102 | 60 | 101 | 87.7 mean |
| deliveries @120k | 50 | 4,362 | 7,007 | 3,806.3 mean |

**Played bed + four scramblers:**

| | seed 1 | seed 2 | seed 3 | mean/sum |
|---|---|---|---|---|
| plants @60k | 489 | 368 | 543 | 466.7 |
| plants @120k | 332 | 99 | 190 | 207.0 |
| fruit_dropped (cumulative) | 193 | 101 | 61 | 118.3 mean |
| `windfall_bitten` (real fruit, owned) | 3 | 10 | 0 | 13 sum |
| `windfall_bitten_ownerless` | 0 | 19 | 0 | 19 sum |
| **total real-fruit bites** | 3 | 29 | 0 | **32 sum** |
| seeds_spilled | 3 | 5 | 0 | 8 sum |
| plants_from_pip | 0 | 0 | 0 | **0 sum** |
| pips_rotted | 3 | 3 | 0 | 6 sum |
| pips_eaten | 0 | 1 | 0 | 1 sum |
| intake @120k (J) | 361,311 | 678,766 | 939,892 | 660,656 mean |
| born @120k | 144 | 292 | 485 | 307.0 mean |
| alive @120k | 57 | 181 | 144 | 127.3 mean |
| deliveries @120k | 74 | 88 | 6,971 | 2,377.7 mean |

**Conservation check on both tables** (spilled = plants_from_pip + pips_rotted
+ pips_eaten + whatever is still standing as a `pip`, the identity PR #301's
own tests assert): every seed balances exactly, with one pip left standing
uncounted at frame 120,000 on scrambler seed 2 (5 spilled = 0 + 3 + 1 + 1
standing). The bookkeeping is sound; the question is what the numbers say.

`standing windfall` at the three sampled instants (0/60,000/120,000) read
**zero in every single run, both beds.** Not a bug in the new census (it was
built with its own positive/negative control, `census`'s `windfall` field
tested against a hand-planted cell regardless of gut) — a windfall cell's
own residence time is a few hundred frames (per the brief's own framing),
and a 60,000-frame sampling interval misses a phenomenon that short by
construction almost every time. The standing count is not informative at
this cadence; `fruit_dropped` (cumulative) and `windfall_bitten` (cumulative)
are the numbers that carry the finding.

### The verdict

**Fruit reaches the floor far more often — mean cumulative `fruit_dropped`
went from 12.3 to 118.3, about 9.6x.** This is the clean, first-order effect
of a ground-level fruiting habit versus herb's stem-top fruit, and it is not
seed-dependent: every one of the three thicket seeds beat every one of the
three plain-bed seeds.

**Bites go from rare to still-rare, not to "dozens."** Total real-fruit
bites (owned + ownerless) across the three-seed sweep went 5 → 32, roughly
6.4x — a real increase, and one seed-dependent enough that it should not be
read as a rate: scrambler seed 3 landed **zero** bites despite 61 fruit
reaching the ground (more than any plain-bed seed), while scrambler seed 2
landed 29. `seeds_spilled` (a bite that survived the gut) went 2 → 8. The
mechanism visibly strengthens; it does not arrive at the frequency the
brief's "dozens" framing hoped for, at four founders on a 512-wide bed.

**`plants_from_pip` was zero in every one of the six runs — the seedling
half of the loop was never once demonstrated, on either bed, in 720,000
combined frames.** Fruit reaching the floor is up nearly 10x, real bites are
up over 6x, pips that survive a bite are up 4x (2 → 8) — and not one of
those eight surviving pips grew into a plant before either dying to a
second bite (1, scrambler seed 2 only) or rotting out (8 total across both
beds) or, on one seed, still standing unresolved at the 120,000-frame cutoff.
This is the sharpest finding in the round: the mechanism PR #301 built is
demonstrably closer to firing with a thicket in the bed, and the specific
claim "a colony that gardens survives" — a seedling growing from a pip a
colony's own foraging produced — has *never* been observed live, at any
scale this lane tested.

**Colony feeding: no separable signal.** Mean intake, births and survivors
at 120,000 frames all read higher on the thicket bed (660,656 J vs 574,484 J
intake; 127.3 vs 87.7 alive; 307.0 vs 246.7 born), but the per-seed spread
is wide enough on both beds (alive ranges 57-181 on the thicket bed, 60-102
on the plain one) that three seeds cannot tell "the thicket feeds the colony
better" from ordinary seed-to-seed variance — consistent with the trail-
recruitment finding this brief cited (`Reports/lanes/evolution-lab-
coordinator.md`) that no bed tried has been patchy enough to pay through
recruitment, and a fruiting thicket is exactly that shape of patchy food.
Deliveries are noisier still and dominated by one outlier seed on each bed
(50/4,362/7,007 on the plain bed; 74/88/6,971 on the thicket bed) — the same
seed (3) is the outlier on both, which smells like a seed-level property of
the colony's own trail geometry rather than anything the thicket changed.

### What surprised me

**`windfall_bitten_ownerless` reproduces PR #301's own played-bed numbers
exactly (0, 0, 1) but spikes on the thicket bed's seed 2 (19), where every
other run — both beds, all three seeds otherwise — reads 0 or low single
digits.** The brief expected this counter "must be ~0 with #300 in," and it
is, on the bed #300 was measured against. It is not on this one seed of the
new bed. `labshot`'s own felling counters for the thicket bed run
noticeably hotter than the plain bed's (severed-by-support-check 1,596-1,658
against roughly comparable magnitudes on the plain bed, shed and rotted both
higher) — scrambler's sympodial, repeatedly-terminating architecture sheds
and re-roots far more often than herb's single erect stem, and I did not
trace whether that structural churn is what is orphaning windfall cells
faster than #300's fix covers on this species, or whether it is ordinary
seed-to-seed variance in a rare event (one seed out of three at 19, the
other two at 0). Flagging it rather than either dismissing it or building a
fix — that fix, if one is needed, belongs to whoever owns `src/sim/rigid.rs`
and `structural.rs`, not to this measurement round.

**The bite count did not scale with fruit dropped anywhere near linearly.**
Scrambler seed 1 dropped almost twice seed 2's fruit (193 vs 101) and was
bitten a tenth as often (3 vs 29 total). Seed 3 dropped more than seed 1
(61 vs 12 on the corresponding plain-bed seed, or against its own bed's
other seeds) and was bitten zero times. Whatever determines whether a
colony's foraging path crosses a given thicket's fruit is not captured by
"how much fruit exists" at all — it is about where the ants' trails happen
to run, which four founders at fixed columns cannot control for.

### What I could not separate

- **Whether the thicket feeds the colony better.** Covered above — the
  three-seed spread on both beds is wider than the mean difference between
  them. A real answer needs more seeds than this brief's budget, not a
  different metric.
- **The `windfall_bitten_ownerless` spike's cause.** Sized it (one seed at
  19, everything else at 0-1) and named the two live candidates (species-
  specific structural churn vs. ordinary rare-event variance) without
  tracing which. Flagged with the reproduction (`played_bed_scrambler seed=2
  frames=120000`) rather than left as a bare number.
- **Whether four founders is the right count to answer "should this be in
  the default bed."** This round tested one placement (four, bracketing the
  nest band). Whether two would already show the same 9.6x fruit-drop effect,
  or eight would meaningfully change the bite rate, is untested.

## Files touched

- `src/sim/plant.rs` — `windfall_bitten`, incremented in
  `seed_survives_bite` right before the survival roll, gated to real fruit
  only (excludes the `"seed"` no-fruit sentinel); one new guard test with a
  positive and a negative control.
- `src/sim/world.rs` — the `windfall_bitten: u64` field and its doc.
- `examples/labforage.rs` — `Sample::plants` (live plant-organism count,
  the plant-side twin of `ants`) and `Sample::windfall` (raw standing
  windfall-material count, independent of the census gut filter); both
  printed per-sample and on the `SUMMARY` line, alongside `fruit_dropped`
  and `windfall_bitten`; `census()`'s two new fields covered by the existing
  `control=selftest` positive/negative controls.
- `assets/lab_scenarios/played_bed_scrambler.ron` — new scenario, the played
  bed plus four `scrambler` founders.

None of `src/sim/creature.rs`, `src/sim/organism.rs` or `src/sim/rigid.rs`
were touched. The `windfall_bitten_ownerless` spike above is a finding for
whoever owns the latter two, not a fix attempted here.
