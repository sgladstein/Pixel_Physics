//! **Does an ant get trapped among plants?** (plant-food lane, 2026-10-25, the owner: "Can our new ant move through
//! trees like our old ant could or does it get trapped by branches and trunks".) Runs a lab scenario through the lab's
//! own `Lab`, as `labsurvive` does, with the hand-built ant (`founders=old`) or the shipped trained pool
//! (`founders=std`), and every `every=` frames samples each ant's head. An ant is **stuck** when its head has stayed
//! within `r=` cells of where it was `win=` frames ago; it is **by wood** when a living woody cell above ground (a stem,
//! a trunk, a branch; not a root) is within 1 cell of its head. Prints the share of ant-samples stuck by wood, stuck elsewhere, the trunk
//! crossings the hand-built walk made, how many columns the colony reached and how far out each way.
//!
//! The control is the hand-built ant with `CROSS_TRUNK=0` (wood a wall): if wood traps, that arm must show it.
//!
//! ```text
//! cargo run --release --example treewalk -- founders=old scenario=played_bed seed=1 after=20000 grant=3
//! ```
use pixel_physics::lab::scenario::Scenario;
use pixel_physics::sim::organism::DeathCause;
use pixel_physics::render::Renderer;
use pixel_physics::sim::particle::ParticleSystem;
use std::collections::HashMap;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| a.strip_prefix(&format!("{key}=")).and_then(|v| v.parse().ok()))
}

fn main() {
    let name: String = arg("scenario").unwrap_or_else(|| "played_bed".to_string());
    let after: u64 = arg("after").unwrap_or(20_000);
    let grant: f32 = arg("grant").unwrap_or(3.0);
    let every: u64 = arg("every").unwrap_or(25);
    let win: u64 = arg("win").unwrap_or(1000);
    let r: i32 = arg("r").unwrap_or(3);
    // `shot=F` (frames after landing) writes `out=` (default treewalk.png): the lab's own draw at 2x, every ant painted
    // over -- red stuck by wood, yellow stuck elsewhere, magenta moving.
    let shot: Option<u64> = arg("shot");
    let out: String = arg("out").unwrap_or_else(|| "treewalk.png".to_string());
    let founders: String = arg("founders").expect("founders=old|std");
    let mut sc = Scenario::load(&name).unwrap_or_else(|e| panic!("scenario {name}: {e}"));
    if let Some(sd) = arg::<u64>("seed") {
        sc.bed.seed = sd;
    }
    let cross = std::env::var("CROSS_TRUNK").unwrap_or_else(|_| "on".into());
    println!("treewalk scenario={name} seed={} founders={founders} after={after} grant={grant} every={every} win={win} r={r} CROSS_TRUNK={cross}", sc.bed.seed);
    let (bed_w, bed_h) = (sc.bed.width, sc.bed.height);
    let mut lab = pixel_physics::lab::Lab::new(sc.bed.clone());
    lab.load_scenario(sc);
    match founders.as_str() {
        "old" => {}
        "std" => {
            lab.set_new_ant(true);
        }
        f => panic!("founders={f}: old or std"),
    }
    let lag = (win / every) as usize;
    let mut trail: HashMap<u32, Vec<(i32, i32)>> = HashMap::new();
    // A spell: consecutive samples one ant spent stuck by wood. Ended by moving off, or by the run's end; a spell is
    // counted from the sample that first found it stuck, so it is `win` frames short of the time the ant stood there.
    let mut spell: HashMap<u32, u64> = HashMap::new();
    let mut spells: Vec<u64> = Vec::new();
    let (mut landed, mut door) = (None::<u64>, 0i32);
    let (mut samples, mut stuck_wood, mut stuck_else, mut by_wood) = (0u64, 0u64, 0u64, 0u64);
    let mut cols = vec![false; bed_w as usize];
    let (mut lo, mut hi) = (i32::MAX, i32::MIN);
    let mut c0 = (0u64, 0u64, 0u64);
    let starved = DeathCause::Starved as usize;
    let mut s0 = 0;
    loop {
        lab.tick_for_harness();
        let w = &lab.world;
        let f = w.frame;
        let sp = w.species.id_of("ant");
        // Roots are woody too, and they line every tunnel near a plant: a resting ant in its own nest is not one caught
        // by a trunk. Stems, trunks and branches only.
        let root = w.materials.id_of("rootwood");
        assert!(root.is_some(), "no rootwood material");
        let heads: Vec<(u32, (i32, i32))> = w
            .live_organism_ids()
            .into_iter()
            .filter_map(|id| w.organism(id).filter(|s| Some(s.species) == sp).and_then(|s| s.chain.first().map(|&h| (id, h))))
            .collect();
        if landed.is_none() && !heads.is_empty() {
            landed = Some(f);
            door = heads.iter().map(|h| h.1 .0).sum::<i32>() / heads.len() as i32;
            if grant > 0.0 {
                println!("granted {} ants {grant} tanks", lab.grant_ants(grant));
            }
            let st = &lab.world.creature_stats;
            c0 = (st.crossings, st.crossings_completed, st.crossings_abandoned);
            s0 = lab.world.deaths_by_cause[starved];
            continue;
        }
        let Some(l) = landed else { continue };
        let snap = shot.is_some_and(|k| f == l + k);
        let mut marks: Vec<((i32, i32), [u8; 3])> = Vec::new();
        if f.is_multiple_of(every) || snap {
            for &(id, (x, y)) in &heads {
                let t = trail.entry(id).or_default();
                t.push((x, y));
                cols[x.clamp(0, bed_w - 1) as usize] = true;
                lo = lo.min(x);
                hi = hi.max(x);
                if t.len() <= lag {
                    continue;
                }
                let counted = f.is_multiple_of(every);
                samples += counted as u64;
                let wood = (-1..=1).any(|dy| {
                    (-1..=1).any(|dx| {
                        let (cx, cy) = (x + dx, y + dy);
                        w.in_bounds(cx, cy) && {
                            let c = w.get(cx, cy);
                            c.organism_id() != 0 && Some(c.material) != root && w.materials.get(c.material).woody
                        }
                    })
                });
                let span = t[t.len() - 1 - lag..].iter().all(|p| (p.0 - x).abs() <= r && (p.1 - y).abs() <= r);
                marks.push(((x, y), if span && wood { [255, 40, 40] } else if span { [255, 230, 0] } else { [255, 0, 255] }));
                if !counted {
                    continue;
                }
                by_wood += wood as u64;
                if span && wood {
                    *spell.entry(id).or_default() += every;
                } else if let Some(n) = spell.remove(&id) {
                    spells.push(n);
                }
                if span {
                    if wood {
                        stuck_wood += 1;
                    } else {
                        stuck_else += 1;
                    }
                }
            }
        }
        if snap {
            let (vw, vh) = (bed_w as u32, bed_h as u32);
            let mut buf = vec![0u8; (vw * vh * 4) as usize];
            let mut renderer = Renderer::new();
            let touched = lab.world.take_touched_chunks();
            renderer.draw(&lab.world, &ParticleSystem::new(), &touched, &mut buf, (vw, vh), true);
            let z = 2u32;
            let mut big = vec![0u8; (vw * z * vh * z * 4) as usize];
            for y in 0..vh * z {
                for x in 0..vw * z {
                    let (s, d) = ((((y / z) * vw + x / z) * 4) as usize, ((y * vw * z + x) * 4) as usize);
                    big[d..d + 4].copy_from_slice(&buf[s..s + 4]);
                }
            }
            for &((x, y), rgb) in &marks {
                for py in (y * 2 - 1).max(0)..(y * 2 + 3).min((vh * z) as i32) {
                    for px in (x * 2 - 1).max(0)..(x * 2 + 3).min((vw * z) as i32) {
                        let d = ((py as u32 * vw * z + px as u32) * 4) as usize;
                        big[d..d + 3].copy_from_slice(&rgb);
                        big[d + 3] = 255;
                    }
                }
            }
            image::save_buffer(&out, &big, vw * z, vh * z, image::ColorType::Rgba8).expect("writing the shot");
            let stuck = marks.iter().filter(|m| m.1 == [255, 40, 40]).count();
            println!("SHOT frame {f} ({} after landing): {} ants, {stuck} stuck by wood -> {out}", f - l, marks.len());
        }
        if f >= l + after {
            break;
        }
    }
    spells.extend(spell.values());
    spells.sort_unstable();
    let q = |p: f64| spells.get(((spells.len() as f64 - 1.0) * p).round() as usize).copied().unwrap_or(0);
    println!(
        "SPELLS stuck-by-wood spells={} p50={} p90={} max={} frames, total={} ant-frames, over 5000 frames: {}",
        spells.len(),
        q(0.5),
        q(0.9),
        spells.last().copied().unwrap_or(0),
        spells.iter().sum::<u64>(),
        spells.iter().filter(|&&n| n >= 5000).count()
    );
    let w = &lab.world;
    let st = &w.creature_stats;
    let pct = |n: u64| 100.0 * n as f64 / samples.max(1) as f64;
    println!(
        "RESULT alive={} starved={} samples={samples} by_wood={:.1}% stuck_by_wood={:.1}% stuck_else={:.1}% crossings={} completed={} abandoned={} columns={} reach_left={} reach_right={} door={door}",
        w.live_organism_ids().into_iter().filter(|&id| w.organism(id).is_some_and(|s| Some(s.species) == w.species.id_of("ant"))).count(),
        w.deaths_by_cause[starved] - s0,
        pct(by_wood),
        pct(stuck_wood),
        pct(stuck_else),
        st.crossings - c0.0,
        st.crossings_completed - c0.1,
        st.crossings_abandoned - c0.2,
        cols.iter().filter(|&&c| c).count(),
        door - lo,
        hi - door
    );
}
