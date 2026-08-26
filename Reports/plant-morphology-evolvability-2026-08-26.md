# Evolved or authored? What the substrate can discover on its own (2026-08-26)

**Status: findings note, answering a direct owner question.** Successor in
role to `plant-morphology-reach-2026-08-23.md`, which asked *can this
substrate be shaped like a sunflower*; this asks the question the owner put
next:

> "Can we modify our system to cause these types of plants (not exactly
> them) to evolve naturally through our game engine or do we need to
> manually create them?"

Every "today" claim below was verified in source at the commit this was
written on; file:line addresses are given so they can be re-checked rather
than trusted. No code was written for this note.

---

## 1. The short answer

**Both — and the split is not where the reach report draws it.**

- **Primitives must be hand-built.** Evolution cannot invent a `Flower`
  cell type or a determinacy counter. The substrate has to be able to
  *express* a trait before selection can act on it. This is not a defect;
  it is the normal division of labour.
- **Combinations can be evolved** — but only if the primitives land as
  **genome loci with a price**, not as per-species authored constants.
  `plant-morphology-reach-2026-08-23.md` §3 proposes the latter, and if it
  ships that way the answer to the owner's question is "manually create
  them", permanently.

## 2. Why nothing archetype-level can evolve today

Three hard stops, each verified rather than assumed.

**2a. Species identity is immovable.** `OrganismState::species`
(`organism.rs:1375`) is a `SpeciesId` into `SpeciesRegistry`, whose table is
built at startup from the `include_str!`'d `EMBEDDED` list
(`organism.rs:2149`) and written only by `upsert`, called from `builtin()`
and `reload()` — the loader and the F5 asset reload. Nothing else in `src/`
or `examples/` calls it. `plant::set_seed` copies the parent's `species`
verbatim (`plant.rs:842`). **No lineage can change species and no species
can arise at runtime.** Species is authored, always.

**2b. The genome moves numbers, never the behaviour graph.**
`SpeciesDef::cell_types: Vec<(CellType, Vec<Behavior>)>`
(`organism.rs:931`) is what decides what a cell *does*. Against that:

| genetic channel | reach |
|---|---|
| 10 continuous slots (`GENOTYPE_TRAITS`) | jitter parameters of behaviours that already exist |
| 6 discrete loci (`DISCRETE_LOCI`) | scale `branch_angle`/`internode`, flip sympody and tropism, set wood density and leaf economy |

Not one can add or remove a `Behavior`, add a tier to `ByOrder`, or
introduce a `CellType`. The reachable morphology space of any lineage is
*"the shape this species already makes, with its dials moved."*

**2c. The dials demonstrably do not reach far enough.** This is measured,
and it is measured on the same question one organ down.
`root-morphology-findings.md` is this exact enquiry asked about roots, and
carries the owner's constraint verbatim — *"I am not asking you to hardcode
or specifically design these types of roots, but create a system where these
types of morphologies can develop or evolve naturally."* Its finding:
**taproot is not at one end of a current axis, it is off the map**, because
roots cannot thicken at all (`thicken`'s `can_widen` soil gate) and
`allocate_to_frontier` has no apical dominance, so a democratic frontier
produces a fibrous system *by construction*.

The shoot version is the identical shape and the reach report states it
without drawing the conclusion: every axis is indeterminate, which is why
every species is a variation on "tree" (that report's §2b).

## 3. The failure mode the reach report walks into

§3's bill of materials lands all four primitives as "species-`.ron` work
plus at most two new `Behavior` variants and two `CellType`s" — i.e.
`sunflower.ron`, `tomato.ron`, `vine.ron`.

Ship it that way and the result is what the world has now: N hand-authored
archetypes that do not interbreed, do not radiate, and are slight variations
on whatever the author typed. The owner's own second root verdict predicts
the reception: *"You are taking a structure that is already chaotic and
variable between the plants and making slight changes that are not clear."*

## 4. What changes it: identity as a locus, not a default

| reach-report primitive | as §3 proposes it | as a heritable locus |
|---|---|---|
| determinate axes (§2b) | `determinate: [n]` in `ByOrder`, per species | discrete locus + a continuous slot for N; indeterminate is one allele of it |
| terminal organ (§2a) | `Behavior::Flower` on the species that wants one | present on every tip entry; a `never` allele disables the trigger |
| attachment (§2d) | species `attaches` flag | discrete locus; climbing tropism weight a continuous slot |
| rosette placement (§2c) | authored per species | rides the whorl mechanism's own parameterisation |

**Mechanically this is cheap.** `LOCUS_ALLELES` goes `[2,3,3,2,2,3]` to
something like `[2,3,3,2,2,3,2,2,2]`, plus two or three appended genotype
slots. Both arrays are fixed-width, and `plant::set_seed`'s split loop
(`SEQUENCED_TRAITS`) is already the documented convention for appending
without shifting the draws that follow.

**The expensive half is that every one must pay.** A determinate plant
trades lifetime carbon for early seed. A fruit costs carbon that cannot go
into wood. A vine trades self-support — cheap wood, no cantilever
requirement — for dependence on a wall being there. Without a price,
selection collapses onto whichever allele is free and the loci degenerate
into drift; this is exactly the failure the paired
`LEAF_RATE_ALLELES`/`LEAF_TRANSPIRATION_ALLELES` construction already exists
to prevent, and `WOOD_DENSITY_ALLELES` states the same rule ("one number for
both on purpose, so tuning cannot quietly turn the trade into a free lunch").

**Why loci and not more continuous slots**: `organism.rs:1997` already
argues it and the argument is the same here — a continuous genome yields a
Gaussian cloud however hard selection pushes, and identity-level traits are
categorical. "Herb or tree" is not a scalar.

## 5. What you would actually get

**They radiate; they are not named.** With priced identity loci and one
ancestor sown broadly, the mechanism at `organism.rs:1997` — sit on a value,
spread around it, occasionally jump — should produce clumps: determinate
short-lived plants in dry and disturbed country, indeterminate tall ones
where light is the binding constraint, wall-huggers where there are walls.

Some of those will read as *"that is a sunflower-ish thing"*. None will be a
sunflower. That is the owner's framing rather than a shortfall — the
question named the three archetypes as examples explicitly.

**What stays authored even then**: the vocabulary (cell types and
behaviours) and the palettes. And the reach report's §5 caveat is untouched
— none of this makes two oaks read differently.

## 6. Three things that gate whether it works

1. **Nothing selects across archetypes today, because sowing is authored per
   niche.** `worldgen/passes.rs` (~3870–4060) assigns species to moisture
   bands, least-tolerant first. Sow a hand-authored archetype into its own
   country and it never competes for that niche — the answer selection was
   meant to find has been authored in. Radiation needs one ancestor sown
   broadly, or the niche table has to stop naming species.
2. **Generation throughput.** 4,095 concurrent organism slots
   (`world.rs:29–39`); births past it are refused and counted, not fatal.
   Woody plants over 45,000-frame runs give very few generations. The annual
   path (reach report §7 call 3; `OrganismState::senescent` and
   `remains_half_life` already exist) is what makes radiation measurable
   inside one session — which is why that report's §6 puts A2/P3 first, and
   that judgement holds.
3. **Judging it needs the presentation method the root work had to invent.**
   Within-stand spread is enormous — one measured stand ran root cells min
   90, median 438, max 1,435 — so a morphology claim compared
   stand-against-stand disappears inside within-stand variance. Grid of N
   plants per treatment, or single plants at high zoom, paired with the
   discriminating count in the card's `meta`.

## 7. Recommendation

Build the primitives by hand — unavoidable — but land each as **a locus with
a price** rather than a species default, and ship the first package with
**one** ancestral species carrying all the loci instead of three new `.ron`
files.

The acceptance artifact then is not "here is a sunflower". It is a card of
twelve siblings from one genome showing three or four visibly different
habits, with the allele histogram beside it — a claim about a *system*,
which is what the owner asked for both times this question has been put.

## 8. Open calls for the owner

1. **Does the first organ package ship as loci on one ancestor, or as new
   species files?** This note recommends loci; §3 of the reach report
   assumes files. They are not compatible defaults and the choice decides
   whether the answer to the owner's question is "evolve" or "author".
2. **Does the niche table keep naming species?** Radiation and authored
   per-niche sowing are in direct tension (§6.1). One ancestor sown broadly
   is the version that can be selected on.
