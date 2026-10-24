//! **The lone predator's place in a training world** (lone predator thread, 2026-10-23): the spider
//! (`assets/species/spider.ron`) with a mind, as `examples/predtrain.rs` founds and breeds it beside a colony. Kept
//! in the library rather than the harness so a game page can stock a trained spider the same way.
//!
//! A spider's mind is the ant's design (`creature::mind`), cut to its own verbs ([`VERBS`]) and laying no scent;
//! its bite is the Attack act, which acts only under `PIXEL_PHYSICS_MIND_ATTACK=on`
//! (`creature::options::attack_on`). Off, a spider given a mind never bites.
use crate::sim::cell::OrganismId;
use crate::sim::creature::{self, mind, options::Verb};
use crate::sim::material;
use crate::sim::organism::SpeciesId;
use crate::sim::World;
use std::sync::Arc;

/// The spider's verbs: walk, turn round, rest, bite, pick up (a corpse, or anything its gut takes) and eat. No dig
/// (its jaw cannot cut soil), no put-down, no feed: it carries nothing anywhere and shares with nobody.
pub const VERBS: &[Verb] = &[Verb::Walk, Verb::TurnAround, Verb::Rest, Verb::Attack, Verb::PickUp, Verb::Eat];
/// It lays no scent.
pub const EMITS: [bool; mind::EMITS] = [false; mind::EMITS];

/// A spider's mind: `genome` with every verb but [`VERBS`] held off and every scent silent.
pub fn spider_mind(genome: &[f32]) -> Arc<mind::MindGenome> {
    let mut g = genome.to_vec();
    mind::apply_bans(&mut g, VERBS, EMITS);
    Arc::new(mind::MindGenome::new(g))
}

/// Ground as the dig arena counts it: powder or solid, not a body.
pub fn ground(w: &World, x: i32, y: i32) -> bool {
    let c = w.get(x, y);
    c.material != material::EMPTY
        && matches!(w.materials.kind(c.material), material::MaterialKind::Powder | material::MaterialKind::Solid)
        && c.organism_id() == 0
}

/// The world's box, `(min_x, min_y, max_x, max_y)` (inclusive); a bed is always bounded.
fn bounds(w: &World) -> (i32, i32, i32, i32) {
    let b = w.bounds().expect("a training bed is bounded");
    (b.min_x, b.min_y, b.max_x, b.max_y)
}

/// The first row of ground under open air in column `x`, from the top (the bed's bottom row if none): a sealed
/// box's lid is ground too, so the scan starts at the first open cell.
pub fn surface(w: &World, x: i32) -> i32 {
    let (_, y0, _, y1) = bounds(w);
    let air = (y0..=y1).find(|&y| w.get(x, y).material == material::EMPTY).unwrap_or(y1);
    (air..=y1).find(|&y| ground(w, x, y)).unwrap_or(y1)
}

/// Every living organism of species `sp`, in id order.
pub fn of_species(w: &World, sp: SpeciesId) -> Vec<OrganismId> {
    let mut v: Vec<OrganismId> = w.live_organism_ids().into_iter().filter(|&id| w.organism(id).map(|s| s.species) == Some(sp)).collect();
    v.sort_unstable();
    v
}

/// **A spider on the ground at column `x`**, or the nearest column within 12 that takes the body, in `colony`, with
/// `genome` as its mind, `grants` full tanks and its home where it landed. The first spider of a world passes
/// `None` and founds the predators' colony; the rest join it, so spiders are kin and do not bite each other.
pub fn place_spider(w: &mut World, x: i32, colony: Option<u32>, genome: Arc<mind::MindGenome>, grants: f32) -> Option<OrganismId> {
    let sp = w.species.id_of("spider")?;
    let full = grants * w.species.get(sp).creature.as_ref().map_or(0.0, |d| d.start_energy);
    let (x0, _, x1, _) = bounds(w);
    for dx in [0, 3, -3, 6, -6, 9, -9, 12, -12] {
        let cx = (x + dx).clamp(x0 + 4, x1 - 4);
        let top = surface(w, cx);
        for dy in 1..10 {
            let y = top - dy;
            if !w.in_bounds(cx, y) || w.get(cx, y).material != material::EMPTY {
                continue;
            }
            if let Some(site) = creature::plant_founder_in(w, cx, y, "spider", colony, None) {
                let crate::sim::scheduler::ActiveKind::Creature { organism } = site.kind else { continue };
                w.schedule_active_site(site);
                mind::give(w, organism, genome);
                if let Some(s) = w.organism_mut(organism) {
                    s.energy = full;
                    s.forage_anchor = (cx, y);
                }
                return Some(organism);
            }
        }
    }
    None
}

/// **A fresh ant hatched in the nest**, not on the surface: the deepest open cell within `reach` columns of
/// `nest_x` that takes the body, in `colony`, homed on `anchor`, with `genome` as its mind and `grants` full tanks.
/// **Why not at the door** (`inrun::birth_in`, which tries the rows above the door): with spiders on the bed, a
/// newborn set down on the surface is set down among them, and the first trial run fed 255 of them straight to
/// the spiders waiting at the door in 20,000 frames. A colony's young come up from below.
pub fn hatch_ant(w: &mut World, colony: u32, anchor: (i32, i32), nest_x: i32, reach: i32, genome: Arc<mind::MindGenome>, grants: f32) -> Option<OrganismId> {
    let sp = w.species.id_of("ant")?;
    let full = grants * w.species.get(sp).creature.as_ref().map_or(0.0, |d| d.start_energy);
    let (_, _, _, y1) = bounds(w);
    let mut spots: Vec<(i32, i32)> = Vec::new();
    for x in nest_x - reach..=nest_x + reach {
        let top = surface(w, x);
        for y in top + 1..y1 {
            if w.get(x, y).material == material::EMPTY && ground(w, x, y + 1) {
                spots.push((x, y));
            }
        }
    }
    // Deepest first; ties nearest the door.
    spots.sort_by_key(|&(x, y)| (-y, (x - nest_x).abs(), x));
    for (x, y) in spots {
        if let Some(site) = creature::plant_founder_in(w, x, y, "ant", Some(colony), Some(anchor)) {
            let crate::sim::scheduler::ActiveKind::Creature { organism } = site.kind else { continue };
            w.schedule_active_site(site);
            mind::give(w, organism, genome);
            if let Some(s) = w.organism_mut(organism) {
                s.energy = full;
            }
            return Some(organism);
        }
    }
    None
}
