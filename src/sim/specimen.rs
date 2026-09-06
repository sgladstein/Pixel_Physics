//! **The specimen shelf: an individual's genetics taken out of the box,
//! kept, and put back — as itself, or drifted.**
//!
//! Owner brief, 2026-08-31: *"we need to be able to save genetics of
//! creatures and animals clone them or mutate."* And the standing direction
//! this sits under, from the round-three handoff: *"Give me the tools, data,
//! access to the parameters that need to be tweaked and I do that testing
//! myself in the game. That is the game."*
//!
//! `Reports/lanes/evolution-lab-coordinator.md` names the gap this closes as
//! the first of two: **"There is no save — a parameter he changes and cannot
//! keep is a toy."** The same sentence is true of an animal. A box that
//! throws away the one good forager it produced is a slot machine, not an
//! experiment.
//!
//! # A specimen is a genetics record, not a species file
//!
//! `species_export` already writes an individual out — as a complete
//! `assets/species/<name>.ron` that the game's own loader reads. **That is a
//! different verb and both are wanted**, so it is worth saying plainly which
//! is which:
//!
//! | | `species_export` | this module |
//! |---|---|---|
//! | writes | a whole species | one individual's genome |
//! | covers | creatures only | **plants and creatures** |
//! | needs | a paired `assets/materials/<name>.ron` | nothing; it reuses its species' |
//! | reaches the world by | an `include_str!` line and a rebuild, or F5 | **being released, now, from the shelf** |
//! | on a name collision | refuses, so it cannot eat a hand-authored file | numbers up, because a jar is the player's |
//! | is for | *promoting a lineage into the game* | *keeping a specimen while you work* |
//!
//! **Size is deliberately not in that table**, though it was: a plant jar
//! measures 2,929 bytes and a *generated* ant species 2,280, so "a jar is
//! smaller" is not true and never was. The 37 KB of `assets/species/ant.ron`
//! is its comments, which a generated file has none of — comparing the two
//! measured the documentation. What actually separates the two exits is the
//! four rows above.
//!
//! The shelf is the working set and the export is the way out of the lab.
//! [`Specimen::species`] holds a **name**, resolved against the registry at
//! release, which is what lets a shelf record be small: everything about the
//! animal that is not heritable — body plan, metabolism, palette, dig force —
//! still lives on its species, where a shelf jar has no business duplicating
//! it. The cost is stated rather than hidden: **a jar whose species has been
//! renamed or removed cannot be released**, and says so
//! ([`ShelfError::NoSuchSpecies`]).
//!
//! # What "its genetics" actually is, per kingdom
//!
//! Nothing in the engine had a name for this, and the two kingdoms do not
//! agree on what a genome is:
//!
//! - **A plant** carries `genotype_draws` (ten continuous traits),
//!   `alleles` (six discrete loci that jump rather than drift) and `fates`
//!   (its production rule — the thing that makes a *growth form* evolvable
//!   at all, see [`organism::FateGenome`]). Its foliage and bark bands are
//!   *derived* from the alleles and so are not stored; its flower and fruit
//!   bands have no locus yet and are stored, because otherwise a released
//!   specimen would not be the plant you looked at.
//! - **A creature** carries `genome` (12,352 brain weights) and `traits`
//!   (gut bias, birth grant). The genome goes to disk in `brain::Wiring`'s
//!   **sparse named form**, not as 12,352 floats: `brain.rs`'s own argument
//!   is that raw weights are "not something anyone can review", and
//!   `species_export`'s round-trip test proves the form is bit-exact.
//!
//! # The dial is counted in broods, and applies the engine's own operator
//!
//! **Clone and mutate are one verb with a number on it, not two verbs**, and
//! that is `CLAUDE.md`'s first law rather than a convenience: an outcome is a
//! distribution, not a binary. A shelf offering *exactly this animal* or *a
//! fresh random one* has the same defect the old rubble had.
//!
//! So [`release`] takes `broods: u32`. **Zero is an exact clone. One is
//! precisely as different as this individual's own child would have been.**
//! Three is a great-grandchild's worth of drift. It is an integer count and
//! each brood applies the engine's real per-birth mutation **once** —
//! `brain::mutate` at the species' authored `mutation_rate` and the trait
//! jitter at its `trait_variance` for a creature; `genotype_jitter`,
//! `organism::jump_alleles` and one `FateGenome::mutate` roll for a plant.
//!
//! **No new constant is invented and none is calibrated here**, which is the
//! whole reason for the integer. A dial that scaled a rate would be a fresh
//! knob with no measured meaning, sitting next to a shared budget it
//! reallocates — the failure `CLAUDE.md` records under *"a term in a weighted
//! sum is not an independent knob"*. Applying the shipped operator n times is
//! a quantity the engine already has an answer for.
//!
//! # Growing a genome without invalidating the shelf
//!
//! `dead-ends.md` records the positional-genome law: slots are append-only,
//! and the day something persisted a genome outside the process a stale file
//! became possible. It is possible here too, and the answer is the same one
//! `species_export` reached, plus one cheap extra:
//!
//! - **Creatures carry `brain::genome_manifest()`** and [`release`] refuses a
//!   mismatch. The stamp is name-addressed on inputs and outputs and
//!   positional on hidden units — see `SpeciesDef::genome_manifest` for what
//!   that does and does not catch.
//! - **The per-slot vectors are `Vec`, not fixed arrays.** A `draws`,
//!   `alleles` or `traits` list that is *shorter* than the engine's current
//!   width loads and is padded with the species mean (0.0), which is exactly
//!   what a lawful append means: the individual predates the slot and has the
//!   ancestral value in it. A list that is *longer* is refused, because that
//!   is a file from a future build and its extra number means something this
//!   build cannot read. Fixed arrays would have turned a lawful append into
//!   "every jar on the shelf fails to parse".
//!
//! # Provenance, and why a jar remembers its parent jar
//!
//! [`Provenance`] records the frame, generation, lineage and world seed a
//! specimen was taken at, and `from_jar` — **the shelf name it was drifted
//! from**. Selection is the lab's whole premise and a shelf without pedigree
//! cannot answer *"is this line diverging"*: `OrganismState::lineage`'s own
//! doc makes the argument one level down, that a genome scatter cannot
//! separate a slot under selection from a slot nothing reads, and only who
//! descends from whom can. A shelf is where the player's *own* selection
//! decisions are recorded, so it is the one place that pedigree survives a
//! reset of the box.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::creature;
use super::brain;
use super::organism::{self, CellType, Fate, OrganismState, Species};
use super::rng::Rng;
use super::world::World;

/// Where a shelf lives by default: `assets/shelf/<jar>.ron`.
///
/// Beside `assets/species` and deliberately not in it. That directory is
/// full of hand-authored files whose comments carry the reasoning behind
/// every number in them, and a `ron::ser` round trip destroys comments —
/// the same reason `species_export::save_to` refuses to overwrite and
/// `tunables::write_field_value` exists at all. Nothing here can reach a
/// species file; promoting a jar into one is `species_export`'s job and
/// goes through its refusal.
pub const SHELF_DIR: &str = "assets/shelf";

/// Environment override for [`SHELF_DIR`].
///
/// **Three callers need it and none of them is a player.** A test that kept
/// a specimen would otherwise write into the working tree; a harness
/// comparing two racks needs two; and `/tmp` and the checkout are both
/// *shared between agents* in this project's containers, where one lane has
/// already captured another lane's screenshot
/// (`Reports/lanes/evolution-lab-coordinator.md`). One variable is cheaper
/// than threading a path through every call site, and the default is the
/// only thing the game itself ever uses.
pub const SHELF_DIR_ENV: &str = "PIXEL_PHYSICS_SHELF_DIR";

/// Where the shelf actually is, this run.
pub fn shelf_dir() -> PathBuf {
    std::env::var(SHELF_DIR_ENV).map(PathBuf::from).unwrap_or_else(|_| PathBuf::from(SHELF_DIR))
}

// ------------------------------------------------------------- the record

/// **One kept individual's heritable identity.**
///
/// Small on purpose: everything about the animal that is not heritable
/// still lives on its species. See the module doc for what that buys and
/// what it costs.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Specimen {
    /// What the player called this jar. Also its file stem, so it obeys
    /// [`is_usable_name`].
    pub name: String,
    /// The species this individual belongs to, **by name**, resolved
    /// against the registry at release.
    pub species: String,
    pub taken: Provenance,
    pub genetics: Genetics,
}

/// Where a specimen came from. Measurement and pedigree, never behaviour —
/// nothing in [`release`] reads any of it.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Provenance {
    /// The frame it was taken at.
    pub frame: u64,
    /// How many generations of breeding stood between this individual and
    /// the founder of its line, **in the box it was taken from**.
    pub generation: u16,
    /// Its `OrganismState::lineage` — which founder it descended from.
    pub lineage: u32,
    /// The world seed of the box it was taken from, so two jars from two
    /// runs can be told apart.
    pub world_seed: u64,
    /// **The jar this one was drifted from**, if it came off the shelf
    /// rather than out of the box, and how far.
    ///
    /// This is the shelf's own pedigree, and it is the reason the shelf is
    /// worth more than a folder of files: a player's selection history is
    /// otherwise unrecorded anywhere, and the box gets reset.
    pub from_jar: Option<(String, u32)>,
}

/// A plant genome or a creature genome. **There is no shared one**, and
/// pretending otherwise is what would make this module lie: the two
/// kingdoms inherit different quantities by different rules.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Genetics {
    Plant(PlantGenetics),
    Creature(CreatureGenetics),
}

impl Genetics {
    /// `"PLANT"` or `"CREATURE"`, for a readout.
    pub fn kingdom(&self) -> &'static str {
        match self {
            Genetics::Plant(_) => "PLANT",
            Genetics::Creature(_) => "CREATURE",
        }
    }
}

/// Everything a plant passes to its seed, and nothing else.
///
/// Mirrors `plant::bear_seed_at`, which is the operator of record; if the
/// two ever disagree about what is heritable, that function is right and
/// this one is stale.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PlantGenetics {
    /// `organism::GENOTYPE_TRAITS` continuous traits, each `-1..=1` as a
    /// multiple of the species' own variance width. A `Vec` so a lawful
    /// slot append does not invalidate the shelf — see the module doc.
    pub draws: Vec<f32>,
    /// `organism::DISCRETE_LOCI` jumping genes.
    pub alleles: Vec<u8>,
    /// The production rule, flattened the way `FateGenome::to_table` gives
    /// it. **Order is load-bearing** — lookup is first-match-wins.
    pub fates: Vec<(CellType, Vec<Fate>)>,
    /// **This individual's parameter overrides** — see
    /// `organism::ParamGenome`. `#[serde(default)]` so every jar written
    /// before this field existed still loads, as an empty genome, which is
    /// exactly what those specimens carried.
    #[serde(default)]
    pub params: Vec<organism::ParamOverride>,
    /// Organ colour. Stored rather than derived because these two have no
    /// locus yet (`OrganismState::flower_band`'s doc records the gap), so
    /// a released specimen that re-drew them would not be the plant the
    /// player pointed at.
    pub flower_band: u8,
    pub fruit_band: u8,
    /// What its parent provisioned it with — species plumbing today, and
    /// carried so a released specimen starts from the stake the original
    /// had rather than a founder's.
    pub endowment: f32,
    /// **`OrganismState::lineage_seed` — the developmental identity.**
    ///
    /// Under `DevelopmentalKey::Plant` this decides *which shape* a genome
    /// grows into, so a jar that stored `draws`, `alleles`, `fates` and
    /// `params` and dropped this hands the player back a plant that grows
    /// like something else — the exact failure `params`' own note records,
    /// a few fields up. **Worse than that one**, because a specimen is sown
    /// with `inherited = true` and so never draws a seed of its own: every
    /// copy released from every jar shared **0**, making a shelf of different
    /// specimens a shelf of one developmental clone.
    ///
    /// `#[serde(default)]` so jars written before this field existed still
    /// load; they load as 0, which is exactly what a plant kept before the
    /// developmental key existed actually had.
    #[serde(default)]
    pub lineage_seed: u64,
}

/// Everything a creature passes to its bud, and nothing else.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CreatureGenetics {
    /// `brain::genome_manifest()` at the moment of capture. [`release`]
    /// refuses a mismatch rather than reinterpreting the weights.
    pub manifest: u32,
    /// **The shape the manifest is a hash of**, so a *lawful append* can be
    /// recognised as one instead of refused.
    ///
    /// The manifest alone can only say equal-or-not, and the whole point of
    /// the 64-slot reserve is that appending a sense moves no existing
    /// weight — so without this every jar on the shelf dies on a change the
    /// scaffold was built to allow. See `brain::GenomeLayout`.
    ///
    /// `Option`, and absent means a jar written before this field existed:
    /// those fall back to strict manifest equality, which is what they were
    /// stored under.
    #[serde(default)]
    pub layout: Option<brain::GenomeLayout>,
    /// `organism::CREATURE_TRAITS` body traits. A `Vec`, for the same
    /// reason `PlantGenetics::draws` is.
    pub traits: Vec<f32>,
    /// The brain, in `brain::Wiring`'s sparse named form. Bit-exact
    /// through `brain::genome_from_wiring` — asserted by
    /// `species_export`'s round-trip test and again by this module's.
    pub instincts: Vec<brain::Instinct>,
    pub hidden: Vec<brain::HiddenWire>,
    pub outputs: Vec<brain::OutputWire>,
    pub recurrence: Vec<brain::Recurrence>,
}

// -------------------------------------------------------------- the errors

/// Everything that can go wrong keeping or releasing a specimen. Every
/// variant is something a player can cause, so every one has to be sayable
/// on the bar in a few words — see [`ShelfError::say`].
#[derive(Clone, Debug)]
pub enum ShelfError {
    /// Nothing alive at the cell that was clicked.
    NothingThere,
    /// The name would not be a safe file stem.
    BadName(String),
    /// A jar of that name is already on the shelf. **Never overwritten**,
    /// on `species_export::save_to`'s reasoning: a kept specimen is the
    /// one thing in the lab the player cannot regenerate.
    Exists(PathBuf),
    /// The species this jar names is not in the registry — renamed,
    /// removed, or from another build.
    NoSuchSpecies(String),
    /// The genome layout moved under a saved creature. The two manifests
    /// are `(stored, current)`.
    StaleGenome(u32, u32),
    /// The scaffold moved in a way that is **not** a lawful append -- a
    /// rename, a reorder, or a shrink. Carries the clause that failed,
    /// because "this jar predates the current brain layout" is unactionable
    /// and "output 9 was 'Feed' and is now 'Impulse'" is not.
    StaleLayout(String),
    /// A per-slot vector is longer than this build's width: a file from a
    /// future build. `(what, stored, current)`.
    FromTheFuture(&'static str, usize, usize),
    /// No room in the world at the release point, or the organism table is
    /// full.
    NoRoom,
    Serialize(String),
    Io(String),
}

impl ShelfError {
    /// One short line, upper case, for the lab's status strip.
    pub fn say(&self) -> String {
        match self {
            ShelfError::NothingThere => "NOTHING ALIVE HERE TO KEEP".into(),
            ShelfError::BadName(n) => format!("BAD JAR NAME {n}"),
            ShelfError::Exists(p) => format!("A JAR IS ALREADY CALLED {}", stem_of(p)),
            ShelfError::NoSuchSpecies(s) => format!("NO SPECIES CALLED {s} IN THIS BUILD"),
            ShelfError::StaleGenome(..) => "THIS JAR PREDATES THE CURRENT BRAIN LAYOUT".into(),
            ShelfError::StaleLayout(..) => "THE BRAIN LAYOUT MOVED UNDER THIS JAR".into(),
            ShelfError::FromTheFuture(what, ..) => format!("THIS JAR'S {what} IS FROM A NEWER BUILD"),
            ShelfError::NoRoom => "NO ROOM TO RELEASE HERE".into(),
            ShelfError::Serialize(e) => format!("COULD NOT WRITE THE JAR: {e}"),
            ShelfError::Io(e) => format!("COULD NOT WRITE THE JAR: {e}"),
        }
    }
}

fn stem_of(p: &Path) -> String {
    p.file_stem().map(|s| s.to_string_lossy().to_uppercase()).unwrap_or_default()
}

impl std::fmt::Display for ShelfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShelfError::StaleGenome(stored, current) => {
                write!(f, "genome manifest {stored:#010x} does not match this build's {current:#010x}")
            }
            ShelfError::StaleLayout(why) => write!(f, "the brain layout moved under this jar and not by appending: {why}"),
            ShelfError::FromTheFuture(what, stored, current) => {
                write!(f, "{what} has {stored} slots; this build has {current}")
            }
            _ => write!(f, "{}", self.say().to_lowercase()),
        }
    }
}

impl std::error::Error for ShelfError {}

// ------------------------------------------------------------- taking one

/// A jar name that is safe to use as a file stem.
///
/// The same rule as `species_export::is_usable_name`, and deliberately the
/// same rule rather than a looser one: a jar can be promoted into a species
/// and a species name is a registry key. Letting a jar be called `My Ant #2`
/// would move the rejection from the moment of naming to the moment of
/// promotion, which is the wrong end.
pub fn is_usable_name(name: &str) -> bool {
    !name.is_empty() && name.len() <= 64 && name.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

/// **Take the genetics of the organism at `organism_id`.**
///
/// Reads and never writes: the individual goes on living in the box. That
/// is the point — a specimen is a *copy*, so keeping one is not a decision
/// the player has to weigh against letting it breed.
pub fn capture(world: &World, organism_id: u16, name: &str) -> Result<Specimen, ShelfError> {
    if !is_usable_name(name) {
        return Err(ShelfError::BadName(name.to_string()));
    }
    let Some(state) = world.organism_state(organism_id) else {
        return Err(ShelfError::NothingThere);
    };
    let species = world.species.get(state.species);
    Ok(Specimen {
        name: name.to_string(),
        species: species.name.clone(),
        taken: Provenance {
            frame: world.frame,
            generation: state.generation,
            lineage: state.lineage,
            world_seed: world.seed,
            from_jar: None,
        },
        genetics: genetics_of(species, state),
    })
}

/// The genetics half of [`capture`], split out so the mutation path can
/// reuse it and so a test can build one without a world.
fn genetics_of(species: &Species, state: &OrganismState) -> Genetics {
    if species.creature.is_some() {
        let wiring = brain::wiring_from_genome(&state.genome);
        Genetics::Creature(CreatureGenetics {
            manifest: brain::genome_manifest(),
            layout: Some(brain::layout()),
            traits: state.traits.to_vec(),
            instincts: wiring.instincts,
            hidden: wiring.hidden,
            outputs: wiring.outputs,
            recurrence: wiring.recurrence,
        })
    } else {
        Genetics::Plant(PlantGenetics {
            draws: state.genotype_draws.to_vec(),
            alleles: state.alleles.to_vec(),
            fates: state.fates.to_table(),
            params: state.params.overrides().to_vec(),
            flower_band: state.flower_band,
            fruit_band: state.fruit_band,
            endowment: state.endowment,
            lineage_seed: state.lineage_seed,
        })
    }
}

/// **A `Cell::organism_id` at a world position, if anything alive owns
/// it.** The lab's `LOOK` and `CULL` both do this by hand; the take tool
/// needs the same answer and it is one line worth having a name.
pub fn organism_at(world: &World, x: i32, y: i32) -> Option<u16> {
    let id = world.get(x, y).organism_id();
    world.organism_state(id).map(|_| id)
}

// ----------------------------------------------------------- drifting one

/// **A copy of `spec`, drifted by `broods` generations of the engine's own
/// mutation.**
///
/// `broods == 0` returns an exact copy under the new name. See the module
/// doc for why the dial is an integer count of broods and not a rate.
///
/// `moved` in the returned [`Drifted`] is the count of genome slots that
/// actually changed — the "did it fire at all" number `CLAUDE.md` asks to
/// be printed beside the picture, and not inferable from the picture at
/// this zoom.
pub fn drift(world: &World, spec: &Specimen, broods: u32, name: &str, rng: &mut Rng) -> Result<Drifted, ShelfError> {
    if !is_usable_name(name) {
        return Err(ShelfError::BadName(name.to_string()));
    }
    let Some(species_id) = world.species.id_of(&spec.species) else {
        return Err(ShelfError::NoSuchSpecies(spec.species.clone()));
    };
    let species = world.species.get(species_id);
    let mut out = spec.clone();
    out.name = name.to_string();
    out.taken.from_jar = Some((spec.name.clone(), broods));
    let mut moved = 0;
    match (&mut out.genetics, &species.creature) {
        (Genetics::Creature(g), Some(def)) => {
            let mut genome = genome_of(g)?;
            let mut traits: [f32; organism::CREATURE_TRAITS] = padded(&g.traits, "TRAITS")?;
            for _ in 0..broods {
                moved += brain::mutate(&mut genome, def.mutation_rate, rng);
                for (slot, (t, &width)) in traits.iter_mut().zip(def.trait_variance.iter()).enumerate() {
                    if width > 0.0 {
                        // The bud path's own clamp, and it is the axis
                        // rather than a tuning choice — every slot in
                        // `CREATURE_TRAITS` is defined on `-1..=1`.
                        //
                        // **Through `creature::allele_bound`, which is the
                        // whole reason that function exists rather than a
                        // literal here.** The two arms-race slots travel as
                        // far as `World::trait_reach` says, and this is the
                        // *second* place a creature's alleles are drawn: a
                        // jar that clamped at the shared axis while a birth
                        // clamped at the reach would breed a colony the box
                        // could not have produced, in the direction that
                        // quietly disarms it.
                        let bound = creature::allele_bound(slot, world.trait_reach);
                        let next = (*t + (rng.unit_f32() * 2.0 - 1.0) * width).clamp(-bound, bound);
                        if next != *t {
                            moved += 1;
                        }
                        *t = next;
                    }
                }
            }
            let wiring = brain::wiring_from_genome(&genome);
            g.traits = traits.to_vec();
            g.instincts = wiring.instincts;
            g.hidden = wiring.hidden;
            g.outputs = wiring.outputs;
            g.recurrence = wiring.recurrence;
        }
        (Genetics::Plant(g), _) => {
            let mut draws: [f32; organism::GENOTYPE_TRAITS] = padded(&g.draws, "DRAWS")?;
            let mut alleles: [u8; organism::DISCRETE_LOCI] = padded_u8(&g.alleles, "ALLELES")?;
            let mut fates = organism::FateGenome::from_table(&g.fates);
            let mut params = organism::ParamGenome::from_overrides(&g.params);
            let species_id = world.species.id_of(&out.species);
            for _ in 0..broods {
                // **Every slot from the one stream, where `bear_seed_at`
                // splits at `SEQUENCED_TRAITS`.** That split exists because
                // its `Rng` is borrowed from a caller that goes on using
                // it, so consuming one extra draw there shifts everything
                // downstream; here the stream is built for this call and
                // dropped at the end of it, so there is nothing to protect
                // and no reason to key ten substreams. The *jitter* is the
                // shipped one either way — one `genotype_jitter`, one
                // `MUTATION_SIGMA`, so the two paths cannot drift apart.
                for d in draws.iter_mut() {
                    let next = (*d + super::plant::genotype_jitter(rng, world.mutation_sigma)).clamp(-1.0, 1.0);
                    if next != *d {
                        moved += 1;
                    }
                    *d = next;
                }
                moved += organism::jump_alleles(&mut alleles, rng);
                if rng.chance(world.fate_mutation_chance) && fates.mutate(rng).is_some_and(|m| m.applied) {
                    moved += 1;
                }
                // **A brood is one of every mutation the engine performs**,
                // so the parameter genome drifts here too or the dial would
                // mean something different from what a birth does. At the
                // shipped `param_mutation_chance` of 0.0 this is inert,
                // which is the same relationship the dial has to breeding.
                if let Some(id) = species_id {
                    if rng.chance(world.param_mutation_chance)
                        && organism::mutate_params(&mut params, &world.species, id, world.param_mutation_sigma, rng)
                    {
                        moved += 1;
                    }
                }
            }
            g.draws = draws.to_vec();
            g.alleles = alleles.to_vec();
            g.fates = fates.to_table();
            g.params = params.overrides().to_vec();
        }
        // A jar holding a creature genome whose species has since lost its
        // creature block. Nothing to mutate it with, so it is returned
        // unchanged rather than silently drifted by a plant's operator.
        (Genetics::Creature(_), None) => {}
    }
    Ok(Drifted { specimen: out, moved })
}

/// The result of [`drift`]: the new specimen, and how many genome slots
/// actually moved.
#[derive(Clone, Debug)]
pub struct Drifted {
    pub specimen: Specimen,
    pub moved: u32,
}

/// Expand a stored wiring back to a dense genome, checking the manifest
/// first.
fn genome_of(g: &CreatureGenetics) -> Result<Vec<f32>, ShelfError> {
    let current = brain::genome_manifest();
    if g.manifest != current {
        // **The manifest is the fast path, not the rule.** A mismatch means
        // the scaffold moved; it does not yet mean the jar is unreadable,
        // because the one move the scaffold is *designed* for -- appending a
        // sense or a hidden unit into the reserve -- leaves every stored
        // weight meaning exactly what it meant. `GenomeLayout::accepts` is
        // what tells the two apart, and a jar too old to carry one falls
        // back to the strict equality it was written under.
        match &g.layout {
            Some(stored) => stored.accepts().map_err(ShelfError::StaleLayout)?,
            None => return Err(ShelfError::StaleGenome(g.manifest, current)),
        }
    }
    Ok(brain::genome_from_wiring(&g.instincts, &g.hidden, &g.outputs, &g.recurrence))
}

/// A stored per-slot vector widened to this build's width.
///
/// Short is lawful and means "this individual predates the slot", so the
/// missing tail reads as the species mean. Long is refused — see the
/// module doc.
fn padded<const N: usize>(stored: &[f32], what: &'static str) -> Result<[f32; N], ShelfError> {
    if stored.len() > N {
        return Err(ShelfError::FromTheFuture(what, stored.len(), N));
    }
    let mut out = [0.0; N];
    out[..stored.len()].copy_from_slice(stored);
    Ok(out)
}

/// [`padded`] for the discrete loci, whose ancestral value is allele 0
/// rather than a zero mean.
fn padded_u8<const N: usize>(stored: &[u8], what: &'static str) -> Result<[u8; N], ShelfError> {
    if stored.len() > N {
        return Err(ShelfError::FromTheFuture(what, stored.len(), N));
    }
    let mut out = [0u8; N];
    out[..stored.len()].copy_from_slice(stored);
    Ok(out)
}

// ---------------------------------------------------------- releasing one

/// What a release put in the world.
#[derive(Clone, Debug)]
pub struct Released {
    /// The organism handle. A creature is alive immediately; a plant is a
    /// `CellType::Seed` that still has to fall and germinate, which is the
    /// same deal the `PLANT` tool offers.
    pub organism: u16,
    pub at: (i32, i32),
    /// Genome slots the dial moved on the way in — zero for a clone, and
    /// the number to print beside the picture otherwise.
    pub moved: u32,
}

/// **Put a specimen back in the box at `(x, y)`, drifted by `broods`.**
///
/// The released individual is a **founder**: a fresh lineage, generation
/// zero, and a founder's endowment. It is not anybody's child — nothing in
/// the box paid for it — so booking it as a birth would put energy in the
/// ledger that was never earned and would count a player's decision as a
/// reproductive success. What it carries from the jar is its *genome*,
/// which is the whole point.
///
/// `rng` is the caller's, and is consumed: the dial's draws and the
/// placement's shade come out of it.
pub fn release(world: &mut World, spec: &Specimen, x: i32, y: i32, broods: u32, rng: &mut Rng) -> Result<Released, ShelfError> {
    release_in(world, spec, x, y, broods, rng, None)
}

/// [`release`] into an existing colony — `Some(label)` joins it, `None`
/// founds one (`OrganismState::colony`). The lab's release verb lays a
/// colony out one station at a time and passes the first animal's label to
/// the rest, so a jar released as fifty animals is one colony and not fifty.
/// A plant ignores it: plants have no colony.
pub fn release_in(world: &mut World, spec: &Specimen, x: i32, y: i32, broods: u32, rng: &mut Rng, colony: Option<u32>) -> Result<Released, ShelfError> {
    // Drift first, against a name that is already known good, so a bad
    // dial cannot half-place an animal.
    let drifted = drift(world, spec, broods, &spec.name, rng)?;
    let moved = drifted.moved;
    match &drifted.specimen.genetics {
        Genetics::Creature(g) => {
            let genome = genome_of(g)?;
            let traits: [f32; organism::CREATURE_TRAITS] = padded(&g.traits, "TRAITS")?;
            let organism = super::creature::release_creature_specimen(world, x, y, &spec.species, genome, traits, colony).ok_or(ShelfError::NoRoom)?;
            Ok(Released { organism, at: (x, y), moved })
        }
        Genetics::Plant(g) => {
            let draws: [f32; organism::GENOTYPE_TRAITS] = padded(&g.draws, "DRAWS")?;
            let alleles: [u8; organism::DISCRETE_LOCI] = padded_u8(&g.alleles, "ALLELES")?;
            let seeded = super::plant::sow_specimen_seed(
                world,
                x,
                y,
                &spec.species,
                draws,
                alleles,
                organism::FateGenome::from_table(&g.fates),
                organism::ParamGenome::from_overrides(&g.params),
                g.flower_band,
                g.fruit_band,
                g.endowment,
                g.lineage_seed,
                rng,
            );
            let organism = seeded.ok_or(ShelfError::NoRoom)?;
            Ok(Released { organism, at: (x, y), moved })
        }
    }
}

// -------------------------------------------------------------- the shelf

/// Where a jar lands: `dir/<name>.ron`.
pub fn jar_path(dir: impl AsRef<Path>, name: &str) -> PathBuf {
    dir.as_ref().join(format!("{name}.ron"))
}

/// Render a specimen as the RON text a jar file holds.
///
/// `struct_names(false)` matches `species_export::to_ron` and every
/// hand-written asset in the tree.
pub fn to_ron(spec: &Specimen) -> Result<String, ShelfError> {
    let pretty = ron::ser::PrettyConfig::new().struct_names(false);
    let mut text = ron::ser::to_string_pretty(spec, pretty).map_err(|e| ShelfError::Serialize(e.to_string()))?;
    text.push('\n');
    Ok(text)
}

/// Write a jar to `dir`, returning the path.
///
/// **Refuses to overwrite.** A kept specimen is the one thing in the lab
/// the player cannot regenerate — the box moves on, the individual dies,
/// and there is no undo. [`next_free_name`] is how the UI avoids ever
/// asking the player to resolve this.
pub fn save_to(spec: &Specimen, dir: impl AsRef<Path>) -> Result<PathBuf, ShelfError> {
    if !is_usable_name(&spec.name) {
        return Err(ShelfError::BadName(spec.name.clone()));
    }
    let path = jar_path(&dir, &spec.name);
    if path.exists() {
        return Err(ShelfError::Exists(path));
    }
    let text = to_ron(spec)?;
    std::fs::create_dir_all(dir.as_ref()).map_err(|e| ShelfError::Io(e.to_string()))?;
    std::fs::write(&path, text).map_err(|e| ShelfError::Io(e.to_string()))?;
    Ok(path)
}

/// [`save_to`] the shelf this run is using.
pub fn save(spec: &Specimen) -> Result<PathBuf, ShelfError> {
    save_to(spec, shelf_dir())
}

/// `stem`, or the first `stem_2`, `stem_3`... that is not taken.
///
/// The shelf never overwrites, so something has to pick the next name, and
/// it should not be the player: naming a jar is the moment they are looking
/// at the animal, not at the file system.
pub fn next_free_name(dir: impl AsRef<Path>, stem: &str) -> String {
    let stem = sanitise(stem);
    if !jar_path(&dir, &stem).exists() {
        return stem;
    }
    // Bounded rather than unbounded: at 999 jars of one stem the shelf is
    // not the thing that has gone wrong, and an unbounded loop over a
    // filesystem call is a hang wearing a helpful face.
    for n in 2..1000 {
        let candidate = format!("{stem}_{n}");
        if !jar_path(&dir, &candidate).exists() {
            return candidate;
        }
    }
    format!("{stem}_full")
}

/// Force an arbitrary string into [`is_usable_name`]'s shape.
///
/// Used on species names on the way to a default jar name, never on
/// anything the player typed — a name that comes back different from what
/// was typed is worse than a refusal.
pub fn sanitise(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .map(|c| c.to_ascii_lowercase())
        .map(|c| if c.is_ascii_lowercase() || c.is_ascii_digit() { c } else { '_' })
        .take(64)
        .collect();
    if cleaned.is_empty() {
        "jar".to_string()
    } else {
        cleaned
    }
}

/// Read every jar in `dir`, newest file first.
///
/// **A jar that will not parse is skipped, not fatal.** The shelf is a
/// directory a player can put things in, and one bad file must not cost
/// them the rest of the rack. Returns the parsed specimens and the names
/// of the files that were skipped, so the UI can say how many.
pub fn load_from(dir: impl AsRef<Path>) -> (Vec<Specimen>, Vec<String>) {
    let mut jars = Vec::new();
    let mut skipped = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir.as_ref()) else {
        return (jars, skipped);
    };
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "ron"))
        .collect();
    // By name, so the rack is stable between sessions and between
    // machines. Modification time would reshuffle the shelf every time a
    // jar was rewritten, and a rack whose rows move is a rack you cannot
    // click twice.
    files.sort();
    for path in files {
        match std::fs::read_to_string(&path).ok().and_then(|t| ron::from_str::<Specimen>(&t).ok()) {
            Some(spec) => jars.push(spec),
            None => skipped.push(stem_of(&path)),
        }
    }
    (jars, skipped)
}

/// [`load_from`] the shelf this run is using.
pub fn load() -> (Vec<Specimen>, Vec<String>) {
    load_from(shelf_dir())
}

/// Take a jar off the shelf this run is using, for good.
pub fn discard(name: &str) -> Result<(), ShelfError> {
    discard_from(shelf_dir(), name)
}

/// Take a jar off a named shelf for good.
pub fn discard_from(dir: impl AsRef<Path>, name: &str) -> Result<(), ShelfError> {
    if !is_usable_name(name) {
        return Err(ShelfError::BadName(name.to_string()));
    }
    std::fs::remove_file(jar_path(dir, name)).map_err(|e| ShelfError::Io(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::cell::Cell;
    use crate::sim::chunk::Rect;
    use crate::sim::creature::plant_creature_seed;
    use crate::sim::material;
    use crate::sim::organism::{DISCRETE_LOCI, GENOTYPE_TRAITS};
    use crate::sim::rng;

    fn scratch_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("pixel_physics_shelf_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        dir
    }

    /// A world with a stone floor an ant can stand on.
    fn floored_world() -> World {
        let mut w = World::new(Rect::new(0, 0, 199, 199));
        for x in 60..160 {
            w.set(x, 101, Cell::new(material::STONE, 0).with_attached(true));
        }
        w
    }

    /// A live ant whose genome and traits have been moved off the
    /// ancestral values, so a round trip that silently substituted the
    /// species' own genome would fail rather than pass.
    fn distinctive_ant(w: &mut World) -> u16 {
        use crate::sim::brain::{self, BrainInput, BrainOutput, BRAIN_HIDDEN};
        let ant_id = w.species.id_of("ant").expect("ant species");
        let mut genome = w.species.get(ant_id).genome.clone();
        genome[brain::hh_slot(1)] = 0.75;
        genome[brain::hh_slot(BRAIN_HIDDEN - 1)] = -0.4;
        genome[brain::io_slot(BrainInput::TempAboveAmb, BrainOutput::Turn)] = 1.3;
        genome[brain::ih_slot(BrainInput::Energy, 2)] = 0.031_25;
        genome[brain::ho_slot(2, BrainOutput::Feed)] = -1.75;
        w.species.set_genome(ant_id, genome);
        plant_creature_seed(w, 100, 100, "ant").expect("an ant hatches on the stone floor");
        let id = w.get(100, 100).organism_id();
        assert_ne!(id, 0, "nothing hatched, so there is no individual to keep");
        w.organism_mut(id).expect("live ant").traits = [0.625; organism::CREATURE_TRAITS];
        id
    }

    /// A live plant whose genome has been moved off every default.
    fn distinctive_plant(w: &mut World) -> u16 {
        assert!(w.plant_tree_species(120, 100, "herb"), "the herb seed did not go in");
        let id = w.get(120, 100).organism_id();
        assert_ne!(id, 0);
        let state = w.organism_mut(id).expect("live plant");
        for (slot, d) in state.genotype_draws.iter_mut().enumerate() {
            *d = (slot as f32) / 9.0 - 0.5;
        }
        state.alleles = [1, 2, 1, 1, 1, 2];
        state.flower_band = 3;
        state.fruit_band = 2;
        state.endowment = 4.25;
        // **A distinctive plant needs a distinctive development too**, or the
        // round-trip test asserts over a field that is zero on both sides and
        // would pass with the jar dropping it -- which is how the real defect
        // reached a play test. `plant_tree_species` writes the cell without
        // drawing a genome (the lane note's finding), so this is set the way
        // `seed_genotype` would.
        state.lineage_seed = 0xC0FF_EE00_D15E_A5E5;
        id
    }

    fn shelf_rng() -> rng::Rng {
        rng::stream(0xA11CE, 7, 11, 13)
    }

    // ------------------------------------------------------ the deliverable

    #[test]
    fn a_kept_creature_comes_back_the_same_animal() {
        // **The deliverable, creature half.** Everything else in this
        // module is machinery for this assertion: what goes on the shelf
        // is what comes off it.
        let mut w = floored_world();
        let id = distinctive_ant(&mut w);
        let before = w.organism(id).expect("live ant");
        let (genome, traits) = (before.genome.clone(), before.traits);

        let spec = capture(&w, id, "keeper").expect("an ant is keepable");
        let mut r = shelf_rng();
        let out = release(&mut w, &spec, 140, 100, 0, &mut r).expect("released onto bare stone");

        assert_eq!(out.moved, 0, "a zero-brood release is a clone and must move nothing");
        let after = w.organism(out.organism).expect("the released ant");
        assert_eq!(after.genome, genome, "the released ant is not carrying the genome that was kept");
        assert_eq!(after.traits, traits, "the released ant's body traits did not survive the jar");
    }

    #[test]
    fn a_kept_plant_comes_back_the_same_plant() {
        // **The deliverable, plant half** — and the half `species_export`
        // has never been able to do at all.
        let mut w = floored_world();
        let id = distinctive_plant(&mut w);
        let before = w.organism(id).expect("live plant");
        let (draws, alleles, fates) = (before.genotype_draws, before.alleles, before.fates);
        let (flower, fruit, endowment) = (before.flower_band, before.fruit_band, before.endowment);
        let lineage_seed = before.lineage_seed;
        assert_ne!(lineage_seed, 0, "test setup: the kept plant must have a developmental identity to lose");

        let spec = capture(&w, id, "keeper").expect("a plant is keepable");
        let mut r = shelf_rng();
        let out = release(&mut w, &spec, 130, 100, 0, &mut r).expect("sown into an empty cell");

        assert_eq!(out.moved, 0);
        let after = w.organism(out.organism).expect("the sown seed");
        assert_eq!(after.genotype_draws, draws, "the continuous genome did not survive the jar");
        // **The developmental identity, and it was dropped once already.**
        // A specimen is sown with `inherited = true`, so `seed_genotype`
        // returns at the top and never draws one: without the jar carrying
        // it, every copy of every specimen came back as seed 0 -- a shelf of
        // different genomes that all grow the same shape. Reported from a
        // play test, not caught here, which is why this assertion exists.
        assert_eq!(
            after.lineage_seed, lineage_seed,
            "the developmental identity did not survive the jar -- the released plant grows like something else"
        );
        assert_eq!(after.alleles, alleles, "the discrete loci did not survive the jar");
        assert_eq!(after.fates, fates, "the production rule did not survive the jar");
        assert_eq!((after.flower_band, after.fruit_band), (flower, fruit));
        assert_eq!(after.endowment, endowment);
        assert!(after.inherited, "a released seed must be flagged inherited or `seed_genotype` redraws over the genome it was just given");
    }

    #[test]
    fn a_released_seeds_genome_survives_germination() {
        // The assertion above checks the seed. This checks the thing the
        // `inherited` flag is actually *for*: `seed_genotype` runs at
        // germination and redraws from the germination coordinate, so a
        // jar could round-trip perfectly into a seed and still be erased
        // one tick later. Calling it directly is the tightest form of the
        // question -- it is the function that would do the erasing.
        let mut w = floored_world();
        let id = distinctive_plant(&mut w);
        let spec = capture(&w, id, "keeper").expect("keepable");
        let mut r = shelf_rng();
        let out = release(&mut w, &spec, 130, 100, 0, &mut r).expect("sown");
        let sown = w.organism(out.organism).expect("the sown seed").genotype_draws;

        crate::sim::plant::seed_genotype(&mut w, out.organism, 130, 100);
        assert_eq!(w.organism(out.organism).expect("still there").genotype_draws, sown, "germination redrew the genome the jar carried");
    }

    // ------------------------------------------------------------ the dial

    #[test]
    fn the_brood_dial_moves_a_creature_and_zero_does_not() {
        // Negative and positive control in one test, which is the pairing
        // `CLAUDE.md` asks for: a dial that never fires and a dial that
        // always fires look identical from either side alone.
        let mut w = floored_world();
        let id = distinctive_ant(&mut w);
        let spec = capture(&w, id, "keeper").expect("keepable");

        let mut r = shelf_rng();
        let clone = drift(&w, &spec, 0, "clone", &mut r).expect("drifts");
        assert_eq!(clone.moved, 0, "zero broods moved a slot");

        let mut r = shelf_rng();
        let child = drift(&w, &spec, 1, "child", &mut r).expect("drifts");
        assert!(child.moved > 0, "one brood moved nothing; the dial is not connected to the genome");

        let mut r = shelf_rng();
        let great = drift(&w, &spec, 4, "great", &mut r).expect("drifts");
        assert!(great.moved > child.moved, "four broods moved no more than one; the loop is not iterating");
    }

    #[test]
    fn the_brood_dial_moves_a_plant_and_zero_does_not() {
        let mut w = floored_world();
        let id = distinctive_plant(&mut w);
        let spec = capture(&w, id, "keeper").expect("keepable");

        let mut r = shelf_rng();
        assert_eq!(drift(&w, &spec, 0, "clone", &mut r).expect("drifts").moved, 0);

        let mut r = shelf_rng();
        let child = drift(&w, &spec, 1, "child", &mut r).expect("drifts");
        // **At least** every continuous slot: `moved` also counts allele
        // jumps and an applied fate mutation, which are rare and must not
        // be asserted on -- a run in which neither fired is a legitimate
        // brood, and a test that demanded them would flake at their own
        // rates (`DISCRETE_MUTATION_CHANCE` is 0.03).
        assert!(child.moved as usize >= GENOTYPE_TRAITS, "one brood jittered {} slots, fewer than the {GENOTYPE_TRAITS} continuous ones; a loop is short", child.moved);

        // The genome actually differs, not just the counter -- and every
        // slot of it, which is the claim the counter is standing in for.
        let Genetics::Plant(a) = &spec.genetics else { panic!("plant") };
        let Genetics::Plant(b) = &child.specimen.genetics else { panic!("plant") };
        assert!(a.draws.iter().zip(b.draws.iter()).all(|(x, y)| x != y), "a continuous slot came through a brood unchanged");
    }

    #[test]
    fn a_drifted_jar_records_which_jar_it_came_from() {
        // The shelf's own pedigree. Without it a rack of twenty jars is
        // twenty unrelated animals and the player's selection history --
        // the thing the lab is for -- is nowhere.
        let mut w = floored_world();
        let id = distinctive_ant(&mut w);
        let spec = capture(&w, id, "founder0").expect("keepable");
        let mut r = shelf_rng();
        let child = drift(&w, &spec, 2, "child0", &mut r).expect("drifts").specimen;
        assert_eq!(child.taken.from_jar, Some(("founder0".to_string(), 2)));
        assert_eq!(spec.taken.from_jar, None, "a jar taken from the box has no parent jar");
    }

    // ------------------------------------------------------- release policy

    #[test]
    fn a_released_individual_is_a_founder_not_a_birth() {
        // A player's own release is not a reproductive success. If it
        // booked as a birth, every selection decision would inflate the
        // one number the lab reads to decide whether the box works.
        let mut w = floored_world();
        let id = distinctive_ant(&mut w);
        let spec = capture(&w, id, "keeper").expect("keepable");
        let births = w.creature_stats.births;
        let spawned = w.creature_stats.spawned;

        let mut r = shelf_rng();
        let out = release(&mut w, &spec, 140, 100, 0, &mut r).expect("released");
        assert_eq!(w.creature_stats.births, births, "a release was booked as a birth");
        assert_eq!(w.creature_stats.spawned, spawned + 1, "a release was not booked as a spawn");

        let state = w.organism(out.organism).expect("released");
        assert_eq!(state.generation, 0, "a released individual starts a new line at generation zero");
        assert!(state.stocked, "the release is not flagged as coming off the shelf");
        assert!(!state.inherited, "a released creature was not borne in this box");
        let parent = w.organism(id).expect("the original is still alive");
        assert_ne!(state.lineage, parent.lineage, "a release must claim its own lineage, not join the one it was copied from");
    }

    #[test]
    fn keeping_an_individual_does_not_disturb_it() {
        // A specimen is a copy. If keeping cost the player the animal, the
        // decision to keep would compete with the decision to let it
        // breed, which is not a trade this tool should impose.
        let mut w = floored_world();
        let id = distinctive_ant(&mut w);
        let before = w.organism(id).expect("live").genome.clone();
        let _ = capture(&w, id, "keeper").expect("keepable");
        assert_eq!(w.organism(id).expect("still alive").genome, before);
    }

    /// **A released animal takes ticks.** Owner, from play: *"if you copy an
    /// ant to the jar and then try to place a clone of it, it just gets stuck
    /// in midair (or wherever you click) but it cannot move."*
    ///
    /// `release_creature_specimen` built the body, took the slot and handed
    /// back the site its first tick had to be booked at -- and threw the site
    /// away. Every other placement path in the engine schedules it. Nothing
    /// went red: the animal was in the world, in the organism table and in the
    /// census, and it never took a tick. Gravity is part of a creature's own
    /// tick rather than a separate pass, so an unscheduled one does not even
    /// fall.
    ///
    /// **Paired against a founder of the same species dropped from the same
    /// height**, because a bare "it moved" assertion cannot tell a fixed
    /// release from a world where nothing falls at all -- and the founder path
    /// was never broken, so it is the positive control this needs.
    #[test]
    fn a_released_animal_falls_like_a_founder_does() {
        let mut w = floored_world();
        let id = distinctive_ant(&mut w);
        let spec = capture(&w, id, "keeper").expect("keepable");

        // Twenty rows of air over the floor at y == 101, well clear of the
        // ant `distinctive_ant` left standing on it.
        const DROP: i32 = 81;
        let mut r = shelf_rng();
        let released = release(&mut w, &spec, 140, DROP, 0, &mut r).expect("released").organism;
        let control = {
            let site = plant_creature_seed(&mut w, 150, DROP, "ant").expect("a founder hatches in mid-air");
            let organism = w.get(150, DROP).organism_id();
            w.schedule_active_site(site);
            organism
        };

        let lowest = |w: &World, id: u16| {
            w.organism(id).expect("alive").cells.keys().map(|(_, y)| *y).max().expect("a body has cells")
        };
        assert_eq!(lowest(&w, released), DROP, "the release did not start where it was put");

        let mut particles = crate::sim::particle::ParticleSystem::default();
        let mut blasts = crate::sim::explosion::Blasts::default();
        let tuning = crate::sim::player::Tuning::default();
        for _ in 0..400 {
            crate::sim::frame::step(&mut w, &mut particles, &mut blasts, crate::sim::player::PlayerInput::default(), &tuning);
        }

        let control_fell = lowest(&w, control) - DROP;
        assert!(control_fell > 0, "the control founder did not fall either, so this world cannot answer the question");
        assert!(
            lowest(&w, released) - DROP > 0,
            "a released animal did not move at all in 400 frames while a founder dropped from the same height fell {control_fell} rows -- it is in the world and off the schedule",
        );
    }

    #[test]
    fn a_release_with_no_room_says_so_rather_than_half_placing() {
        let mut w = floored_world();
        let id = distinctive_ant(&mut w);
        let spec = capture(&w, id, "keeper").expect("keepable");
        let mut r = shelf_rng();
        // Straight into the stone floor.
        let out = release(&mut w, &spec, 100, 101, 0, &mut r);
        assert!(matches!(out, Err(ShelfError::NoRoom)), "a blocked release must refuse, got {out:?}");
    }

    // ------------------------------------------------------------ the file

    #[test]
    fn a_jar_round_trips_through_ron() {
        let mut w = floored_world();
        let id = distinctive_ant(&mut w);
        let spec = capture(&w, id, "keeper").expect("keepable");
        let text = to_ron(&spec).expect("serializes");
        let back: Specimen = ron::from_str(&text).expect("a jar this module wrote does not parse");

        let Genetics::Creature(a) = &spec.genetics else { panic!("creature") };
        let Genetics::Creature(b) = &back.genetics else { panic!("creature") };
        assert_eq!(genome_of(a).expect("expands"), genome_of(b).expect("expands"), "the brain did not survive the file");
        assert_eq!(a.traits, b.traits);
        assert_eq!(back.species, "ant");
    }

    #[test]
    fn a_plant_jar_round_trips_through_ron() {
        let mut w = floored_world();
        let id = distinctive_plant(&mut w);
        let spec = capture(&w, id, "keeper").expect("keepable");
        let back: Specimen = ron::from_str(&to_ron(&spec).expect("serializes")).expect("parses");
        let Genetics::Plant(a) = &spec.genetics else { panic!("plant") };
        let Genetics::Plant(b) = &back.genetics else { panic!("plant") };
        assert_eq!(a.draws, b.draws);
        assert_eq!(a.alleles, b.alleles);
        assert_eq!(a.fates, b.fates, "the production rule did not survive the file");
    }

    #[test]
    fn the_shelf_never_overwrites_a_jar() {
        let mut w = floored_world();
        let id = distinctive_ant(&mut w);
        let spec = capture(&w, id, "keeper").expect("keepable");
        let dir = scratch_dir("no_overwrite");
        save_to(&spec, &dir).expect("first write");
        assert!(matches!(save_to(&spec, &dir), Err(ShelfError::Exists(_))), "a second write overwrote a kept specimen");
        assert_eq!(next_free_name(&dir, "keeper"), "keeper_2");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_shelf_loads_what_it_saved_and_skips_what_it_cannot_read() {
        let mut w = floored_world();
        let ant = distinctive_ant(&mut w);
        let plant = distinctive_plant(&mut w);
        let dir = scratch_dir("load");
        save_to(&capture(&w, ant, "one").expect("keepable"), &dir).expect("writes");
        save_to(&capture(&w, plant, "two").expect("keepable"), &dir).expect("writes");
        // A file a player dropped in that is not a jar. One bad file must
        // not cost the rest of the rack.
        std::fs::write(dir.join("three.ron"), "this is not a specimen").expect("writes");

        let (jars, skipped) = load_from(&dir);
        assert_eq!(jars.len(), 2, "the shelf lost a jar it wrote");
        assert_eq!(jars[0].name, "one", "the rack is not in a stable order");
        assert_eq!(skipped, vec!["THREE".to_string()]);

        discard_from(&dir, "one").expect("discards");
        assert_eq!(load_from(&dir).0.len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    // --------------------------------------------------------- the refusals

    #[test]
    fn a_stale_brain_layout_refuses_rather_than_reinterpreting() {
        // `dead-ends.md` records the genome layout law and the day it
        // became possible for a stale file to exist. A jar is a second
        // such file. Reinterpreting 12,352 weights against a moved slot
        // map produces a plausible animal that is not the one that was
        // saved, and nothing on screen would say so.
        //
        // **Two refusals, because since 2026-09-02 two different things can
        // decide.** A jar old enough to carry no `layout` is judged on the
        // manifest it was stored under, which is strict equality; a jar that
        // carries one is judged on whether the scaffold *appended* or moved.
        // The test exercises both, or it would go on passing while covering
        // only the path that no longer runs.
        let mut w = floored_world();
        let id = distinctive_ant(&mut w);

        // 1. No layout, wrong manifest -- the old contract, unchanged.
        let mut old_jar = capture(&w, id, "keeper").expect("keepable");
        let Genetics::Creature(g) = &mut old_jar.genetics else { panic!("creature") };
        g.manifest ^= 1;
        g.layout = None;
        let mut r = shelf_rng();
        assert!(matches!(release(&mut w, &old_jar, 140, 100, 0, &mut r), Err(ShelfError::StaleGenome(..))));

        // 2. A layout whose names are not a prefix of this build's -- a
        // rename or a reorder, which is the case the manifest exists for and
        // the one the prefix rule must still refuse.
        let mut moved = capture(&w, id, "keeper2").expect("keepable");
        let Genetics::Creature(g) = &mut moved.genetics else { panic!("creature") };
        g.manifest ^= 1;
        let mut l = brain::layout();
        l.input_names[3] = "SomethingElse".into();
        g.layout = Some(l);
        assert!(matches!(release(&mut w, &moved, 150, 100, 0, &mut r), Err(ShelfError::StaleLayout(..))));
    }

    #[test]
    fn a_jar_stored_before_a_sense_and_a_hidden_unit_were_appended_still_loads() {
        // **The migration this design is for, on the brain axis.** Appending
        // a sense or a hidden unit is lawful -- the reserve is 64 wide on
        // every axis and no stored weight moves -- but the manifest is one
        // `u32` and can only say equal-or-not, so before `GenomeLayout` a
        // lawful append killed every specimen on the shelf. The owner ruled
        // the shelf expendable; this is it not needing to be.
        //
        // The stored layout is this build's with the *last* input, the last
        // output and half the hidden units removed, which is exactly the
        // shape of a jar written before an append on all three axes.
        let mut w = floored_world();
        let id = distinctive_ant(&mut w);
        let mut spec = capture(&w, id, "keeper").expect("keepable");
        let Genetics::Creature(g) = &mut spec.genetics else { panic!("creature") };
        let mut l = brain::layout();
        l.input_names.pop();
        l.output_names.pop();
        l.hidden /= 2;
        g.layout = Some(l);
        // The manifest is deliberately left wrong, because that is the whole
        // point: an append always changes it, and the layout is what says
        // the change was an append.
        g.manifest ^= 1;

        let mut r = shelf_rng();
        let released = release(&mut w, &spec, 140, 100, 0, &mut r);
        assert!(released.is_ok(), "a jar from before a lawful append must still load, got {released:?}");
    }

    #[test]
    fn a_jar_from_before_a_slot_was_appended_still_loads() {
        // The migration this design is *for*. Appending a genome slot is
        // lawful; making every jar on the shelf unreadable is not. A short
        // vector reads as "this individual predates the slot", so the
        // missing tail is the species mean.
        let mut w = floored_world();
        let id = distinctive_plant(&mut w);
        let mut spec = capture(&w, id, "keeper").expect("keepable");
        let Genetics::Plant(g) = &mut spec.genetics else { panic!("plant") };
        g.draws.truncate(GENOTYPE_TRAITS - 2);
        g.alleles.truncate(DISCRETE_LOCI - 1);
        let kept = g.draws.clone();

        let mut r = shelf_rng();
        let out = release(&mut w, &spec, 130, 100, 0, &mut r).expect("an older jar still releases");
        let after = w.organism(out.organism).expect("sown");
        assert_eq!(&after.genotype_draws[..kept.len()], &kept[..], "the slots the jar did have were not carried");
        assert_eq!(after.genotype_draws[GENOTYPE_TRAITS - 1], 0.0, "an appended slot must read as the species mean, not as noise");
    }

    #[test]
    fn a_jar_from_a_newer_build_refuses() {
        // The other direction, and it must not be padded away: an extra
        // number means something this build cannot read.
        let mut w = floored_world();
        let id = distinctive_plant(&mut w);
        let mut spec = capture(&w, id, "keeper").expect("keepable");
        let Genetics::Plant(g) = &mut spec.genetics else { panic!("plant") };
        g.draws.push(0.5);

        let mut r = shelf_rng();
        assert!(matches!(release(&mut w, &spec, 130, 100, 0, &mut r), Err(ShelfError::FromTheFuture("DRAWS", ..))));
    }

    #[test]
    fn a_jar_naming_a_species_this_build_does_not_have_says_so() {
        let mut w = floored_world();
        let id = distinctive_ant(&mut w);
        let mut spec = capture(&w, id, "keeper").expect("keepable");
        spec.species = "hoverfly".to_string();
        let mut r = shelf_rng();
        let out = release(&mut w, &spec, 140, 100, 0, &mut r);
        assert!(matches!(out, Err(ShelfError::NoSuchSpecies(_))), "got {out:?}");
    }

    #[test]
    fn a_jar_name_has_to_be_a_safe_file_stem() {
        let mut w = floored_world();
        let id = distinctive_ant(&mut w);
        assert!(matches!(capture(&w, id, "My Ant #2"), Err(ShelfError::BadName(_))));
        assert!(matches!(capture(&w, id, ""), Err(ShelfError::BadName(_))));
        assert!(capture(&w, id, "ant_2").is_ok());
        // `sanitise` is for species names on the way to a default, never
        // for what the player typed.
        assert_eq!(sanitise("My Ant #2"), "my_ant__2");
        assert_eq!(sanitise(""), "jar");
    }

    #[test]
    fn nothing_alive_is_not_keepable() {
        let w = floored_world();
        assert!(matches!(capture(&w, 0, "keeper"), Err(ShelfError::NothingThere)));
        assert!(organism_at(&w, 5, 5).is_none());
    }
}
