//! **Pricing the three routes past the body stamp** — Lane I's instrument.
//!
//! `creature_probe` answers *what is one ant sensing and deciding*. This
//! answers a different question that no existing harness could: **what
//! would it take for the shipped ant to afford a child**, and does any of
//! the three named routes past the `body_energy * cells` stamp actually
//! get there in this world.
//!
//! Three things here that `creature_probe` does not have, each of which is
//! the reason this is a separate binary rather than four more flags:
//!
//! * **`gut=`.** Route 3 (economics §3.5) is a claim about the diet gene
//!   and nothing else, and `creature_probe` has no way to set it. It is
//!   applied to the species *and* read back off a live founder below, so a
//!   run cannot silently measure the neutral gut.
//! * **A ceiling priced on the food that is *standing here*.** This is the
//!   correction to `creature_probe`'s known trap. That readout takes the
//!   best cell in the whole material table, so it quotes a flower at
//!   1,440 whether or not any ant in this world has ever been near one --
//!   and its own comment says a bound read that way "can rule out and can
//!   never rule in". The whole of route 3 turns on which foods are
//!   *reachable*, so this censuses the world and prices the best mouthful
//!   among materials that actually exist in it. Both numbers are printed;
//!   the gap between them is the point.
//! * **A `SUMMARY` line.** Everything here is a seed sweep -- creature
//!   outcomes spread 0.103-0.541 across random genomes (dead-end 552) and
//!   six seeds is not a sweep -- so one greppable line per run is what
//!   makes an order statistic over eighteen of them cheap.
//!
//! ```text
//! cargo run --release --example stamp_probe -- gut=-1.0 frames=24000 seed=1
//! cargo run --release --example stamp_probe -- body_energy=240 frames=24000   # route 1's stamp
//! cargo run --release --example stamp_probe -- body_energy=0   frames=24000   # route 2's stamp
//! ```
//!
//! **The two stamp arguments are proxies, and the report says so.** Routes
//! 1 and 2 need mechanism in `creature.rs`; `body_energy=` reproduces the
//! *birth arithmetic* each one implies and nothing else about it. What it
//! cannot see is named in the report rather than left for a reader to
//! assume away.

use pixel_physics::render::Renderer;
use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::creature::{birth_cost, diet_yield, food_value, grant_fraction, EAT_YIELD_THRESHOLD};
use pixel_physics::sim::material;
use pixel_physics::sim::organism::{TRAIT_BIRTH_GRANT, TRAIT_GUT_BIAS};
use pixel_physics::sim::{parallel, Cell, World};

fn main() {
    let mut frames = 24_000usize;
    let mut ants = 55usize;
    let mut seed = 0xA17u64;
    let mut world_terrain = true;
    let mut start_energy = -1.0f32;
    let mut body_energy = -1.0f32;
    let mut threshold = -1.0f32;
    let mut hunger = -1.0f32;
    let mut mutation_rate = -1.0f32;
    let mut grant = -1.0f32;
    // The one knob `creature_probe` does not have. `-2` means "leave the
    // species file alone", because `-1` is a legal position on the axis
    // and is in fact the setting route 3 is about.
    let mut gut = -2.0f32;
    // Where to write a picture of the colony at the end of the run, if
    // anywhere. **A colony that sustains itself is a judge-by-eye claim**
    // and this project's rule is to post the picture rather than describe
    // it -- with the discrete event count beside it, because two very
    // different mechanisms look identical at the zoom a card is read at.
    let mut png = String::new();
    for arg in std::env::args().skip(1) {
        let Some((k, v)) = arg.split_once('=') else { continue };
        match k {
            "frames" => frames = v.parse().expect("frames"),
            "ants" => ants = v.parse().expect("ants"),
            "seed" => seed = v.parse().expect("seed"),
            "terrain" => world_terrain = v == "world",
            "start_energy" => start_energy = v.parse().expect("start_energy"),
            "body_energy" => body_energy = v.parse().expect("body_energy"),
            "threshold" => threshold = v.parse().expect("threshold"),
            "hunger" => hunger = v.parse().expect("hunger"),
            "mutation_rate" => mutation_rate = v.parse().expect("mutation_rate"),
            "grant" => grant = v.parse().expect("grant"),
            "gut" => gut = v.parse().expect("gut"),
            "png" => png = v.to_string(),
            other => panic!(
                "unknown arg {other:?}; known: frames, ants, seed, terrain, start_energy, body_energy, threshold, hunger, mutation_rate, grant, gut, png"
            ),
        }
    }

    let (w, h) = if world_terrain { (512i32, 320i32) } else { (320i32, 120i32) };
    let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
    world.seed = seed;
    let nest = world.materials.id_of("nest").expect("nest is compiled in");
    let ant = world.materials.id_of("ant").expect("ant is compiled in");

    let species = world.species.id_of("ant").expect("ant species");
    {
        let mut def = world.species.get(species).creature.clone().expect("ant is a creature");
        if start_energy >= 0.0 {
            def.start_energy = start_energy;
        }
        if body_energy >= 0.0 {
            def.body_energy = body_energy;
            // **The flesh-pricing invariant, held rather than broken.**
            // A bitten live ant must be worth to the biter exactly what
            // its stamp took out of the world, or scavenging is an energy
            // pump (dead-ends §13l). `creature_probe` holds it the same
            // way and for the same reason; a harness that opened the pump
            // would then measure it.
            for name in ["ant", "corpse"] {
                if let Some(id) = world.materials.id_of(name) {
                    world.materials.get_mut(id).food_energy = body_energy;
                }
            }
        }
        if threshold >= 0.0 {
            def.reproduce_threshold = threshold;
        }
        if grant >= 0.0 {
            def.traits[TRAIT_BIRTH_GRANT] = grant * 2.0 - 1.0;
        }
        if hunger >= 0.0 {
            def.digest_rate = hunger;
        }
        if mutation_rate >= 0.0 {
            def.mutation_rate = mutation_rate;
        }
        if gut >= -1.0 {
            def.traits[TRAIT_GUT_BIAS] = gut.clamp(-1.0, 1.0);
        }
        world.species.set_creature(species, def);
    }
    let def = world.species.get(species).creature.clone().expect("ant is a creature");
    let gut_bias = def.traits[TRAIT_GUT_BIAS];
    let bar = birth_cost(&def);

    if world_terrain {
        let (presets, _) = pixel_physics::worldgen::WorldgenPresets::load();
        let name = presets.default_name();
        let params = presets.get(&name).expect("the default preset exists");
        pixel_physics::worldgen::generate(&mut world, pixel_physics::worldgen::Spec::Generated { params, seed });
        for _ in 0..600 {
            parallel::step(&mut world);
            world.step_active_sites();
            world.step_fields();
        }
        let surface_of = |world: &World, x: i32| -> i32 {
            (0..h)
                .find(|&y| {
                    world.get(x, y).organism_id() == 0
                        && matches!(world.materials.kind(world.get(x, y).material), material::MaterialKind::Solid | material::MaterialKind::Powder)
                })
                .unwrap_or(h - 1)
        };
        for x in 96..160 {
            let sy = surface_of(&world, x);
            world.set(x, sy, Cell::new(nest, 0).with_attached(true));
        }
        for i in 0..ants as i32 {
            let ax = 100 + i * 2;
            let sy = surface_of(&world, ax);
            world.plant_ant(ax, sy - 1);
        }
    } else {
        let floor = h - 8;
        for x in 0..w {
            for y in floor..h {
                world.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
            }
        }
        for x in 16..120 {
            world.set(x, floor, Cell::new(nest, 0).with_attached(true));
        }
        for i in 0..ants as i32 {
            world.plant_ant(20 + i * 2, floor - 1);
        }
    }

    // **How many founders actually got onto the ground.** Measured, not
    // assumed, and it is the difference between a result and a dud.
    // `seed=1` seats *none* of its 55 -- `surface_of` finds no walkable
    // ground where the colony strip is laid, so the run reports `births 0`
    // about a world that never had an ant in it, which reads identically
    // to an ant that could not afford one. A scene that does not contain
    // the situation under test will always look like the mechanism being
    // dead (`CLAUDE.md`), and here the two failures are one column apart.
    let placed = count_live(&world, w, h);

    // **Read the gut back off a live founder, not off the def.** A knob
    // nobody can see the value of is a knob nobody can tell is
    // disconnected, and this one has a real way to fail silently:
    // `diet_yield` is called with the *organism's* heritable trait
    // (`creature.rs` fn gut_of), so a species-level write that did not
    // reach the founders would leave every run measuring the neutral gut
    // while the header claimed otherwise.
    let founder_gut = live_founder_gut(&world, w, h);

    // The whole-table ceiling `creature_probe` prints, reproduced here so
    // the two are comparable, and then the honest one beside it.
    let table_best = (0..world.materials.len())
        .map(|i| material::MaterialId(i as u16))
        .map(|id| yield_of(&world, id, &def, gut_bias))
        .fold(0.0f32, f32::max);
    let standing = food_census(&world, w, h, &def, gut_bias);
    let world_best = standing.iter().map(|r| r.yield_here).fold(0.0f32, f32::max);

    println!(
        "stamp probe: {frames} frames, {ants} founders requested and {placed} actually seated | terrain={} seed={seed}",
        if world_terrain { "world" } else { "slab" }
    );
    println!(
        "  economy: start_energy {:.0} body_energy {:.0} crop {:.0} digest {:.2}/tick threshold {:.0} mutation_rate {:.3} grant {:.2} (= {:.0}) | gut {gut_bias:+.2} (founder reads {})",
        def.start_energy,
        def.body_energy,
        def.crop_capacity,
        def.digest_rate,
        def.reproduce_threshold,
        def.mutation_rate,
        grant_fraction(def.traits[TRAIT_BIRTH_GRANT]),
        pixel_physics::sim::creature::birth_grant(&def, &def.traits),
        founder_gut.map_or("NO LIVE FOUNDER".to_string(), |g| format!("{g:+.2}")),
    );
    // **A rate, not a ceiling, and the distinction is this harness's whole
    // subject.** Its finding was that one number -- `ceiling - bar` --
    // governs regardless of mechanism, every negative margin giving exactly
    // zero births. The crop removes the ceiling (an animal digests at a rate
    // and nothing stops it eating), so the governing number becomes whether
    // an animal out-eats its upkeep, and how long a child then takes. The
    // *shape* of the finding survives: a non-positive rate is still an exact
    // zero-births prediction.
    let upkeep = def.idle_cost_per_cell * def.body.len() as f32;
    let rate_on = |mouthful: f32, face: f32| -> f32 {
        def.digest_rate * if face > 0.0 { mouthful / face } else { 0.0 } - upkeep
    };
    println!(
        "  bar: birth costs {bar:.0} | upkeep {upkeep:.3}/tick | best mouthful on the whole table {table_best:.0}, standing in this world {world_best:.0}",
    );
    println!("  standing food this gut can see (>{EAT_YIELD_THRESHOLD:.0}), before the run:");
    for r in &standing {
        println!(
            "    {:<12} {:>7} cells  face {:>6.0}  yield {:>6.0}  {}",
            r.name,
            r.cells,
            r.face,
            r.yield_here,
            if r.yield_here > EAT_YIELD_THRESHOLD { "edible" } else { "INVISIBLE to this gut" }
        );
    }

    for _ in 0..frames {
        parallel::step(&mut world);
        world.step_active_sites();
        world.step_fields();
        world.step_pheromones();
    }

    if !png.is_empty() {
        write_colony_png(&mut world, &png, w, h);
    }

    // **Census the food again at the end.** The pre-run census is taken
    // 600 frames after generation, and the two foods that would rescue a
    // specialised gut -- `fruit` at 960 and `flower` at 1,440 -- are
    // *grown*, not generated, so a world that has none at frame 600 may
    // have some at frame 24,000. Reading only the opening census would
    // answer "what does worldgen lay down" when the question is "what can
    // an ant reach", which is this harness's whole reason to exist.
    let standing_end = food_census(&world, w, h, &def, gut_bias);
    let world_best_end = standing_end.iter().map(|r| r.yield_here).fold(0.0f32, f32::max);
    println!("\n  standing food after {frames} frames:");
    for r in &standing_end {
        println!(
            "    {:<12} {:>7} cells  face {:>6.0}  yield {:>6.0}  {}",
            r.name,
            r.cells,
            r.face,
            r.yield_here,
            if r.yield_here > EAT_YIELD_THRESHOLD { "edible" } else { "INVISIBLE to this gut" }
        );
    }

    let st = world.creature_stats;
    let mut lineages: std::collections::HashMap<u32, usize> = std::collections::HashMap::new();
    let mut deepest = 0u16;
    let mut richest = 0.0f32;
    let mut live = 0usize;
    let mut gut_sum = 0.0f64;
    for x in 0..w {
        for y in 0..h {
            let cell = world.get(x, y);
            if pixel_physics::sim::organism::cell_type(cell.aux()) != Some(pixel_physics::sim::organism::CellType::Head) {
                continue;
            }
            let Some(state) = world.organism(cell.organism_id()) else { continue };
            if world.species.get(state.species).creature.is_none() {
                continue;
            }
            live += 1;
            deepest = deepest.max(state.generation);
            richest = richest.max(state.energy);
            gut_sum += state.traits[TRAIT_GUT_BIAS] as f64;
            *lineages.entry(state.lineage).or_default() += 1;
        }
    }
    let _ = ant;
    println!(
        "\nreproduction: births {} denied-no-space {} refused-no-slot {} deaths {} eats {} | live {live} deepest generation {deepest} lineages {} | richest bank {richest:.0} against a bar of {bar:.0} (ratio {:.3})",
        st.births,
        st.births_denied_no_space,
        world.organisms_refused(),
        st.deaths,
        st.eats,
        lineages.len(),
        richest / bar.max(1.0),
    );
    // **One greppable line, and it carries every operand a reader would
    // otherwise have to reconstruct.** `births` alone is unreadable --
    // `births 0 / live 0` is an empty world rather than a failed birth,
    // and a scene running `denied-no-space` in the hundreds of thousands
    // is space-limited, so its births column says nothing about energy.
    println!(
        "SUMMARY seed={seed} placed={placed} gut={gut_bias:.2} body_energy={:.0} start_energy={:.0} grant={:.0} bar={bar:.0} \
         births={} deaths={} live={live} nospace={} gen={deepest} lineages={} richest={richest:.0} ratio={:.3} eats={} meangut={:.3} \
         reproduce_at={:.0} ceil_world={:.0} ceil_end={:.0} best_end={world_best_end:.0}",
        def.body_energy,
        def.start_energy,
        pixel_physics::sim::creature::birth_grant(&def, &def.traits),
        st.births,
        st.deaths,
        st.births_denied_no_space,
        lineages.len(),
        richest / bar.max(1.0),
        st.eats,
        if live > 0 { gut_sum / live as f64 } else { f64::NAN },
        pixel_physics::sim::creature::reproduce_at(&def).unwrap_or(f32::NAN),
        world_best,
        world_best_end,
    );
}

/// One material's worth to this gut, with a corpse's per-cell stamp put
/// into `aux` -- everything else reads its material's face value, so a
/// probe cell without a stamp reads the whole carrion half as worthless.
fn yield_of(world: &World, id: material::MaterialId, def: &pixel_physics::sim::organism::CreatureDef, gut_bias: f32) -> f32 {
    let aux = if world.materials.get(id).worth_in_aux { def.body_energy.round().clamp(0.0, 65535.0) as u16 } else { 0 };
    diet_yield(world, Cell::new(id, 0).with_aux(aux), gut_bias)
}

struct FoodRow {
    name: String,
    cells: usize,
    face: f32,
    yield_here: f32,
}

/// **What this world is actually offering, as opposed to what the material
/// table could offer.** Counts real cells, and prices each material with
/// the cell's own `aux` where that is where its worth lives, so a corpse
/// that carries a dead animal's leftover bank is priced at what it is
/// worth rather than at its stamp.
fn food_census(world: &World, w: i32, h: i32, def: &pixel_physics::sim::organism::CreatureDef, gut_bias: f32) -> Vec<FoodRow> {
    let mut counts: std::collections::HashMap<u16, usize> = std::collections::HashMap::new();
    for x in 0..w {
        for y in 0..h {
            let cell = world.get(x, y);
            if food_value(world, cell) > 0.0 {
                *counts.entry(cell.material.0).or_default() += 1;
            }
        }
    }
    let mut rows: Vec<FoodRow> = counts
        .into_iter()
        .map(|(m, cells)| {
            let id = material::MaterialId(m);
            FoodRow {
                name: world.materials.get(id).name.clone(),
                cells,
                face: world.materials.get(id).food_energy,
                yield_here: yield_of(world, id, def, gut_bias),
            }
        })
        .collect();
    rows.sort_by(|a, b| b.yield_here.partial_cmp(&a.yield_here).unwrap_or(std::cmp::Ordering::Equal).then(b.cells.cmp(&a.cells)));
    rows
}

/// Live creature heads standing in the world.
fn count_live(world: &World, w: i32, h: i32) -> usize {
    let mut n = 0;
    for x in 0..w {
        for y in 0..h {
            let cell = world.get(x, y);
            if pixel_physics::sim::organism::cell_type(cell.aux()) != Some(pixel_physics::sim::organism::CellType::Head) {
                continue;
            }
            if let Some(state) = world.organism(cell.organism_id()) {
                if world.species.get(state.species).creature.is_some() {
                    n += 1;
                }
            }
        }
    }
    n
}

fn live_founder_gut(world: &World, w: i32, h: i32) -> Option<f32> {
    for x in 0..w {
        for y in 0..h {
            let cell = world.get(x, y);
            if pixel_physics::sim::organism::cell_type(cell.aux()) != Some(pixel_physics::sim::organism::CellType::Head) {
                continue;
            }
            if let Some(state) = world.organism(cell.organism_id()) {
                if world.species.get(state.species).creature.is_some() {
                    return Some(state.traits[TRAIT_GUT_BIAS]);
                }
            }
        }
    }
    None
}

/// **A picture of the colony**, cropped to the nest rather than to the
/// whole world: at 512x320 an ant is one or two cells and a full-world
/// sheet renders it at the size of the noise.
///
/// The PNG writer is one stored-deflate IDAT, copied from
/// `examples/terrain_shade.rs` -- the repo's other sheet-writers do the
/// same, and pulling an encoder in for a debug harness is not worth the
/// dependency.
fn write_colony_png(world: &mut World, path: &str, w: i32, h: i32) {
    // A tight band on the nest, zoomed hard. **An ant is one or two
    // cells of dark on dark soil**, so a 256-wide crop of a colony renders
    // it at the size of the ground texture -- tried, and the picture was
    // unreadable. `creature-appearance-design.md` is the standing account
    // of why, and this crop is the most this harness can do about it.
    const VW: u32 = 128;
    const VH: u32 = 80;
    // Centre on the nest strip the founders were stood on (x 96..160), at
    // whatever height the ground there ended up.
    let cx = 128;
    // **Ground, not "anything solid" -- and this is not a nicety.** The
    // first non-empty cell down a column in a wooded world is the top of
    // the *canopy*, so the obvious version of this line framed a picture
    // of treetops with no ant in it. Same rule the founders were placed
    // with: skip anything that belongs to an organism, and take the first
    // Solid or Powder.
    let cy = (0..h)
        .find(|&y| {
            world.get(cx, y).organism_id() == 0
                && matches!(
                    world.materials.kind(world.get(cx, y).material),
                    material::MaterialKind::Solid | material::MaterialKind::Powder
                )
        })
        .unwrap_or(h / 2);
    let mut renderer = Renderer::new();
    renderer.set_camera(cx - VW as i32 / 2, cy - VH as i32 / 2, (VW, VH), world.bounds());
    let particles = ParticleSystem::new();
    let touched = world.take_touched_chunks();
    let mut frame = vec![0u8; (VW * VH * 4) as usize];
    renderer.draw(world, &particles, &touched, &mut frame, (VW, VH), true);
    let _ = w;
    // **Upscale by an integer factor before writing.** The stills the owner
    // has actually been able to judge are 700-950 px across; a 256-px crop
    // of a world whose ants are two cells long is a picture with nothing
    // visible in it. Nearest-neighbour, so no pixel is invented.
    const Z: usize = 6;
    let (zw, zh) = (VW as usize * Z, VH as usize * Z);
    let mut big = vec![0u8; zw * zh * 4];
    for y in 0..zh {
        for x in 0..zw {
            let src = ((y / Z) * VW as usize + (x / Z)) * 4;
            big[(y * zw + x) * 4..(y * zw + x) * 4 + 4].copy_from_slice(&frame[src..src + 4]);
        }
    }
    write_png(path, &big, zw, zh);
    // **How many ants are actually inside the frame.** Without this the
    // picture cannot distinguish "the colony is invisible at this scale"
    // from "the crop does not contain the colony", and this project has
    // already read a collapse as working from an image whose event count
    // was zero. An image says what and where; only the number says
    // whether there was anything there to see.
    let (x0, y0) = (cx - VW as i32 / 2, cy - VH as i32 / 2);
    let mut in_frame = 0;
    for y in y0..y0 + VH as i32 {
        for x in x0..x0 + VW as i32 {
            if world.organism(world.get(x, y).organism_id()).is_some_and(|st| world.species.get(st.species).creature.is_some()) {
                in_frame += 1;
            }
        }
    }
    println!("  creature cells inside the crop: {in_frame}");
    println!(
        "  wrote {path} ({zw}x{zh}, {Z}x upscale of a {VW}x{VH} crop centred on the nest at world x {cx} y {cy})"
    );
}

fn write_png(path: &str, rgba: &[u8], w: usize, h: usize) {
    fn crc32(data: &[u8]) -> u32 {
        let mut table = [0u32; 256];
        for (i, e) in table.iter_mut().enumerate() {
            let mut c = i as u32;
            for _ in 0..8 {
                c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
            }
            *e = c;
        }
        let mut c = 0xFFFF_FFFFu32;
        for &b in data {
            c = table[((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8);
        }
        c ^ 0xFFFF_FFFF
    }
    fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], body: &[u8]) {
        out.extend_from_slice(&(body.len() as u32).to_be_bytes());
        out.extend_from_slice(kind);
        out.extend_from_slice(body);
        let mut crc_in = kind.to_vec();
        crc_in.extend_from_slice(body);
        out.extend_from_slice(&crc32(&crc_in).to_be_bytes());
    }
    let mut raw = Vec::with_capacity((w * 4 + 1) * h);
    for y in 0..h {
        raw.push(0);
        raw.extend_from_slice(&rgba[y * w * 4..(y + 1) * w * 4]);
    }
    // Stored (uncompressed) deflate: zlib header, then 64KiB-max blocks.
    let mut z = vec![0x78u8, 0x01];
    for (i, block) in raw.chunks(65535).enumerate() {
        let last = if (i + 1) * 65535 >= raw.len() { 1u8 } else { 0u8 };
        z.push(last);
        z.extend_from_slice(&(block.len() as u16).to_le_bytes());
        z.extend_from_slice(&(!(block.len() as u16)).to_le_bytes());
        z.extend_from_slice(block);
    }
    let mut adler = (1u32, 0u32);
    for &b in &raw {
        adler.0 = (adler.0 + b as u32) % 65521;
        adler.1 = (adler.1 + adler.0) % 65521;
    }
    z.extend_from_slice(&((adler.1 << 16) | adler.0).to_be_bytes());

    let mut out = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&(w as u32).to_be_bytes());
    ihdr.extend_from_slice(&(h as u32).to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"IDAT", &z);
    chunk(&mut out, b"IEND", &[]);
    std::fs::write(path, out).expect("write png");
}
