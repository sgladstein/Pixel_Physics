# The fallback fork, decided: the growth program has no net under it

**Status: landed 2026-08-30.** Closes §3 of
`Reports/lanes/plant-evolution-handoff-2026-08-30.md` — the one item that
handoff marked *open, waiting on the owner*, with the standing instruction
**"Do not change the shipped behaviour without the owner's answer."**

The answer arrived on review card `20260829T204941423Z-880e13`. In full:

> **No safety net**

---

## 0. What you may quote from this

The direction is decided and is not a measurement. **The numbers here say the
change is currently a no-op at the shipped mutation rate**, and that is the
part most likely to be misread. Two sentences that are both true:

- a mutation that removes a production rule now removes the behaviour;
- nothing on screen changed, and nothing was expected to.

Anyone who quotes the first without the second is claiming a result this
report does not contain.

## 1. The mechanism that was removed

`plant::fate_for` answered every query by walking three layers: the
individual's `FateGenome`, then its species' authored table, then
`plant::builtin_fate`. The genome is founded from the species table at
`World::push_organism`, so a founder and its species agree by construction —
and a *mutant* did too, because any slot its genome vacated was refilled from
underneath with the value the mutation had just removed.

That is what made three of the four mutation operators near-inert
(`plant-fate-operator-gate-2026-08-29.md` §3): on the woody base `delete` was
0% effective in 40 draws, `recondition` 2%, `insert` 8%. Only `retarget`,
which rewrites a rule in place rather than removing one, could be seen at all.

The fork was left open on purpose. The net is also the thing that stops a
lineage inheriting a body plan that does not work, so which side to take is a
question about what the world should be, not about which is correct. It went
to the owner and it came back.

## 2. What ships

`FateLookup::GenomeOnly` is the default. `Full` (the old three-layer walk) and
`NoSpecies` remain reachable through `PIXEL_PHYSICS_FATE_LOOKUP=full` /
`=nospecies`, so every measurement below can be re-run; nothing in the engine
selects them.

## 3. The measurement: at the shipped rate the net never fires

**Method.** A counter inside `fate_for_under`, incremented on every query where
the genome had no answer *and* a lower layer supplied one — i.e. every time the
net actually caught something — plus a total-call counter beside it. Run under
`genome_drift`, `founders=8`, one world seed, on the pre-flip build.

| species | mutation rate | fate queries | net saves |
|---|---|---|---|
| `herb` | 0.01 (shipped), 60,000 frames | 88,909 | **0** |
| `herb` | 0.01 (shipped), 20,000 frames | 26,006 | **0** |
| `herb` | 0.1 (10x), 20,000 frames | 28,253 | **0** |
| `herb` | 0.9 (90x), 20,000 frames | 39,340 | 1,305 |
| `tree` | 0.01, 20,000 frames | 26,074 | **0** |
| `tree` | 0.1, 20,000 frames | 26,074 | **0** |
| `moss` | 0, 0.1, 0.9 | **0 — never queries at all** | 0 |

Corroborated independently: `genome_drift` logs for `moss`, `tree` and `herb`
came back **byte-identical** between `Full` and `GenomeOnly` at both 0 and 10x
mutation (md5 over the log with the mode banner removed), with the banner
confirming the mode each time.

**The total-call counter is why the zeros mean anything.** `CLAUDE.md`: *a
null looks the same whether the mechanism is quiet or the probe never reached
it.* The first version of this probe reported three zeros and could not
distinguish "the net never fires" from "the counter is not on the path". Adding
`calls` separated them in one run — 88,909 real queries against 0 saves, and
`moss` at 0 calls, which is a different fact and the one that matters for §5.

**And the probe is proven sensitive**, by the 90x row: the same counter reads
1,305 when the net does catch something. A counter that could only ever read
zero would produce this table too.

**Why the shipped rate reads zero.** Not because mutation is absent — the same
`herb` run applies 153 genome changes and carries 161 standing drifted genomes.
It is that a drifted genome mostly still answers: `retarget` (60% of the
budget) rewrites in place and vacates nothing, `insert` (15%) only adds. The
operators that *can* vacate a slot are the 25% that were inert precisely
because the net absorbed them, and at 0.01 with a mean generation depth of
**2.04 at 60,000 frames** there are too few of them in too few lineages to land
on a slot anything queries.

**So generation turnover is the bottleneck, not the fallback depth.** That is
the same conclusion `plant-evolution-handoff-2026-08-30.md` §5 reaches from the
selection side, arrived at here by a different route.

## 4. One withdrawn claim: `builtin_fate` is not the absorber

`FateLookup`'s own doc said, of the three layers, that *"the species table is
not what absorbs a mutation on this base; `builtin_fate` underneath it is"*. It
inferred that from `NoSpecies` and `Full` measuring byte-identical at 10x —
remove the species layer, nothing changes, therefore the species layer was not
doing the work.

The premise is right and the inference is wrong. At 90x, where saves actually
occur, **all 1,305 were taken by the species layer and `builtin_fate` took
0.** The reason removing the middle layer changed nothing is that the two
layers *agree* on those slots — `an_authored_fate_table_agrees_with_the_builtin_rule`
is the guard that makes them agree, by design. Agreement, not absorption.

Nothing downstream depended on the wrong version, because both readings gave
the same advice (weigh `Full` against `GenomeOnly`, treat `NoSpecies` as a
control). It is corrected here because the next person to reason from it might
not be asking that question.

## 5. Two hazards, both checked rather than argued

**`moss.ron` authors no fate table at all.** Its genome is therefore empty, and
under a genome-authoritative lookup every query returns `None` — which would be
a shipped species silently frozen. It is safe because moss's only behaviour is
`Divide`, which never consults a fate: the call counter reads **0** across
every rate tried. This was found by enumerating, per species, the slots where
`builtin_fate` answers and the species table does not; moss showed all seven.

**`(RootTip, Node)` is the one slot every vascular species leaves to
`builtin_fate`** — `tree`, `conifer`, `shrub`, `creeper`, `grass`, `herb` and
`scrambler` alike. It is unreachable: every species files a root at
`plastochron: 0`, and `Grow` computes `leaf_due` as
`plastochron_interval > 0 && ...`, so a root never reaches `Node` and the slot
is never queried.

Both are now held by
`a_species_table_answers_every_slot_its_own_growth_can_reach`, which **computes**
reachability from the species files rather than listing it — `Grow` on a type
demands `(type, Grew)` and `(type, Stale)`, and `(type, Node)` only where that
type's `plastochron` is above zero; `BudBreak` demands `(DormantBud, Flush)`.
An allowlist would have rotted at the next species file; this fails the moment
one declares a behaviour it does not author a rule for, moss included.

**It was watched going red**, per `CLAUDE.md`'s rule that a guard's green is
not evidence until the fault it names has been put back: deleting
`tree.ron`'s `(GrowingTip, Grew)` rule fails it with the slot named. It also
carries both anti-vacuity assertions — at least five species found to grow, at
least twenty slots checked — so a typo in a species name cannot make it green.

## 6. What an emptied slot actually does

"A lineage can delete its way to a plant that cannot grow" is the phrase this
fork has been discussed in, and it over-reads what the code does. At the `Grow`
site a missing fate resolves as:

```
self_type_after_grow = fate.map_or(cell_type, |f| f.becomes)
child_type           = fate.and_then(|f| f.child).unwrap_or(cell_type)
lateral_type         = fate.and_then(|f| f.lateral).unwrap_or(cell_type)
```

so the tip does not stop — it **never retires**. Its children are tips, and
their children are tips. The lineage grows as an ever-advancing frontier that
never becomes `MatureBody`, and so never thickens, never anchors and never
carries load.

That is a graded outcome rather than a binary one, which is what `CLAUDE.md`'s
first law asks for: *an outcome is a distribution, not a binary*. A lineage
that vacates its growth rule is not deleted, it is deformed, and how badly
depends on which slot went.

## 7. What this does not establish

- **That the change is visible.** It is not, at the shipped rate. §3.
- **That mutations now produce fitness variation.** That is the DFE question,
  still open and still the payoff of the line
  (`Reports/lanes/settled-rerun.md` §8). This change is a precondition for it
  being answerable on `delete` and `recondition`, not an answer.
- **That `FATE_MUTATION_CHANCE = 0.01` is right.** The operator gate's §4
  showed the rate in the file is not the rate in effect. Removing the net
  raises the effective rate — the operators it suppressed now act — by an
  amount nobody has measured. That is the natural next measurement, and it
  wants generations turning over first.
- **Anything about `moss`.** It never enters this code path at all.

## 8. Provenance

- Owner's decision: review card `20260829T204941423Z-880e13`, answered
  2026-08-30T00:45:06Z, verbatim *"No safety net"*.
- Fork as originally framed: `plant-fate-operator-gate-2026-08-29.md` §4.
- The handoff that carried it forward:
  `Reports/lanes/plant-evolution-handoff-2026-08-30.md` §3.
- Guards: `a_species_table_answers_every_slot_its_own_growth_can_reach` (new),
  `a_slot_a_genome_vacates_stays_vacant` (was
  `..._is_refilled_by_its_species_table`; scaffolding kept, expectations
  flipped, each assertion paired with the `Full` answer it used to make),
  `the_three_fallback_depths_give_three_different_answers` (unchanged — it
  passes the depth explicitly and was already the sensitivity control),
  `an_authored_fate_table_agrees_with_the_builtin_rule` (repointed at
  `FateLookup::Full`, which is the depth its claim was always about).
