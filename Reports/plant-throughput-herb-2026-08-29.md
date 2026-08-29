# Herb clears the generational barrier by an order of magnitude (2026-08-29)

**Status: measurement, 3 seeds + 4 controls.** Run to settle which species the
evolution programme should actually use. The answer is **`herb`**, it needs
nothing built, and two earlier conclusions have to be withdrawn to state it.

```
./target/release/examples/plant_probe species=<sp> trees=16 frames=45000 worldseed=<1..3>
```

## 1. The result

| | seeds set | born / died | established carrying an inherited genome | **deepest established generation** |
|---|---|---|---|---|
| `tree` (2026-08-27) | 143–196 | — | **0 of 16**, in 7 of 8 seeds | **1** |
| `grass` (2026-08-27) | 19–32 | — | 4–10 | **2**, in 7 of 8 |
| `grass` (today) | **0** | 16 / 0 | 0 of 8 | **0** |
| **`herb` seed 1** | 10,926 | 11,139 / 8,636 | **110 of 125** | **5** |
| **`herb` seed 2** | 9,006 | 9,120 / 6,825 | **108 of 119** | **7** |
| **`herb` seed 3** | 7,974 | 8,011 / 6,199 | **75 of 90** | **3** |

Herb's generation histogram is not a population that reached generation 2, it
is one running a life cycle: seed 2 reads
`[gen 0: 11, gen 1: 374, gen 2: 807, gen 3: 436, gen 4: 392, gen 5: 191, gen 6: 52, gen 7: 30]`.
**88% of established plants carry an inherited genome**, against tree's zero.

## 2. Why this decides the substrate, with the operator gate alongside

Throughput alone does not make a species evolvable — `plant-fate-operator-gate-2026-08-29.md`
measured that three of the four mutation operators are inert on a genome whose
slots `builtin_fate` backfills. Put the two measurements together and only one
species has both halves:

| | generations | `delete` | `recondition` | `insert` | mutable? |
|---|---|---|---|---|---|
| `tree` | 1 | 0% | 2% | 8% | no |
| `grass` | 0 today, 2 at best | *predicted 0%* | *~2%* | *~8%* | no |
| **`herb`** | **5–7** | **48%** | **42%** | **18%** | **yes** |

**Grass's prediction is not a guess and not a gap in the measurement.**
`grass.ron`'s `fates:` block is **byte-identical to `tree.ron`'s** — six rules
over the same six `(owner, when)` slots, every one of them backfilled. Whatever
grass's throughput turns out to be, its production rule is as immobile as
tree's. What separates herb is its three organ rules: `Ripe` is the only
condition `builtin_fate` answers `None` for, so organ rules are the only ones a
mutation can actually remove or shadow.

That is the whole argument for herb in one line: **it is the only species whose
genome can move and whose lineages last long enough to move it.**

## 3. Two earlier conclusions this withdraws

**3a. "Run evolution experiments on grass, not tree."** From
`plant-recruitment-measurement-2026-08-27.md` §5, repeated in
`plant-heritable-fates-handoff-2026-08-29.md` §3c. Grass sets **zero** seeds on
`main` today and reaches generation 0. Filed as `open-bugs-handoff.md` §1n with
four scene controls ruling out the two parameters that changed since — the
world width and the soil depth — in all combinations. Anyone following that
advice will measure a population that never turns over and read it as a result.

**3b. "The 4,095-organism ceiling is nowhere near binding."** Same report, from
grass at 24–36 slots and tree at 58–72. **Herb runs at 1,812–2,503 live in
1,923–2,633 slots** — 44–61% of the ceiling, with births still outrunning
deaths at the end of the run. No birth was refused in these three runs, so it
is not binding *yet*; at more founders, more frames, or a wider world it will
be. Whoever runs the lineage census needs to size for that rather than inherit
"nowhere near".

## 4. What is not established

- **Three seeds, one scene** (16 founders, 45,000 frames, flat undisturbed
  bed). Enough for a threshold question — *does it turn over at all* — and not
  for any rate.
- **Herb's fecundity may itself be an artifact.** 8,000–11,000 seeds set per
  run against grass's 19–32 is a very large ratio, and nothing here checks
  whether a population that dense is *healthy* or merely crowded. Both carbon
  binds are severe — leaf construction refuses 45–48% of wanted cells and organ
  clusters 31–36% — so these plants are poor and numerous. That is a plausible
  ruderal life history and it is not verified as one.
- **The grass regression is not bisected.** The window is two days and carries
  four lanes.
- **Nothing here measures drift.** That a lineage *can* reach generation 7 is
  not evidence that its production rule moved on the way. No probe prints a
  genome's rule table yet — the handoff's §3d, and now unblocked, because it
  finally has a population deep enough to ask.
