//! **Save a creature, so it can be added to the game.**
//!
//! The command-line half of `sim::species_export` — decision E8, in the
//! owner's words: *"we can use it to create new creatures that get saved
//! and added to the game. So it can be used as a dev tool."*
//!
//! ```text
//! cargo run --release --example species_export -- from=ant name=leafcutter
//! cargo run --release --example species_export -- from=ant name=grazer genome=r041
//! cargo run --release --example species_export -- from=ant name=grazer genome=r041 gut=-1.0
//! ```
//!
//! It writes `assets/species/<name>.ron` and **never overwrites**, so it
//! cannot eat a hand-authored species file and the reasoning in its
//! comments.
//!
//! # Two things the export cannot do for you
//!
//! **A creature species needs a material of the same name.**
//! `creature::plant_creature_seed` resolves the body's material as
//! `materials.id_of(species_name)`, so a species called `grazer` with no
//! `assets/materials/grazer.ron` hatches nothing at all — it returns
//! `None`, silently. This binary checks and says so, and deliberately does
//! not write the material itself: a material is a **palette**, and what a
//! new creature looks like is exactly the thing E8 says must not come out
//! as a recoloured ant. That is the owner's call, not a serialiser's.
//!
//! **And it has to be embedded to reach a headless harness.** Add one
//! `include_str!` line to `organism.rs`'s `EMBEDDED` list; the app's F5
//! reload reads the directory, harnesses read only that list (P-7).
//!
//! # Where the genome comes from, today and after S6
//!
//! `genome=` names the individual to save:
//!
//! - `authored` (default) — the species' own instincts. Useful for
//!   producing a variant that differs only in its traits.
//! - `rNNN` — the random genome `examples/creature_space.rs` labels `rNNN`,
//!   from the same `brain::sweep_genome_seed` so the row in that sweep and
//!   the file written here are the *same animal*. That sweep already ranks
//!   400 sampled genomes and found one that beats the hand-authored ant
//!   (survival 0.541 against 0.504); until now there was no way to keep it.
//!
//! **After S6 the interesting source is a live individual from a run**, and
//! that needs no work here: `species_export::organism_as_species` already
//! takes an `&OrganismState`, and a harness that has one calls it directly.
//! This binary exists so the exit is reachable from a shell before then.
//!
//! `verify=1` (the default) reads the file back through
//! `SpeciesRegistry::reload` and asserts the genome it gets is the genome
//! it wrote. It is cheap and it is the whole claim, so it is on by default.

use pixel_physics::sim::brain;
use pixel_physics::sim::organism::{SpeciesRegistry, ASSET_DIR, CREATURE_TRAITS, TRAIT_GUT_BIAS};
use pixel_physics::sim::species_export;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let arg = |key: &str| -> Option<String> { args.iter().find_map(|a| a.strip_prefix(&format!("{key}="))).map(str::to_string) };

    let from = arg("from").unwrap_or_else(|| "ant".to_string());
    let Some(name) = arg("name") else {
        eprintln!("usage: species_export from=<species> name=<new species> [genome=authored|rNNN] [gut=<-1..1>] [dir=<path>] [verify=0]");
        std::process::exit(2);
    };
    let dir = arg("dir").unwrap_or_else(|| ASSET_DIR.to_string());
    let verify = arg("verify").map(|v| v != "0").unwrap_or(true);

    // The registry the game itself starts from, then the assets directory
    // over it if there is one -- so exporting from a species that has been
    // edited on disk but not rebuilt does the thing the caller means,
    // rather than silently exporting the compiled-in copy. That is the
    // `include_str!` gotcha aimed the other way.
    let mut registry = SpeciesRegistry::builtin();
    match registry.reload(ASSET_DIR) {
        Ok(_) => {}
        // A missing directory is the normal case for a bare checkout, but
        // a *malformed* or stale-manifest file there is not, and swallowing
        // it would silently export from the compiled-in copy of a species
        // the caller has since edited -- the `include_str!` gotcha aimed
        // the other way, which is the whole reason this reload is here.
        Err(e) if std::path::Path::new(ASSET_DIR).exists() => {
            eprintln!("could not read {ASSET_DIR}: {e}");
            eprintln!("refusing to export from the compiled-in species set while the directory is broken");
            std::process::exit(1);
        }
        Err(_) => {}
    }

    let Some(id) = registry.id_of(&from) else {
        eprintln!("no species called '{from}'");
        std::process::exit(1);
    };
    let parent = registry.get(id);

    let source = arg("genome").unwrap_or_else(|| "authored".to_string());
    let genome = match source.as_str() {
        "authored" => parent.genome.clone(),
        s if s.starts_with('r') => match s[1..].parse::<u64>() {
            Ok(index) => brain::random_genome(brain::sweep_genome_seed(index)),
            Err(_) => {
                eprintln!("'{s}' is not a creature_space genome label; they look like r041");
                std::process::exit(1);
            }
        },
        s => {
            eprintln!("unknown genome source '{s}': use 'authored' or a creature_space label like r041");
            std::process::exit(1);
        }
    };

    let mut traits = parent.creature.as_ref().map(|c| c.traits).unwrap_or([0.0; CREATURE_TRAITS]);
    if let Some(gut) = arg("gut") {
        match gut.parse::<f32>() {
            Ok(v) => traits[TRAIT_GUT_BIAS] = v.clamp(-1.0, 1.0),
            Err(_) => {
                eprintln!("gut= wants a number in -1..1");
                std::process::exit(1);
            }
        }
    }

    let def = match species_export::individual_as_species(parent, &genome, traits, &name) {
        Ok(def) => def,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    let path = match species_export::save_to(&def, &dir) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    let creature = def.creature.as_ref().expect("a creature export has a creature block");
    println!("wrote {}", path.display());
    println!(
        "  from {from}, genome {source}: {} instincts, {} hidden wires, {} hidden outputs, {} recurrence",
        creature.instincts.len(),
        creature.hidden_wiring.len(),
        creature.hidden_outputs.len(),
        creature.recurrence.len(),
    );
    println!("  traits {:?}, manifest {}", creature.traits, def.genome_manifest.expect("an export stamps its manifest"));

    // A species file alone does not make a creature that can be planted.
    // Said here rather than left to be discovered as "nothing hatches",
    // which is what the failure actually looks like.
    if registry_materials_lack(&name) {
        println!("  NOTE: no material called '{name}'. `plant_creature_seed` resolves a body's");
        println!("        material by species name, so this creature cannot be planted until");
        println!("        assets/materials/{name}.ron exists. Copy assets/materials/ant.ron and");
        println!("        give it its own palette -- what it looks like is a design call.");
    }
    println!("  to reach a headless harness, add an include_str! line to organism.rs's EMBEDDED");

    if verify {
        // Read it back through the loader the game uses, not through the
        // struct we still have in hand -- a round trip that never touches
        // the file proves nothing about the file.
        let mut check = SpeciesRegistry::builtin();
        match check.reload(&dir) {
            Ok(_) => match check.id_of(&name) {
                Some(back) if check.get(back).genome == genome => println!("  verified: the loader reads it back to the same genome"),
                Some(_) => {
                    eprintln!("  VERIFY FAILED: the loader read it back as a different genome");
                    std::process::exit(1);
                }
                None => {
                    eprintln!("  VERIFY FAILED: the loader did not find '{name}' after writing it");
                    std::process::exit(1);
                }
            },
            Err(e) => {
                eprintln!("  VERIFY FAILED: {e}");
                std::process::exit(1);
            }
        }
    }
}

/// Whether the material registry has nothing under this name.
///
/// Built fresh rather than threaded through `main`, because this runs once
/// at the end and the registry `main` holds is the *species* one.
fn registry_materials_lack(name: &str) -> bool {
    let mut materials = pixel_physics::sim::material::MaterialRegistry::builtin();
    let _ = materials.reload(pixel_physics::sim::material::ASSET_DIR);
    materials.id_of(name).is_none()
}
