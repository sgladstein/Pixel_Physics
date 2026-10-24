//! **Training the lone predator** (lone predator thread, 2026-10-23; owner: "train a new lone predator creature
//! using the same in colony training scheme the ants use"): spiders (`assets/species/spider.ron`) with minds, on the
//! colony training bed beside a colony of the current ant, selected inside the living world -- every `judge`
//! frames the best hunters have children (each a mutated copy of its parent's mind) and the worst are culled, and
//! each round founds a fresh bed on the round's best. The colony training scheme (`lab::training`), with the
//! individual as the unit: a spider shares nothing and is paid only for what it eats.
//!
//! **The prey does not evolve** (stage one of the plan: the predator first, co-evolution after). The colony is the
//! game's new ant (the standard pool, `assets/minds/std_pool.txt`, `lab::training::new_ant_minds`; `h1` until
//! 2026-10-24) on the training bed with its food drip, no ant selection, and the colony held up to its headcount by
//! fresh pool ants at the door, so the spiders always have
//! prey and a round measures the hunter, not how fast the colony ran out.
//!
//! cargo run --release --example predtrain -- out=DIR [seed=1] [rounds=6] [frames=100000] [preds=12] [ants=30]
//!     [sel=score|random|off] [judge=1000] [min_age=6000] [turn=20000] [keep=8] [grants=1] [kid_grants=1]
//!     [kpm=0.03] [ksigma=1] [kevents=10] [start_events=20] [founder=PATH|hunter] [pool=DIR] [lo=6] [hi=16] [cooldown=4000]
//!     [frames_out=0] [step=game|harness]
//!
//! Sets `PIXEL_PHYSICS_MIND_ATTACK=on` itself (the Attack act, `creature::options::attack_on`).
//!
//! **Score**: the face worth of the flesh a spider swallowed (a bite off a living ant, or a cell of corpse picked
//! up), per thousand frames alive. **Births**: about one per spider per `turn` frames, to the best adult off its
//! `cooldown` (`sel=score`), or to a random adult (`sel=random`, the control that says whether the score does
//! anything). **Culls**: the worst adults while more than `hi` are alive. A starved spider stays a candidate parent
//! for 20,000 frames at half its score (the hall), so a die-off refills from the best of the dead. `sel=off`: no
//! births or culls.
//!
//! out/rN/sel.csv      frame,alive,adults,births,culls,starved,median,best,kills,flesh,corpse,ants,ant_refills
//! out/rN/births.csv   frame,id,parent,family,gen,kind,links,score
//! out/rN/pool/ID.raw.txt   the round's `keep` best minds, next round's founders
//! out/rN/frames/fNNNNNNN.txt   (frames_out=N) every N frames: `x0 y0 w h`, then rows: `.` open, `A` ant, `S` spider,
//!     `x` corpse, `f` provisions, `#` ground
//! stdout              one line per round
use pixel_physics::lab::training::{self, Founder, Mode, Round, Settings};
use pixel_physics::sim::cell::OrganismId;
use pixel_physics::sim::creature::{self, mind};
use pixel_physics::sim::material;
use pixel_physics::sim::organism::{DeathCause, SpeciesId};
use pixel_physics::sim::World;
use pixel_physics::train::challenges::colony_bed;
use pixel_physics::train::predator::{self, ground, of_species, spider_mind, EMITS, VERBS};
use pixel_physics::train::{self, Rng};
use std::collections::BTreeMap;
use std::io::Write;

const HALL: u64 = 20_000;

fn arg<'a>(a: &'a [String], k: &str) -> Option<&'a str> {
    a.iter().find_map(|s| s.strip_prefix(&format!("{k}=")))
}

/// Size-neutral mutation, `lab::training`'s and the in-run harness's: live links and settings take a weight step
/// with probability `pm`, then `events` structural changes, each a fair coin between adding a link and removing one.
fn neutral_mutate(g: &[f32], slots: &[usize], pm: f32, sigma: f32, events: usize, rng: &mut Rng) -> Vec<f32> {
    let first = mind::FIRST_LINK;
    let mut c = g.to_vec();
    for &i in slots {
        if (c[i] != 0.0 || i < first) && rng.unit() < pm {
            c[i] = (c[i] + sigma * rng.step()).clamp(-40.0, 40.0);
        }
    }
    for _ in 0..events {
        let add = rng.unit() < 0.5;
        let pool: Vec<usize> = slots.iter().copied().filter(|&i| i >= first && (c[i] == 0.0) == add).collect();
        if pool.is_empty() {
            continue;
        }
        let i = pool[rng.below(pool.len())];
        c[i] = if add { (sigma * rng.step()).clamp(-40.0, 40.0) } else { 0.0 };
    }
    c
}

/// One spider's ledger.
#[derive(Clone)]
struct Who {
    born: u64,
    credit: f64,
    family: String,
    gen: u32,
    genome: Vec<f32>,
    last_kid: u64,
    kids: u32,
}

impl Who {
    fn score(&self, frame: u64) -> f64 {
        self.credit / ((frame.saturating_sub(self.born)) as f64 / 1000.0).max(1.0)
    }
}

/// A column for a spider: anywhere on the bed at least 30 columns from the nest's door, so nobody is founded on the
/// colony.
fn spider_column(rng: &mut Rng) -> i32 {
    let bw = colony_bed::SIZE.0;
    loop {
        let x = 8 + rng.below((bw - 16) as usize) as i32;
        if (x - colony_bed::NEST).abs() >= 30 {
            return x;
        }
    }
}

fn write_frame(w: &World, path: &std::path::Path, ant: SpeciesId, spider: SpeciesId) {
    let (bw, bh) = colony_bed::SIZE;
    let corpse = w.materials.known(material::Known::Corpse);
    let food = w.materials.id_of("provisions");
    let mut f = std::io::BufWriter::new(std::fs::File::create(path).unwrap());
    writeln!(f, "0 0 {bw} {bh}").unwrap();
    for y in 0..bh {
        let mut row = String::with_capacity(bw as usize);
        for x in 0..bw {
            let c = w.get(x, y);
            let o = c.organism_id();
            let sp = (o != 0).then(|| w.organism(o).map(|s| s.species)).flatten();
            row.push(if sp == Some(ant) {
                'A'
            } else if sp == Some(spider) {
                'S'
            } else if Some(c.material) == corpse {
                'x'
            } else if Some(c.material) == food {
                'f'
            } else if c.material == material::EMPTY {
                '.'
            } else if ground(w, x, y) {
                '#'
            } else {
                '.'
            });
        }
        writeln!(f, "{row}").unwrap();
    }
}

/// **One picture of the bed**: every cell in its material's colour, the sky a pale blue, and a red ring for 120
/// frames where an ant was killed -- a one-cell event is unreadable without one.
fn picture(w: &World, (x0, y0, cw, ch): (i32, i32, i32, i32), scale: u32, kills: &[(u64, (i32, i32))], frame: u64, ant: SpeciesId, spider: SpeciesId) -> image::RgbaImage {
    let mut img = image::RgbaImage::new(cw as u32 * scale, ch as u32 * scale);
    for cy in 0..ch {
        for cx in 0..cw {
            let c = w.get(x0 + cx, y0 + cy);
            let o = c.organism_id();
            let sp = (o != 0).then(|| w.organism(o).map(|s| s.species)).flatten();
            let rgb = if c.material == material::EMPTY {
                [178, 204, 226]
            } else if sp == Some(spider) {
                [150, 40, 160]
            } else if sp == Some(ant) {
                [20, 16, 14]
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
                img.put_pixel(px as u32, py as u32, image::Rgba([230, 30, 30, 255]));
            }
        }
    }
    img
}

struct Cfg {
    seed: u64,
    frames: u64,
    preds: usize,
    ants: usize,
    mode: Mode,
    judge: u64,
    min_age: u64,
    turn: u64,
    keep: usize,
    grants: f32,
    kid_grants: f32,
    kpm: f32,
    ksigma: f32,
    kevents: usize,
    lo: usize,
    hi: usize,
    cooldown: u64,
    frames_out: u64,
    game: bool,
    /// `gif=R:START:END:EVERY` -- round R's frames START..END, one picture every EVERY frames, into rR/round.gif.
    gif: Option<(usize, u64, u64, u64)>,
    /// `crop=X,Y,W,H` for the GIF; `scale=` pixels a cell.
    crop: (i32, i32, i32, i32),
    scale: u32,
}

/// What one round hands the next.
struct RoundOut {
    pool: Vec<(Founder, f64)>,
    line: String,
}

fn run_round(r: usize, founders: &[Founder], cfg: &Cfg, dir: &std::path::Path) -> RoundOut {
    std::fs::create_dir_all(dir.join("pool")).unwrap();
    if cfg.frames_out > 0 {
        std::fs::create_dir_all(dir.join("frames")).unwrap();
    }
    let seed = cfg.seed * 1000 + r as u64;
    let s = Settings { mode: Mode::Off, round_frames: cfg.frames, ..Default::default() };
    let pool = training::std_pool();
    let mix: Vec<_> = pool
        .iter()
        .enumerate()
        .map(|(k, g)| (Founder { tag: format!("p{k:02}"), genome: g.clone() }, cfg.ants / pool.len() + usize::from(k < cfg.ants % pool.len())))
        .filter(|(_, n)| *n > 0)
        .collect();
    let (mut round, mut w, sc): (Round, World, _) = Round::found(r, seed, &mix, &s);
    let ant = w.species.id_of("ant").expect("ant");
    let spider = w.species.id_of("spider").expect("spider in the species table");
    let mut def = w.species.get(spider).creature.clone().expect("spider is a creature");
    def.reproduce_threshold = 1.0e30;
    def.life_half_life = 0;
    w.species.set_creature(spider, def);
    let ant_minds = training::new_ant_minds();
    let (colony, anchor) = of_species(&w, ant).first().and_then(|&i| w.organism(i)).map(|st| (st.colony, st.forage_anchor)).expect("a colony");
    let slots = mind::trainable(VERBS, EMITS);
    let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0x5151);
    let mut book: BTreeMap<OrganismId, Who> = BTreeMap::new();
    let mut hall: Vec<(u64, Who)> = Vec::new();
    let mut pred_colony: Option<u32> = None;
    for k in 0..cfg.preds {
        let f = &founders[k % founders.len()];
        let x = spider_column(&mut rng);
        if let Some(id) = predator::place_spider(&mut w, x, pred_colony, spider_mind(&f.genome), cfg.grants) {
            pred_colony = pred_colony.or_else(|| w.organism(id).map(|s| s.colony));
            book.insert(id, Who { born: 0, credit: 0.0, family: f.tag.clone(), gen: 0, genome: f.genome.clone(), last_kid: 0, kids: 0 });
        }
    }
    let mut sel = std::io::BufWriter::new(std::fs::File::create(dir.join("sel.csv")).unwrap());
    writeln!(sel, "frame,alive,adults,births,culls,starved,median,best,kills,flesh,corpse,ants,ant_refills").unwrap();
    let mut births_f = std::io::BufWriter::new(std::fs::File::create(dir.join("births.csv")).unwrap());
    writeln!(births_f, "frame,id,parent,family,gen,kind,links,score").unwrap();
    let (mut births, mut culls, mut starved, mut ant_refills, mut swallows) = (0u64, 0u64, 0u64, 0u64, 0u64);
    let mut quota = 0.0f64;
    let mut shots: Vec<image::RgbaImage> = Vec::new();
    let mut kills_seen: Vec<(u64, (i32, i32))> = Vec::new();
    let mut last_at: BTreeMap<OrganismId, (i32, i32)> = BTreeMap::new();
    let mut particles = pixel_physics::sim::ParticleSystem::new();
    let mut blasts = pixel_physics::sim::explosion::Blasts::new();
    let tuning = pixel_physics::sim::player::Tuning::default();
    let ants_killed0 = w.minds.as_ref().map_or(0, |m| m.counts.kills);
    for fr in 1..=cfg.frames {
        w.cut_log.get_or_insert_with(Vec::new).clear();
        w.food_log.get_or_insert_with(Vec::new);
        w.digest_log.get_or_insert_with(Vec::new);
        w.death_log.get_or_insert_with(Vec::new);
        if cfg.game {
            pixel_physics::lab::tick_world(&mut w, &mut particles, &mut blasts, &tuning, &round.bed, Some(&sc));
        } else {
            pixel_physics::sim::update::step(&mut w);
            w.step_active_sites();
            w.step_fields();
            w.step_pheromones();
        }
        // The spiders' ledger, read before the bed's own drains the logs.
        if let Some(log) = w.food_log.as_ref() {
            for row in log.iter().filter(|r| r.kind == creature::FOOD_SWALLOW) {
                if let Some(b) = book.get_mut(&row.who) {
                    b.credit += row.worth as f64 / 100.0;
                    swallows += 1;
                }
            }
        }
        if let Some(log) = w.death_log.as_ref() {
            for d in log.iter().filter(|d| d.species == ant && d.cause == DeathCause::Killed) {
                if let Some(at) = w.organism(d.id).and_then(|s| s.chain.first().copied()).or(last_at.get(&d.id).copied()) {
                    kills_seen.push((d.frame, at));
                }
            }
            for d in log.iter() {
                if let Some(b) = book.remove(&d.id) {
                    if d.cause == DeathCause::Starved {
                        starved += 1;
                    }
                    hall.push((d.frame, b));
                }
            }
        }
        if let Some((gr, g0, g1, _)) = cfg.gif {
            if gr == r && fr + 1 >= g0 && fr <= g1 {
                last_at.clear();
                for id in of_species(&w, ant) {
                    if let Some(h) = w.organism(id).and_then(|s| s.chain.first().copied()) {
                        last_at.insert(id, h);
                    }
                }
            }
        }
        kills_seen.retain(|(t, _)| w.frame.saturating_sub(*t) < 120);
        round.after_step(&mut w, &s);
        let frame = w.frame;
        hall.retain(|(t, _)| frame.saturating_sub(*t) <= HALL);
        if fr % 500 == 0 {
            let alive = of_species(&w, ant).len();
            for _ in alive..cfg.ants {
                let mind = ant_minds[ant_refills as usize % ant_minds.len()].clone();
                if predator::hatch_ant(&mut w, colony, anchor, colony_bed::NEST, 12, mind, 3.0).is_some() {
                    ant_refills += 1;
                }
            }
        }
        if let Some((gr, g0, g1, ge)) = cfg.gif {
            if gr == r && fr >= g0 && fr <= g1 && (fr - g0) % ge == 0 {
                shots.push(picture(&w, cfg.crop, cfg.scale, &kills_seen, frame, ant, spider));
            }
        }
        if cfg.frames_out > 0 && fr % cfg.frames_out == 0 {
            write_frame(&w, &dir.join("frames").join(format!("f{fr:07}.txt")), ant, spider);
        }
        if fr % cfg.judge != 0 {
            continue;
        }
        // `PP_PRED_SCENT=1`: the scent a spider can follow, summed over the six rows above the surface in 32-column
        // bands, every 5,000 frames. Built to ask whether a spider far from the nest has any trail to climb.
        if std::env::var("PP_PRED_SCENT").is_ok() && fr % 5000 == 0 {
            use pixel_physics::sim::pheromone::Channel;
            for (name, ch) in [("A", Channel::A), ("B", Channel::B), ("alarm", Channel::Alarm)] {
                let mut by_band = [0f64; 9];
                for x in 0..colony_bed::SIZE.0 {
                    let top = predator::surface(&w, x);
                    for y in (top - 6).max(0)..top {
                        by_band[((x - colony_bed::NEST + 128) / 32).clamp(0, 8) as usize] += w.pheromone_at(ch, x, y) as f64;
                    }
                }
                eprintln!("scent {name} f{fr} surface, by 32-col band from the left: {:?}", by_band.map(|v| v as u64));
            }
        }
        // ---- the judgement
        let ids: Vec<OrganismId> = book.keys().copied().collect();
        let adults: Vec<OrganismId> = ids.iter().copied().filter(|i| frame.saturating_sub(book[i].born) >= cfg.min_age).collect();
        let mut sc: Vec<(f64, OrganismId)> = adults.iter().map(|&i| (book[&i].score(frame), i)).collect();
        sc.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
        let median = if sc.is_empty() { f64::NAN } else { sc[sc.len() / 2].0 };
        let best = sc.first().map_or(f64::NAN, |b| b.0);
        let (mut nb, mut nc) = (0usize, 0usize);
        if cfg.mode != Mode::Off {
            quota += book.len().max(cfg.lo) as f64 * cfg.judge as f64 / cfg.turn as f64;
            // Parents: living adults off cooldown, best first (or shuffled), then the hall at half score.
            let mut parents: Vec<(f64, Option<OrganismId>, Who)> = sc
                .iter()
                .filter(|(_, i)| frame.saturating_sub(book[i].last_kid) >= cfg.cooldown || book[i].kids == 0)
                .map(|&(v, i)| (v, Some(i), book[&i].clone()))
                .collect();
            parents.extend(hall.iter().map(|(_, b)| (0.5 * b.score(frame), None, b.clone())));
            if cfg.mode == Mode::Random {
                for i in (1..parents.len()).rev() {
                    parents.swap(i, rng.below(i + 1));
                }
            } else {
                parents.sort_by(|a, b| b.0.total_cmp(&a.0));
            }
            let mut want = quota.floor() as usize;
            if book.len() < cfg.lo {
                want = want.max(cfg.lo - book.len());
            }
            let mut k = 0;
            while nb < want && k < parents.len() {
                let (v, pid, p) = &parents[k];
                k += 1;
                // Nobody below the median breeds on the score (the colony's rule), unless the bed is below its band.
                if cfg.mode == Mode::Score && book.len() >= cfg.lo && !median.is_nan() && *v < median {
                    continue;
                }
                let child = neutral_mutate(&p.genome, &slots, cfg.kpm, cfg.ksigma, cfg.kevents, &mut rng);
                let x = spider_column(&mut rng);
                if let Some(id) = predator::place_spider(&mut w, x, pred_colony, spider_mind(&child), cfg.kid_grants) {
                    let links = child[mind::FIRST_LINK..].iter().filter(|v| **v != 0.0).count();
                    writeln!(births_f, "{frame},{id},{},{},{},{},{links},{v:.3}", pid.unwrap_or(0), p.family, p.gen + 1, if pid.is_some() { "birth" } else { "hall" }).unwrap();
                    book.insert(id, Who { born: frame, credit: 0.0, family: p.family.clone(), gen: p.gen + 1, genome: child, last_kid: 0, kids: 0 });
                    if let Some(pid) = pid {
                        if let Some(b) = book.get_mut(pid) {
                            b.last_kid = frame;
                            b.kids += 1;
                        }
                    }
                    nb += 1;
                }
            }
            quota -= nb as f64;
            quota = quota.max(0.0);
            // Culls: the worst adults (or random ones) while above the band.
            let mut victims: Vec<OrganismId> = sc.iter().rev().map(|&(_, i)| i).collect();
            if cfg.mode == Mode::Random {
                for i in (1..victims.len()).rev() {
                    victims.swap(i, rng.below(i + 1));
                }
            }
            for id in victims {
                if book.len() <= cfg.hi {
                    break;
                }
                if creature::cull_organism(&mut w, id) {
                    let b = book.remove(&id).expect("in the book");
                    writeln!(births_f, "{frame},{id},0,{},{},cull,,{:.3}", b.family, b.gen, b.score(frame)).unwrap();
                    nc += 1;
                }
            }
        }
        births += nb as u64;
        culls += nc as u64;
        let m = w.minds.as_ref().map(|m| m.counts).unwrap_or_default();
        let corpse_cells = swallows.saturating_sub(m.flesh);
        writeln!(
            sel,
            "{frame},{},{},{nb},{nc},{starved},{median:.3},{best:.3},{},{},{corpse_cells},{},{ant_refills}",
            book.len(),
            adults.len(),
            m.kills - ants_killed0,
            m.flesh,
            of_species(&w, ant).len()
        )
        .unwrap();
    }
    if !shots.is_empty() {
        let delay = image::Delay::from_saturating_duration(std::time::Duration::from_millis(80));
        let frames: Vec<image::Frame> = shots.into_iter().map(|img| image::Frame::from_parts(img, 0, 0, delay)).collect();
        let file = std::fs::File::create(dir.join("round.gif")).unwrap();
        let mut enc = image::codecs::gif::GifEncoder::new(file);
        enc.set_repeat(image::codecs::gif::Repeat::Infinite).unwrap();
        enc.encode_frames(frames).unwrap();
    }
    // ---- the round's pool: the best living, then the hall, at the score they ended on.
    let frame = w.frame;
    let mut pool: Vec<(f64, Who)> = book.values().map(|b| (b.score(frame), b.clone())).collect();
    pool.extend(hall.iter().map(|(_, b)| (0.5 * b.score(frame), b.clone())));
    pool.sort_by(|a, b| b.0.total_cmp(&a.0));
    pool.truncate(cfg.keep);
    let mut out = Vec::new();
    for (k, (v, b)) in pool.iter().enumerate() {
        let tag = format!("{}.r{r}k{k}", b.family.split('.').next().unwrap_or(""));
        train::save_genome(&dir.join("pool").join(format!("{k:02}.raw.txt")), &b.genome).unwrap();
        out.push((Founder { tag, genome: b.genome.clone() }, *v));
    }
    let m = w.minds.as_ref().map(|m| m.counts).unwrap_or_default();
    let mean_links = if pool.is_empty() { 0.0 } else { pool.iter().map(|(_, b)| b.genome[mind::FIRST_LINK..].iter().filter(|v| **v != 0.0).count() as f64).sum::<f64>() / pool.len() as f64 };
    let line = format!(
        "round {r} seed {seed}: spiders alive {} births {births} culls {culls} starved {starved} | ants killed {} flesh cells {} corpse cells {} | ants alive {} refills {ant_refills} | pool best {:.3} mean {:.3} links {mean_links:.0} | bites {}",
        book.len(),
        m.kills - ants_killed0,
        m.flesh,
        swallows.saturating_sub(m.flesh),
        of_species(&w, ant).len(),
        pool.first().map_or(0.0, |p| p.0),
        if pool.is_empty() { 0.0 } else { pool.iter().map(|p| p.0).sum::<f64>() / pool.len() as f64 },
        m.bites,
    );
    // The eye's count (`options::sight_on`), cumulative over the run: whether the sense fired at all.
    let line = if creature::options::sight_on() {
        let ld = |a: &std::sync::atomic::AtomicU64| a.load(std::sync::atomic::Ordering::Relaxed);
        format!("{line} | sight saw prey {} of {} casts", ld(&creature::options::SIGHT_SEEN), ld(&creature::options::SIGHT_CASTS))
    } else {
        line
    };
    RoundOut { pool: out, line }
}

fn main() {
    // SAFETY: set before any thread exists; read once through `attack_on`'s `OnceLock`.
    std::env::set_var("PIXEL_PHYSICS_MIND_ATTACK", "on");
    assert!(creature::options::attack_on(), "the Attack act did not switch on");
    creature::mutation_off_for_measuring();
    let a: Vec<String> = std::env::args().skip(1).collect();
    let known = [
        "seed", "rounds", "frames", "preds", "ants", "sel", "judge", "min_age", "turn", "keep", "grants", "kid_grants", "kpm", "ksigma", "kevents", "start_events", "founder", "lo", "hi", "cooldown", "out", "frames_out", "step", "gif", "crop", "scale", "pool",
    ];
    for x in &a {
        let k = x.split('=').next().unwrap_or("");
        assert!(known.contains(&k), "unknown argument {x:?}; known: {known:?}");
    }
    let num = |k: &str, d: f64| arg(&a, k).map_or(d, |v| v.parse().unwrap_or_else(|_| panic!("{k}={v}: not a number")));
    let cfg = Cfg {
        seed: num("seed", 1.0) as u64,
        frames: num("frames", 100_000.0) as u64,
        preds: num("preds", 12.0) as usize,
        ants: num("ants", 30.0) as usize,
        mode: match arg(&a, "sel").unwrap_or("score") {
            "score" => Mode::Score,
            "random" => Mode::Random,
            "off" => Mode::Off,
            o => panic!("sel={o}: score, random or off"),
        },
        judge: num("judge", 1000.0) as u64,
        min_age: num("min_age", 6000.0) as u64,
        turn: num("turn", 20_000.0) as u64,
        keep: num("keep", 8.0) as usize,
        grants: num("grants", 1.0) as f32,
        kid_grants: num("kid_grants", 1.0) as f32,
        kpm: num("kpm", 0.03) as f32,
        ksigma: num("ksigma", 1.0) as f32,
        kevents: num("kevents", 10.0) as usize,
        lo: num("lo", 6.0) as usize,
        hi: num("hi", 16.0) as usize,
        cooldown: num("cooldown", 4000.0) as u64,
        frames_out: num("frames_out", 0.0) as u64,
        gif: arg(&a, "gif").map(|v| {
            let p: Vec<u64> = v.split(':').map(|x| x.parse().expect("gif=R:START:END:EVERY")).collect();
            assert_eq!(p.len(), 4, "gif=R:START:END:EVERY");
            (p[0] as usize, p[1], p[2], p[3].max(1))
        }),
        crop: arg(&a, "crop").map_or((0, 20, 256, 70), |v| {
            let p: Vec<i32> = v.split(',').map(|x| x.parse().expect("crop=X,Y,W,H")).collect();
            (p[0], p[1], p[2], p[3])
        }),
        scale: num("scale", 3.0) as u32,
        game: match arg(&a, "step").unwrap_or("game") {
            "game" => true,
            "harness" => false,
            o => panic!("step={o}: game or harness"),
        },
    };
    let rounds = num("rounds", 6.0) as usize;
    let out = std::path::PathBuf::from(arg(&a, "out").expect("out=DIR"));
    // Round 0: the founder (a blank mind on the spider's verbs, or `founder=`) and mutants of it.
    // `pool=DIR`: every `*.raw.txt` in DIR, in name order, as the round-0 founders (no mutants): a trained pool, to
    // judge it (`sel=off`) or to train on from it.
    let pool: Option<Vec<Founder>> = arg(&a, "pool").map(|d| {
        let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(d).unwrap_or_else(|e| panic!("pool={d}: {e}")).filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.to_string_lossy().ends_with(".raw.txt")).collect();
        files.sort();
        assert!(!files.is_empty(), "pool={d}: no .raw.txt in it");
        files
            .iter()
            .enumerate()
            .map(|(k, p)| {
                let text = std::fs::read_to_string(p).unwrap();
                Founder { tag: format!("q{k:02}"), genome: train::parse_genome(&text, mind::GENOME_LEN, &p.display().to_string()) }
            })
            .collect()
    });
    let base = match arg(&a, "founder") {
        Some(name) if !name.contains('/') && !name.ends_with(".txt") => mind::preset(name, predator::VERBS).unwrap_or_else(|| panic!("founder={name}: no such preset")),
        Some(p) => {
            let text = std::fs::read_to_string(p).unwrap_or_else(|e| panic!("founder={p}: {e}"));
            train::parse_genome(&text, mind::GENOME_LEN, p)
        }
        None => mind::blank(VERBS),
    };
    let slots = mind::trainable(VERBS, EMITS);
    let mut rng = Rng(cfg.seed ^ 0xF00D);
    let start_events = num("start_events", 20.0) as usize;
    let mut founders: Vec<Founder> = if let Some(p) = pool { p } else { (0..cfg.preds)
        .map(|k| Founder { tag: format!("p{k:02}"), genome: if k == 0 { base.clone() } else { neutral_mutate(&base, &slots, cfg.kpm, cfg.ksigma, start_events, &mut rng) } })
        .collect() };
    println!(
        "predtrain: seed {} rounds {rounds} x {} frames, {} spiders, {} ants, sel {:?}, judge {} min_age {} turn {} keep {} band {}..{}",
        cfg.seed, cfg.frames, cfg.preds, cfg.ants, cfg.mode, cfg.judge, cfg.min_age, cfg.turn, cfg.keep, cfg.lo, cfg.hi
    );
    let t0 = std::time::Instant::now();
    for r in 0..rounds {
        let ro = run_round(r, &founders, &cfg, &out.join(format!("r{r}")));
        println!("{} ({:.0}s)", ro.line, t0.elapsed().as_secs_f64());
        if ro.pool.is_empty() {
            println!("no spider left to found round {}", r + 1);
            break;
        }
        founders = ro.pool.into_iter().map(|(f, _)| f).collect();
    }
}
