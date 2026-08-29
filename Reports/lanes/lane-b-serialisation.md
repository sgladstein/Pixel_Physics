# Lane B — the dev-tool exit (genome serialisation)

Branch `claude/creature-genome-serialisation-lane-b`. Head SHA is at the
bottom of this file and is how the coordinator finds the work; the PR body
is `Reports/lanes/lane-b-pr-body.md` on the same branch.

## 2026-08-29 — the exit is built, and it found a live genome block nothing could describe

**Shipped.** An evolved individual can be written back out as an
`assets/species/<name>.ron` that the existing loader reads to the same
animal. Decision **E8** — *"we can use it to create new creatures that get
saved and added to the game"* — had no implementation at all: `Serialize`
was derived on nothing in the species tree. Full account, with the design
calls, in `Reports/creature-export-design.md`. Gates green:
`cargo test --lib` 1,001 passed / 0 failed / 54 ignored, clippy on both the
container's 1.94.1 and CI's 1.98.0, `docscheck` clean.

### → coordinator: the number worth carrying

**10 of the first 41 genomes `creature_space` samples carry a nonzero
hidden self-recurrence weight — 24%.** That block (`hh[h]`, 4 slots) is read
by `eval_brain` every tick and counted live by `is_live_slot`, so
`random_genome` fills it and **S6's mutation operator will move it** — and
until this branch there was no authored form that could write it. The three
existing wiring lists (`instincts`, `hidden_wiring`, `hidden_outputs`) reach
the other three blocks and not that one.

Exporting such an animal through the old lists produces a file that loads,
spawns, and has **no memory**. Nothing on screen or in any test would have
said so. Fixed with a fourth list, `recurrence: [(0, 0.75)]`, empty on every
shipped species (which is exactly their current behaviour, so nothing moves).

**What this costs you at S6:** one line. `genome_from_wiring` now takes a
fourth slice; if your mutation operator iterates `live_slots()` — which
`brain.rs`'s own doc requires — it already covers the recurrence block and
you need do nothing. If it iterates the three lists instead, it is skipping
four heritable weights.

### → coordinator: the migration condition is now met

`dead-ends.md`'s entry on the unlawful 6→9 output growth carried the re-test
condition *"lawful only while nothing persists a genome (after Stage 4 any
growth is a migration)"*. **This branch is the thing that persists one.** The
entry is annotated in place (not duplicated); `bugindex.py --check` clean.

The layout law itself is unchanged and the reserved-slot layout still makes
an append lawful in all three directions. What changed is that a *stale file*
can now exist. `SpeciesDef::genome_manifest: Option<u32>` stamps
`brain::genome_manifest()` on every export and both load sites refuse a
mismatch — `reload` returns `SpeciesError::Parse`, `builtin()` panics.

**Stated at its real strength rather than its advertised one:** an exported
file is *mostly* name-addressed and would survive a lawful append without any
stamp at all. The stamp is load-bearing on exactly one axis — `hidden_wiring`,
`hidden_outputs` and `recurrence` index hidden units **positionally**, and a
hidden unit has no name to check against. It cannot see a slot whose meaning
was redefined under an unchanged name, and it performs no conversion. Do not
cite it as a migration.

### → any lane touching `assets/species/*.ron` or `CreatureDef`

Two things that will surprise you:

- **`CreatureDef` has a fourth wiring list**, `recurrence`, `#[serde(default)]`
  and empty everywhere today. `genome_from_wiring` takes it as a fourth
  argument; six call sites updated, two of them in `creature.rs` tests.
- **f32 *values* round-trip exactly; the decimal *text* does not.** `ant.ron`
  authors `synapse_fraction: 0.0000022222222` and an export writes
  `0.0000022222223` — two spellings of the same f32, since serde emits the
  shortest string that parses back to identical bits. Guards compare the
  reloaded *value*. **Do not add a byte-comparison test over exported RON**;
  it would fail for a reason that is not a defect.

### → any lane: a derive with no caller is still a broken channel

`ByOrder<T>`'s `Serialize` shipped writing a **tuple** where its own
`Deserialize` reads a **list** — serde serializes `[T; N]` as a tuple, and
RON spells that `(a, b, c, d)`. Every plant the export touched produced a
file that would not parse, and **every gate was green**: `cargo test --lib`,
both clippy toolchains, `docscheck`. It was invisible because
`individual_as_species` refuses a plant, so the impl had no caller — the
compiler checks neither end of a channel, and this is the writer-with-no-
reader case.

Found by writing the test *because* the impl had no caller. One word
(`self.values.as_slice()`). The guard now sweeps **every `.ron` in
`assets/species/`** — put the fault back and it goes red, checked rather
than assumed. If your lane adds `Serialize` to a type nothing exports yet,
it owes that type a round trip in the same change.

### → the plant lane specifically

Merged your organs/fates work in. Two things it now carries from here:

- **`Fate` and `FateWhen` have `Serialize` derived**, and `Species` has a
  `fates()` accessor beside `cell_types()`. Both exist only for the export.
- **`individual_as_species` writes every `SpeciesDef` field by hand** — no
  `..Default::default()` — so **a field you add to `SpeciesDef` is a compile
  error in `src/sim/species_export.rs`**, with a doc comment saying what to
  do (copy it from `parent` unless the individual owns it). That is the
  intended behaviour, not an obstacle: the alternative silently drops your
  field from every exported creature. Your six new fields cost six lines.

### → coordinator: "added to the game" needs one thing the export cannot write

A creature species needs an `assets/materials/<name>.ron` of the **same
name** — `plant_creature_seed` resolves the body material by species name and
returns `None` without one, so an exported creature with no material hatches
*nothing*, silently. The example checks and prints what to do.

Not written automatically on purpose: a material is a palette, and E8 says
new creatures "must **not** look like recoloured ants", which a generated
palette would be. That is an owner call, and if you want it made it wants a
review card, not a lane.

### Files touched, so you can see the collision surface

`src/sim/species_export.rs` (new), `examples/species_export.rs` (new),
`src/sim/brain.rs`, `src/sim/organism.rs`, `src/sim/mod.rs`,
`examples/ant_ablation.rs`, plus docs. **`src/sim/creature.rs` is touched on
two lines only**, both inside `#[cfg(test)]`, both the mechanical fourth
argument — it is the 39-landing hotspot and I stayed out of it.

### Two small things fixed in passing, both flagged rather than assumed

- `Reports/instruments.md` said **31** `examples/` binaries against an actual
  **35** — stale before this branch. Recounted and dated in place.
- `Reports/lanes/docs-audit.md` is 47,168 B against the 12,000 B cap.
  `lanecheck` warns and by protocol only its owner may trim it, so this is a
  pointer, not an action.

### What I did not do

- **No `scene=colony` measurement and no review card from it**, per the
  brief — §R and §R2 are the coordinator's. Nothing here needed a scene:
  the export's claim is bit-identity through the loader, which is a test
  result, not a judge-by-eye question. Nothing about the game's appearance
  changes on this branch.
- **No selection and no breeding.** Nothing decides which individual is worth
  saving. The tests use a *synthetic* evolved individual — a loaded ant's
  genome moved by hand in the three ways an authored genome never exercises
  (a recurrence weight, a sub-`W_EPS` weight, a sign flip) — because S6 does
  not exist. When it lands, `species_export::organism_as_species` already
  takes an `&OrganismState` and the loop closes with no work here.

---

**Head SHA:** _pending — set in the final commit on this branch._
