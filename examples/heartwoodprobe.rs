//! **Heartwood, played headless.** Runs the Heartwood probe's valley under a
//! scripted player and prints what happened, season by season, with the
//! frame cost -- and writes the real window's pictures.
//!
//! ```text
//! cargo run --release --example heartwoodprobe -- play=plan seeds=1,2,3
//! cargo run --release --example heartwoodprobe -- play=idle,random,plan seeds=1-6
//! cargo run --release --example heartwoodprobe -- play=plan seeds=1 shots=out/hw gif=out/hw.gif
//! ```
//!
//! `play=` is one or more of:
//! - `idle`: never plays a card (the do-nothing control -- it must lose);
//! - `random`: whenever it can afford something, plays a random card from
//!   the hand on a random tip (Forks everywhere, leaves in its own shade);
//! - `plan`: a fixed plan a first-time player could follow -- climb, fork
//!   twice, lean toward the light, leaf out, and keep leafing the tips that
//!   stand in the sun;
//! - `combo`: the design's showpiece on one tip -- Fork, Fork, Lean, Leaf Out
//!   with a Bud under it holding Grow Up -- then the plan;
//! - the 2026-11-08 playtest's players, which choose nothing: `mash` (key 1
//!   every half second on whatever tip the game has picked), `top` (always
//!   the highest tip, card 1), `overhang` (race the top tip above the rival,
//!   then lean and leaf toward it), `leafonly` (only Leaf Out, on the
//!   best-lit tip) and `selectall` (shift-pick every tip, play card 1; since
//!   one card goes to one tip, the last tip clicked).
//!
//! `shots=PREFIX` writes `PREFIX-<play>-s<seed>-<season>.png` at the end of
//! each season. `gif=PATH` writes one GIF of the first (play, seed) pair,
//! every `every=` frames (default 30). Run from the repository root.

use std::time::Instant;

use pixel_physics::heartwood::hud::{Screen, FRAME_H, FRAME_W};
use pixel_physics::heartwood::{self, CardKind, Heartwood, SEASONS, SEASON_FRAMES};

fn arg(name: &str) -> Option<String> {
    std::env::args().find_map(|a| a.strip_prefix(&format!("{name}=")).map(str::to_string))
}

fn seeds() -> Vec<u64> {
    let s = arg("seeds").unwrap_or_else(|| "1".to_string());
    let mut out = Vec::new();
    for part in s.split(',') {
        if let Some((a, b)) = part.split_once('-') {
            out.extend(a.parse::<u64>().unwrap()..=b.parse::<u64>().unwrap());
        } else {
            out.push(part.parse().unwrap());
        }
    }
    out
}

#[derive(Clone, Copy, PartialEq)]
enum Play {
    Idle,
    Random,
    Plan,
    Combo,
    /// The 2026-11-08 playtest's thoughtless players, so a fix is judged
    /// against the strategies that showed the cards were not a decision.
    Mash,
    Top,
    Overhang,
    LeafOnly,
    SelectAll,
}

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0 >> 33
    }
}

/// One decision, every 15 frames -- a player's pace, not a machine's.
fn decide(g: &mut Heartwood, play: Play, rng: &mut Lcg, combo_done: &mut bool) {
    if g.over {
        return;
    }
    let tips = g.tips();
    if tips.is_empty() || g.hand.is_empty() {
        return;
    }
    match play {
        Play::Idle => {}
        Play::Random => {
            let i = (rng.next() as usize) % g.hand.len();
            if g.sugar < g.hand[i].cost() {
                return;
            }
            let (tag, _) = tips[(rng.next() as usize) % tips.len()];
            g.select(tag);
            g.play(i);
        }
        Play::Combo if !*combo_done => {
            // Wait for a tip a little way up, then stack the combo on it,
            // drawing the cards it needs from wherever they are in the hand
            // (the harness cheats the draw so the showpiece always plays).
            let Some(&(tag, (_, y))) = tips.iter().min_by_key(|&&(_, (_, y))| y) else { return };
            if !g.idle(tag) {
                return;
            }
            if heartwood::GROUND_Y - y < 30 {
                force(g, tag, CardKind::GrowUp);
                force(g, tag, CardKind::GrowUp);
                force(g, tag, CardKind::GrowUp);
                return;
            }
            for c in [CardKind::Bud, CardKind::GrowUp, CardKind::Fork, CardKind::Fork, CardKind::Lean, CardKind::LeafOut] {
                force(g, tag, c);
            }
            *combo_done = true;
        }
        Play::Plan | Play::Combo => plan(g),
        Play::Mash => {
            if g.t % 30 < 15 {
                g.play(0);
            }
        }
        Play::Top => {
            if let Some(&(tag, _)) = tips.iter().min_by_key(|&&(t, (_, y))| (y, t)) {
                g.select(tag);
                g.play(0);
            }
        }
        Play::Overhang => overhang(g),
        Play::LeafOnly => {
            // Leaf Out whenever it is in hand, on the best-lit idle tip; any
            // other card only when the hand is full, so the hand keeps moving.
            let light = |p: (i32, i32)| pixel_physics::sim::plant::ambient_light_above(&g.world, p.0, p.1 - 2);
            let card = g.hand.iter().position(|&c| c == CardKind::LeafOut).or(if g.hand.len() >= heartwood::HAND { Some(0) } else { None });
            if let Some(i) = card {
                if let Some(&(tag, _)) = tips.iter().filter(|&&(t, _)| g.idle(t)).max_by(|a, b| light(a.1).total_cmp(&light(b.1))) {
                    g.select(tag);
                    g.play(i);
                }
            }
        }
        Play::SelectAll => {
            // Picking every tip is no longer possible (one card, one tip):
            // this is what the same clicks do now -- the last tip picked
            // gets card 1.
            for &(tag, _) in &tips {
                g.select(tag);
            }
            g.play(0);
        }
    }
}

/// Put `card` on `tag`'s queue as if it had been in the hand and played.
fn force(g: &mut Heartwood, tag: u32, card: CardKind) {
    g.hand.insert(0, card);
    g.select(tag);
    if !g.play(0) {
        g.hand.remove(0);
    }
}

fn plan(g: &mut Heartwood) {
    // What a player who has read the cards and the outline would do: keep
    // every tip busy, split early for a crown of tips, lean a tip in shadow
    // out toward the low sun, leaf a sunlit one, send a low one up -- and
    // never play a card the outline shows red.
    let tips = g.tips();
    let sun = |p: (i32, i32)| heartwood::sun_at(&g.world, p);
    // Idle tips first; with a full hand, any tip (the card waits its turn).
    let full = g.hand.len() >= heartwood::HAND;
    let mut idle: Vec<(u32, (i32, i32))> = tips.iter().copied().filter(|&(t, _)| full || g.idle(t)).collect();
    idle.sort_by(|a, b| sun(b.1).total_cmp(&sun(a.1)).then(a.0.cmp(&b.0)));
    let n = tips.len();
    for (tag, p) in idle {
        let height = heartwood::GROUND_Y - p.1;
        let lit = sun(p);
        let want: &[CardKind] = if height < 30 {
            &[CardKind::GrowUp, CardKind::Lean, CardKind::Fork]
        } else if n < 5 {
            &[CardKind::Fork, CardKind::LeafOut, CardKind::Lean]
        } else if lit < 0.5 {
            &[CardKind::Lean, CardKind::Fork, CardKind::GrowUp]
        } else {
            &[CardKind::LeafOut, CardKind::Fork, CardKind::Lean]
        };
        for &c in want {
            if let Some(i) = g.hand.iter().position(|&h| h == c) {
                if matches!(g.verdict(c, tag), heartwood::Verdict::No(_)) {
                    continue;
                }
                g.select(tag);
                g.play(i);
                return;
            }
        }
    }
}

/// The playtest's "clever" player: get the top tip above the rival's crown,
/// then lean over toward it and leaf out.
fn overhang(g: &mut Heartwood) {
    let tips = g.tips();
    let Some(&(tag, (_, y))) = tips.iter().min_by_key(|&&(t, (_, y))| (y, t)) else { return };
    let rival_top = g.world.organism(g.rival).and_then(|s| s.cells.keys().map(|c| c.1).min()).unwrap_or(heartwood::GROUND_Y);
    let want: &[CardKind] = if y > rival_top - 4 {
        &[CardKind::GrowUp, CardKind::Lean, CardKind::Fork]
    } else {
        &[CardKind::Lean, CardKind::LeafOut, CardKind::Fork, CardKind::GrowUp]
    };
    for &c in want {
        if let Some(i) = g.hand.iter().position(|&h| h == c) {
            g.select(tag);
            g.play(i);
            return;
        }
    }
    // Nothing it wants in hand: a full hand still gets played.
    if g.hand.len() >= heartwood::HAND {
        g.select(tag);
        g.play(0);
    }
}

fn main() {
    let plays: Vec<(String, Play)> = arg("play")
        .unwrap_or_else(|| "idle,random,plan".to_string())
        .split(',')
        .map(|p| {
            (
                p.to_string(),
                match p {
                    "idle" => Play::Idle,
                    "random" => Play::Random,
                    "plan" => Play::Plan,
                    "combo" => Play::Combo,
                    "mash" => Play::Mash,
                    "top" => Play::Top,
                    "overhang" => Play::Overhang,
                    "leafonly" => Play::LeafOnly,
                    "selectall" => Play::SelectAll,
                    other => panic!("unknown play={other}"),
                },
            )
        })
        .collect();
    let seeds = seeds();
    let shots = arg("shots");
    let gif = arg("gif");
    let every: u64 = arg("every").and_then(|v| v.parse().ok()).unwrap_or(30);
    let gfrom: u64 = arg("gfrom").and_then(|v| v.parse().ok()).unwrap_or(0);
    let gto: u64 = arg("gto").and_then(|v| v.parse().ok()).unwrap_or(u64::MAX);
    let trace: u64 = arg("trace").and_then(|v| v.parse().ok()).unwrap_or(0);
    let extra: u64 = arg("extra").and_then(|v| v.parse().ok()).unwrap_or(0);
    println!(
        "heartwoodprobe: plays={:?} seeds={:?} season={} frames x{} speedup={} shots={:?} gif={:?}",
        plays.iter().map(|p| &p.0).collect::<Vec<_>>(),
        seeds,
        SEASON_FRAMES,
        SEASONS,
        heartwood::SPEEDUP,
        shots,
        gif
    );
    println!("play    seed  season-shares        stars  cards  sugar-spent  forks  branches-lost  cells-lost  buds-fired  my-leaves  rival-leaves  my-tips  tick-mean  tick-worst  draw-worst  blocked");
    let mut first = true;
    for (name, play) in &plays {
        for &seed in &seeds {
            let mut g = Heartwood::new(seed);
            if let Some(c) = arg("cadence") {
                g.world.plant_size_cadence = c == "1";
            }
            let why = arg("why").is_some();
            if why {
                g.world.grow_log = Some(Vec::new());
            }
            let mut whys: std::collections::BTreeMap<String, u64> = Default::default();
            let mut screen = Screen::new();
            let mut frame = vec![0u8; (FRAME_W * FRAME_H * 4) as usize];
            let mut rng = Lcg(seed ^ 0xA5A5);
            let mut combo_done = false;
            let mut ticks: Vec<f64> = Vec::new();
            let mut draws: Vec<f64> = Vec::new();
            let mut gif_frames: Vec<Vec<u8>> = Vec::new();
            let want_gif = first && gif.is_some();
            let total = SEASON_FRAMES * SEASONS as u64 + extra;
            let mut worst_at = (0u64, 0usize);
            let mut prev_cells: std::collections::BTreeSet<(i32, i32)> = Default::default();
            let mut recent: Vec<(u64, (i32, i32), u16, u32)> = Vec::new();
            let mut prev_kind: std::collections::BTreeMap<(i32, i32), &str> = Default::default();
            let mut lost_to: std::collections::BTreeMap<String, u64> = Default::default();
            for f in 0..total {
                if f % 15 == 0 {
                    decide(&mut g, *play, &mut rng, &mut combo_done);
                }
                let steered: std::collections::BTreeSet<(i32, i32)> =
                    if why { g.world.tip_cards.as_ref().map_or_else(Default::default, |t| t.steer.keys().copied().collect()) } else { Default::default() };
                let t0 = Instant::now();
                g.tick();
                let ms = t0.elapsed().as_secs_f64() * 1000.0;
                if why {
                    for r in g.world.grow_log.as_mut().map(std::mem::take).unwrap_or_default() {
                        if r.organism == g.me && !r.root && steered.contains(&(r.x, r.y)) {
                            *whys.entry(format!("{:?}", r.why)).or_default() += 1;
                        }
                    }
                }
                if ms > ticks.iter().copied().fold(0.0, f64::max) {
                    worst_at = (f, g.tips().len());
                }
                ticks.push(ms);
                if arg("whydrop").is_some() {
                    let now: std::collections::BTreeSet<(i32, i32)> =
                        g.world.organism(g.me).map_or_else(Default::default, |s| s.cells.keys().copied().collect());
                    for c in prev_cells.difference(&now) {
                        let cell = g.world.get(c.0, c.1);
                        recent.push((g.t, *c, cell.material.0, cell.organism_id()));
                        let was = prev_kind.get(c).copied().unwrap_or("?");
                        *lost_to.entry(format!("{was}->{}", g.world.materials.get(cell.material).name)).or_default() += 1;
                    }
                    prev_kind = now
                        .iter()
                        .map(|&(x, y)| {
                            let k = match pixel_physics::sim::organism::cell_type(g.world.get(x, y).aux()) {
                                Some(pixel_physics::sim::organism::CellType::Leaf) => "leaf",
                                Some(pixel_physics::sim::organism::CellType::GrowingTip) => "tip",
                                Some(pixel_physics::sim::organism::CellType::RootTip) => "roottip",
                                Some(pixel_physics::sim::organism::CellType::MatureBody) => "body",
                                _ => "other",
                            };
                            ((x, y), k)
                        })
                        .collect();
                    recent.retain(|r| r.0 + 120 > g.t);
                    if prev_cells.len() > now.len() + 20 && arg("whydrop") == Some("2".into()) {
                        let mut early: Vec<_> = recent.iter().filter(|r| r.0 + 3 < g.t).collect();
                        early.sort_by_key(|r| (r.0, -r.1 .1));
                        println!("  DROP at t={} {} -> {}; earlier losses (t, cell, now material, now owner): {:?}", g.t, prev_cells.len(), now.len(), &early[..early.len().min(30)]);
                    }
                    prev_cells = now;
                }
                if trace > 0 && f % trace == 0 {
                    let fl = g.world.structural_failures;
                    let rv = g.world.organism(g.rival);
                    println!(
                        "  t={:>5} felling: snapped {} severed {} (alive {}) shed shade/drought/stranded {}/{}/{} | rival cells {} leaves {} income {:.2}",
                        g.t,
                        fl.snapped_under_load,
                        fl.severed_organism_cells,
                        fl.severed_living_cells,
                        g.world.shed_shade,
                        g.world.shed_drought,
                        g.world.shed_stranded,
                        rv.map_or(0, |s| s.cells.len()),
                        heartwood::leaf_cells(&g.world, g.rival),
                        rv.map_or(0.0, |s| s.income),
                    );
                    let st = g.world.organism(g.me);
                    println!(
                        "  t={:>5} cells={:>4} shoot={:>4} root={:>4} leaves={:>4} tips={:?} doing={:?} queues={:?} sugar={:.1} income={:.3} light=({:.1},{:.1}) hand={:?} held={} steps={}",
                        g.t,
                        st.map_or(0, |s| s.cells.len()),
                        st.map_or(0, |s| s.shoot_cells),
                        st.map_or(0, |s| s.root_cells),
                        heartwood::leaf_cells(&g.world, g.me),
                        g.tips(),
                        g.doing,
                        g.queues.iter().filter(|(_, q)| !q.is_empty()).collect::<Vec<_>>(),
                        g.sugar,
                        st.map_or(0.0, |s| s.income),
                        g.light.0,
                        g.light.1,
                        g.hand,
                        g.world.tip_cards.as_ref().map_or(0, |t| t.held_visits),
                        g.world.tip_cards.as_ref().map_or(0, |t| t.steered_steps),
                    );
                }
                let need_draw = want_gif && f % every == 0 || shots.is_some() && (f + 1) % SEASON_FRAMES == 0 || f % 60 == 0;
                if need_draw {
                    let t1 = Instant::now();
                    screen.draw(&mut g, &mut frame, None);
                    draws.push(t1.elapsed().as_secs_f64() * 1000.0);
                    if want_gif && f % every == 0 && f >= gfrom && f <= gto {
                        gif_frames.push(frame.clone());
                    }
                    if let Some(prefix) = &shots {
                        if (f + 1) % SEASON_FRAMES == 0 {
                            let season = (f + 1) / SEASON_FRAMES;
                            let path = format!("{prefix}-{name}-s{seed}-{season}.png");
                            image::save_buffer(&path, &frame, FRAME_W, FRAME_H, image::ColorType::Rgba8).expect("png");
                        }
                    }
                }
            }
            if why {
                whys.retain(|_, v| *v > 0);
                println!("  steered-tip visits by why: {whys:?}");
            }
            ticks.sort_by(f64::total_cmp);
            draws.sort_by(f64::total_cmp);
            let mean = ticks.iter().sum::<f64>() / ticks.len() as f64;
            let shares: Vec<String> = g.tally.seasons.iter().map(|s| format!("{:>3.0}", s * 100.0)).collect();
            println!(
                "{:<7} {:>4}  {:<20} {:>5}  {:>5}  {:>11.0}  {:>5}  {:>13}  {:>10}  {:>10}  {:>9}  {:>12}  {:>7}  {:>7.2}ms  {:>7.2}ms  {:>8.2}ms  {:>7}",
                name,
                seed,
                shares.join(" "),
                g.stars(),
                g.tally.cards_played,
                g.tally.sugar_spent,
                g.tally.forks,
                g.tally.limbs_lost,
                g.tally.cells_lost,
                g.tally.buds_fired,
                heartwood::leaf_cells(&g.world, g.me),
                heartwood::leaf_cells(&g.world, g.rival),
                g.tips().len(),
                mean,
                ticks.last().copied().unwrap_or(0.0),
                draws.last().copied().unwrap_or(0.0),
                g.tally.blocked,
            );
            if !lost_to.is_empty() {
                println!("        my cells lost (was -> now): {lost_to:?}");
            }
            {
                let p = &g.tally.predictions;
                let mut errs: Vec<f32> = p.iter().map(|x| x.0).collect();
                errs.sort_by(f32::total_cmp);
                let med = |v: &[f32]| if v.is_empty() { 0.0 } else { v[v.len() / 2] };
                let mut travel: Vec<f32> = p.iter().map(|x| x.1).collect();
                travel.sort_by(f32::total_cmp);
                let within = |r: f32| p.iter().filter(|x| x.0 <= r).count();
                println!(
                    "        prediction: {} cards ran out; end missed the outline by median {:.1} cells (p90 {:.1}) over a median outline of {:.1}; within 3 cells {}, within 6 {}",
                    p.len(),
                    med(&errs),
                    errs.get(errs.len() * 9 / 10).copied().unwrap_or(0.0),
                    med(&travel),
                    within(3.0),
                    within(6.0)
                );
            }
            println!("        blocked by reason: {:?}; refused before paying: {:?}; sugar left {:.0}, earned {:.0}, upkeep {:.0}", g.tally.blocked_why, g.tally.refused_why, g.sugar, g.tally.sugar_earned, g.tally.upkeep_paid);
            println!("        worst tick at frame {} with {} tips; p99 tick {:.2} ms", worst_at.0, worst_at.1, ticks[(ticks.len() - 1) * 99 / 100]);
            if want_gif {
                let path = gif.clone().unwrap();
                write_gif(&path, &gif_frames);
                println!("        wrote {path} ({} frames)", gif_frames.len());
            }
            first = false;
        }
    }
}

fn write_gif(path: &str, frames: &[Vec<u8>]) {
    let file = std::fs::File::create(path).expect("gif file");
    let mut enc = image::codecs::gif::GifEncoder::new_with_speed(file, 20);
    enc.set_repeat(image::codecs::gif::Repeat::Infinite).expect("repeat");
    // Half size, so a whole year fits a phone.
    let (w, h) = (FRAME_W / 2, FRAME_H / 2);
    for f in frames {
        let img = image::RgbaImage::from_raw(FRAME_W, FRAME_H, f.clone()).expect("frame");
        let small = image::imageops::resize(&img, w, h, image::imageops::FilterType::Triangle);
        enc.encode_frame(image::Frame::from_parts(small, 0, 0, image::Delay::from_numer_denom_ms(80, 1))).expect("gif frame");
    }
}
