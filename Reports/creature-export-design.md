# The dev-tool exit: writing an evolved creature back out

**Status: built and landed on `claude/creature-genome-serialisation-lane-b`,
2026-08-29.** Independent of the economy spine, so nothing here waits on S6.

## 0. The gap, and how long it had been there

The owner's whole framing for evolution has an exit in it, and the exit had
no implementation. On the E5 decision card (`20260823T090318176Z-652436`,
recorded as decision **E8** in `creature-evolution-plan.md`):

> *"We do want the evolution and it may not be visible in play but we can use
> it to create new creatures that get saved and added to the game. So it can
> be used as a dev tool."*

Evolve creatures → **save the good ones** → add them to the game. Measured on
`main` at `ba6fc98`: `serde::Deserialize` was derived on every species and
genome type in the tree, and `Serialize` on **none** of them —
`grep -rn "Serialize" src/ --include=*.rs | grep -v Deserialize` returned two
comment lines and no derive. Everything in the creature line could measure,
film and rank an individual. Nothing could keep one.

## 1. What a saved creature is: a species file, not a genome sidecar

**One `assets/species/<name>.ron`, self-contained, read by the deserialiser
that was already there.** The alternative considered and rejected was a
species reference plus a genome sidecar. Three reasons, in increasing order
of importance:

- **A species file is the unit the game already adds.** The owner's
  requirement is that the result "can be added to the game"; that is one file
  in `assets/species/` plus one `include_str!` line in `organism.rs`'s
  `EMBEDDED`. A sidecar needs a pairing convention, a second loader, and a
  rule for a half-missing pair.
- **A genome is not the whole animal.** An individual owns a genome and a
  trait vector. Body plan, metabolism, nest material, dig force and sensor
  offset are its *species'*. A sidecar carrying only the genome leaves the
  new creature a dependent of the ant — which is what E8 says the result must
  not be ("new creatures must **not** look like recoloured ants").
- **It stays reviewable.** `brain.rs`'s own argument for sparse wiring lists
  is that "168 raw floats is not something anyone can review", and that does
  not stop being true because evolution wrote the numbers.

**So the export is not a new format.** `brain::wiring_from_genome` inverts
`genome_from_wiring` — a dense 584-float genome becomes the same
named-connection lists a human authors, and the existing loader expands them
back. The round trip is the deliverable; the derive is the smallest part of
it.

The house pattern is `explosion::Tuning::save` — a full `ron::ser` round trip
to a fixed asset path — and its own doc records the licence and the duty that
comes with it: a generated file has no hand-written reasoning in its comments
to destroy, and a hand-authored one does. **`save_to` therefore refuses to
overwrite anything**, so the dev tool cannot eat `ant.ron` because someone
reused a name.

## 2. The finding: a live genome block that no species file could describe

The genome has four blocks. Three of them — input→output, input→hidden,
hidden→output — have an authored form (`instincts`, `hidden_wiring`,
`hidden_outputs`). The fourth, **hidden self-recurrence** (`hh[h]`, four
slots), does not:

- `eval_brain` reads it every tick (`sum = hh[h] * state[h]`);
- `is_live_slot` counts all four slots as **live**, so `random_genome` fills
  them and S6's mutation operator, which iterates `live_slots()`, will move
  them;
- `genome_from_wiring` never writes them, and no field on `CreatureDef`
  reached them.

**Measured, and this is why it is a finding rather than a tidy-up: 10 of the
first 41 genomes `examples/creature_space.rs` samples carry a nonzero
recurrence weight — 24%.** Those are the labelled `rNNN` individuals that
sweep already ranks, one of which beats the hand-authored ant (survival
0.541 against 0.504). Exporting any of them through the three original lists
would have written a file that loads, spawns, and has **no memory** — a
different animal, with nothing on screen or in any test saying so. It was
invisible for exactly as long as nothing wrote a genome out.

The fix is a fourth list, `recurrence: [(0, 0.75)]`, empty on every shipped
species (which is precisely their current behaviour). The sensitivity control
is a test that drops it and asserts the round trip goes red —
`the_recurrence_block_is_reachable_only_through_the_fourth_list`.

**Sub-`W_EPS` weights are written out too, and that is not an oversight.** A
weight below `W_EPS` is "no connection" to `eval_brain`, so dropping it on
export looks free. It is not: mutation is partly *proportional*
(`creature-evolution-plan.md` §2.6, `width = MUT_ABS_FLOOR + MUT_REL * |w|`),
so a 0.004 weight is one birth away from being a connection and its sign is
inherited. Rounding it out would be a silent edit to the animal's
**descendants** rather than to the animal — invisible to any test written
against this generation's behaviour.

## 3. The migration question, which this work makes live

`dead-ends.md`'s entry on the 6→9 output growth carries the re-test condition
*"lawful only while nothing persists a genome (after Stage 4 any growth is a
migration)"*. **This is the change that makes something persist a genome**, so
the condition is now met and the register entry is annotated accordingly.

`SpeciesDef::genome_manifest: Option<u32>` is the answer, and its asymmetry is
the design:

- An **authored** file carries none. It is a claim about *meanings* — it names
  `Crowding` and `Turn`, so a lawful append renumbers nothing it refers to and
  an unlawful rename fails to parse. A stamp would only be a literal that goes
  stale on every legitimate scaffold change.
- An **exported** file carries `brain::genome_manifest()`. Most of it is
  name-addressed too and survives the same appends — but `hidden_wiring`,
  `hidden_outputs` and `recurrence` address hidden units by **index**, and a
  hidden unit has no name to check against. The manifest hashes `BRAIN_HIDDEN`
  and `HIDDEN_SLOTS` among its six dimensions, so a change on that axis
  becomes a refusal to load rather than a silent reinterpretation.

**Stated honestly rather than oversold: this is a provenance stamp with one
load-bearing axis, not a migration.** It cannot see a slot whose *meaning* was
redefined under an unchanged name, and it performs no conversion. What it buys
is that the day the reserve fills and `brain.rs`'s own doc says "a real
migration is needed", every exported creature in `assets/species/` says so out
loud. `SpeciesRegistry::reload` returns `SpeciesError::Parse`; `builtin()`
panics, because an embedded species that cannot be read is a broken build.

## 4. What round-trips, and what does not

**The f32 *values* round-trip exactly; the decimal *text* does not, and only
the first matters.** `ant.ron` authors `synapse_fraction: 0.0000022222222`
and an export writes `0.0000022222223` — two spellings of the same f32, since
serde emits the shortest string that parses back to the identical bits. The
guard is `assert_eq!(out.synapse_fraction, src.synapse_fraction)` on the
reloaded value, not a text diff. **Do not add a byte-comparison test over
exported RON**; it would fail for a reason that is not a defect.

**Comments do not round-trip, and cannot.** An exported file carries the
numbers and none of the reasoning, which is the same trade `explosion.rs`
made and the reason `save_to` will not overwrite.

**`ByOrder<T>` serializes a *slice*, not the array, and that one word was a
real bug.** `self.values` is a `[T; BRANCH_ORDERS]`; serde serializes a
fixed-size array as a **tuple**, which RON writes as `(0.03, 0.12, 0.2,
0.25)` — while `ByOrder`'s `Deserialize` reads a `Vec<T>` and demands a
list. So the whole *plant* half of the export produced files that would not
parse (`SpannedError { code: ExpectedArray }`), and nothing noticed.

**Why nothing noticed is the part worth keeping.** `individual_as_species`
refuses a plant, so `Behavior`, `Fate` and `ByOrder` had `Serialize` derived
and no caller — `CLAUDE.md`'s "a channel needs a writer and a reader, and the
compiler checks neither", in its worse direction. Every gate was green:
`cargo test --lib`, both clippy toolchains, `docscheck`. The bug was found by
writing the test *because* the impl had no caller, not by anything failing;
the test (`every_species_file_survives_the_serializer_most_of_them_never_asked_for`)
sweeps **every `.ron` in `assets/species/` — 11 of them** with a text fixed
point and a fate-table comparison, plus a per-tier read of the tree's
short-form root list, and it is what stops the plant half rotting again. The
sweep is over the directory rather than one file on purpose: the drift this
really guards is a lane adding a field or a type that only one species uses,
and `assets/species/` is where that arrives. Put the fault back and it goes
red — checked, not assumed. **The same shape is
one derive away in this module at any time: if a lane adds `Serialize` to a
type nothing exports, it owes it a round trip.**

`ByOrder` also writes the full `BRANCH_ORDERS` list, not the shortest form
that pads back to the same values. Re-deriving the short form is a
compression pass that can only be wrong: `[0.05]` and `[0.05, 0.05, 0.05,
0.05]` load identically *today* and would stop doing so the moment
`BRANCH_ORDERS` grows, at which point every generated file written short
would silently gain a tier.

## 5. What shipped

| | |
|---|---|
| `src/sim/species_export.rs` | new — `individual_as_species`, `organism_as_species`, `to_ron`, `save_to`/`save`, and 11 tests |
| `src/sim/brain.rs` | `Wiring`, `wiring_from_genome` (the inverse), `Recurrence`, `INPUTS`/`OUTPUTS` slot tables, `Serialize` on the wiring types, 4 tests |
| `src/sim/organism.rs` | `Serialize` across the species tree (`SpeciesDef`, `CreatureDef`, `BodyPlan`, `CellType`, `Behavior`, `Tropism`, `PaletteBands`, `FateWhen`, `Fate`) incl. a custom `ByOrder` impl; `CreatureDef::recurrence`; `SpeciesDef::genome_manifest` + `check_genome_manifest` at both load sites; `Species::cell_types()`/`fates()` |
| `examples/species_export.rs` | new — the shell-reachable exit, `from=`/`name=`/`genome=`/`gut=`/`dir=`/`verify=` |

Tests, all in-tree and green: the round trip through `Species::from`; the
round trip through `SpeciesRegistry::reload` from a **live** ant standing in a
world; every non-genome species field arriving unchanged; `eval_brain`
deciding identically over two ticks (the second reads the recurrence); the
recurrence sensitivity control; a stale manifest refused through the real
loader; an authored file accepted without one; overwrite refused; a plant
refused; name validation.

## 5b. What a species file does not carry: the material

**A creature species needs an `assets/materials/<name>.ron` of the same
name.** `creature::plant_creature_seed` resolves a body's material as
`materials.id_of(species_name)` and returns `None` if there is none — so an
exported `grazer` with no matching material hatches *nothing*, silently, and
the failure looks like "the export did not work".

Nothing here writes that file, and the reason is a design one rather than a
scope one: a material is a **palette**, and what a new creature looks like is
the one thing E8 is explicit about — *"New creatures must **not** look like
recoloured ants."* A generated palette would be precisely a recoloured ant.
So the export states the requirement and leaves the call to the owner;
`examples/species_export.rs` checks for the material and prints what to do.

The other pairing is embedding: the app's F5 reload reads
`assets/species/`, but a headless harness reads only `organism.rs`'s
`EMBEDDED` list (P-7), so a creature that is to be measured needs one
`include_str!` line.

## 5c. The one place drift will bite, and where it announces itself

`individual_as_species` writes **every `SpeciesDef` field by hand**. No
`..Default::default()`, no struct update. That is deliberate: either would
let a field added to `SpeciesDef` be silently dropped from every exported
creature — the enumeration-that-must-stay-complete failure `World::set`'s own
comment says this project keeps rediscovering. Listing them makes a new field
a **compile error in one function** with a doc comment saying what to do.

It fired within the hour: merging `main` brought six new `SpeciesDef` fields
(`flower_material`, `fruit_material`, `windfall_material`, `flower_bands`,
`fruit_bands`, `fates`) and a new type needing `Serialize`. The error named
the function, the fix was six lines, and nothing was lost.

## 6. What this deliberately does not do

- **It does not select.** Nothing decides which individual is worth saving;
  that is a run's job, and `creature_space` already ranks the sampled
  population.
- **It does not breed.** S6 does not exist, so the tests use a *synthetic*
  evolved individual — a loaded ant's genome moved by hand in the three ways
  an authored genome never exercises (a recurrence weight, a sub-`W_EPS`
  weight, a sign flip). When S6 lands the loop closes with no work here:
  `organism_as_species` already takes an `&OrganismState`.
- **It does not migrate.** See §3.
- **It writes nothing into `assets/species/` by default in CI.** The example
  takes `dir=`, and every test uses a per-test scratch directory.
