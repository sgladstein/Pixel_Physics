# Evolved or authored? What the substrate can discover on its own (2026-08-26)

**Status: findings note, answering a direct owner question.** Successor in
role to `plant-morphology-reach-2026-08-23.md`, which asked *can this
substrate be shaped like a sunflower*; this asks the question the owner put
next, and then sharpened:

> "Can we modify our system to cause these types of plants (not exactly
> them) to evolve naturally through our game engine or do we need to
> manually create them?"

> "I'm not against having to develop some primitives like Bryophytes,
> Pteridophytes, Gymnosperms, Angiosperms that can differentiate into more
> specific species, but I don't like the idea of creating a tomato and
> hoping it will turn into a cucumber. I'm worried it would just end up with
> slight variations on tomatoes. is this possible or unrealistic."

**TWICE CORRECTED — read §5a and §5b before §6.** The first draft
recommended loci-on-one-ancestor; the owner's second question refuted it
(§5a). The replacement, §6.2, named `ByOrder` as the rule table to make
heritable; an independent review refuted *that* (§5b), and the refutation
was verified in source. **§6.2 is withdrawn.** The diagnosis in §2 and the
clade factoring in §6.1 survive both refutations.

**§7 of the first draft of this note recommended loci-on-one-ancestor, and
the owner's second question is the correct objection to it.** That
recommendation is withdrawn and replaced in §6; §5a records why, because the
failure is a general one and worth not repeating.

Every "today" claim is verified in source at the commit this was written on;
file:line addresses are given so they can be re-checked rather than trusted.
No code was written for this note.

---

## 1. The short answer

**Possible, not unrealistic — and the owner's clade-level factoring is the
right place to draw the hand-authored line.**

The distinction that decides it is not *which* archetypes are wanted. It is
**what the genome holds**:

| | genome holds | mutation does | reachable space |
|---|---|---|---|
| **parametric** (today) | numbers that scale a program authored in the `.ron` | moves a dial | a blob around the authored point |
| **developmental** (the ask) | the program itself | rewrites a rule | a neighbourhood of tomato that contains non-tomatoes |

Everything below follows from that row. A parametric genome cannot produce
novel body plans *however many loci are added to it*, which is why §5a
withdraws the first draft's recommendation rather than tuning it.

## 2. Why nothing archetype-level can evolve today

Three hard stops, each verified rather than assumed.

**2a. Species identity is immovable.** `OrganismState::species`
(`organism.rs:1375`) is a `SpeciesId` into `SpeciesRegistry`, whose table is
built at startup from the `include_str!`'d `EMBEDDED` list
(`organism.rs:2149`) and written only by `upsert`, called from `builtin()`
and `reload()` — the loader and the F5 asset reload. Nothing else in `src/`
or `examples/` calls it. `plant::set_seed` copies the parent's `species`
verbatim (`plant.rs:842`). **No lineage can change species and no species
can arise at runtime.**

**2b. The genome moves numbers, never the behaviour graph.**
`SpeciesDef::cell_types: Vec<(CellType, Vec<Behavior>)>` (`organism.rs:931`)
decides what a cell *does*. The 10 continuous slots jitter parameters of
behaviours that already exist; the 6 discrete loci scale `branch_angle` and
`internode`, flip sympody and tropism, and set wood density and leaf
economy. None can add or remove a `Behavior`, add a `ByOrder` tier, or
introduce a `CellType`.

**2c. The dials are measured not to reach.** `root-morphology-findings.md`
is this enquiry one organ down, carrying the owner's identical constraint,
and found **taproot off the map rather than at one end of an axis** — roots
cannot thicken at all (`thicken`'s `can_widen` soil gate) and
`allocate_to_frontier` has no apical dominance, so a democratic frontier
gives fibrous *by construction*. `plant-evolution-design.md`'s form table
lists nine plant forms of which six are "values of knobs that already
exist" — and carries the owner's own correction directly beneath it: *"that
is a reachability claim on paper, and what has actually been seen is three
slightly different trees of similar vibe."*

## 3. Why this engine is unusually well-placed for the developmental route

Three facts already in the record, none of them acted on:

1. **The engine already is an L-system.** `tree-procedural-prior-art.md` §3:
   a context-sensitive L-system with length-preserving productions *is*
   formally a cellular automaton, and the `Behavior`-dispatch-over-`CellType`
   is the 2D generalisation. Open L-systems' `?E(x)` environment-query module
   is what `Grow` reading world fields already does. **"Nothing to port."**
2. **`ByOrder` already is the production set.** `plant-evolution-design.md`,
   "What the machinery actually is" item 2: every architectural weight is a
   `ByOrder` array, *"the engine's answer to an L-system's productions, and
   it is the shape language"* — a conifer is "orthotropic order 0,
   plagiotropic above", two entries in a table rather than two mechanisms.
3. **Therefore the developmental program is already expressed as a rule
   table. It is in the wrong file.** Moving that table from the `.ron` into
   the genome is, in principle, the whole change.

Growth here is genuinely developmental — per-cell, per-tip, local field
reads — rather than a shape template with noise on it. That is the property
the whole route depends on, and it was bought years of work ago.

## 4. The literature precedent, already in this repo's own research note

`plant-simulation-research.md` §7a names exactly this as the second of two
genome levels, and only the first was ever built:

> **Parameter vector.** Perturb the numeric fields of `Behavior` variants.
> Safe, always produces a viable organism. *(built — this is today)*
>
> **Structural.** Which `Behavior`s each `CellType` carries, and what the
> cell types transition into. Richer, and **where genuinely novel body plans
> would come from**, but most mutations produce nonviable organisms.

with Ochoa, *On Genetic Algorithms and Lindenmayer Systems* (PPSN 1998) as
the named reference on making structural mutation of a developmental grammar
behave.

§7c goes further:

- **Bornhofen & Lattaud (2009)**, *Competition and evolution in virtual
  plant communities* — *"essentially the target system, minus the physics"*:
  L-system morphology plus transport–resistance physiology, many
  generations, **mutation on both the L-system and a parameter set**.
- **Bornhofen, Barot & Lattaud (2011)** got Grime's CSR life-history
  strategies to **emerge — not be coded** — from a physiological plus
  architectural model, *given* heterogeneous resource availability and
  varying disturbance frequency. The report's own note: *"Disturbance is the
  half most simulations lack and this engine has in abundance"* — fire,
  explosions, structural collapse, the player's brush.

So the answer to "unrealistic" is: it has been done, twice, in engines with
less physics than this one.

## 5. The precedent that argues against, and how far it transfers

**D4** (`dead-ends.md`, creatures): NEAT-style topology evolution was
rejected for the creature brain — *"hours-of-noise bootstrap, illegibility,
and every downside traced to topology mutation, so topology got caged in a
fixed scaffold with evolvable weights."* Its re-test condition reads
*"re-litigating requires an owner decision, not new measurement"*, which is
precisely what the owner's question is.

Checked objection by objection against plants:

| D4 objection | transfers to plants? |
|---|---|
| needs speciation machinery to protect innovations | **No.** That cost exists because crossover between unlike topologies is destructive; the entry says so itself — *"Crossover compatibility across one shared scaffold is the entire reason."* `set_seed` takes one `parent_id` and copies (`plant.rs:841`): reproduction here is asexual, so there is no crossover to protect. |
| illegible results | **No.** A brain topology is unreadable; a plant is a picture, judged the way everything here is judged. |
| hours-of-noise bootstrap | **Yes** — and it is the real cost. |

One of three grounds survives, and it is a cost rather than an
impossibility. D4 is not a bar to this; it is a warning about the bootstrap.

### 5a. Why the first draft's §7 was withdrawn

That draft recommended shipping the reach report's primitives as **priced
genome loci on one ancestral species** rather than as new `.ron` files. That
is strictly better than authoring `tomato.ron` — and it is still a
*parametric* genome, so it still buys a blob around whatever ancestor was
authored. Adding loci enlarges the blob; it does not change its shape. The
owner's objection ("I'm worried it would just end up with slight variations
on tomatoes") is the correct diagnosis of it.

**The general lesson, and the reason this is recorded rather than quietly
edited:** *a better version of the wrong mechanism reads as progress and
measures as progress.* More loci would have shown more variance on every
instrument here while leaving the reachable set the same shape. Ask what the
genome **holds**, not how many knobs it has.


### 5b. Why §6.2's target was withdrawn too — independent review, 2026-08-26

An independent adversarial review was commissioned on this note and its
central objection was verified in source before being accepted.

**`ByOrder` is not a production set.** It is
`values: [T; BRANCH_ORDERS]` with `BRANCH_ORDERS = 4`
(`organism.rs:2433`, `:2356`) — a fixed-arity array of four values, appearing
as **nine** fields inside the single `Behavior::Grow` variant. It has no
arity to mutate, no successors and no symbols. Making it heritable yields a
longer vector of numbers scaling an authored program: **row 1 of this note's
own §1 table.** The genuine structural level is
`SpeciesDef::cell_types: Vec<(CellType, Vec<Behavior>)>` (`organism.rs:931`)
— which §2b names correctly as unreachable, and which §6.2 then silently
substituted `ByOrder` for.

**So §5a's lesson applied to §5a's own replacement**, which is worth stating
plainly: the withdrawal reasoning was correct and the successor still landed
on a parametric target one level up. *Ask what the genome holds* is easy to
assent to and hard to apply.

**And the repo had already measured this target against the owner's eye.**
`plant-evolution-design.md:467-515`, WP-C's three probes, two posted blind:
`weeping` moved **only** `upward_weight` on orders ≥1 — a `ByOrder` field,
and a *placement* lever rather than a labelling one — and came back *"same
plant"*; `prostrate` moved order-0 `Plagiotropic` plus a tiny `internode`,
both `ByOrder`, and came back *"Not that different"* at 2/5. Its summary:
*"across three probes, every group change came from the size budget and none
came from an architectural knob."*

That is the specific evidence against this note's defence of §6.2 — that a
rule-table mutation changes *how many* cells are placed rather than *which
label* they carry. `upward_weight` is exactly such a lever and it did not
read.

**Four further corrections from the same review**, each checked:

1. **"Nothing to port" was quoted against its source.**
   `tree-procedural-prior-art.md` §3 is headed *"least transferable, because
   the engine already is one"* — the claim is that L-system theory buys this
   engine nothing, not that a mutable grammar exists. Its §4 is
   counter-evidence this note walked past: *"every model surveyed starts from
   an authored seedling"*, trunk/crown separation *"never emergent"*.
2. **The D4 analysis fails on objection 1.** Creature reproduction is asexual
   too (`brain.rs`, quoted at `creature-direction.md:663`) — the speciation
   cost was about *forward* crossover compatibility, so "plants are asexual"
   dissolves nothing.
3. **The literature was oversold.** Bornhofen 2009/2011 are one group, two
   papers, not two replications; "less physics" makes the problem *easier*,
   not harder, so it is not a margin in our favour. And §7b was omitted
   entirely — Niklas's canonical plant-evolution simulation ran on a
   **six-variable parametric** morphospace, which weakens §1's table as
   written.
4. **§7b answers gate 3, which this note left blank.** *"Selecting on at
   least three conflicting tasks is not a nice-to-have; it is the
   mechanism."* With `plant-species-authoring.md` §3's measured *"a
   light-optimal stand is a slab"*, that reframes the problem as primarily
   one of **selection**, not encoding — the largest single update to this
   note and the one it under-weighted.

**Gates the review added** (see `plant-evolvability-facts-2026-08-27.md` for
the verified numbers): the 4-bit `CellType` budget, per-organism genome
memory, and determinism under a *variable-length* genome, where
`SEQUENCED_TRAITS`' frozen-prefix trick does not generalise because the draw
count itself becomes heritable.

**What survived the review:** §2's diagnosis in full, §6.1's clade
factoring, gate 4 (the position-keyed RNG confound, independently recorded as
creature gotcha P-21), and the acceptance-artifact framing.

## 6. Recommendation, revised

Three layers, and the owner's clade framing is layer one:

1. **Clade as hand-authored inventory + constraints.** A clade difference is
   an *inventory* difference — which tissues and organs exist, and what
   transport can do over what distance. Those must be hand-built regardless
   (cell types, materials, behaviours), so this is the right place for
   authored work. Bryophyte: no vascular tissue, so no transport at
   distance, so small and damp — `moss.ron` substantially is this already.
   Angiosperm: flower and fruit organs exist at all. Each clade is a
   vocabulary plus a constraint set.
2. **~~The `ByOrder` production set becomes heritable~~ — WITHDRAWN, see
   §5b.** `ByOrder` is a fixed-arity array of four numbers, so making it
   heritable is a parametric change, and its fields are the ones the WP-C
   register already measured as not reading. The genuine structural level is
   `cell_types: Vec<(CellType, Vec<Behavior>)>`. **What replaces this layer
   is open** and is the subject of the three-way review commissioned
   2026-08-27; §7b's "selection, not encoding" finding is the strongest
   candidate to displace it.
3. **Species as an outcome, not an input.** A "species" becomes a cluster in
   rule-table space that persists, which is the definition
   `organism.rs:1997` already argues for at the locus level.

**Before either of 1 or 2, two things decide whether they are worth doing
at all** (§7 gates 1 and 2). Both are cheap relative to the build, and both
can return "no".

**The acceptance artifact is not a sunflower.** It is: one clade, one
ancestor, run to N generations across a heterogeneous world with
disturbance, and a grid of the twelve commonest resulting morphologies with
their rule tables printed beside them. Three or four genuinely different
habits nobody wrote = it works. Twelve tomatoes = it does not, discovered
cheaply.

## 7. Four gates, in the order they should be answered

1. **Viability rate — measure before committing.** Most structural
   mutations are nonviable (§4). In a GA that is fatal; here it is largely
   fine, because a nonviable plant simply dies and the economy is already
   the filter. But the *rate* decides whether radiation moves or stalls, and
   it is a cheap experiment: mutate the rule table N ways, count how many
   reach reproductive size.
2. **Generation throughput — the biggest practical gate.**
   `plant-simulation-research.md` §7d already states it: plants mature in
   seconds of wall clock but evolution needs thousands of generations, so
   this needs a headless fast-forward with its own clock (`examples/ascii.rs`
   named as the right seed). Compounded by the 4,095 concurrent organism
   slots (`world.rs:29–39`) and by woody generation times — which is why the
   annual/herb path (reach report §7 call 3; `senescent` and
   `remains_half_life` already exist) is on this critical path rather than
   beside it.
3. **Selection must discriminate by shape.** CSR emerged *given*
   heterogeneous resources and varying disturbance (§4). The disturbance
   exists. Whether anything today kills plants differentially **by
   morphology** is **not measured here and is not claimed either way** — if
   every morphology survives equally, the walk is a random walk wearing an
   evolutionary label.
4. **A confound already on the record.** §7d: `Chunk::rng` is seeded from
   chunk coordinates, so the same genome planted in two places draws a
   different sequence — position becomes a hidden inherited variable, *"which
   is exactly the kind of thing that produces a spurious evolutionary
   result."* A per-organism stream keeps determinism and removes it. Land it
   **before** any radiation run, not after.

**The judging half is already built.** `divergence` is axis-agnostic —
*"adding an axis is one arm on `Axis` and nothing else"* — and
`Reports/instruments.md` already lists *"does a new genome locus move
morphology at all?"* among the questions it answers as a run rather than a
build. Pair it with `crown_census` and `flora_census where=/at=`, per the
within-stand-variance trap in §2c's source report.

## 8. Open calls for the owner

1. **Is the developmental (structural) genome accepted as the target?** It
   re-opens D4 on the one ground that transfers — bootstrap noise — and it
   is the largest architectural change proposed for plants. §7 gates 1 and 2
   are the cheap way to buy information before committing.
2. **Which clades, and in what order?** Bryophyte is mostly built. The
   cheapest *second* clade is the one whose inventory differs most from moss
   while sharing the most machinery.
3. **Does the niche table keep naming species?** Radiation and authored
   per-niche sowing (`worldgen/passes.rs` ~3870–4060) are in direct tension:
   sowing a named archetype into its own country authors the answer
   selection was meant to find.
