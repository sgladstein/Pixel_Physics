# The production rule has been watched drifting (2026-08-29)

**Status: measurement, 3 seeds + a `tree` control.** Closes §3d of
`plant-heritable-fates-handoff-2026-08-29.md` — *"Nobody has watched a lineage
drift... 'the production rule is evolving' is an inference from the mechanism
rather than an observation."* It is an observation now. It also returns a
**null** on the question everyone actually wants answered, and records how
underpowered that null is.

```
cargo run --release --example genome_drift -- species=herb founders=16 frames=45000 every=5000 worldseed=<1..3>
```

## 1. It drifts, and it plateaus

`herb`, seed 1, against a founding table of 9 rules:

| frame | organisms | drifted | longer | shorter | changed | **distinct tables** |
|---|---|---|---|---|---|---|
| 5,000 | 308 | 1 | 0 | 0 | 1 | 2 |
| 15,000 | 1,735 | 13 | 3 | 0 | 10 | 13 |
| 30,000 | 2,460 | 21 | 6 | 0 | 15 | 17 |
| 45,000 | 2,496 | 22 | 5 | 1 | 16 | **20** |

**Twenty coexisting developmental programs in a world that shipped with one.**
Both built-in controls read zero at every sample in every run: no generation-0
organism ever differs from its species table, and no genome is empty.

**All four operator signatures appear in a live population**, which the harness
gate could not show — `longer` is an `insert`, `shorter` a `delete`. Both are
rare and both are real. This is consistent with
`plant-fate-operator-gate-2026-08-29.md`: `delete` shows up on `herb` and
essentially not on `tree`, because herb's two `Ripe` rules are the only slots
`builtin_fate` does not backfill.

**Drift does not accumulate.** From frame 15,000 it sits near 1% of the
population and stays there. This is a standing polymorphism, not a march to
fixation.

## 2. The `tree` control, and an apparent contradiction that is not one

`tree` drifts too — 4 of 221 at frame 45,000, 5 distinct tables, including two
`longer` and one `shorter`. The operator gate reported `tree delete` at **0 of
40 effective**. Both are true, and the distinction is exactly what the gate's
*silent* column encodes: a `delete` on tree **changes the genome** (drifted
here) and **does not change the plant** (silent there).

So the herb/tree contrast is not "herb's genome mutates and tree's does not".
Tree's drifted *fraction* is if anything higher (1.8% against 0.9%). What tree
lacks is **persistence**: it reaches generation 1, so every drifted genome sits
in a seed that never establishes and nothing compounds. Herb's twenty tables
are carried by a population that turns over.

## 3. Does a drifted table cost a plant its establishment? — **no detectable effect**

The in-vivo form of the gate's question, and the gate cannot ask it: a harness
mutates one rule and grows one stand; this asks what a living population does
with the mutations it actually makes.

| seed | population drift | distinct plants that ever established | of which ever drifted |
|---|---|---|---|
| 1 | 0.88% | 148 | **0** |
| 2 | 1.53% | 136 | **3** |
| 3 | 1.10% | 101 | **0** |

Pooled: **3 of 385 (0.78%)** against a population rate of ~1.17%. Expected
under the null is 4.5; observed 3; `P(<=3 | lambda=4.5) ~ 0.34`. **Nothing.**

**And the power is poor enough to say so plainly.** With ~385 distinct
established plants at a ~1% base rate, the expected count is under five. This
design can detect a mutation that is close to lethal and nothing subtler. A
real answer needs either far more plants or the strong form — competing arms in
one world, read at an order statistic — which does not exist for plants.

### 3a. The denominator was wrong first, and the tell was tidiness

Recorded because the wrong version told a **better story than the truth**. The
first pass pooled established counts across samples and divided, which counts
one persistent plant once per sample it survives. It printed *"drifted among
established plants: 0 of 989 (0.00%)"* for two of three seeds — a headline
finding of strong selection against drift.

Two seeds at *exactly* 0.00% is not what a 1% rate over ~1,000 trials looks
like: `P(0 of 989) ~ 4e-5`, twice over. `CLAUDE.md`'s rule that a clean first
result is evidence of an artifact before it is evidence of a strong effect, and
"ask what your number counts" in its exact form — the percentage was
arithmetically correct and its denominator was organism-samples where the
question needs organisms. Now counted as distinct individuals.

## 4. The open question this leaves: drift runs 2.6x below the model

Under a pure per-birth model at `FATE_MUTATION_CHANCE = 0.01`, seed 1's own
generation histogram predicts **2.33%** of organisms carrying a drifted table.
Observed: **0.88%**.

| | |
|---|---|
| expected (from the run's own generation histogram) | 58 of 2,496 — **2.33%** |
| observed | 22 of 2,496 — **0.88%** |

Declined operator draws account for about 1% of mutations on herb, nowhere near
the gap. §3 rules out establishment selection as the explanation at the power
available. So something between "a birth happens" and "a genome differs" is not
what the model assumes, and this run does not say what.

**The measurement that would settle it is cheap and named.** `FateGenome::mutate`
now returns `Option<FateMutation>` carrying which operator ran and whether it
applied; the call site in `plant::bear_seed_at` discards it. A world-level
counter there gives the number of mutations that actually fired, against the
number of drifted genomes standing — `CLAUDE.md`'s "pair every *it fired*
counter with an effect counter from the far side of the call". Until that
exists, the 2.6x is a discrepancy and not a finding.

## 5. What is not established

- **Three seeds, one scene.** Enough for *does it drift at all*; not for a rate.
- **Drift is not fitness, and this measures neither.** That twenty tables
  coexist says the space is being explored, not that anything in it is good.
- **The plateau is unexplained** — §4.
- **No lineage was followed individually.** This censuses a population; it does
  not trace one plant's descendants, so it cannot say whether a drifted table
  is inherited intact or re-mutated.
