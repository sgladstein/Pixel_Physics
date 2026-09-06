//! **The dev-tool exit: an evolved individual written back out as a species
//! file the game can load.**
//!
//! `Reports/creature-evolution-plan.md` decision E8, in the owner's words:
//! *"We do want the evolution and it may not be visible in play but we can
//! use it to create new creatures that get saved and added to the game. So
//! it can be used as a dev tool."*
//!
//! Evolve creatures, save the good ones, add them to the game. Until this
//! module the last two thirds of that sentence had no implementation at
//! all: `serde::Deserialize` was derived on every species and genome type
//! in the tree and `Serialize` on none of them, so a live individual could
//! be measured, filmed and ranked, and then only died.
//!
//! # What a saved creature is: a species file, not a genome blob
//!
//! **One `assets/species/<name>.ron`, self-contained, that the existing
//! deserialiser reads with no new machinery.** Not a species plus a genome
//! sidecar. Three reasons, in increasing order of importance:
//!
//! - **The owner's requirement is that the result can be added to the
//!   game.** A species file *is* the unit the game already adds: drop it in
//!   `assets/species/`, add one `include_str!` line to `organism.rs`'s
//!   `EMBEDDED`, and the creature exists. A sidecar would need a pairing
//!   convention, a second loader, and a rule for what happens when half of
//!   the pair is missing.
//! - **A genome is not the whole animal.** An individual owns a genome and
//!   a trait vector; everything else about it — body plan, metabolism,
//!   nest material, dig force, sensor offset — lives on its species. A
//!   sidecar carrying only the genome is a creature that is still a
//!   dependent of the ant, which is exactly what E8 says the result must
//!   not be ("new creatures must not look like recoloured ants").
//! - **It stays reviewable.** `brain.rs`'s argument for sparse wiring
//!   lists is that "168 raw floats is not something anyone can review",
//!   and that does not stop being true because evolution wrote the
//!   numbers. `brain::wiring_from_genome` inverts the expansion, so what
//!   lands on disk is the same named-connection form a human authors —
//!   the owner can read what the evolved animal believes.
//!
//! # The house pattern this follows
//!
//! `explosion::Tuning::save` is the precedent: a full `ron::ser`
//! round trip to a fixed asset path. Its own doc records why it may do
//! that where `tunables::write_field_value` may not — a generated file has
//! no hand-written reasoning in its comments to destroy. Everything this
//! module writes is generated, so the same licence applies, and the same
//! duty comes with it: **never write over a hand-authored species file.**
//! `save` refuses to overwrite anything, which makes that structural
//! rather than a convention (see `SaveError::Exists`).
//!
//! # What a species file does not carry, and has to be paired with
//!
//! **A material of the same name.** `creature::plant_creature_seed`
//! resolves a body's material as `materials.id_of(species_name)`, so an
//! exported `grazer` with no `assets/materials/grazer.ron` returns `None`
//! and hatches nothing. Nothing here writes that file, deliberately: a
//! material is a palette, and what a new creature *looks like* is the one
//! thing E8 is explicit about ("new creatures must not look like
//! recoloured ants"). A generated palette would be exactly that. The
//! example binary checks and says so.
//!
//! # What is not here
//!
//! Reproduction (S6). Nothing in the engine breeds yet, so there is no
//! evolved individual to export — the round-trip tests below build one by
//! hand, mutating a loaded ant's genome and traits. That is deliberate:
//! this is the one piece of the evolution programme with no dependency on
//! the economy, so it can be finished and waiting when S6 lands.

use std::path::{Path, PathBuf};

use super::brain;
use super::organism::{OrganismState, Species, SpeciesDef, ASSET_DIR};

/// Everything that can go wrong turning an individual into a file.
#[derive(Debug)]
pub enum SaveError {
    /// The species has no `creature` block, so there is no genome to write
    /// — a plant.
    NotACreature(String),
    /// The name would not be a safe file stem. Deliberately strict: the
    /// name becomes a path, and an export is a dev tool that will be
    /// handed names from harness arguments.
    BadName(String),
    /// A file already exists at the target path. **Never overwritten**, so
    /// an export cannot destroy a hand-authored species and the comments
    /// in it.
    Exists(PathBuf),
    Serialize(String),
    Io(String),
}

impl std::fmt::Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SaveError::NotACreature(n) => write!(f, "species '{n}' has no creature block, so it has no genome to save"),
            SaveError::BadName(n) => write!(f, "'{n}' is not a usable species name: use lowercase letters, digits and underscores"),
            SaveError::Exists(p) => write!(f, "{} already exists; an export never overwrites a species file", p.display()),
            SaveError::Serialize(e) => write!(f, "serializing species: {e}"),
            SaveError::Io(e) => write!(f, "writing species: {e}"),
        }
    }
}

impl std::error::Error for SaveError {}

/// A species name that is safe to use as a file stem.
///
/// Lowercase ASCII, digits and underscores, non-empty. Not a general
/// sanitiser — a species name is also the key `SpeciesRegistry::id_of`
/// resolves and what a `.ron` writes in `nest:` and material fields, so
/// keeping it to the shape every shipped species already has is cheaper
/// than reasoning about the rest.
pub fn is_usable_name(name: &str) -> bool {
    !name.is_empty() && name.len() <= 64 && name.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

/// Build a complete [`SpeciesDef`] describing one individual.
///
/// `genome` and `traits` are the two things an individual owns that its
/// species does not; everything else is copied from `parent`. The result
/// is a new species, named `name`, whose *generation zero* is this
/// animal — which is what "added to the game" has to mean, since the game
/// spawns from species data.
///
/// **The genome round-trips through `brain::wiring_from_genome`, not
/// through a new format.** That is the whole design: the file this
/// produces is read by the deserialiser that was already there.
///
/// **Every field is written out by hand, and that is deliberate.** A
/// `..Default::default()` or a struct update here would let a field added
/// to `SpeciesDef` be silently dropped from every exported creature — the
/// enumeration-that-must-stay-complete failure `World::set`'s own comment
/// says this project keeps rediscovering. Listing them means a new field
/// is a **compile error in this function**, which is the cheapest possible
/// reminder. If you are here because of that error: copy the field from
/// `parent` unless it is something the *individual* owns.
pub fn individual_as_species(parent: &Species, genome: &[f32], traits: [f32; super::organism::CREATURE_TRAITS], name: &str) -> Result<SpeciesDef, SaveError> {
    if !is_usable_name(name) {
        return Err(SaveError::BadName(name.to_string()));
    }
    let Some(creature) = parent.creature.as_ref() else {
        return Err(SaveError::NotACreature(parent.name.clone()));
    };

    let wiring = brain::wiring_from_genome(genome);
    let mut creature = creature.clone();
    creature.traits = traits;
    creature.instincts = wiring.instincts;
    creature.hidden_wiring = wiring.hidden;
    creature.hidden_outputs = wiring.outputs;
    creature.recurrence = wiring.recurrence;

    Ok(SpeciesDef {
        name: name.to_string(),
        // Stamped, because this file's hidden-unit indices are positional
        // and a hidden unit has no name to check them against. See
        // `SpeciesDef::genome_manifest` for the full account of what this
        // does and does not catch.
        genome_manifest: Some(brain::genome_manifest()),
        foliage_bands: parent.foliage_bands,
        bark_bands: parent.bark_bands,
        stomatal_reserve: parent.stomatal_reserve,
        shoot_material: parent.shoot_material.clone(),
        root_material: parent.root_material.clone(),
        leaf_material: parent.leaf_material.clone(),
        flower_material: parent.flower_material.clone(),
        fruit_material: parent.fruit_material.clone(),
        windfall_material: parent.windfall_material.clone(),
        flower_bands: parent.flower_bands,
        fruit_bands: parent.fruit_bands,
        seed_half_life: parent.seed_half_life,
        remains_half_life: parent.remains_half_life,
        life_half_life: parent.life_half_life,
        cell_types: parent.cell_types().to_vec(),
        fates: parent.fates().to_vec(),
        creature: Some(creature),
    })
}

/// [`individual_as_species`] for a live organism — the form a harness or
/// the app calls, once something is actually alive worth saving.
///
/// `parent` must be the species `state.species` names; the caller has the
/// registry and this does not, which keeps this module out of `World`.
pub fn organism_as_species(parent: &Species, state: &OrganismState, name: &str) -> Result<SpeciesDef, SaveError> {
    individual_as_species(parent, &state.genome, state.traits, name)
}

/// Render a species def as the RON text a `.ron` file holds.
///
/// `struct_names(false)` matches `explosion::Tuning::save` and the shape
/// every hand-written species file already has — an unnamed outer tuple,
/// `(name: "ant", ...)`.
pub fn to_ron(def: &SpeciesDef) -> Result<String, SaveError> {
    let pretty = ron::ser::PrettyConfig::new().struct_names(false);
    let mut text = ron::ser::to_string_pretty(def, pretty).map_err(|e| SaveError::Serialize(e.to_string()))?;
    text.push('\n');
    Ok(text)
}

/// Where an exported species lands: `assets/species/<name>.ron`.
pub fn export_path(dir: impl AsRef<Path>, name: &str) -> PathBuf {
    dir.as_ref().join(format!("{name}.ron"))
}

/// Write the species to `dir`, returning the path written.
///
/// **Refuses to overwrite.** An export is a generated file and the
/// directory it lands in is full of hand-authored ones carrying the
/// reasoning behind every number in them; `tunables::write_field_value`
/// exists precisely because a `ron::ser` round trip destroys those
/// comments. Making the refusal structural means the dev tool cannot eat
/// `ant.ron` because someone reused a name.
pub fn save_to(def: &SpeciesDef, dir: impl AsRef<Path>) -> Result<PathBuf, SaveError> {
    if !is_usable_name(&def.name) {
        return Err(SaveError::BadName(def.name.clone()));
    }
    let path = export_path(dir, &def.name);
    if path.exists() {
        return Err(SaveError::Exists(path));
    }
    let text = to_ron(def)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| SaveError::Io(e.to_string()))?;
    }
    std::fs::write(&path, text).map_err(|e| SaveError::Io(e.to_string()))?;
    Ok(path)
}

/// [`save_to`] the engine's own asset directory.
pub fn save(def: &SpeciesDef) -> Result<PathBuf, SaveError> {
    save_to(def, ASSET_DIR)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::brain::{self, BrainInput, BrainOutput, BRAIN_HIDDEN, W_EPS};
    use crate::sim::cell::Cell;
    use crate::sim::chunk::Rect;
    use crate::sim::creature::plant_creature_seed;
    use crate::sim::material;
    use crate::sim::organism::{SpeciesRegistry, TRAIT_GUT_BIAS};
    use crate::sim::world::World;

    /// A synthetic evolved individual: the authored ant's genome, moved.
    ///
    /// **Synthetic because nothing breeds yet.** S6 is where a child takes
    /// its parent's genome jittered; until then the only honest way to
    /// test the exit is to make the changes evolution will make and check
    /// they survive. So this deliberately touches the three cases an
    /// authored genome never exercises: a weight in the hidden
    /// **self-recurrence** block, which no species file could write before
    /// `brain::Recurrence`; a weight below `W_EPS`, which is not a
    /// connection today and is one birth from being one; and a sign flip
    /// on an authored instinct.
    fn evolved_ant(base: &[f32]) -> Vec<f32> {
        let mut g = base.to_vec();
        g[brain::hh_slot(1)] = 0.75;
        g[brain::hh_slot(BRAIN_HIDDEN - 1)] = -0.4;
        g[brain::io_slot(BrainInput::Crowding, BrainOutput::Tumble)] = W_EPS * 0.4;
        g[brain::io_slot(BrainInput::TempAboveAmb, BrainOutput::Turn)] = 1.3;
        g[brain::io_slot(BrainInput::LightHere, BrainOutput::Move)] = -2.752_413;
        g[brain::ih_slot(BrainInput::Energy, 2)] = 0.031_25;
        g[brain::ho_slot(2, BrainOutput::Feed)] = -1.75;
        g
    }

    fn registry() -> SpeciesRegistry {
        SpeciesRegistry::builtin()
    }

    /// Push a def back through the loader the game actually uses, from
    /// text — `ron::from_str` then `Species::from`, which is exactly what
    /// `SpeciesRegistry::builtin` and `reload` do between them.
    fn reload_from_text(text: &str) -> Species {
        let def: SpeciesDef = ron::from_str(text).expect("exported species does not parse");
        def.check_genome_manifest().expect("exported species fails its own manifest check");
        Species::from(def)
    }

    #[test]
    fn an_exported_individual_reloads_to_the_same_animal() {
        // **The deliverable.** A live individual's genome and traits go out
        // as a species file, and the existing deserialiser reads them back
        // bit-identical. Everything else in this module is machinery for
        // this assertion.
        let reg = registry();
        let ant = reg.get(reg.id_of("ant").expect("ant species"));
        let genome = evolved_ant(&ant.genome);
        let traits = [-0.8_f32; crate::sim::organism::CREATURE_TRAITS];

        let def = individual_as_species(ant, &genome, traits, "leafcutter").expect("ant is a creature");
        let text = to_ron(&def).expect("serializes");
        let back = reload_from_text(&text);

        assert_eq!(back.name, "leafcutter");
        assert_eq!(back.genome, genome, "the reloaded genome is not the one that was written");
        let creature = back.creature.as_ref().expect("the reload kept the creature block");
        assert_eq!(creature.traits, traits, "the individual's traits did not survive");
        assert_eq!(creature.traits[TRAIT_GUT_BIAS], -0.8);
    }

    #[test]
    fn every_species_field_the_individual_did_not_own_comes_through_unchanged() {
        // The other half of "the same animal": a genome is not a creature.
        // Body plan, metabolism, nest material, dig force and sensor offset
        // are the species' and have to arrive intact, or the export is a
        // brain in a default body.
        let reg = registry();
        let ant = reg.get(reg.id_of("ant").expect("ant species"));
        let src = ant.creature.as_ref().expect("creature");
        let def = individual_as_species(ant, &ant.genome, ant.creature.as_ref().unwrap().traits, "leafcutter").expect("exports");
        let back = reload_from_text(&to_ron(&def).expect("serializes"));
        let out = back.creature.as_ref().expect("creature");

        assert_eq!(out.body.len(), src.body.len());
        assert_eq!(out.body.is_rigid(), src.body.is_rigid());
        assert_eq!(out.tick_interval, src.tick_interval);
        assert_eq!(out.start_energy, src.start_energy);
        assert_eq!(out.idle_cost_per_cell, src.idle_cost_per_cell);
        assert_eq!(out.move_cost_per_cell, src.move_cost_per_cell);
        assert_eq!(out.synapse_fraction, src.synapse_fraction, "the synapse tax is a 7-digit exponent literal; a lossy float write shows up here first");
        assert_eq!(out.body_energy, src.body_energy);
        assert_eq!(out.crop_capacity, src.crop_capacity);
        assert_eq!(out.digest_rate, src.digest_rate);
        assert_eq!(out.trait_variance, src.trait_variance);
        assert_eq!(out.climbs_over_kin, src.climbs_over_kin);
        assert_eq!(out.eats_kin, src.eats_kin);
        assert_eq!(out.nest, src.nest);
        assert_eq!(out.dig_force, src.dig_force);
        assert_eq!(out.bite_force, src.bite_force, "an unauthored bite_force must survive as None, not be written out as a number -- the Option is what keeps 'bites as hard as it digs' readable in a species file");
        assert_eq!(out.sensor_offset, src.sensor_offset);
        assert_eq!(out.sight_range, src.sight_range, "the eye is a species field and has to survive an export like every other one");
        assert_eq!(back.cell_types().len(), ant.cell_types().len());
        assert_eq!(back.remains_half_life, ant.remains_half_life);
        assert_eq!(back.life_half_life, ant.life_half_life);
        assert_eq!(back.seed_half_life, ant.seed_half_life);
        assert_eq!(back.shoot_material, ant.shoot_material);
    }

    #[test]
    fn the_reloaded_brain_decides_identically() {
        // Genome equality already implies this, so it is a positive
        // control rather than an independent claim: it says the numbers
        // being compared are the numbers a creature actually thinks with,
        // and would catch an export that round-tripped a genome nothing
        // reads.
        let reg = registry();
        let ant = reg.get(reg.id_of("ant").expect("ant species"));
        let genome = evolved_ant(&ant.genome);
        let def = individual_as_species(ant, &genome, [0.25; crate::sim::organism::CREATURE_TRAITS], "leafcutter").expect("exports");
        let back = reload_from_text(&to_ron(&def).expect("serializes"));

        // Inputs chosen so nothing is zero: an all-zero input vector
        // cannot tell two genomes apart at all.
        let mut inputs = [0.0f32; brain::BRAIN_INPUTS];
        for (i, slot) in inputs.iter_mut().enumerate() {
            *slot = 0.1 + i as f32 * 0.037;
        }
        // **Built from `BRAIN_HIDDEN`, not written out.** A literal
        // four-element array here was a compile error the moment the hidden
        // layer grew, which is the good case; the bad one is a literal that
        // happens to still be the right length and silently stops covering
        // the new units.
        let seed_state = || {
            let mut v = [0.0f32; brain::BRAIN_HIDDEN];
            for (i, slot) in v.iter_mut().enumerate() {
                *slot = 0.3 - 0.5 * (i % 4) as f32 + 0.2 * (i / 4) as f32;
            }
            v
        };
        let (mut a, mut b) = (seed_state(), seed_state());
        // Two ticks, because the second is the one that reads the
        // recurrence weights this export exists to carry.
        for _ in 0..2 {
            let (before, _) = brain::eval_brain(&genome, &inputs, &mut a);
            let (after, _) = brain::eval_brain(&back.genome, &inputs, &mut b);
            assert_eq!(before, after, "the reloaded brain does not decide what the original did");
        }
        assert_eq!(a, b, "hidden state diverged, so the recurrence weights did not survive");
    }

    #[test]
    fn dropping_the_recurrence_list_is_a_different_animal() {
        // The sensitivity control for the fourth wiring list. Without it
        // this whole module would pass its round trip and quietly discard
        // every evolved memory weight -- the file would load, the animal
        // would spawn, and only its behaviour over time would differ.
        let reg = registry();
        let ant = reg.get(reg.id_of("ant").expect("ant species"));
        let genome = evolved_ant(&ant.genome);
        let mut def = individual_as_species(ant, &genome, [0.0; crate::sim::organism::CREATURE_TRAITS], "leafcutter").expect("exports");

        assert!(!def.creature.as_ref().unwrap().recurrence.is_empty(), "the evolved individual has no recurrence to lose; this control proves nothing");
        def.creature.as_mut().unwrap().recurrence.clear();
        let back = reload_from_text(&to_ron(&def).expect("serializes"));
        assert_ne!(back.genome, genome, "the guard cannot fail: dropping the recurrence changed nothing");
    }

    #[test]
    fn a_live_individual_exports_through_the_asset_loader() {
        // The end-to-end leg: a creature that is actually standing in a
        // world, written to a directory, and read back by
        // `SpeciesRegistry::reload` -- the same call the app's F5 makes.
        // The in-memory tests above go through `Species::from`; this one
        // goes through the file.
        let mut w = World::new(Rect::new(0, 0, 199, 199));
        for x in 90..120 {
            w.set(x, 101, Cell::new(material::STONE, 0).with_attached(true));
        }
        let ant_id = w.species.id_of("ant").expect("ant species");
        let genome = evolved_ant(&w.species.get(ant_id).genome);
        w.species.set_genome(ant_id, genome.clone());

        plant_creature_seed(&mut w, 100, 100, "ant").expect("an ant hatches on bare stone");
        let organism = w.get(100, 100).organism_id();
        assert_ne!(organism, 0, "nothing was planted, so there is no individual to export");
        let traits = [0.625_f32; crate::sim::organism::CREATURE_TRAITS];
        w.organism_mut(organism).expect("the live organism").traits = traits;

        let state = w.organism(organism).expect("the live organism");
        assert_eq!(state.genome, genome, "the spawn path did not give this ant the genome under test");
        let def = organism_as_species(w.species.get(ant_id), state, "leafcutter").expect("exports");

        let dir = scratch_dir("live_export");
        let path = save_to(&def, &dir).expect("writes");
        assert_eq!(path, dir.join("leafcutter.ron"));

        let mut reg = SpeciesRegistry::builtin();
        let count = reg.reload(&dir).expect("the loader reads what the export wrote");
        assert_eq!(count, 1);
        let back = reg.get(reg.id_of("leafcutter").expect("the exported species is in the registry"));
        assert_eq!(back.genome, genome);
        assert_eq!(back.creature.as_ref().expect("creature").traits, traits);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_stale_manifest_refuses_to_load() {
        // The migration question this work makes live. `dead-ends.md`
        // records the genome layout law as "lawful only while nothing
        // persists a genome"; this module is the thing that persists one,
        // so a file written against a different brain scaffold has to fail
        // loudly rather than spawn as a fresh animal.
        let reg = registry();
        let ant = reg.get(reg.id_of("ant").expect("ant species"));
        let mut def = individual_as_species(ant, &ant.genome, [0.0; crate::sim::organism::CREATURE_TRAITS], "leafcutter").expect("exports");

        assert_eq!(def.genome_manifest, Some(brain::genome_manifest()), "an export must stamp the scaffold it was written against");
        def.check_genome_manifest().expect("its own manifest is this build's");

        def.genome_manifest = Some(brain::genome_manifest() ^ 1);
        let err = def.check_genome_manifest().expect_err("a foreign manifest loaded silently");
        assert!(err.contains("leafcutter"), "the error must name the file: {err}");

        // Through the real loader, not just the check, so the wiring
        // between the two is what is being tested.
        let text = to_ron(&def).expect("serializes");
        let dir = scratch_dir("stale_manifest");
        std::fs::create_dir_all(&dir).expect("scratch");
        std::fs::write(dir.join("leafcutter.ron"), text).expect("write");
        let mut loaded = SpeciesRegistry::builtin();
        assert!(loaded.reload(&dir).is_err(), "the loader accepted a species from another brain scaffold");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_authored_species_needs_no_manifest_and_is_not_refused() {
        // The other side of the asymmetry: every shipped `.ron` is
        // name-addressed and carries no stamp, and `builtin()` would have
        // panicked in this test's setup if the check were applied to them.
        let reg = registry();
        assert!(reg.id_of("ant").is_some());
        let def: SpeciesDef = ron::from_str(include_str!("../../assets/species/ant.ron")).expect("ant.ron parses");
        assert_eq!(def.genome_manifest, None);
        def.check_genome_manifest().expect("an authored file is not refused");
    }

    #[test]
    fn an_export_never_overwrites_a_species_file() {
        // `assets/species/` is full of hand-authored files whose comments
        // carry the reasoning behind every number in them, and a
        // `ron::ser` round trip destroys comments -- which is the whole
        // reason `tunables::write_field_value` exists. Structural rather
        // than a convention, because the name comes from a harness
        // argument.
        let reg = registry();
        let ant = reg.get(reg.id_of("ant").expect("ant species"));
        let def = individual_as_species(ant, &ant.genome, [0.0; crate::sim::organism::CREATURE_TRAITS], "leafcutter").expect("exports");
        let dir = scratch_dir("no_overwrite");
        save_to(&def, &dir).expect("the first write lands");
        let err = save_to(&def, &dir).expect_err("the second write overwrote the first");
        assert!(matches!(err, SaveError::Exists(_)), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_plant_has_no_individual_to_export_and_says_so() {
        let reg = registry();
        let tree = reg.get(reg.id_of("tree").expect("tree species"));
        // `expect_err` wants the Ok type to be `Debug` and `SpeciesDef` is
        // not, so this matches rather than unwraps.
        match individual_as_species(tree, &[], [0.0; crate::sim::organism::CREATURE_TRAITS], "notatree") {
            Err(SaveError::NotACreature(name)) => assert_eq!(name, "tree"),
            Err(e) => panic!("a tree was refused for the wrong reason: {e}"),
            Ok(_) => panic!("a tree exported as a creature"),
        }
    }

    #[test]
    fn a_name_that_is_not_a_safe_file_stem_is_refused() {
        // The name becomes a path and the caller is a dev tool taking
        // harness arguments, so this is a traversal guard as much as a
        // tidiness one.
        for bad in ["", "../ant", "Ant", "ant.ron", "a b", "ant/x"] {
            assert!(!is_usable_name(bad), "'{bad}' should not be a usable species name");
        }
        for good in ["ant", "leafcutter", "grazer_2"] {
            assert!(is_usable_name(good), "'{good}' should be a usable species name");
        }
        let reg = registry();
        let ant = reg.get(reg.id_of("ant").expect("ant species"));
        assert!(matches!(
            individual_as_species(ant, &ant.genome, [0.0; crate::sim::organism::CREATURE_TRAITS], "../ant"),
            Err(SaveError::BadName(_))
        ));
    }

    /// **Every species file in the tree, round-tripped.** The plant half,
    /// which nothing else here exercises, and the drift guard.
    ///
    /// `individual_as_species` refuses a plant, so `Behavior`, `Fate` and
    /// `ByOrder<T>` have `Serialize` derived and no caller — a channel with
    /// a writer and no reader, which `CLAUDE.md` names as the failure this
    /// project has hit three times. It is not hypothetical here: `ByOrder`
    /// shipped writing a *tuple* where its own `Deserialize` reads a
    /// **list**, so every plant produced a file that would not parse, with
    /// `cargo test --lib`, both clippy toolchains and `docscheck` green.
    ///
    /// **Over the directory rather than one file**, because the failure this
    /// is really guarding is a lane adding a field or a type to `SpeciesDef`
    /// that only one species uses. `assets/species/` is where such a thing
    /// arrives, so that is what gets swept.
    ///
    /// Two checks per file: a text **fixed point** (a second pass adds and
    /// loses nothing), and the **short-form `ByOrder`** read at every tier —
    /// `tree.ron`'s root arm authors `branch_chance: [0.0]`, one value
    /// standing for four, and an export writes all four back.
    #[test]
    fn every_species_file_survives_the_serializer_most_of_them_never_asked_for() {
        let dir = std::path::Path::new(crate::sim::organism::ASSET_DIR);
        let mut paths: Vec<_> = std::fs::read_dir(dir)
            .expect("the species asset directory")
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "ron"))
            .collect();
        paths.sort();
        assert!(paths.len() >= 8, "only {} species files found; the sweep is reading the wrong directory", paths.len());

        for path in &paths {
            let text = std::fs::read_to_string(path).expect("readable");
            let text = text.strip_prefix('\u{feff}').unwrap_or(&text).to_string();
            let name = path.display();
            let def: SpeciesDef = ron::from_str(&text).unwrap_or_else(|e| panic!("{name} does not parse: {e}"));
            let first = to_ron(&def).unwrap_or_else(|e| panic!("{name} does not serialize: {e}"));
            let again: SpeciesDef = ron::from_str(&first).unwrap_or_else(|e| panic!("{name} does not parse back after export: {e}"));
            let second = to_ron(&again).unwrap_or_else(|e| panic!("{name} does not re-serialize: {e}"));
            assert_eq!(first, second, "{name} is not a fixed point of the serializer");

            let (before, after) = (Species::from(def), Species::from(again));
            assert_eq!(before.genome, after.genome, "{name}: the genome moved");
            assert_eq!(before.leaf_material, after.leaf_material, "{name}");
            assert_eq!(before.seed_half_life, after.seed_half_life, "{name}");
            assert_eq!(before.cell_types().len(), after.cell_types().len(), "{name}");
            assert_eq!(before.fates().len(), after.fates().len(), "{name}: the fate table lost a cell type");
            for ((ct, a), (ct2, b)) in before.fates().iter().zip(after.fates()) {
                assert_eq!(ct, ct2, "{name}: the fate table reordered");
                assert_eq!(a, b, "{name}: a fate rule did not survive the round trip");
            }
        }

        // The short-form `ByOrder` list, at every tier rather than at the one
        // the file wrote: `[0.0]` means "and so on", and an export that wrote
        // the short form back would pass the fixed point above and silently
        // gain a tier the day BRANCH_ORDERS grows.
        let def: SpeciesDef = ron::from_str(include_str!("../../assets/species/tree.ron")).expect("tree.ron parses");
        let again: SpeciesDef = ron::from_str(&to_ron(&def).expect("serializes")).expect("parses back");
        let chance = |sp: &Species| -> Vec<f32> {
            let b = sp
                .behaviors(crate::sim::organism::CellType::RootTip)
                .iter()
                .find_map(|b| match b {
                    crate::sim::organism::Behavior::Grow { branch_chance, .. } => Some(*branch_chance),
                    _ => None,
                })
                .expect("the tree's root arm grows");
            (0..8).map(|order| b.at(order)).collect()
        };
        assert_eq!(chance(&Species::from(def)), chance(&Species::from(again)), "a per-branch-order list did not survive the round trip");
    }

    /// A per-test scratch directory. The test binary runs many threads at
    /// once and `reload` reads a whole directory, so two tests sharing one
    /// would see each other's files.
    fn scratch_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("pixel_physics_species_export_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        dir
    }
}
