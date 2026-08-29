# The four mutation operators, gated (2026-08-29)

**Status: measurement.** Closes §3a of
`plant-heritable-fates-handoff-2026-08-29.md` — *"three of the four operators
have no viability gate"* — and answers it with a different problem from the one
that was being hedged against.

Three of the four operators that mutate a plant's production rule shipped
unmeasured, and the weighting (retarget 60%, recondition 15%, insert 15%,
delete 10%) was set to reflect that: most of the budget to the one operator
with a number behind it. The worry the weighting encodes is that the other
three might be *dangerous*.

They are not. **Almost everything they produce lives. They hardly ever produce
anything.**

## 1. The harness was measuring a lookalike operator

Before any of the above could be trusted, one thing had to be fixed.
`examples/fate_viability.rs` did not call `FateGenome`'s operator; it
reimplemented a point mutation over its own copy of the rule table. The two had
diverged in two ways:

| | harness | `FateGenome` |
|---|---|---|
| replacement cell types | **6** on `tree`, 8 on `herb` | **8 on every base** (`PLANT_CELL_TYPES`) |
| which rule is picked | a cell-type **group** uniformly, then a rule inside it | a **rule** uniformly, from the flat genome |

The two extra types are `Flower` and `Fruit`. `tree.ron` declares no organ
material, no `Ripen` behaviour and no `Ripe` rule, so the shipped operator can
hand a tree lineage a rule that builds a flower out of wood which never ripens.
The harness prevented that by narrowing its draw set — which made the number
readable by making it a number about a mutation nothing in the engine performs.

The harness now routes genome -> `FateGenome::mutate` -> `to_table` -> RON ->
founder genome, so there is one operator rather than two. The confound is
**counted** rather than designed out: mutants reaching an organ on a base that
cannot ripen one get their own column.

**What it cost, on the woody base:**

| `base=tree op=retarget`, n=38 effective | established |
|---|---|
| all effective | **74%** (on record, via the lookalike: 92%) |
| reaches `Flower`/`Fruit` (n=9) | 44% |
| stays inside the old six types (n=29) | 83% |

`base=herb` reproduces its recorded **97%** exactly, which is the control: herb's
draw set was already 8 types and herb can actually ripen an organ, so nothing
should have moved there and nothing did.

**A hypothesis this measurement killed.** The group-first rule selection looked
like the other half of the explanation: it gave `tree`'s single benign
`DormantBud.Flush` rule **a third** of all draws where the engine gives it a
sixth, and each growth-critical `GrowingTip` rule 1/9 where the engine gives
1/6. Reweighting the *same* per-rule survival data by the two schemes gives
**79% against 76%** — about three points. It is real and it is small. The
measurable effect is the draw set alone.

## 2. The gate

`mutants=40 frames=12000 founders=3 worldseed=7`, both controls green on every
run (positive: 3/3 established; negative, shoot child -> `Seed`: 0/3).

**`effective` excludes two classes that a single rate hides.** *Declined* means
the operator itself changed nothing — a redraw that never found a different
value, `delete` at the one-rule floor, `insert` at `MAX_FATES`. *Silent* means
the genome changed and the stand came out identical anyway. A mutant in either
class grows the base plant, so counting it as tolerance is quoting the positive
control back as a result.

| base | operator | drawn | declined | silent | effective | **does anything** | viable when it does |
|---|---|---|---|---|---|---|---|
| `tree` | retarget | 40 | 0 | 2 | 38 | **95%** | 74% |
| `tree` | recondition | 40 | 5 | 39 | 1 | **2%** | 1/1 |
| `tree` | insert | 40 | 0 | 37 | 3 | **8%** | 3/3 |
| `tree` | delete | 40 | 0 | 40 | **0** | **0%** | — |
| `herb` | retarget | 40 | 0 | 4 | 36 | **90%** | 97% |
| `herb` | recondition | 40 | 3 | 23 | 17 | **42%** | 17/17 |
| `herb` | insert | 40 | 0 | 33 | 7 | **18%** | 7/7 |
| `herb` | delete | 40 | 0 | 21 | 19 | **48%** | 19/19 |

Of 46 effective mutations across the three unmeasured operators, **46 lived.**
The unmeasured operators are not a hazard; they are close to a no-op.

## 3. Why — one mechanism with three faces

`plant.rs`'s `fate_for` consults three layers **per query**, not per genome:

```rust
individual genome -> species table -> builtin_fate
```

Each is asked for one `(cell type, when, metamers)`. So a slot the genome
leaves empty is answered by the layer beneath, with a working rule. A mutation
is absorbed when:

- **it vacates a slot** — `delete` removes a rule; `recondition` moves one out
  of its old slot. The layer beneath refills it.
- **it lands in an occupied slot** — lookup is first-match-wins, so an
  `insert` whose drawn position falls *below* an existing rule for the same
  `(owner, when)` is dead on arrival.
- **it lands in a slot nothing queries** — `GrowingTip.Ripe` is never asked,
  because a shoot tip has no `Ripen` behaviour to ask it.

**`retarget` is the only operator that changes a rule in its existing slot**,
which is why it alone is 90–95% effective while the other three are 0–48%.

The decisive evidence is `tree delete`, **40 of 40 silent**. All six of `tree`'s
rules sit in slots `builtin_fate` covers, so removing any one of them is
invisible — *including the `GrowingTip.Grew` rule the whole plant grows by*.
`herb delete` is effective on exactly the cases the mechanism allows: its two
`Ripe` rules, the only slots `builtin_fate` answers `None` for, and its one
duplicated `GrowingTip.Node` slot, where a second genome rule shadows the layer
beneath.

**Which layer absorbs differs between the harness and the engine, and the
harness understates it.** `fate_viability` registers each mutated table as its
own species and plants that, so genome and species table are the *same* mutated
table and `builtin_fate` does the refilling. In the live engine a seed's genome
is mutated and its species file is not, so the species table refills first —
with the original rule. A live lineage therefore has *two* layers behind it
where a harness variant has one, and the deletions the harness scores as
effective (the organ clock) are precisely the ones a real species table would
still answer. **Every "does anything" figure above is an upper bound on the
engine's.**

`a_slot_a_genome_vacates_is_refilled_by_its_species_table` pins this. It was
**blind in its first form** and fault injection is what caught it: asserting on
`tree`'s vacated `(GrowingTip, Grew)` proved nothing, because `builtin_fate`
returns the identical rule for that slot, so a genome-authoritative lookup left
it green. The discriminating case has to be a rule only a species file can
author — `herb`'s `Node -> Flower @8`, since `builtin_fate` is `after_metamers:
None` throughout by construction.

## 4. What this changes

**`FATE_MUTATION_CHANCE = 0.01` is not the rate in effect.** Weighting the
per-operator effectiveness by the shipped 60/15/15/10 gives **58% on `tree` and
68% on `herb`** — and those are upper bounds, per §3. The handoff calls the rate
"a guess"; it is worse than that, because the number in the file and the number
acting on the world differ by a factor nobody had measured.

**`delete` cannot matter on a genome founded from a species table.** It only
ever *un-shadows*: it changes an answer only where the genome holds two rules
for one `(owner, when)`. Founders have no duplicate slots (herb has exactly one,
by authorship), so a lineage must first `insert` a duplicate before `delete` can
do anything at all. As shipped, 10% of the mutation budget is spent on an
operator that is inert until another operator has acted first.

**The argument for the flexible operator still stands, and is now priced.**
Insert was chosen because with retargeting alone a `tree` lineage could never
acquire a flower — `tree.ron` has no `Ripe` rule to retarget and nothing could
create one. That reasoning is sound. Its cost is that the mechanism works 8% of
the time on `tree`.

**Not proposed here:** any change to the weighting, to the fallback, or to
`insert`'s position draw. Making the lookup genome-authoritative would turn
`delete` into a real operator and would also let a lineage delete its way to a
plant that cannot grow; that is an owner call, and it now has numbers under it.

## 5. What is not established

- **One world seed** (7). Legitimate for a rate over *mutations* — a mutation
  that destroys the frontier destroys it at any seed — and not for comparing
  arms. Per-arm seed counts in the logs are not comparable to each other.
- **n = 40 per cell.** The `tree` retarget headline (74% against a recorded
  92%) is about two standard errors at n=38, and the organ split rests on n=9.
  See the addendum for the tightened run.
- **Viability is not fitness.** "17/17 lived" says the substrate tolerates the
  change, not that the change is any good. Within-genome spread here runs
  31–153 cells, so the seed counts are hypotheses.
- **The engine's own effective rate is unmeasured.** This gates the *operator*;
  what a lineage does with it over generations is the handoff's §3b and §3d, and
  still needs the lineage census and the genome probe.

## Reproduce

```
cargo build --release --examples
./target/release/examples/fate_viability base=tree op=delete mutants=40 frames=12000 founders=3 worldseed=7
```

`op=` is `all` (the shipped mixture) or one of `retarget|recondition|insert|delete`.
