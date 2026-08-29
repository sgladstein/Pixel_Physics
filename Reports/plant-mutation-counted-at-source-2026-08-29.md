# The 2.6x drift gap was the model, not a loss (2026-08-29)

**Status: measurement, 3 seeds, with an exact control on the instrument.**
Closes §4 of `Reports/plant-rule-drift-observed-2026-08-29.md` — *"drift runs
2.6x below the model... something between 'a birth happens' and 'a genome
differs' is not what the model assumes"*.

Nothing is between them. The draw fires at its nominal rate, the operator
applies almost every time, and the missing drift was never missing: the model
it was compared against predicts a standing count that the run's own mortality
makes arithmetically unreachable.

```
./target/release/examples/genome_drift species=herb founders=16 frames=45000 every=5000 worldseed=<1..3>
```

## 1. The control comes first, because it is exact

`World::fate_mutation_rolls` counts births that reached the mutation draw.
`plant_probe` independently counts births. They are written by different code
at different call sites and were never compared until now:

| seed | `plant_probe` born | `fate_mutation_rolls` | difference |
|---|---|---|---|
| 1 | 11,139 | 11,123 | **16** |
| 2 | 9,120 | 9,104 | **16** |
| 3 | 8,011 | 7,995 | **16** |

**Sixteen on every seed, and the run plants sixteen founders.** A founder is
created by `World::push_organism` and never passes through
`plant::bear_seed_at`, so the only births the counter can see are bred ones.
An off-by-anything-else would have shown here.

This is the check `CLAUDE.md` asks for under *ask what your number counts when
nothing is wrong* — and it is the strong form, because the answer was predicted
before it was read rather than rationalised after.

## 2. Where the drift goes

| seed | births | draw fired | rate | applied | applied % | standing drifted |
|---|---|---|---|---|---|---|
| 1 | 11,123 | 108 | 0.971% | 106 | 98.1% | 22 |
| 2 | 9,104 | 69 | 0.758% | 61 | 88.4% | 35 |
| 3 | 7,995 | 100 | 1.251% | 99 | 99.0% | 20 |
| **pooled** | **28,222** | **277** | **0.982%** | **266** | **96.0%** | **77** |

**The draw is not the explanation.** 0.982% pooled against a nominal 1.000%.

**The operator is not the explanation.** 96% of draws changed the genome; the
4% that declined agree in order with the gate's harness figure.

Both candidate causes named in §4 of the drift report are ruled out by
measurement rather than by argument.

## 3. The model was unreachable, and that is the finding

The 2.6x was measured against `1 - (1 - p)^g` at the standing population's mean
generation. Pooled, that model predicts **155** standing drifted organisms
against 77 observed.

**Check what it is asking for.** Only 266 mutations were ever applied across
the three runs. The model needs 155 of them — **58%** — to be represented in
the standing population. An ordinary birth survives to the census at **23%**
(6,599 live of 28,222 born).

So the model implicitly assumes drifted lineages outperform everyone else by
about 2.5x. It is not a neutral prediction wearing a neutral label; it is an
equilibrium expectation applied to a population that is still turning over,
where most mutations are recent and have not had generations to spread.

**Scale it by ordinary mortality and it lands.**

| | predicted | observed |
|---|---|---|
| per-birth model, pooled | 155 | 77 — overpredicts **2.0x** |
| applied x survival, pooled | 62 | 77 — under-predicts **1.24x** |

The corrected model misses in the *opposite* direction, which is the useful
part: drifted lineages are doing at least as well as undrifted ones. That
agrees with the drift report's §3 null and is a second, independent route to
it.

## 3a. It also explains the plateau, which was filed separately

`plant-rule-drift-observed-2026-08-29.md` lists *"the plateau is unexplained"*
in its §5 as an open item distinct from the 2.6x. It is the same finding seen
from the other side, and it needs no extra mechanism.

Drift plateaus because it is a **birth-death balance**, not an accumulation.
Mutations enter at a constant rate per birth and leave with their carriers at
the ordinary death rate, so the standing fraction settles at roughly
`p x (mean generations a lineage survives)` and stays there. Accumulation to
fixation — which is what `1 - (1 - p)^g` describes — requires mutations to
persist and spread, and in a population turning over at 23% survival most of
them simply die with the plant carrying them.

So "a standing polymorphism, not a march to fixation" is the *expected*
behaviour of a neutral mutation in a high-turnover population, rather than
evidence that something is holding drift down. Both of that report's open
items resolve to the same arithmetic.

## 4. Seed 2 is the one worth looking at again

| seed | applied | survival of an ordinary birth | expected standing | observed |
|---|---|---|---|---|
| 1 | 106 | 22.4% | 23.8 | 22 |
| 3 | 99 | 22.6% | 22.4 | 20 |
| **2** | **61** | **25.2%** | **15.4** | **35** |

Seeds 1 and 3 land within two individuals of the survival-scaled prediction.
Seed 2 carries **2.3x** more standing drift than its mutation count and
mortality allow, and it is also the run that reached the deepest generation
(max 7, against 6 and 4).

The straightforward reading is one mutant lineage that expanded. **It is one
lineage in one run and nothing here follows it**, so it is a thing to go and
look at, not a result. It is also the first sign in this programme of a
mutation doing better than average rather than merely surviving — which is the
question `plant-fate-operator-gate-2026-08-29.md` §6 says no experiment in this
repo asks for plants.

## 5. What is not established

- **Three seeds, one scene, one species.** Enough to rule out the draw and the
  operator as causes, which are per-mutation properties. Not a rate.
- **Nothing here follows a lineage.** §4's excursion is inferred from a
  standing count, exactly the inference §3 warns against making from one.
- **The keyed substream is over-dispersed and this does not settle by how
  much.** The draw comes from `rng::stream(seed, sx, sy, generation)` rather
  than a per-birth roll, so seeds landing on one cell from same-generation
  parents get one answer. Observed per-seed rates spread 0.76%-1.25% where a
  binomial at n~9,000 would give about +/-0.10 points; that is roughly 2.5x
  wider, on three points. The pooled mean is unaffected and sits on nominal,
  so this changes nothing above — but a single-run rate is worth less than its
  precision suggests, and a future sweep should pool.
- **`applied` is not `visible`.** A mutation that changes the genome may leave
  the plant identical — the gate's *silent* class, which the per-query fallback
  makes the common case.
