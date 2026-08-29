# Gate 1: how much does the production rule tolerate being mutated? (2026-08-28)

**Status: measurement. Gate 1 of `plant-morphology-evolvability-2026-08-26.md`
§7, which is explicitly allowed to return "no".** It did not. Raw log:
`Reports/data/fate-viability-2026-08-28/48-mutants-seed7.log`.

Run before organs get cell types, materials and a pricing pass, so that a
"no" would have been cheap. Instrument: `examples/fate_viability.rs`.

## 1. The answer

**The substrate tolerates structural mutation far better than the literature
predicted, and the tolerance is not uniform — it has a precise shape.**

| | |
|---|---|
| point mutations run | 48 |
| silent (stand identical to base — the field is never read) | 8 (17%) |
| **effective mutations** | **40** |
| established at all | 37/40 (92%) |
| set at least one seed | 35/40 (88%) |

Controls, printed on every run and without which the rate is unreadable:
**base** (unmutated table) 3/3 established, 80 seeds; **lethal**
(shoot `child` → `Seed`) 0/3, 0 seeds.

This contradicts `plant-simulation-research.md` §7a, which is the reason the
gate existed: *"most mutations produce nonviable organisms"* (Ochoa, PPSN
1998). That holds for L-system grammars. It does not hold here, and §7 already
gave the reason it might not: **a nonviable plant simply dies and the economy
is already the filter.** The rule table is small, typed, and every value in it
is a legal cell type, so there is no syntactic garbage to generate.

## 2. The shape of the tolerance, which matters more than the rate

Every failure was the same kind of mutation:

| field mutated | mutations | sterile |
|---|---|---|
| `child` (what a straight continuation is created as) | 6 | **5** |
| `becomes` (what the acting cell turns into) | — | **0** |
| `lateral` (what a branch child is created as) | — | **0** |
| `becomes` + `lateral` together | 34 | **0** |

**The only way to kill the plant is to destroy the frontier.** `child` on a
frontier type decides what carries growth forward; point it at anything that
does not grow — `MatureBody`, `DormantBud`, `Seed` — and the axis terminates
on its first step. Everything else the substrate absorbs.

**This is the finding that bears on the machinery**, and it is a favourable
one:

- A **determinate axis ending in an organ** is a `becomes` mutation.
- A **truss** — a lateral bearing something other than more shoot — is a
  `lateral` mutation.

So the two fields the organ work needs are exactly the two the substrate
tolerates without a single failure in 34 tries, and the one field that kills is
the one organs never need to touch. `child` is still where novelty lives (it
decides what a shoot is *made of*), but it is also the field a genome should
mutate most cautiously.

## 3. What this does NOT establish, stated because the numbers invite it

Several arms out-produced the base: `RootTip.Grew.becomes → Seed` read 6 plants
/ 109 seeds against the base's 3 / 80, and four others read above 80.

**None of that is a result.** It is one world seed. Within-genome spread in this
engine runs 31–153 cells for identical genomes, so a single arm reading 109
against 80 is one sample from a wide distribution. Comparing arms needs an order
statistic over seeds, and that is **gate 3's** experiment (*does anything kill
or favour plants differentially by morphology*), not this one's. The harness now
prints that caveat itself so the numbers cannot be quoted without it.

The viability *rate* is not subject to the same objection: the sample is the 48
mutations, and a mutation that destroys the frontier destroys it at any seed.

## 4. Two instrument bugs, both caught by controls rather than by reading the code

Recorded because the failure mode is the one this repo keeps paying for — a
number that is arithmetically correct and about the wrong thing.

1. **The positive control reported the *unmutated* table as 0/3 established.**
   `trial` registered the variant species *after* `PlantScene::build` had
   already tried to plant it, so nothing was ever planted. Every mutant would
   have read nonviable: a clean, decisive, entirely false 0%, and one that
   would have cancelled the organ work. Fixed by making the ordering a property
   of the scene (`PlantScene::species_ron`, registered inside `build` before it
   plants) rather than something each caller must remember.

2. **The first full run read 92% and was inflated two ways.** Mutations drawn
   uniformly over six cell types land on the value already held about one time
   in six and change nothing (7 of 48); and `RootTip.Grew.lateral` pointed at
   four *different* types produced exactly the base's 80 seeds every time,
   because a root never takes the lateral path in this scene, so the field is
   never read. Both are the *identical-output-across-settings* tell. The draw
   now redraws until the value moves, and mutants whose stand is identical to
   the base are reported as **silent** and excluded from the denominator.

**Worth noting that fixing (2) did not move the answer** — 92% before, 92%
after, because the silent arms were spread across viable outcomes rather than
concentrated. The correction was still right: an uncorrected instrument that
happens to agree with the corrected one is luck, not evidence, and the next
question asked of it would not have been so lucky.

## 5. What it unblocks

Gate 1 passes, so the structural-genome direction is not disqualified on
viability grounds. The remaining gates are unchanged: **gate 2** (generation
throughput — the biggest practical one, measured depth 1–2) and **gate 3**
(does selection discriminate by morphology at all).
