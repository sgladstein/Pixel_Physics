# Re-deriving `FATE_MUTATION_CHANCE`: the shipped rate is not a small effect, it is no effect

**Status: measurement, 4 world seeds x 7 rates x 2 species, 35 runs, plus
five controls and one blind A/B with the owner.** Run to answer the question
`Reports/plant-fate-fallback-fork-2026-08-30.md` §7 leaves open — *"that
`FATE_MUTATION_CHANCE = 0.01` is right"* is not established — now that
`FateLookup::GenomeOnly` ships and the operators the old per-query fallback
suppressed are real.

```
PIXEL_PHYSICS_FATE_MUTATION_CHANCE=<rate> \
  ./target/release/examples/genome_drift species=<herb|tree> founders=8 \
  frames=<20000|60000> every=5000 worldseed=<1..3>      # omit for a 4th, default bed
```

## 0. The answer, and the one number that decides it

**Recommendation: `FATE_MUTATION_CHANCE = 0.30`, up from 0.01.**

The measurement that forces a change is not a trade-off curve. It is this, and
it is a **full-log diff** rather than a summary statistic:

> At the shipped 0.01, over 60,000 frames of `herb`, **every line of
> `genome_drift`'s output is identical to the same world with the rate set to
> zero** — same 873 live organisms, same 74 establishers, same 309
> germinations, same 5,858 births, same body sizes, same per-slot population
> means and spreads at all six samples. The only lines that differ are the
> genome census and the mutation counters themselves.

45 mutations fired in that run. 28 distinct individuals ever carried a changed
production rule. **None of the 28 ever reached 20 cells**, and the population
standing at the end held **zero** plants with a body and a drifted rule. The
mechanism is running and it is not connected to anything: what the world does
at 0.01 is what it does at 0.

So the choice is not "is 0.01 a bit low". It is "the growth program does not
in fact mutate, and at what rate does it start to".

**The trade this was expected to balance against turns out to be almost
empty, and that is the study's second finding.** The task named four
quantities that trade against each other. Three of them — establishment,
throughput, and whether the stand still stands — **never move**, at any rate
up to and including **1.0, where every single birth mutates**. Median across
three seeds, each against its own rate-0 control:

| rate | establishment | births | mean body size | expressed variation |
|---|---|---|---|---|
| 0.01 | **1.00** | **1.00** | **1.00** | **0%** |
| 0.10 | 1.08 | 1.00 | 1.00 | 4–15% |
| 0.30 | 1.04 | 1.00 | 0.96 | 29–40% |
| 0.50 | 1.00 | 1.02 | 1.11 | 36–64% |
| 1.0 | 1.13 | 1.03 | **0.87** | 83–88% |

The only quantity that consistently declines anywhere is mean body size, and
only at 1.0 (−13%, −15%, +1%). Establishment at 1.0 runs **−13%, +13%, +28%** —
it does not even have a sign. So the constraint the task expected to bind —
*"a rate high enough to load every lineage with broken production rules buys
variation nobody can select on because nothing lives"* — **does not bind
anywhere in the range that was searched.** There is no cliff to leave headroom
below; §8 says what that does and does not license.

That collapses a four-way trade into a one-sided question: *how much variation
should a species carry?* Three measured facts answer it.

- **0.01 carries none**, on three seeds and at both budgets. A fourth seed
  (§2b) puts it at 3% and those are two 17-cell *seedlings*, below the
  establishment bar, against seventeen full plants at 0.30.
- **Below 0.30 the owner's own decision is a dead letter.** `GenomeOnly` and
  `Full` — the no-safety-net flip and the behaviour it replaced — produce
  **byte-identical worlds at 0.10** on both seeds tested, and **different**
  worlds at 0.30 on both. A rate at which the fallback would never have fired
  anyway does not honour a ruling about what happens when it does. This
  narrows the fork report's own bracket for the flip from `(0.1, 0.9]` to
  `(0.1, 0.3]`.
- **1.0 stops the species being one.** 83–88% of plants carry a variant rule
  and the founding program is essentially gone; this is also the only place a
  cost shows.

**0.30 is the smallest rate that satisfies the first two and is far from the
third.** Two thirds of plants still run the species program; a third do not.
That is the middle `CLAUDE.md`'s first law asks for — *an outcome is a
distribution, not a binary* — where 0.01 gives one of the binary's ends and 1.0
gives the other.

0.10 is the defensible conservative alternative, and what it costs is stated
rather than hidden: expressed variation of 4.7%, 15.2%, 4.0% — on seed 1 that
is *three plants out of sixty-four* — and the shipped semantics stay inert.

**`tree` does not want a different rate — it cannot express any rate.** §4.

## 1. Method, and what the instrument had to grow first

`genome_drift` already censused the production rule. It counted **tables**, and
this question is about **plants** — `CLAUDE.md`'s *a genome that changed is not
a plant that changed*, which the plant line has now got wrong three times. Two
columns were added for this study and are the ones most of the argument rests
on:

- **`phenotype, drifted against undrifted`** — mean body cells and mean
  *mature fraction*, split by whether the individual's table is still its
  species'. The mature fraction is aimed at one specific deformation: the fork
  report §6 records that an emptied `Grow` slot leaves `self_type_after_grow`
  falling back to the cell's own type, so the tip never *retires* and the
  lineage runs on as a frontier of tips that never thickens and never anchors.
  That plant has cells and can establish; only the mature fraction sees it.
- **`throughput over the whole run`** — cumulative `germinations` and
  `fruit_dropped`, because every other column is a standing count at one
  instant and a rate that quietly shut the reproductive engine down would show
  there only as a smaller `n`, which is also what a crowded stand looks like.

The four judging quantities are then read as:

| | column |
|---|---|
| standing variation | `DRIFTED / live` (whole population, ~90% seed bank) and the new `n drift / (n drift + n base)` (bodied plants only — *expressed* variation) |
| establishment | `DISTINCT plants that ever established` |
| throughput | `births that reached the draw`, and `germinations` |
| lethal load | the paired same-seed comparison against rate 0 — see §5, which is where an instrument was caught lying |

**Every run's banner was read**, per `CLAUDE.md`'s knob-you-cannot-see rule.
`genome_drift` prints `fate_lookup=` and `fate_mutation_chance=` on line 2, and
the `Full` arm in §3 is only trustworthy because its banner says
`fate_lookup=Full`.

**Positive control, run first.** At rate 0 the drifted count must be exactly
0. It is, on every seed and both species, with `gen0` and `empty` also 0 — and
the instrument is *sensitive*, because the same columns read 96.4% drift and
485 distinct tables at rate 1.0. A readout that could only report zero would
have produced the rate-0 rows too.

**The binary was rebuilt and the rebuild controlled.** After the phenotype
columns landed, one configuration was re-run and diffed against its pre-change
log: the only difference was the three new lines, every shared number
identical. So the seed-1 ladder taken on the earlier binary remains comparable.

## 2. `herb`, 20,000 frames, three world seeds

`expr%` is the share of plants **with a body** carrying a drifted rule — the
variation actually on screen. `pooled cells` is the whole-population mean body
size, which is the size statistic that is *not* age-confounded (§5).

| rate | seed | live | drift% | **expr%** | **estab** | pooled cells | germ | **births** | empty | gen0 |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | 1 | 738 | 0.0 | 0.0 | 62 | 95.8 | 179 | 1361 | 0 | 0 |
| 0 | 2 | 846 | 0.0 | 0.0 | 56 | 102.1 | 162 | 1642 | 0 | 0 |
| 0 | 3 | 752 | 0.0 | 0.0 | 36 | 98.4 | 98 | 1408 | 0 | 0 |
| **0.01** | 1 | 738 | 0.3 | **0.0** | 62 | 95.8 | 179 | 1361 | 0 | 0 |
| **0.01** | 2 | 846 | 0.5 | **0.0** | 56 | 102.1 | 162 | 1642 | 0 | 0 |
| **0.01** | 3 | 752 | 0.4 | **0.0** | 36 | 98.4 | 98 | 1408 | 0 | 0 |
| 0.03 | 1 | 738 | 1.6 | 1.6 | 62 | 95.8 | 179 | 1361 | 0 | 0 |
| 0.10 | 1 | 738 | 10.2 | 4.7 | 62 | 95.8 | 179 | 1361 | 0 | 0 |
| 0.10 | 2 | 875 | 12.5 | 15.2 | 63 | 104.8 | 182 | 1692 | 0 | 0 |
| 0.10 | 3 | 740 | 13.0 | 4.0 | 39 | 92.3 | 95 | 1388 | 0 | 0 |
| 0.20 | 2 | 937 | 20.4 | 12.9 | 61 | 93.7 | 202 | 1745 | 0 | 0 |
| 0.20 | 3 | 733 | 24.1 | 12.0 | 44 | 93.1 | 97 | 1389 | 0 | 0 |
| **0.30** | 1 | 797 | 47.9 | **40.0** | 64 | 103.1 | 212 | 1507 | 0 | 0 |
| **0.30** | 2 | 872 | 35.0 | **30.8** | 58 | 90.8 | 181 | 1634 | 0 | 0 |
| **0.30** | 3 | 737 | 39.2 | **28.9** | 38 | 94.0 | 108 | 1366 | 0 | 0 |
| 0.50 | 1 | 845 | 69.6 | 63.6 | 57 | 106.8 | 184 | 1515 | 0 | 0 |
| 0.50 | 2 | 864 | 59.1 | 35.9 | 57 | 94.9 | 229 | 1670 | 0 | 0 |
| 0.50 | 3 | 628 | 58.8 | 40.5 | 36 | 108.9 | 91 | 1225 | 0 | 0 |
| 1.0 | 1 | 746 | 96.4 | 87.3 | 54 | 83.7 | 232 | 1407 | 0 | 0 |
| 1.0 | 2 | 880 | 97.6 | 87.8 | 63 | 87.1 | 216 | 1637 | 0 | 0 |
| 1.0 | 3 | 816 | 98.2 | 83.0 | 46 | 99.5 | 262 | 1532 | 0 | 0 |

**Read the three rate-0 rows first.** Establishment spans **36 to 62** across
three seeds with nothing varying but the world. That spread is the ruler every
other row has to be read against, and it is why a single seed cannot settle
this: seed 3's baseline of 36 is *below* seed 1's value at rate 1.0.

**Establishment, within seed, against that seed's own rate-0 control:**

| rate | s1 | s2 | s3 |
|---|---|---|---|
| 0.01 | 62 (=) | 56 (=) | 36 (=) |
| 0.10 | 62 (=) | 63 (+13%) | 39 (+8%) |
| 0.30 | 64 (+3%) | 58 (+4%) | 38 (+6%) |
| 0.50 | 57 (−8%) | 57 (+2%) | 36 (=) |
| 1.0 | 54 (−13%) | 63 (+13%) | 46 (+28%) |

**Establishment does not have a sign** — not at 0.30, and not even at 1.0.
This is **not** a claim that mutation helps: once mutation bites, the two runs
are different worlds by the next frame, so each is one draw from a wide
distribution. What it rules out is the failure the task named. Nothing is
being loaded with broken rules to the point of not living, at any rate tried.

**Throughput never becomes the constraint either.** Births per run stay in
1,225–1,745 at every rate on every seed against 1,361–1,642 at rate 0; the
lowest is seed 3 at 0.50 (−13%), which is not reproduced at 1.0 on the same
seed (+9%). The reproductive engine keeps running with every birth mutating.

**The one cost that does appear** is mean body size at 1.0: 0.87, 0.85, 1.01
of each seed's own control. Two of three, and only at the extreme.

### 2b. A fourth seed, and what it takes off the claim

The review card in §9a renders `filmstrip scene=grove`, which does not set a
world seed, so the pair was censused on that **default** bed too — a fourth
seed, arrived at for a different reason and therefore not chosen to suit.
21,600 frames:

| rate | live | drift% | expr% | distinct | estab | cells drifted / rest | births |
|---|---|---|---|---|---|---|---|
| 0.01 | 912 | 1.1 | **3.0** (2 of 66) | 8 | 64 | 17.0 / 105.9 | 1,789 |
| 0.30 | 868 | 42.5 | **32.1** (17 of 53) | 153 | 56 | 97.6 / 127.8 | 1,788 |

It refines two things and overturns neither.

- **"0.01 carries no expressed variation" is 0% on three seeds and 3% here.**
  The honest statement is *0–3%*, and what appears at 0.01 on this bed is two
  plants averaging **17 cells** — below the 20-cell establishment bar, i.e.
  seedlings, against 66 ordinary plants at 105.9. At 0.30 the same column is
  17 plants at 97.6 against 36 at 127.8. The difference between the two rates
  is not 3% against 32%, it is *two seedlings* against *seventeen plants*.
- **Establishment at 0.30 gets its first negative**: 56 against 64, −13%,
  where seeds 1–3 gave +3%, +4%, +6%. Four seeds now read **+3, +4, +6, −13**.
  That is a wider spread than the first three suggested and it is the right
  correction to make — a three-seed run of same-signed results was tidier than
  this engine usually is. It does not move the recommendation (throughput is
  untouched at 1,788 against 1,789, and body size is *up*), but "no cost at
  0.30" should be read as *no consistent cost across four seeds*, not as *no
  seed loses anything*.

**`empty` is 0 in all 35 runs**, including rate 1.0 on both species.
`FateGenome::mutate`'s delete floor holds: no lineage ever deleted its way to a
genome with no rules at all, which is the failure mode
`fate_for_under`'s own comment says this column exists to catch.

### 2a. The same ladder at 60,000 frames, which is the reference budget

The tables above run 20,000 frames. The fork report's and the task's baseline
numbers are at 60,000, where `herb` has ~4.3x more births to mutate. Seed 1:

| rate | live | drift% | **expr%** | estab | pooled cells | germ | births | gen max |
|---|---|---|---|---|---|---|---|---|
| 0 | 873 | 0.0 | 0.0 | 74 | 123.1 | 309 | 5,858 | 5 |
| **0.01** | 873 | 0.6 | **0.0** | 74 | 123.1 | 309 | 5,858 | 5 |
| 0.10 | 873 | 22.2 | 13.1 | 74 | 123.1 | 309 | 5,858 | 5 |
| **0.30** | 1,082 | 56.4 | **44.6** | 80 | 130.5 | 422 | 6,925 | 6 |

**The longer budget changes how much variation a rate buys and does not
introduce a cost.** Expressed variation at 0.10 goes 4.7% → 13.1%, at 0.30
40.0% → 44.6%. At 0.30 every aggregate is *up* against the same seed's rate-0
control — establishment 80 against 74, births 6,925 against 5,858,
germinations 422 against 309, body size 130.5 against 123.1.

**And seed 1's stand is still bit-identical at 0.10 even here**, with eight
bodied plants carrying a drifted rule. Their sizes partition the *same* 61
plants: 8 at 105.8 and 53 at 125.7 pool to exactly the 123.1 of the rate-0 run.
Every mutation on this world at this rate is silent. On seeds 2 and 3 it is
not. **The silent fraction is large and world-dependent**, which is what
`Reports/lanes/plant-evolution-handoff-2026-08-30.md` §5a predicted for the
DFE and is worth carrying into it.

**Why drift keeps growing with run length, and where it stops.** `DRIFTED` is
measured against the *founding* table, so a lineage that mutates stays drifted
for ever and so do its descendants — this is a cumulative quantity in
generations, not an equilibrium one. The per-birth model
`1 - (1-r)^g` at the run's own mean generation predicts it closely: at
r = 0.30, g = 2.34 it gives **56.5%** against **56.4%** observed. Mean
generation is itself the thing that equilibrates (the handoff puts it near
2.9), so at 0.30 standing drift tends to ~62% and expressed variation to
roughly half the stand however long the world runs. It does not run away.

## 3. Where the shipped semantics start to exist

The owner's ruling on review card `20260829T204941423Z-880e13` was **"No safety
net"**. Whether that ruling changes anything is a function of the rate, and it
is directly testable: run the same seed and rate under
`PIXEL_PHYSICS_FATE_LOOKUP=full` (the withdrawn three-layer walk) and under the
shipped `GenomeOnly`, and ask whether the worlds differ.

| rate | seed | `GenomeOnly` vs `Full` |
|---|---|---|
| 0.10 | 1 | **identical**, every sample |
| 0.10 | 2 | **identical**, every sample |
| 0.30 | 1 | **differ**, from frame 10,000 (317 live against 315) |
| 0.30 | 2 | **differ**, from frame 5,000 |

The fork report measured the net catching 0 of 88,909 queries at 0.01 and 0 at
0.1, and 1,305 at 0.9, leaving the first bite somewhere in `(0.1, 0.9]`. This
puts it in **`(0.1, 0.3]`** and does it with a different instrument — a world
divergence rather than a save counter — so the two do not share a failure mode.

## 4. `tree`: the knob is not expressible there, at any setting

Seed 1, 20,000 frames, rates 0 through 1.0:

| rate | live | drift% | **expr%** | estab | births | gen max |
|---|---|---|---|---|---|---|
| 0 | 178 | 0.0 | 0.0 | 10 | 341 | 1 |
| 0.01 | 178 | 0.0 | 0.0 | 10 | 341 | 1 |
| 0.03 | 178 | 3.4 | **0.0** | 10 | 341 | 1 |
| 0.10 | 178 | 7.3 | **0.0** | 10 | 341 | 1 |
| 0.30 | 178 | 29.8 | **0.0** | 10 | 341 | 1 |
| 1.0 | 182 | 94.5 | 27.3 | 8 | 349 | 1 |

Every row from 0 to 0.30 is the **same world** — same live count, same
establishers, same throughput, same body sizes (1,444.4 cells, mature fraction
0.645). Standing genome variation rises to 29.8% and **not one carrier ever has
a body**: `tree` reaches generation 1 and stops, so the mutants are all seeds
that never germinate inside the run. At 1.0 three of them finally do and stall
at **5 cells** against the founders' 2,193, and establishment falls 10 to 8.

That last row is the sensitivity check the zeros above need: the tree readout
*can* move, so the NaNs are real absences and not a blind column.

**So `tree` and `herb` do not want different rates.** They want the same one,
and on `tree` it will do nothing until the species turns over —
`plant-throughput-herb-2026-08-29.md` measured tree at deepest established
generation **1**, 0 of 16 established plants carrying an inherited genome. The
rate is a `herb`-and-successors knob today. Nothing about raising it makes
`tree` worse; the whole ladder up to 0.30 leaves `tree` bit-identical.

## 5. An instrument that lied, caught by its own control

A 2x2 was added to `genome_drift` for the lethal-load question: establishment
rate **within** the drifted set against **within** the undrifted set, over
distinct individuals. It looks like the right statistic and it is not one.

On `herb` seed 1 at rate 0.10 it reads:

```
drifted     2 established of 106 ever seen   (1.89%)
undrifted  60 established of 1044 ever seen  (5.75%)
```

— an apparent **3x** establishment penalty for carrying a drifted rule, in a
run whose **entire stand is bit-identical to the same seed at rate 0**. Drift
there provably changed nothing, so a 3x cost cannot be a cost.

The confound is **age**. Drift accumulates down a lineage, so a drifted
individual is on average younger; the undrifted set holds the eight founders
and every early birth, and a younger individual has had less time to reach the
20-cell bar. The number is arithmetically correct and answers a different
question — `CLAUDE.md`'s single worst-recurring failure, in its *census*
costume.

It is kept in the harness, with the caption rewritten to say it is confounded
and to name the run that proves it, because deleting it would leave the next
reader to re-derive the same wrong number. **The instrument that does work is
the paired same-seed comparison against rate 0** — establisher count, pooled
body size, throughput — which cancels age, assignment and competition together,
and is what §2 tabulates.

One further trap this study walked into and out of: on seed 1 the stand is
identical from rate 0 all the way to 0.10, which made 0.10 look free *and*
inert. **On seeds 2 and 3 it diverges at 0.10.** A single seed had produced a
clean, tidy, wrong picture — `CLAUDE.md`'s *tidiness is the tell*, and the
reason the ladder was re-run on three seeds rather than reported from one.

## 6. The reallocation question, answered by measurement rather than argument

`CLAUDE.md`: *before starting a change that reallocates a shared budget, name
the constants calibrated against the current behaviour and budget re-deriving
them as part of the work.* Named, and each disposed of:

- **`DISCRETE_MUTATION_CHANCE` (0.03) and `MUTATION_SIGMA` (0.08)** — the
  genotype's discrete and continuous mutation knobs. Not a shared budget:
  three independent probabilities on three independent draws, and the fate
  draw comes from its own keyed substream (`FATE_MUTATION_STREAM`) that
  consumes nothing from the caller's generator. `FATE_MUTATION_CHANCE`'s doc
  said it was *"deliberately below `DISCRETE_MUTATION_CHANCE`"*; that ordering
  was an argument, not a calibration, and this measurement replaces it.
- **Every species-economy constant in `herb.ron`**, all tuned against a stand
  running one production rule. This is the real one, and it is answered
  empirically rather than by re-deriving: at 0.30 the stand's aggregates —
  establishment, germinations, births, pooled body size — sit inside the
  spread the three rate-0 controls already span. There is nothing to
  re-derive because nothing moved. (This is a *measured* answer at 20,000 and
  60,000 frames on the current world, not a proof; see §8.)
- **`a_seed_inherits_its_parents_production_rule`** (`plant.rs`) — a guard
  whose assertion message reads *"At `FATE_MUTATION_CHANCE` this draw does not
  mutate"*. It is calibrated against the rate by construction: its fixed
  `(world seed, landing cell, generation)` key has a threshold rate above
  which the draw fires and the child stops matching its parent exactly. **It
  passes at 0.30, and its green is not a blind one** — per `CLAUDE.md`'s rule
  that a guard's green is not evidence until the fault it names has been put
  back, the same test was run at `PIXEL_PHYSICS_FATE_MUTATION_CHANCE=1.0`,
  where every draw fires, and it **failed**. So it does track the rate and its
  threshold is above 0.30.
- **`selection_arena`** — its arms would carry background polymorphism at
  0.30. It already echoes `fate_mutation_chance` in its banner, and the env
  override sets the rate per run, so an arm comparison that wants a clean
  background can ask for one with no code change. Worth saying out loud in
  the arena's own next report rather than discovering it in a noisy result.

## 7. Gates

- `cargo test --lib sim::plant` at the new constant: **107 passed, 0 failed,
  16 ignored**.
- The one rate-coupled guard run alone, and then run again with the fault put
  back: green at 0.30, **red** at 1.0 (§6).
- `cargo +1.98.0 clippy --all-targets -- -D warnings` (the container ships
  1.94; CI pins 1.98, and a lint's heuristic widens between them).
- `bash scripts/docscheck.sh`: clean.

## 8. What this does not establish

- **That 0.30 is right for a longer-lived world.** The rate is *per birth*,
  and how much variation it accumulates depends on how deep the pedigree gets.
  `herb` reaches generation 5–6 in these runs and §2a's model says drift tends
  to ~62% as mean generation equilibrates near 2.9. M10 streaming makes worlds
  bigger and runs longer, which raises the *mean* generation as well as the
  max. **Re-derive when `herb`'s mean generation over living organisms moves
  past ~4** — that is the operand the model actually reads, and it is printed
  on `genome_drift`'s last line.
- **That the cost curve has been mapped.** No consistent cost was found
  anywhere up to 1.0, which is a statement about *these* four quantities on
  *this* world at *these* budgets. It is not a proof that a high rate is free:
  §5 shows how easily a plausible instrument here reports a cost that is not
  there, and the inverse is equally available. In particular nothing was
  measured about how a stand *looks*, which is the bar `wiki/plants.md` sets
  and the one `CLAUDE.md` says is judged by eye. **That is the check this
  change most needs and does not have** — a `filmstrip` or app capture of a
  herb stand at 0.01 against 0.30, put to the owner.
- **That mutations at 0.30 produce fitness variation selection can act on.**
  That is the DFE question and it is still the payoff of the line
  (`Reports/lanes/settled-rerun.md` §8). This says variation now *exists in
  plants* rather than only in bookkeeping, which is a precondition for
  measuring it, not an answer.
- **Where the cost cliff actually is.** 0.50 was measured on two seeds and 1.0
  on one. The claim is only that 0.30 has headroom, not that the curve is
  mapped.
- **Anything about `tree`, `grass` or `moss` at any rate.** §4: `tree` is
  invariant to the whole ladder because it does not turn over; `grass` sets
  zero seeds on `main` (`open-bugs-handoff.md` §1n); `moss` never enters the
  fate path at all.
- **That the mature-fraction column has caught the deformed-frontier
  phenotype.** It moves — 0.510 against 0.584 at rate 1.0 on seed 1, and
  0.750 on tree's five-cell mutants — but no run isolates a lineage whose
  `Grow` slot is provably empty, so the column is currently evidence that
  *something* differs, not that it is that something.

## 9a. The judgement this cannot make, put to the owner

Everything above is counted. Whether a stand with a third of its plants running
their own program **looks** like one species or like a mess is not a number,
and `CLAUDE.md` is explicit that this is the bar. Two herb stands — same world,
same seed, same frame (noon at 21,600), differing only in the rate — are on the
review queue as a blind A/B, card `20260830T073610061Z-a89324`, with §2b's
counts under each image. If it reads as broken rather than as variation, that
is the answer and the rate comes back down.

## 9. One correction to a neighbouring document

`Reports/README.md`'s index entry for
`plant-fate-fallback-fork-2026-08-30.md` still reads *"Generation turnover
(mean depth 2.04 at 60,000 frames), not the fallback depth, is the
bottleneck."* That claim was **withdrawn by the report it indexes** —
commit `ba3f723`, *"correct the attribution: mutation volume, not generation
turnover"* — which rewrote the report, `README.md`, `PLAN-log.md` and both
`dead-ends.md` entries and did not touch `Reports/README.md`. Corrected in the
same commit as this report. This study is a third, independent route to the
same conclusion: turnover reaches generation 5 at the shipped rate and the
stand still carries zero expressed variation.

## 10. Provenance

- Instrument: `examples/genome_drift.rs`, with the phenotype and throughput
  blocks added here. 35 runs, logs reproducible from the command at the top.
- The rate is read from `PIXEL_PHYSICS_FATE_MUTATION_CHANCE` and echoed on
  line 2 of every log, so no run in this study can be a stale-binary artifact
  of the kind `CLAUDE.md`'s `include_str!` gotcha describes.
- Direct context: `Reports/plant-fate-fallback-fork-2026-08-30.md` (the flip),
  `Reports/plant-fate-operator-gate-2026-08-29.md` §4 (the first statement
  that the number in the file is not the rate in effect),
  `Reports/plant-throughput-herb-2026-08-29.md` (why `herb`, and tree's
  generation 1).
