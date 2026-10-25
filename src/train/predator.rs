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
/// **The nest raider's verbs** (`centipede`, 2026-10-25): the spider's, and Dig and Put Down. Its jaw cannot cut soil;
/// what it can cut is a colony's stored food, which it cannot eat (`CreatureDef::digs_inedible_food`), and the dig
/// packs the cut behind its tail when it is walled in (`options::pack_on`), so Dig is how it shoves through a
/// food-packed nest. Put Down for the pellet in its jaws when it is not.
pub const RAIDER_VERBS: &[Verb] = &[Verb::Walk, Verb::TurnAround, Verb::Rest, Verb::Attack, Verb::PickUp, Verb::Eat, Verb::Dig, Verb::Drop];

/// The verbs of a predator `species`: the raider's for the `centipede`, the spider's for any other.
pub fn verbs_for(species: &str) -> &'static [Verb] {
    if species == "centipede" {
        RAIDER_VERBS
    } else {
        VERBS
    }
}

/// A predator's mind: `genome` with every verb but `species`' ([`verbs_for`]) held off and every scent silent.
pub fn mind_for(species: &str, genome: &[f32]) -> Arc<mind::MindGenome> {
    let mut g = genome.to_vec();
    mind::apply_bans(&mut g, verbs_for(species), EMITS);
    Arc::new(mind::MindGenome::new(g))
}

/// It lays no scent.
pub const EMITS: [bool; mind::EMITS] = [false; mind::EMITS];

/// A spider's mind: `genome` with every verb but [`VERBS`] held off and every scent silent.
pub fn spider_mind(genome: &[f32]) -> Arc<mind::MindGenome> {
    let mut g = genome.to_vec();
    mind::apply_bans(&mut g, VERBS, EMITS);
    Arc::new(mind::MindGenome::new(g))
}

/// **The spider's breeding genes**, `[birth_grant, reproduce_at]` (`organism::TRAIT_BIRTH_GRANT`,
/// `TRAIT_REPRODUCE_AT`): how big a child's start is, and how rich a spider waits to be before it buds. Owner,
/// 2026-10-25: *"Breeding parameters should be part of training."* A pool mind `NN.raw.txt` carries them in
/// `NN.body.txt` beside it ([`read_breeding`]); a mind with none takes its species' own. They act only where spiders
/// breed by the game's rule -- the lab (`examples/spiderlab.rs`, colsel `spiderlab=`) -- because the training bed
/// refills its spiders and never lets one bud. **Why they had to be carried:** in the lab a population booms off a
/// colony and starves out (`Reports/lone-predator-2026-10-24.md`, stage four), and how fast a spider breeds is the
/// half of that a mind cannot change.
pub const BREEDING: [usize; 2] = [crate::sim::organism::TRAIT_BIRTH_GRANT, crate::sim::organism::TRAIT_REPRODUCE_AT];

/// The breeding genes file beside a pool mind: `k001.raw.txt` -> `k001.body.txt`.
pub fn body_path(mind: &std::path::Path) -> std::path::PathBuf {
    let s = mind.to_string_lossy();
    std::path::PathBuf::from(s.strip_suffix(".raw.txt").map_or_else(|| format!("{s}.body.txt"), |b| format!("{b}.body.txt")))
}

/// The breeding genes saved beside `mind` (`birth_grant=X reproduce_at=Y`), or `None` if it has none.
pub fn read_breeding(mind: &std::path::Path) -> Option<[f32; 2]> {
    let text = std::fs::read_to_string(body_path(mind)).ok()?;
    let get = |k: &str| text.split_whitespace().find_map(|t| t.strip_prefix(&format!("{k}=")).and_then(|v| v.parse().ok()));
    Some([get("birth_grant")?, get("reproduce_at")?])
}

/// Save `genes` beside `mind`, as [`read_breeding`] reads them.
pub fn save_breeding(mind: &std::path::Path, genes: [f32; 2]) -> std::io::Result<()> {
    std::fs::write(body_path(mind), format!("birth_grant={} reproduce_at={}\n", genes[0], genes[1]))
}

/// A living spider's breeding genes.
pub fn breeding_of(w: &World, id: OrganismId) -> Option<[f32; 2]> {
    w.organism(id).map(|s| BREEDING.map(|slot| s.traits[slot]))
}

/// Give a living spider these breeding genes.
pub fn set_breeding(w: &mut World, id: OrganismId, genes: [f32; 2]) {
    if let Some(s) = w.organism_mut(id) {
        for (slot, g) in BREEDING.iter().zip(genes) {
            s.traits[*slot] = g;
        }
    }
}

/// **The breeding genes of a placed child**: each takes one uniform step of up to the species' own width for that
/// slot, inside its bound -- `creature::mutate_traits`'s rule, cut to these two slots, so a harness's mutant moves them
/// as far as a birth in the game would. `unit` is a draw in `0..1`.
pub fn mutate_breeding(w: &World, genes: [f32; 2], unit: impl FnMut() -> f32) -> [f32; 2] {
    mutate_breeding_of(w, "spider", genes, unit)
}

/// [`mutate_breeding`] by `species`' own widths, for any predator.
pub fn mutate_breeding_of(w: &World, species: &str, genes: [f32; 2], mut unit: impl FnMut() -> f32) -> [f32; 2] {
    let Some(def) = w.species.id_of(species).and_then(|sp| w.species.get(sp).creature.as_ref()) else {
        return genes;
    };
    let mut out = genes;
    for (g, &slot) in out.iter_mut().zip(BREEDING.iter()) {
        let width = creature::trait_width(def, slot);
        if width > 0.0 {
            let bound = creature::allele_bound(slot, w.trait_reach);
            *g = (*g + (unit() * 2.0 - 1.0) * width).clamp(-bound, bound);
        }
    }
    out
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
    place_predator(w, "spider", x, colony, genome, grants)
}

/// [`place_spider`] for any predator species: the spider, or the nest raider (`centipede`, 2026-10-25). Both take
/// the spider's verbs ([`VERBS`]) and lay no scent; what differs is the body and the numbers in the species file.
pub fn place_predator(w: &mut World, species: &str, x: i32, colony: Option<u32>, genome: Arc<mind::MindGenome>, grants: f32) -> Option<OrganismId> {
    let sp = w.species.id_of(species)?;
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
            if let Some(site) = creature::plant_founder_in(w, cx, y, species, colony, None) {
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

/// **One picture of a predator's world**, for a harness's GIF (`examples/predtrain.rs`, `examples/spiderlab.rs`):
/// the cells of `crop` `(x, y, w, h)` at `scale` pixels a cell. Ants yellow, the predator red, the sky pale blue and
/// **open cells under the ground dark**, so a nest's tunnels read against the soil and a raider can be seen going
/// in (nest raider thread, 2026-10-25; the first pictures drew a tunnel the colour of the sky and ants near-black,
/// which hid both underground). A white ring for 120 frames where an ant was killed: a one-cell event is
/// unreadable without one.
#[allow(clippy::too_many_arguments)]
pub fn picture(w: &World, (x0, y0, cw, ch): (i32, i32, i32, i32), scale: u32, kills: &[(u64, (i32, i32))], frame: u64, ant: SpeciesId, pred: SpeciesId) -> image::RgbaImage {
    let mut img = image::RgbaImage::new(cw as u32 * scale, ch as u32 * scale);
    let tops: Vec<i32> = (0..cw).map(|cx| if w.in_bounds(x0 + cx, y0) { surface(w, x0 + cx) } else { i32::MAX }).collect();
    for cy in 0..ch {
        for cx in 0..cw {
            let (x, y) = (x0 + cx, y0 + cy);
            let c = w.get(x, y);
            let o = c.organism_id();
            let sp = (o != 0).then(|| w.organism(o).map(|s| s.species)).flatten();
            let rgb = if sp == Some(pred) {
                [225, 40, 30]
            } else if sp == Some(ant) {
                [240, 196, 40]
            } else if c.material == material::EMPTY {
                if y > tops[cx as usize] {
                    [34, 26, 22]
                } else {
                    [178, 204, 226]
                }
            } else {
                let pal = &w.materials.get(c.material).palette;
                if pal.is_empty() {
                    [128, 128, 128]
                } else {
                    let p = pal[c.shade as usize % pal.len()];
                    [p[0], p[1], p[2]]
                }
            };
            for py in 0..scale {
                for px in 0..scale {
                    img.put_pixel(cx as u32 * scale + px, cy as u32 * scale + py, image::Rgba([rgb[0], rgb[1], rgb[2], 255]));
                }
            }
        }
    }
    for &(t, (kx, ky)) in kills {
        let age = frame.saturating_sub(t) as f32 / 120.0;
        let r = (scale as f32 * (2.0 + 3.0 * age)) as i64;
        let (cx, cy) = (((kx - x0) as i64 * scale as i64) + scale as i64 / 2, ((ky - y0) as i64 * scale as i64) + scale as i64 / 2);
        for k in 0..64 {
            let a = k as f32 / 64.0 * std::f32::consts::TAU;
            let (px, py) = (cx + (r as f32 * a.cos()) as i64, cy + (r as f32 * a.sin()) as i64);
            if px >= 0 && py >= 0 && (px as u32) < img.width() && (py as u32) < img.height() {
                img.put_pixel(px as u32, py as u32, image::Rgba([255, 255, 255, 255]));
            }
        }
    }
    img
}

/// Write `shots` as a looping GIF, `ms` a frame.
pub fn save_gif(shots: Vec<image::RgbaImage>, path: &std::path::Path, ms: u64) {
    let delay = image::Delay::from_saturating_duration(std::time::Duration::from_millis(ms));
    let frames: Vec<image::Frame> = shots.into_iter().map(|img| image::Frame::from_parts(img, 0, 0, delay)).collect();
    let file = std::fs::File::create(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut enc = image::codecs::gif::GifEncoder::new(file);
    enc.set_repeat(image::codecs::gif::Repeat::Infinite).expect("gif repeat");
    enc.encode_frames(frames).expect("gif frames");
}
