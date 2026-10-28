//! SCRATCH (teacher lane, in-run selection pilot, 2026-10-21): one colony on the training bed with
//! Dig on and surface food, every founder given a mind from a mix of genomes, the body's egg-laying off. Writes the
//! per-ant events the pilot's score is built from, and the nest's open cells every `window` frames.
//!
//! cargo run --release --example inrun -- founders=v2.txt:15,s171.txt:15[,zero:4] frames=200000 seed=1 out=DIR
//!     [dist=40] [drip=5] [every=500] [window=2000] [maps=1] [foodflip=FRAME [foodflip_keep=1]]
//!     (since 2026-10-24 the standard line's settings, `STANDARD`, fill in anything not given; `std=off` for none)
//!
//! Genomes load through the copylog suite, as training loads them. `zero` is an all-zero mind (every option scored
//! alike: a random chooser, the do-nothing-useful control).
//!
//! out/ants.csv     id,tag                     founders, in id order
//! out/events.csv   frame,id,kind,x,y,v,cutter
//!     dig       a cut of a cell that was ground below the datum at frame 0 (v: depth below the datum)
//!     recut     a cut of anything else (spoil, mound, fill); cutter: the tag found within 2 cells, or 0
//!     out       a pellet let go above the datum (left the nest); cutter: the pellet's cutter, or 0
//!     in        a pellet let go at or below the datum; its tag is kept at that spot for a recut
//!     field     a food cell taken into the crop away from home (v: its worth)
//!     deliver   a trip cell put down at the nest: the game's own delivery (`trip_deliveries`; v: its worth as put down)
//!     churn     any other food cell put down within 12 columns of the nest
//!     eat       the crop's digestion paid this ant v (face worth); x,y where its head is
//!     death     v: the cause's index
//! out/map_fNNNNNNN.txt  `x0 y0 w h`, then rows of `.` open / `a` animal / `f` provisions / `#` other, the nest box
//! out/surface.txt       the datum, one row per column
//!
//! **Spiders on the bed** (lone predator thread, stage two, 2026-10-24; off by default: no `spiders=` is the bed
//! bit for bit): `spiders=POOL:N [spider_start=0] [spider_gap=2000] [spider_grants=1] [spider_far=30] [ant_bite=0]
//! [spider_mut=PM:SIGMA:EVENTS[:ELITE]] [spider_out=DIR] [spider_score=total|life] [spider_species=spider|centipede]` keeps N spiders
//! (`assets/species/spider.ron`) with minds from POOL -- a directory of `.raw.txt` (`examples/predtrain.rs`'s
//! `rN/pool`), one `.raw.txt`, or a preset (`hunter`) -- on the bed, each placed at a column at least `spider_far`
//! from the door and replaced `spider_gap` frames after it dies, by the pool's next mind in turn. The pool does not
//! evolve; the first arrive at frame `spider_start`. It sets `PIXEL_PHYSICS_MIND_ATTACK=on`, so the ants' minds are offered the bite and the alarm scent too
//! (`colony_bed::verbs`): a founder that has the bite banned takes it at job constant `ant_bite`
//! (`creature::mind::enter_bite`; 0 = a blank verb). The spiders' own swallows and digestion are kept out of the
//! ants' food rows. More events:
//!     spider    a spider placed (v: its pool index)
//!     spdeath   a spider died (v: the cause's index; 3 = killed, which only an ant can do here)
//!     wound     a living spider lost cells since the frame before (v: how many): what the ants' bites took
//!     death     an ant's, as above: v 3 (killed) is an ant a spider took
use pixel_physics::sim::creature;
use pixel_physics::sim::organism;
use pixel_physics::sim::pheromone;
use pixel_physics::train::challenges::inrun;
use pixel_physics::train::{self};
use std::collections::HashMap;
use std::io::Write;
use std::sync::Arc;

/// Size-neutral mutation (owner, 2026-10-21: brain size should be set by the link cost, not by the mutation
/// operator). Live links and settings take a weight step with probability `pm`; then `events` structural changes,
/// each a fair coin between adding a link (an empty trainable slot gets a step) and removing one (a live link set to
/// 0). The expected change in size is zero at every size, so there is no size the operator drifts toward: `train::mutate`
/// at prune 0 / grow 1 added ~60 links a child (a one-way ratchet), and prune .3 / grow .1 pulled toward ~700.
/// `prot`: per-slot "this slot is a lesson" flags and the factor its weight-step chance is scaled by (review lane,
/// 2026-10-22, `lesson=`); protected slots never take a structural event.
/// SCRATCH (dig-scoring thread): a founder's `.raw.txt`, at its saved length -- GENOME_LEN, or GENOME_LEN plus an
/// evolved dig head's slots under `PP_DIGHEAD_EVOLVE` (a later round's founders keep their heads).
fn load_founder(p: &str) -> Vec<f32> {
    let text = std::fs::read_to_string(p).unwrap_or_else(|e| panic!("genome file {p}: {e}"));
    let saved = text.strip_prefix("genome ").and_then(|t| t.split_whitespace().next()).and_then(|n| n.parse().ok());
    let riders = creature::mind::scratch_log::head_evolve().is_some() || creature::mind::laygene_on();
    let layout = train::genome_text_layout(&text, p);
    // Read at its saved length, so a founder from before the context gates or the reach rows gains them at zero before its riders
    // (`mind::enter_gate`); with no riders read, cut to the bare core.
    let mut g = train::parse_genome(&text, saved.unwrap_or(creature::mind::GENOME_LEN), p);
    creature::mind::enter_gate(&mut g, layout);
    if !riders {
        g.truncate(creature::mind::GENOME_LEN);
    }
    // A founder saved before the last-food memory meets it at weight 0 (`mind::enter_last_food`).
    creature::mind::enter_last_food(&mut g, layout);
    g
}

/// A spider's `.raw.txt`, met by the last-food memory at weight 0 if saved before it (`mind::enter_last_food`).
fn spider_file(text: &str, name: &str) -> Vec<f32> {
    let mut g = train::parse_genome(text, creature::mind::GENOME_LEN, name);
    creature::mind::enter_last_food(&mut g, train::genome_text_layout(text, name));
    // Its gate and reach rows were read where an older file has none: zero, as `mind::enter_gate` would leave them.
    g
}

/// SCRATCH (brood lane, 2026-10-24): under `PP_LAYGENE` (on by default) every genome ends in a lay gate of
/// [`creature::mind::LAY_LEN`] slots ([`creature::mind::lay_bar_scale`]); a founder saved without one gets
/// `PP_LAYGENE_INIT=b,w1,...,w5` (default all 0, which lays exactly as no gate). Switch off, a saved gate is dropped.
/// Returns the genome without its gate, and the gate to put back after the dig head is checked.
fn split_lay(mut g: Vec<f32>) -> (Vec<f32>, Option<Vec<f32>>) {
    let base = creature::mind::GENOME_LEN + creature::mind::scratch_log::head_evolve().unwrap_or(0);
    let n = creature::mind::LAY_LEN;
    let saved = (g.len() == base + n).then(|| g.split_off(base));
    if !creature::mind::laygene_on() {
        return (g, None);
    }
    let lay = saved.unwrap_or_else(|| match std::env::var("PP_LAYGENE_INIT") {
        Ok(v) => {
            let w: Vec<f32> = v.split(',').map(|x| x.trim().parse().expect("PP_LAYGENE_INIT=b,w1,...,w5")).collect();
            assert_eq!(w.len(), n, "PP_LAYGENE_INIT takes {n} numbers");
            w
        }
        Err(_) => vec![0.0; n],
    });
    (g, Some(lay))
}

/// SCRATCH (brood lane, 2026-10-24): **a child's lay gate mutates on its own schedule**, after the mind's operator
/// (which never sees these slots: `inrun::slots` stops before them). Each slot steps with chance `PP_LAYGENE_PM`
/// (default 0.2) by `PP_LAYGENE_SIGMA` (default 0.15) times a unit step. **Why its own**: the neutral operator steps
/// live links and adds a link to any of ~2,800 empty slots at random, so six zero-born gate slots would wait
/// rounds for a first event. Switch off, untouched (no draw, so the run is the run without the gate).
fn lay_mutate(g: &mut [f32], rng: &mut train::Rng) {
    if !creature::mind::laygene_on() {
        return;
    }
    static PM: std::sync::OnceLock<(f32, f32)> = std::sync::OnceLock::new();
    let (pm, sigma) = *PM.get_or_init(|| {
        let f = |k: &str, d: f32| std::env::var(k).ok().map_or(d, |v| v.parse().unwrap_or_else(|_| panic!("{k}")));
        (f("PP_LAYGENE_PM", 0.2), f("PP_LAYGENE_SIGMA", 0.15))
    });
    // PP_LAYGENE_SLOTS=0,3: only these gate slots mutate (default all). The bias alone (`PP_LAYGENE_SLOTS=0`) is the
    // TRAIT_REPRODUCE_AT arm -- "wait for 2^-b times the bar", learned -- with the five local senses held at 0.
    static MASK: std::sync::OnceLock<Vec<bool>> = std::sync::OnceLock::new();
    let mask = MASK.get_or_init(|| match std::env::var("PP_LAYGENE_SLOTS") {
        Ok(v) => {
            let on: Vec<usize> = v.split(',').map(|x| x.trim().parse().expect("PP_LAYGENE_SLOTS=i,j,...")).collect();
            (0..creature::mind::LAY_LEN).map(|i| on.contains(&i)).collect()
        }
        Err(_) => vec![true; creature::mind::LAY_LEN],
    });
    let n = g.len();
    for (v, &on) in g[n - creature::mind::LAY_LEN..].iter_mut().zip(mask) {
        if on && rng.unit() < pm {
            *v += rng.step() * sigma;
        }
    }
}

/// SCRATCH (dig-scoring thread): under `PP_DIGHEAD_EVOLVE`, a genome without its own dig head gets the side file's
/// (`PP_DIGHEAD`) appended; otherwise unchanged.
fn with_head(g: Vec<f32>) -> Vec<f32> {
    let (mut g, lay) = split_lay(g);
    if let Some(hl) = creature::mind::scratch_log::head_evolve() {
        if g.len() == creature::mind::GENOME_LEN {
            g.extend(creature::mind::scratch_log::dighead().expect("PP_DIGHEAD").flat());
        }
        assert_eq!(g.len(), creature::mind::GENOME_LEN + hl, "a founder with a dig head of another shape");
    }
    // SCRATCH (lone predator thread): under PIXEL_PHYSICS_MIND_ATTACK (set by `spiders=`), a founder banned from
    // biting takes the bite at job constant `ant_bite=` (default 0, a blank verb's entry); otherwise nothing.
    let k = std::env::args().find_map(|a| a.strip_prefix("ant_bite=").map(|v| v.parse::<f32>().expect("ant_bite=K"))).unwrap_or(0.0);
    creature::mind::enter_bite(&mut g, k);
    g.extend(lay.unwrap_or_default());
    g
}

fn neutral_mutate(g: &[f32], slots: &[usize], pm: f32, sigma: f32, events: usize, prot: Option<(&[bool], f32)>, rng: &mut train::Rng) -> Vec<f32> {
    let first = creature::mind::FIRST_LINK;
    let mut c = g.to_vec();
    for &i in slots {
        let p = match prot {
            Some((m, f)) if m.get(i).copied().unwrap_or(false) => pm * f,
            _ => pm,
        };
        if (c[i] != 0.0 || i < first) && rng.unit() < p {
            c[i] = (c[i] + sigma * rng.step()).clamp(-40.0, 40.0);
        }
    }
    for _ in 0..events {
        let add = rng.unit() < 0.5;
        let pool: Vec<usize> = slots.iter().copied().filter(|&i| i >= first && (c[i] == 0.0) == add && !prot.is_some_and(|(m, _)| m.get(i).copied().unwrap_or(false))).collect();
        if pool.is_empty() {
            continue;
        }
        let i = pool[rng.below(pool.len())];
        c[i] = if add { (sigma * rng.step()).clamp(-40.0, 40.0) } else { 0.0 };
    }
    c
}

/// SCRATCH (teacher lane, inherited nests, 2026-10-22): the ground around the nest as material names, one char per cell,
/// written at the end of every run as `nestmap.txt` and read back by `inherit=PATH` into the next round's fresh world, so
/// a colony keeps the nest it dug instead of starting every round from the 26-cell stamp (selection bred digging out
/// because most generations lived where there was nothing left worth cutting: Reports/brain-training/inrun-selection/).
/// `?` = leave the fresh world's cell (an organism's body, food, or an animal); other chars map through the legend line.
const NESTMAP: (i32, i32, i32) = (100, 40, 128); // half-width about the door, first row, rows end (exclusive)
fn write_nestmap(w: &pixel_physics::sim::World, path: &std::path::Path) {
    use pixel_physics::sim::material;
    let (hw, y0, y1) = NESTMAP;
    let mut legend: Vec<String> = Vec::new();
    let mut rows = Vec::new();
    let food = w.materials.id_of("provisions");
    for y in y0..y1 {
        let mut row = String::new();
        for x in inrun::NEST - hw..=inrun::NEST + hw {
            let c = w.get(x, y);
            // `%`: a plant's own cell (a root, a stem, a seed), kept apart from `?` so a garden's roots read as the
            // ground they stand in (nestcount.py) instead of as open nest; inherited like `?` (left to the fresh world).
            if c.organism_id() != 0 && w.organism(c.organism_id()).is_some_and(|s| w.species.get(s.species).creature.is_none()) {
                row.push('%');
                continue;
            }
            if c.organism_id() != 0 || Some(c.material) == food || w.materials.kind(c.material) == material::MaterialKind::Creature {
                row.push('?');
                continue;
            }
            let name = w.materials.get(c.material).name.to_string();
            let k = legend.iter().position(|n| *n == name).unwrap_or_else(|| { legend.push(name); legend.len() - 1 });
            row.push(char::from(b'A' + k as u8));
        }
        rows.push(row);
    }
    let mut f = std::fs::File::create(path).unwrap();
    writeln!(f, "{} {y0} {}", inrun::NEST - hw, legend.join(",")).unwrap();
    for r in rows {
        writeln!(f, "{r}").unwrap();
    }
}
/// What `nestmap.txt` writes as `?`, told apart (brood lane, 2026-10-23), over the same window: `f` stored food, `e` `l`
/// `p` an egg, larva or pupa, `a` an animal, `.` anything else. Read only.
fn write_occupants(w: &pixel_physics::sim::World, path: &std::path::Path) {
    use pixel_physics::sim::material;
    let (hw, y0, y1) = NESTMAP;
    let food = w.materials.id_of("provisions");
    let mut f = std::fs::File::create(path).unwrap();
    writeln!(f, "{} {y0}", inrun::NEST - hw).unwrap();
    for y in y0..y1 {
        let row: String = (inrun::NEST - hw..=inrun::NEST + hw)
            .map(|x| {
                let c = w.get(x, y);
                let id = c.organism_id();
                if let Some(b) = (id != 0).then(|| w.organism(id).and_then(|s| s.brood)).flatten() {
                    ['e', 'l', 'p'][b.stage as usize]
                } else if id != 0 || w.materials.kind(c.material) == material::MaterialKind::Creature {
                    'a'
                } else if Some(c.material) == food {
                    'f'
                } else {
                    '.'
                }
            })
            .collect();
        writeln!(f, "{row}").unwrap();
    }
}
/// Write a `nestmap.txt` into a fresh world: every cell whose recorded material differs is set to it, except cells
/// that hold an organism now (the founders, plants) and `?` cells. Returns how many cells changed.
///
/// `nest_only` (`inherit_nest=1`): carry the nest and nothing else. colselinh-1022 carried the whole terrain and every
/// arm got worse (lexicase: starved 8.0 vs 2.2 of 24 in the garden, rooms 0.35 vs 1.0; in-run starvation 4x in round
/// 1): the crater, the spoil let go in old tunnels and the packed mounds round the food heaps all carried over and
/// grew round on round. So here only the open cells joined to the door below the datum are carved, and of those only
/// roofed ones (ground at or below the datum above them) or under a mouth no wider than the stamped one: the crater is
/// sky-open and stays behind, and everything else is the fresh world's.
fn read_nestmap(w: &mut pixel_physics::sim::World, path: &str, nest_only: Option<&[i32]>, link: Option<usize>) -> usize {
    let t = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("inherit={path}: {e}"));
    let mut lines = t.lines();
    let head: Vec<&str> = lines.next().expect("nestmap header").split(' ').collect();
    let (x0, y0): (i32, i32) = (head[0].parse().unwrap(), head[1].parse().unwrap());
    let ids: Vec<_> = head.get(2).map_or(vec![], |l| l.split(',').map(|n| w.materials.id_of(n).unwrap_or_else(|| panic!("nestmap material {n}"))).collect());
    let grid: Vec<&[u8]> = lines.map(|l| l.as_bytes()).filter(|l| !l.is_empty()).collect();
    // nest_only: flood the open cells below the datum from the door, keep the roofed ones and the shaft.
    let keep: Option<std::collections::HashSet<(usize, usize)>> = nest_only.map(|surf| {
        // Passes anything that is not ground (stored food, crumbs, markers, bodies), as nestcount.py does since the
        // crater review (2026-10-23): a larder of stored food used to cut its room off, so the room was not inherited.
        let gnd: Vec<u8> = ids.iter().enumerate().filter(|(_, &m)| ["soil", "packedsoil", "spoil", "stone"].iter().any(|n| w.materials.id_of(n) == Some(m))).map(|(k, _)| b'A' + k as u8).collect();
        let open = |dx: usize, dy: usize| grid.get(dy).and_then(|r| r.get(dx)).is_some_and(|c| !gnd.contains(c) && *c != b'%');
        let below = |dx: usize, dy: usize| { let x = x0 + dx as i32; x >= 0 && (x as usize) < surf.len() && y0 + dy as i32 >= surf[x as usize] };
        let door = (inrun::NEST - x0) as usize;
        let mut seen = std::collections::HashSet::new();
        let mut stack: Vec<(usize, usize)> = (0..grid.len()).filter(|&dy| open(door, dy) && below(door, dy)).map(|dy| (door, dy)).collect();
        while let Some((dx, dy)) = stack.pop() {
            if !seen.insert((dx, dy)) {
                continue;
            }
            for (ex, ey) in [(dx.wrapping_sub(1), dy), (dx + 1, dy), (dx, dy.wrapping_sub(1)), (dx, dy + 1)] {
                if ey < grid.len() && open(ex, ey) && below(ex, ey) && !seen.contains(&(ex, ey)) {
                    stack.push((ex, ey));
                }
            }
        }
        // Nest = roofed by ground (soil, packed soil, spoil, stone) at or below the datum, or under a mouth no wider than
        // the stamped one (5 cells, markers included) -- the same rule as nestcount.py (2026-10-22 20:50Z): the door
        // markers and the 3-wide door column used to carry an open pit across as nest.
        let ground: Vec<u8> = head.get(2).map_or(vec![], |l| l.split(',').enumerate().filter(|(_, m)| matches!(*m, "soil" | "packedsoil" | "spoil" | "stone")).map(|(k, _)| b'A' + k as u8).collect());
        let width = grid.first().map_or(0, |r| r.len());
        let top = |dx: usize| (surf[(x0 + dx as i32).clamp(0, surf.len() as i32 - 1) as usize] - y0).max(0) as usize;
        let roofed = |dx: usize, dy: usize| (top(dx)..dy).any(|yy| grid[yy].get(dx).is_some_and(|c| ground.contains(c)));
        let gap: Vec<bool> = (0..width).map(|dx| grid.get(top(dx)).and_then(|r| r.get(dx)).is_some_and(|c| !ground.contains(c))).collect();
        let mut wide = vec![false; width];
        let mut dx = 0;
        while dx < width {
            if !gap[dx] {
                dx += 1;
                continue;
            }
            let mut k = dx;
            while k < width && gap[k] {
                k += 1;
            }
            wide[dx..k].fill(k - dx > 5);
            dx = k;
        }
        let mut seen: Vec<(usize, usize)> = seen.into_iter().collect();
        seen.sort_unstable();
        let mut kept: std::collections::HashSet<(usize, usize)> = seen.iter().copied().filter(|&(dx, dy)| roofed(dx, dy) || !wide[dx]).collect();
        // `inherit_link=MIN`: join every kept room of at least MIN cells to the door, and drop the smaller pockets
        // (MIN 0 or 1 links them all). The crater filter above leaves a room whose way to the door ran through the
        // crater as a sealed pocket in the fresh world (192 maps of nestmaps-1022, 2026-10-23: median 115 cells
        // carved, 30 joined to the door -- the stamp and little else), and a sealed pocket is no nest to an ant. So
        // each pocket also keeps the cheapest run of last round's open cells to what the fresh world already joins to
        // the door, two cells wide like the stamped shaft, crater or not. Only the run is carried: the crater round it
        // stays behind. 20 is the tuned MIN (same 192 maps): linking every pocket gave 49 maps runs longer than the
        // pockets they served -- a crater bowl's edge scraps turned into a tunnel tree no ant dug (cs3lexles c20, 105
        // cells) -- against 4 at MIN 10, 2 at 15, 0 at 20 and 30; 20 keeps 86% of the pocket cells and 50 starts
        // dropping real rooms.
        if let Some(min) = link {
            let wg: Vec<_> = ["soil", "packedsoil", "spoil", "stone"].iter().filter_map(|n| w.materials.id_of(n)).collect();
            let rows = grid.len();
            let fresh_open = |dx: usize, dy: usize| dx < width && dy < rows && below(dx, dy) && !wg.contains(&w.get(x0 + dx as i32, y0 + dy as i32).material);
            let nbrs = |(dx, dy): (usize, usize)| [(dx.wrapping_sub(1), dy), (dx + 1, dy), (dx, dy.wrapping_sub(1)), (dx, dy + 1)];
            // What the new world joins to the door: kept cells and the fresh world's own open ground (the stamp), flooded
            // from the mouth only -- the first passable cell down the door column. Seeding from every open cell in that
            // column (as the flood above does) counts a buried pocket that happens to cross it as joined.
            let mut joined = std::collections::HashSet::new();
            let pass = |p: (usize, usize)| kept.contains(&p) || fresh_open(p.0, p.1);
            let mut stack: Vec<(usize, usize)> = (top(door)..rows).map(|dy| (door, dy)).find(|&p| pass(p)).into_iter().collect();
            while let Some(p) = stack.pop() {
                if !joined.insert(p) {
                    continue;
                }
                stack.extend(nbrs(p).into_iter().filter(|&q| !joined.contains(&q) && pass(q)));
            }
            // Cheapest runs from all of it through last round's open cells; `from` leads each cell back toward the door.
            // Never the surface row itself (an unroofed scrape is the crater over again), and a cell in the top six
            // rows costs four, so a run goes under rather than along the surface where it can (first draft, 2026-10-23:
            // plain shortest runs skimmed the top row across 30 columns on cs3lexles c20).
            let old: std::collections::HashSet<(usize, usize)> = seen.iter().copied().filter(|&(dx, dy)| dy > top(dx)).collect();
            let cost = |(dx, dy): (usize, usize)| if dy < top(dx) + 6 { 4 } else { 1 };
            let mut from: std::collections::HashMap<(usize, usize), (usize, usize)> = std::collections::HashMap::new();
            let mut dist: std::collections::HashMap<(usize, usize), u32> = joined.iter().map(|&p| (p, 0)).collect();
            let mut heap: std::collections::BinaryHeap<std::cmp::Reverse<(u32, (usize, usize))>> = joined.iter().map(|&p| std::cmp::Reverse((0, p))).collect();
            while let Some(std::cmp::Reverse((d, p))) = heap.pop() {
                if d > dist[&p] {
                    continue;
                }
                for r in nbrs(p) {
                    let nd = d + cost(r);
                    if old.contains(&r) && dist.get(&r).is_none_or(|&o| nd < o) {
                        dist.insert(r, nd);
                        from.insert(r, p);
                        heap.push(std::cmp::Reverse((nd, r)));
                    }
                }
            }
            // One run per pocket, from its cell nearest the door.
            let mut done = std::collections::HashSet::new();
            let mut pockets: Vec<(usize, usize)> = kept.iter().copied().filter(|p| !joined.contains(p)).collect();
            pockets.sort_unstable();
            let mut runs = Vec::new();
            let mut dropped = Vec::new();
            let mut joins = Vec::new();
            for &p0 in &pockets {
                if done.contains(&p0) {
                    continue;
                }
                let mut comp = vec![p0];
                let mut i = 0;
                done.insert(p0);
                while i < comp.len() {
                    for r in nbrs(comp[i]) {
                        if kept.contains(&r) && !joined.contains(&r) && done.insert(r) {
                            comp.push(r);
                        }
                    }
                    i += 1;
                }
                // A pocket smaller than `min` is not a room: it is a scrap of the crater's edge left under an overhang
                // (cs3lexles c20, 2026-10-23: one sky-open bowl kept as 5-8 such scraps, and linking each drew 103 cells
                // of branching tunnel no ant dug). It is dropped, not linked.
                if comp.len() < min {
                    dropped.extend(comp);
                    continue;
                }
                if let Some(&best) = comp.iter().filter(|p| dist.contains_key(p)).min_by_key(|p| (dist[p], **p)) {
                    joins.push(comp.len());
                    let mut p = best;
                    while let Some(&prev) = from.get(&p) {
                        runs.push(p);
                        p = prev;
                    }
                }
            }
            let before = kept.len();
            for p in &dropped {
                kept.remove(p);
            }
            for (dx, dy) in runs {
                kept.insert((dx, dy));
                if old.contains(&(dx + 1, dy)) {
                    kept.insert((dx + 1, dy));
                } else if dx > 0 && old.contains(&(dx - 1, dy)) {
                    kept.insert((dx - 1, dy));
                }
            }
            eprintln!("inherit_link={min}: dropped {} cells in small pockets, linked {} cells, pockets joined {joins:?}", dropped.len(), kept.len() + dropped.len() - before);
        }
        kept
    });
    let mut n = 0;
    for (dy, row) in grid.iter().enumerate() {
        for (dx, &ch) in row.iter().enumerate() {
            // `?` (an animal, food, an organism's cell) is left to the fresh world -- except, in nest-only mode, a kept
            // one: the flood passed it because it is not ground, so it stood in an open nest cell (an ant in a tunnel,
            // stored food in a room), and leaving it the fresh world's loose soil plugged the nest at every one of them.
            // `INHERIT_PLUG=1` puts the plugs back, for the before/after. `%` (a plant's own cell, `plants=`) is always
            // left: a root is ground to the flood, never a nest cell to carve.
            let plug = std::env::var("INHERIT_PLUG").as_deref() == Ok("1");
            if ch == b'%' || (ch == b'?' && (keep.is_none() || plug)) || keep.as_ref().is_some_and(|k| !k.contains(&(dx, dy))) {
                continue;
            }
            let (x, y) = (x0 + dx as i32, y0 + dy as i32);
            // Nest-only inheritance carves the kept cells open and copies nothing into them. Since the flood passes
            // stored food (2026-10-23), kept cells include the larder, and copying their material carried every
            // round's stored food into the next: colsel14's colonies came to "deliver" 10-13k a garden (control
            // 11,143 at round 47 against colsel10's 2,016) by re-dropping it, and their nests and craters with it.
            let m = if keep.is_some() { pixel_physics::sim::material::EMPTY } else { ids[(ch - b'A') as usize] };
            let c = w.get(x, y);
            if c.organism_id() != 0 || c.material == m {
                continue;
            }
            w.set(x, y, pixel_physics::sim::Cell::new(m, 0));
            n += 1;
        }
    }
    // Line the carved nest, as the ants that dug it did and as the stamp's founding shaft is lined. Carved into the fresh
    // world's loose soil with nothing tamped, every kept hole was a void under a powder: the old map's lining is no use
    // here, because the crater rule above keeps only the roofed edges of a crater-topped room, and their inner wall is
    // the crater's place in the fresh world -- loose soil. Measured 2026-10-23 (cs3lexlesinh c5, one old ant): 172 kept
    // cells filled from above inside 400 frames and the voids came out at the surface, two rows lost across 80 columns.
    // One pass after the carve, the founding shaft's order (sorted: `keep` is a HashSet). Kept cells holding an
    // organism are left alone. `INHERIT_LINE=0` turns it off, for the before/after.
    if let Some(k) = keep.as_ref().filter(|_| std::env::var("INHERIT_LINE").as_deref() != Ok("0")) {
        let mut cells: Vec<_> = k.iter().copied().collect();
        cells.sort_unstable();
        for (dx, dy) in cells {
            let (x, y) = (x0 + dx as i32, y0 + dy as i32);
            if w.get(x, y).material == pixel_physics::sim::material::EMPTY {
                w.line_dug_cell(x, y);
            }
        }
    }
    n
}

/// **The standard training line, on by default** (owner, 2026-10-24: "if we have decided these are the correct path
/// forward they should be default on"): the per-round settings every colony-selection batch has run since colsel21
/// (Teacher lane; breeding's `kevents`/`kpm` since brood1). They go after the command line, and [`arg`] takes the
/// first match, so anything given wins; `std=off` runs the harness's old defaults. The senses they were trained with
/// (home sense, dig head, light sense, grazing) are default-on in the sim itself, and `founders=` and the lesson
/// stay on the job line.
const STANDARD: &[&str] = &[
    "dist=25",
    "spread=10",
    "grants=3",
    "window=4000",
    "min_age=8000",
    "cooldown=3000",
    "w_deliver=7",
    "w_field=0.05",
    "w_depth=2",
    "rim=15",
    "cap=60",
    "fresh_kid=1",
    "drip=20",
    "mutation=neutral",
    "kevents=4",
    // 0.013 until 2026-10-27 (Teacher lane, plan A1): at 0.013 a child carries ~26 changed weights plus 4 structural
    // events and every birth in a round mutates, which outran colony selection -- drift1's hand-laid come-home latch
    // decayed (garden starved 46 -> 311 in 48 rounds) and self1's std_pool sold its nest for food (596 -> 129); at
    // 0.003 the same selection kept both (drift1: starved 51 -> 28, nest 702 -> 700). Every batch since elite1 passed
    // kpm=0.003 by hand, and clean1 found nothing better (PR #955). A batch that must match an older one passes kpm=0.013.
    "kpm=0.003",
    "w_home_eat=2",
    "kid_cap=4",
    "retire=1",
    "hall=30000",
    "fprice=tank",
    "w_hungry=10",
    "progress=1",
    "turn=30000",
    "kid_grants=3",
    "cut_join=32",
];

fn with_standard(mut a: Vec<String>) -> Vec<String> {
    if arg(&a, "std") != Some("off") {
        let given: Vec<String> = STANDARD.iter().filter(|s| arg(&a, s.split_once('=').unwrap().0).is_none()).map(|s| s.to_string()).collect();
        println!("standard line (std=off for the old defaults), not given so taken: {}", given.join(" "));
        a.extend(given);
    }
    a
}

fn arg<'a>(a: &'a [String], k: &str) -> Option<&'a str> {
    a.iter().find_map(|s| s.strip_prefix(&format!("{k}=")))
}

/// One spider placement in [`Spiders::log`]: (organism id, pool index, genome, born, died, flesh credit, mutant,
/// breeding genes).
type Placement = (u32, usize, Vec<f32>, u64, Option<u64>, f64, bool, [f32; 2]);

/// SCRATCH (lone predator thread, stage two, 2026-10-24): **a pool of spiders on the bed** (`spiders=POOL:N`), the
/// predator the colony learns to survive. Each place is refilled `gap` frames after its spider dies, by the pool's
/// next mind in turn, at a random column at least `far` from the door. The pool does not evolve inside a run; under
/// `spider_mut=PM:SIGMA:EVENTS` (stage three, co-evolution) each placement is a size-neutral mutant of its pool mind,
/// and `spider_out=DIR` writes every placement's genome and score, from which colsel's `spiderevo=` picks the next
/// round's pool.
struct Spiders {
    /// `spider_species=`: `spider` (default) or the nest raider `centipede`.
    species: String,
    pool: Vec<Arc<creature::mind::MindGenome>>,
    /// The pool's genomes as read, the parents of `spider_mut`'s placements.
    raw: Vec<Vec<f32>>,
    /// Each pool mind's breeding genes (`NN.body.txt`, `predator::BREEDING`), `None` for the species' own. Carried and
    /// mutated with the mind, never acted on here: this bed's spiders do not breed. Selected where they do
    /// (colsel `spiderlab=`).
    bodies: Vec<Option<[f32; 2]>>,
    /// The breeding genes' own draw, apart from `rng`, so a run's placements and minds are the run before them.
    brng: train::Rng,
    /// `spider_mut=PM:SIGMA:EVENTS[:ELITE]`, and the slots a spider's mutation moves.
    mutate: Option<(f32, f32, usize, Vec<usize>)>,
    /// `spider_mut`'s ELITE: the share of placements that are their pool mind unmutated, so a parent is judged again
    /// (co-evolution's first batch, 2026-10-24: every placement a mutant, each scored on one life, and ten rounds of
    /// keeping the luckiest made the pool hunt worse -- fewer ants in 49 of 77 paired judgings).
    elite: f32,
    /// Every placement: (organism id, pool index, genome, born, died, flesh credit).
    log: Vec<Placement>,
    n: usize,
    gap: u64,
    far: i32,
    grants: f32,
    colony: Option<u32>,
    /// The living spiders' ids: their food and digestion rows are not the ants'.
    ids: std::collections::HashSet<u32>,
    /// Each living spider's cell count at the last look, for `wound` rows.
    size: HashMap<u32, usize>,
    /// Frames at which an empty place may be refilled (`spider_start` at first: every place is empty).
    due: Vec<u64>,
    next: usize,
    rng: train::Rng,
    sp: organism::SpeciesId,
    ant: organism::SpeciesId,
    placed: u64,
    deaths: u64,
    killed: u64,
    starved: u64,
    ants_killed: u64,
    wounds: u64,
}

impl Spiders {
    fn new(a: &[String], w: &mut pixel_physics::sim::World, seed: u64) -> Option<Spiders> {
        let spec = arg(a, "spiders")?;
        let (pool, n) = spec.rsplit_once(':').expect("spiders=POOL:N");
        let n: usize = n.parse().expect("spiders=POOL:N");
        let species = arg(a, "spider_species").unwrap_or("spider").to_string();
        let verbs = pixel_physics::train::predator::verbs_for(&species);
        let (genomes, bodies): (Vec<Vec<f32>>, Vec<Option<[f32; 2]>>) = if std::path::Path::new(pool).is_dir() {
            let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(pool).unwrap().filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.to_string_lossy().ends_with(".raw.txt")).collect();
            files.sort();
            files
                .iter()
                .map(|p| (spider_file(&std::fs::read_to_string(p).unwrap(), &p.display().to_string()), pixel_physics::train::predator::read_breeding(p)))
                .unzip()
        } else if pool.ends_with(".txt") {
            let g = spider_file(&std::fs::read_to_string(pool).unwrap_or_else(|e| panic!("spiders={pool}: {e}")), pool);
            (vec![g], vec![pixel_physics::train::predator::read_breeding(std::path::Path::new(pool))])
        } else {
            (vec![creature::mind::preset(pool, verbs).unwrap_or_else(|| panic!("spiders={pool}: not a directory, a .raw.txt or a preset"))], vec![None])
        };
        assert!(!genomes.is_empty(), "spiders={pool}: no .raw.txt in it");
        // `spider_species=centipede`: the nest raider (2026-10-25) in the spider's place, on every rule above, with its
        // own verbs (`predator::verbs_for`).
        let sp = w.species.id_of(&species).unwrap_or_else(|| panic!("spider_species={species}: not in the species table"));
        let ant = w.species.id_of("ant").expect("ant");
        // The pool is fixed: no spider breeds or ages out, as on predtrain's bed.
        let mut def = w.species.get(sp).creature.clone().expect("spider is a creature");
        def.reproduce_threshold = 1.0e30;
        def.life_half_life = 0;
        w.species.set_creature(sp, def);
        let num = |k: &str, d: f64| arg(a, k).map_or(d, |v| v.parse().unwrap());
        Some(Spiders {
            species: species.clone(),
            pool: genomes.iter().map(|g| pixel_physics::train::predator::mind_for(&species, g)).collect(),
            raw: genomes.clone(),
            bodies,
            brng: train::Rng(seed.wrapping_mul(0x2545_F491_4F6C_DD1D) ^ 0xB0D1),
            mutate: arg(a, "spider_mut").map(|v| {
                let p: Vec<&str> = v.split(':').collect();
                assert!(p.len() == 3 || p.len() == 4, "spider_mut=PM:SIGMA:EVENTS[:ELITE]");
                let slots = creature::mind::trainable(verbs, pixel_physics::train::predator::EMITS);
                (p[0].parse().unwrap(), p[1].parse().unwrap(), p[2].parse().unwrap(), slots)
            }),
            elite: arg(a, "spider_mut").and_then(|v| v.split(':').nth(3)).map_or(0.0, |e| e.parse().expect("spider_mut ELITE")),
            log: Vec::new(),
            n,
            gap: num("spider_gap", 2000.0) as u64,
            far: num("spider_far", 30.0) as i32,
            grants: num("spider_grants", 1.0) as f32,
            colony: None,
            ids: Default::default(),
            size: HashMap::new(),
            due: vec![num("spider_start", 0.0) as u64; n],
            next: 0,
            rng: train::Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0x5D1D),
            sp,
            ant,
            placed: 0,
            deaths: 0,
            killed: 0,
            starved: 0,
            ants_killed: 0,
            wounds: 0,
        })
    }

    /// Every empty place that is due, refilled.
    fn fill(&mut self, w: &mut pixel_physics::sim::World, frame: u64, ev: &mut impl Write) {
        let bw = inrun::bed_w();
        while self.ids.len() < self.n {
            let Some(k) = self.due.iter().position(|&d| d <= frame) else { break };
            let x = loop {
                let x = 8 + self.rng.below((bw - 16) as usize) as i32;
                if (x - inrun::NEST).abs() >= self.far {
                    break x;
                }
            };
            let g = self.next % self.pool.len();
            // The draw for ELITE is taken only when ELITE is set, so a run without it is the run before it.
            let elite = self.elite > 0.0 && self.rng.unit() < self.elite;
            let (genome, mind, mutant) = match &self.mutate {
                Some((pm, sigma, events, slots)) if !elite => {
                    let child = neutral_mutate(&self.raw[g], slots, *pm, *sigma, *events, None, &mut self.rng);
                    let m = pixel_physics::train::predator::mind_for(&self.species, &child);
                    (child, m, true)
                }
                _ => (self.raw[g].clone(), self.pool[g].clone(), false),
            };
            let Some(id) = pixel_physics::train::predator::place_predator(w, &self.species, x, self.colony, mind, self.grants) else {
                self.due[k] = frame + 100; // no footing there: try another column shortly
                continue;
            };
            // The breeding genes ride with the mind: the pool mind's own (the species' if it has none), stepped as a
            // birth would step them when the placement is a mutant.
            let parent = self.bodies[g].or_else(|| pixel_physics::train::predator::breeding_of(w, id)).unwrap_or_default();
            let genes = if mutant { pixel_physics::train::predator::mutate_breeding_of(w, &self.species, parent, || self.brng.unit()) } else { parent };
            pixel_physics::train::predator::set_breeding(w, id, genes);
            self.log.push((id, g, genome, frame, None, 0.0, mutant, genes));
            self.next += 1;
            self.colony = self.colony.or_else(|| w.organism(id).map(|s| s.colony));
            self.ids.insert(id);
            self.size.insert(id, w.organism(id).map_or(0, |s| s.cells.len()));
            self.due[k] = u64::MAX;
            self.placed += 1;
            let (hx, hy) = w.organism(id).and_then(|s| s.chain.first().copied()).unwrap_or((x, 0));
            writeln!(ev, "{frame},{id},spider,{hx},{hy},{g},0").unwrap();
        }
    }

    /// Every living spider that lost cells since the last look: a `wound` row, so a bite that does not kill still
    /// shows (a colony that wears a spider down is on the way to killing one). Sorted by id: one row order a run.
    fn watch(&mut self, w: &pixel_physics::sim::World, frame: u64, ev: &mut impl Write) {
        let mut ids: Vec<u32> = self.ids.iter().copied().collect();
        ids.sort_unstable();
        for id in ids {
            let Some(n) = w.organism(id).map(|s| s.cells.len()) else { continue };
            let was = self.size.insert(id, n).unwrap_or(n);
            if n < was {
                self.wounds += (was - n) as u64;
                writeln!(ev, "{frame},{id},wound,0,0,{},0", was - n).unwrap();
            }
        }
    }

    /// The bed's food rows before the ants' book reads them: a living spider's swallow is its credit (face worth /
    /// 100, as `examples/predtrain.rs` scores it).
    ///
    /// **Not for a predator that can put food down** (the nest raider, whose verbs carry Put Down for its pellets):
    /// it picked a dropped cell of ant up again and again, and each pick-up was a swallow row -- 7,300 in one
    /// 100,000-frame `predtrain` round against 37 cells bitten. Its credit is what it digests ([`Spiders::digested`]).
    fn ate(&mut self, rows: &[creature::FoodRow]) {
        if self.by_digest() {
            return;
        }
        for r in rows.iter().filter(|r| r.kind == creature::FOOD_SWALLOW && self.ids.contains(&r.who)) {
            if let Some(e) = self.log.iter_mut().rev().find(|e| e.0 == r.who && e.4.is_none()) {
                e.5 += r.worth as f64 / 100.0;
            }
        }
    }

    /// Whether this species' credit is digestion rather than swallows: one whose verbs can put a crop cell down.
    fn by_digest(&self) -> bool {
        pixel_physics::train::predator::verbs_for(&self.species).contains(&creature::options::Verb::Drop)
    }

    /// A living spider's digestion row, as its credit (face worth / 100) when [`Spiders::by_digest`].
    fn digested(&mut self, who: u32, worth: f32) {
        if self.by_digest() {
            if let Some(e) = self.log.iter_mut().rev().find(|e| e.0 == who && e.4.is_none()) {
                e.5 += worth as f64 / 100.0;
            }
        }
    }

    /// `spider_out=DIR`: every placement's genome (`kNNN.raw.txt`), its breeding genes (`kNNN.body.txt`) and `scores.csv`
    /// (k,parent,born,died,credit,score,mutant).
    /// Score: credit per thousand frames alive, as predtrain's, but over at least 20,000 frames -- per frame alive
    /// alone, the first smoke run's best spider was one that gorged for 1,330 frames and was killed (124.8, against
    /// 14.6 for one that hunted for 9,670), and a spider that eats and dies is not the hunter to breed from.
    ///
    /// `spider_score=total` (co-evolution's second batch, 2026-10-24): the food it ate over the time it could have had,
    /// placement to the run's end -- dying early forfeits the rest, and a spider placed late is not marked down for
    /// having less run left. **Why**: under the per-frame-alive score, elitism worked on its own terms (lives 6k -> 22-33k
    /// frames, credit 310 -> 520 a placement) and bred careful hunters that kept away from the colony: judged on the
    /// gardens, they killed fewer ants than the pool they came from in 38 of 48 pairs and were killed half as often.
    /// Dividing by the time alive paid a spider for surviving, not for eating.
    fn save(&self, dir: &std::path::Path, end: u64, total: bool) {
        std::fs::create_dir_all(dir).unwrap();
        let mut f = std::fs::File::create(dir.join("scores.csv")).unwrap();
        writeln!(f, "k,parent,born,died,credit,score,mutant").unwrap();
        for (k, (_, parent, g, born, died, credit, mutant, genes)) in self.log.iter().enumerate() {
            train::save_genome(&dir.join(format!("k{k:03}.raw.txt")), g).unwrap();
            pixel_physics::train::predator::save_breeding(&dir.join(format!("k{k:03}.raw.txt")), *genes).unwrap();
            let life = (died.unwrap_or(end) - born) as f64 / 1000.0;
            let score = if total { credit / ((end - born) as f64 / 1000.0).max(1.0) } else { credit / life.max(20.0) };
            writeln!(f, "{k},{parent},{born},{},{credit:.3},{score:.4},{}", died.map_or(String::new(), |d| d.to_string()), u8::from(*mutant)).unwrap();
        }
    }

    /// A death from the bed's log: a spider's frees its place, an ant's killed is counted.
    fn died(&mut self, r: &pixel_physics::sim::world::DeathRow, frame: u64, ev: &mut impl Write) {
        if r.species == self.ant && r.cause == organism::DeathCause::Killed {
            self.ants_killed += 1;
        }
        if r.species != self.sp || !self.ids.remove(&r.id) {
            return;
        }
        self.size.remove(&r.id);
        if let Some(e) = self.log.iter_mut().rev().find(|e| e.0 == r.id && e.4.is_none()) {
            e.4 = Some(frame);
        }
        self.deaths += 1;
        self.killed += u64::from(r.cause == organism::DeathCause::Killed);
        self.starved += u64::from(r.cause == organism::DeathCause::Starved);
        if let Some(k) = self.due.iter().position(|&d| d == u64::MAX) {
            self.due[k] = frame + self.gap;
        }
        writeln!(ev, "{frame},{},spdeath,0,0,{},0", r.id, r.cause.index()).unwrap();
    }
}

fn main() {
    // SCRATCH: `bank_cap=N` sets the PP_BANK_CAP digestion probe (creature.rs) before any world exists.
    if let Some(v) = std::env::args().find_map(|a| a.strip_prefix("bank_cap=").map(str::to_string)) {
        unsafe { std::env::set_var("PP_BANK_CAP", v) };
    }
    // SCRATCH: `digest_bar=N` (tanks) sets PP_DIGEST_BAR, the appetite gate's reference (`hungergate=`), because this
    // bed turns breeding off with a 1e30 breeding bar and the gate otherwise reads hunger 1 at any bank.
    if let Some(v) = std::env::args().find_map(|a| a.strip_prefix("digest_bar=").map(str::to_string)) {
        unsafe { std::env::set_var("PP_DIGEST_BAR", v) };
    }
    // SCRATCH: `setenv=NAME=VAL[,NAME=VAL...]` sets environment switches before any world exists, so a gate check
    // of another thread's off-by-default switch is one jobs-file argument (teacher lane, 2026-10-22).
    if let Some(v) = std::env::args().find_map(|a| a.strip_prefix("setenv=").map(str::to_string)) {
        for kv in v.split(',') {
            let (k, val) = kv.split_once('=').expect("setenv=NAME=VAL");
            unsafe { std::env::set_var(k, val) };
        }
    }
    // SCRATCH: `nomind=1` sets PP_NOMIND, founding the colony on the old ant (the game's chooser, no mind). Use sel=off.
    if std::env::args().any(|a| a == "nomind=1") {
        unsafe { std::env::set_var("PP_NOMIND", "1") };
    }
    std::env::set_var("PIXEL_PHYSICS_MUTATION", "off");
    // spiders=POOL:N (lone predator thread): the bite acts, for spiders and ants alike, before any world exists.
    if std::env::args().any(|a| a.starts_with("spiders=")) {
        std::env::set_var("PIXEL_PHYSICS_MIND_ATTACK", "on");
    }
    let a: Vec<String> = std::env::args().skip(1).collect();
    let a = with_standard(a);
    // pile=1 (default): every founder starts at the nest door, heaped (the game's own
    // `PIXEL_PHYSICS_NEST_DOOR_FOUNDERS=pile`), not spread along the ground (owner, 2026-10-21).
    if arg(&a, "pile") != Some("0") {
        std::env::set_var("PIXEL_PHYSICS_NEST_DOOR_FOUNDERS", "pile");
    }
    let num = |k: &str, d: u64| arg(&a, k).map_or(d, |v| v.parse().unwrap());
    let (frames, seed, dist, drip, every, window) = (num("frames", 200_000), num("seed", 1), num("dist", 40) as i32, num("drip", 5) as u32, num("every", 500), num("window", 2000));
    // near=N (Teacher lane, 2026-10-25): the west heap sits N columns from the door instead of `dist`, so a colony
    // ramped out to far food still has food at home. **Why**: traced one ant at a time on the herb bed (`lab_herbs`,
    // herbs at the nest), the trained pool's ants spread over the whole box (x quartiles 114 / 277 / 414, the door at
    // 256) while the hand-built ant's stay by it (253 / 257 / 282) and are on a plant 82% of the time by 4,000 frames
    // against the trained 18%; both bite as often when on one. Every bed since the ramp put all food 100+ columns
    // out, so walking away is all the pool has been asked to do. 0 = off (both heaps at `dist`).
    let near = num("near", 0) as i32;
    let west_at = |dist: i32| {
        let d = if near > 0 { near } else { dist };
        if inrun::NEST - d >= 8 { inrun::NEST - d } else { inrun::NEST + dist + 6 }
    };
    let maps = num("maps", 1) == 1;
    let cap = num("cap", 60) as usize;
    let spread = num("spread", 0) as i32;
    // `food=provisions|fruit|mix` (teacher lane, 2026-10-24): what the drip drops; `mix` is provisions east, fruit
    // west. The lab's boxes feed on fruit too, and a colony trained on provisions alone did a third as well on it.
    let food_mats: (&str, &str) = match arg(&a, "food").unwrap_or("provisions") {
        "provisions" => ("provisions", "provisions"),
        "fruit" => ("fruit", "fruit"),
        "mix" | "all" => ("provisions", "fruit"),
        f => panic!("food={f}: provisions, fruit, mix or all"),
    };
    // `food=all` (Teacher lane, 2026-10-27): with `patches=`, each jump draws its heap's material from all four foods the
    // lab's boxes use, from its own stream so the heap's place and timing match `food=mix` on the same seed; without
    // `patches=` it is `mix`. **Why**: the environment review (PR #941) found 9 of the 36 shipped boxes feed on windfall
    // or corpse, which training never offered; in two_larders swapping them for fruit and provisions took the trained
    // pool from 3.6-7.3 ants lived to 37-101 (8/8 seeds), while the hand-built ant lived on them.
    const FOOD_ALL: [&str; 4] = ["provisions", "fruit", "windfall", "corpse"];
    let food_all = arg(&a, "food") == Some("all");
    let mut food_rng = train::Rng(seed.wrapping_mul(0x2545_F491_4F6C_DD1D) ^ 0x5F0D);
    // SCRATCH (evolvable-pheromones lane, 2026-10-25): `patches=T:LO:HI` -- **one** food heap, on a random side at a
    // random distance LO..HI columns from the door, that jumps to a new side and distance every T frames; the old
    // heap is left to be eaten and is never topped up again. Off by default (the two mirrored heaps of `dist=`).
    // **Why**: a signal evolves only when it carries something a nestmate cannot find alone (adversarial review
    // C3, Floreano et al. 2007). Two permanent mirrored heaps fed by a steady drip are learnable by every ant
    // without telling anyone; a heap that moves is news, and which side it is on is exactly what a trail can say.
    // Seeded from the run's seed, so arms pair. The drip, cap and spread are `dist=`'s; `food=mix` alternates the
    // material by side as the two heaps had it (provisions east, fruit west).
    let patches: Option<(u64, i32, i32)> = arg(&a, "patches").map(|v| {
        let p: Vec<&str> = v.split(':').collect();
        assert!(p.len() == 3, "patches=T:LO:HI");
        let (t, lo, hi): (u64, i32, i32) = (p[0].parse().expect("patches T"), p[1].parse().expect("patches LO"), p[2].parse().expect("patches HI"));
        assert!(t > 0 && 0 < lo && lo <= hi, "patches=T:LO:HI with T > 0, 0 < LO <= HI");
        (t, lo, hi)
    });
    let mut patch_rng = train::Rng(seed.wrapping_mul(0xD6E8_FEB8_6659_FD93) ^ 0x9A7C);
    let mut patch_at: (i32, &str) = (0, food_mats.0);
    let mut patch_next = 0u64;
    // `foodflip=F` (Creatures research thread, 2026-10-25; the teacher lane's positive control for learning within a
    // life): food drips at one heap only, east or west as the seed draws it, and from frame F at the other one only,
    // the old heap's loose food cleared at the flip (`foodflip_keep=1` leaves it). A brain fixed at birth cannot know
    // which side comes second, so deliveries after F measure whether ants re-route within their lives; `events.csv`'s
    // `field` rows say which side each bite came from. Off (unset) is the two heaps as before, bit for bit.
    let food_flip: Option<u64> = arg(&a, "foodflip").map(|v| v.parse().expect("foodflip=FRAME"));
    let flip_east_first = (seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 63) == 1;
    let flip_keep = num("foodflip_keep", 0) == 1;
    let out = std::path::PathBuf::from(arg(&a, "out").expect("out=DIR"));
    std::fs::create_dir_all(&out).unwrap();
    if let Some(f) = food_flip {
        let first = if flip_east_first { "east" } else { "west" };
        println!("foodflip: food {first} until frame {f}, then the other side only{}", if flip_keep { "" } else { ", the old heap cleared" });
        std::fs::write(out.join("foodflip.txt"), format!("first {first}\nflip {f}\n")).unwrap();
    }
    // The suite a founder enters by is the game training page's own (`colony_bed::suite`): soil + scarce + trail.
    let suite = pixel_physics::train::challenges::colony_bed::suite();
    // founders=path:count,...
    let mut mix: Vec<(String, Arc<creature::mind::MindGenome>, usize)> = Vec::new();
    // The raw genome behind each mix entry, for the selection loop's children.
    let mut raw: Vec<Vec<f32>> = Vec::new();
    let mut glen = 0;
    for part in arg(&a, "founders").expect("founders=path:count,...").split(',') {
        let (p, n) = part.rsplit_once(':').expect("path:count");
        // path@tag:count names the genome's family (a later round's founders keep their line).
        let (p, tag_over) = p.split_once('@').map_or((p, None), |(p, t)| (p, Some(t.to_string())));
        let n: usize = n.parse().unwrap();
        if p == "zero" {
            mix.push((p.to_string(), Arc::new(creature::mind::MindGenome::new(vec![0.0; creature::mind::GENOME_LEN])), n));
            raw.push(Vec::new());
        } else {
            // A `.raw.txt` genome is taken as saved (a harness's own output), not through the suite's rung entry.
            let g = if p.ends_with(".raw.txt") { load_founder(p) } else { train::start_mind(&suite, p) };
            let g = with_head(g);
            raw.push(g.clone());
            glen = g.len();
            let tag = tag_over.unwrap_or_else(|| std::path::Path::new(p).file_stem().unwrap().to_string_lossy().to_string());
            mix.push((tag, inrun::mind(&g), n));
        }
    }
    // SCRATCH (evolvable-pheromones lane, 2026-10-25): `mutate_to=PATH:PM:SIGMA:EVENTS:MSEED` -- write one
    // size-neutral mutant ([`neutral_mutate`], every trainable slot, no lesson protection) of the first founder to
    // PATH and stop, without running a world. The emergence driver (`scratch/pheromones/emerge.py`) breeds whole
    // colonies from one genome each, so it needs a mutant genome per child colony rather than mutant ants inside one.
    if let Some(spec) = arg(&a, "mutate_to") {
        let v: Vec<&str> = spec.split(':').collect();
        assert!(v.len() == 5, "mutate_to=PATH:PM:SIGMA:EVENTS:MSEED");
        let (pm, sigma, events, ms): (f32, f32, usize, u64) = (v[1].parse().unwrap(), v[2].parse().unwrap(), v[3].parse().unwrap(), v[4].parse().unwrap());
        let mut rng = train::Rng(ms.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0x3C3C);
        let child = neutral_mutate(&raw[0], &inrun::slots(), pm, sigma, events, None, &mut rng);
        let changed = child.iter().zip(&raw[0]).filter(|(x, y)| x != y).count();
        train::save_genome(std::path::Path::new(v[0]), &child).unwrap();
        println!("mutate_to: {} slots changed, written to {}", changed, v[0]);
        return;
    }
    // mutants=N:pm:sigma:mseed -- N trainer-style mutants of the first genome, one founder each (m00..),
    // replacing the founders list's counts; the same mseed gives the same mutants on every colony seed.
    if let Some(spec) = arg(&a, "mutants") {
        let v: Vec<&str> = spec.split(':').collect();
        let (n, pm, sigma, ms): (usize, f32, f32, u64) = (v[0].parse().unwrap(), v[1].parse().unwrap(), v[2].parse().unwrap(), v[3].parse().unwrap());
        let base = with_head(train::start_mind(&suite, arg(&a, "founders").unwrap().split(',').next().unwrap().rsplit_once(':').unwrap().0));
        // SCRATCH (dig-scoring thread): founders' mutants leave an evolvable dig head as the side file has it
        let slots: Vec<usize> = inrun::slots().into_iter().filter(|&i| i < creature::mind::GENOME_LEN).collect();
        let mut rng = train::Rng(ms);
        mix.clear();
        raw.clear();
        let mut f = std::fs::File::create(out.join("mutants.txt")).unwrap();
        for k in 0..n {
            let g = if k == 0 { base.clone() } else { train::mutate(&base, &slots, pm, sigma, 0.0, 1.0, &[], &mut rng) };
            let d: f32 = g.iter().zip(&base).map(|(x, y)| (x - y).abs()).sum();
            writeln!(f, "m{k:02} changed {} L1 {d:.2}", g.iter().zip(&base).filter(|(x, y)| x != y).count()).unwrap();
            mix.push((format!("m{k:02}"), inrun::mind(&g), 1));
            raw.push(g);
        }
    }
    for (m, r) in mix.iter_mut().zip(raw.iter_mut()).filter(|(m, _)| m.0 == "zero") {
        m.1 = inrun::mind(&vec![0.0; glen]);
        *r = vec![0.0; glen];
    }
    let ants: usize = mix.iter().map(|m| m.2).sum();
    // Interleaved round-robin, so no genome owns one end of the founding heap.
    let mut order: Vec<usize> = Vec::with_capacity(ants);
    let mut left: Vec<usize> = mix.iter().map(|m| m.2).collect();
    while order.len() < ants {
        for (i, n) in left.iter_mut().enumerate() {
            if *n > 0 {
                *n -= 1;
                order.push(i);
            }
        }
    }
    let slot = |k: usize| order[k.min(ants - 1)];
    // plants=MIX[:SPACING[:BAND]] (SCRATCH, plant-food lane, 2026-10-23): a planted bed, the colony landing on it after
    // grow=F frames (default 6,000, the played bed's), the mister at rain=R (default 2). Pair with cap=0 for no pile.
    // Off by default: no `plants=` is the bare bed, bit for bit.
    let garden = arg(&a, "plants").map_or_else(inrun::Garden::default, |m| {
        inrun::Garden::parse(m, num("grow", 6000), Some(arg(&a, "rain").map_or(2.0, |v| v.parse().unwrap())))
    });
    let (mut w, sc, sp) = inrun::colony_in(seed, ants, &garden, &|k| mix[slot(k)].1.clone());
    // The world's clock at the colony's landing: every frame below is counted from it, so a garden's growing frames
    // do not age the founders (`b.born`) or shift a window.
    let f0 = w.frame;
    if !garden.plants.is_empty() {
        eprintln!("garden: {} plants ({}), grown {} frames", garden.plants.len(), garden.plants.iter().map(|p| format!("{}@{}", p.0, p.1)).collect::<Vec<_>>().join(" "), garden.grow);
    }
    // inherit=PATH: start in the nest a previous round dug (see `read_nestmap`). Before the datum and the original
    // ground are read below, so the inherited nest is not "original ground" and extending it counts as new cuts.
    if let Some(p) = arg(&a, "inherit") {
        let surf: Vec<i32> = (0..256).map(|x| inrun::surface(&w, &sc, x)).collect();
        let n = read_nestmap(&mut w, p, (arg(&a, "inherit_nest") == Some("1")).then_some(&surf[..]), arg(&a, "inherit_link").map(|v| v.parse().expect("inherit_link=MIN cells")));
        eprintln!("inherit: {n} cells set from {p}");
    }
    // synapse=F: the ant's synapse_fraction for this run (a trial of the link price before it is proposed for the
    // species file, where it would reach every world). Default: the species' own.
    if let Some(v) = arg(&a, "synapse") {
        let mut def = w.species.get(sp).creature.clone().expect("creature");
        def.synapse_fraction = v.parse().unwrap();
        w.species.set_creature(sp, def);
    }
    // hungergate=W: the ant's `digest_hunger_weight` for this run (a trial, not the species file). At 1 the crop stops
    // digesting as the bank nears the breeding bar, so a full forager carries its load home (owner, 2026-10-22: 89-96%
    // of all food digested went into ants already past the bar; the colony ate 2-2.5x what it burned).
    if let Some(v) = arg(&a, "hungergate") {
        let mut def = w.species.get(sp).creature.clone().expect("creature");
        def.digest_hunger_weight = v.parse().unwrap();
        w.species.set_creature(sp, def);
    }
    // oldage=N (environment review, 2026-10-27): the ant's `life_half_life` for this run. The bed sets it to 0 (no old age,
    // `inrun::colony_in`); the lab runs the species file's 40,000, so a colony there loses its founders to age and must
    // breed to stay. `oldage=40000` is the lab's. Unset: the bed's 0, the run unchanged.
    if let Some(v) = arg(&a, "oldage") {
        let mut def = w.species.get(sp).creature.clone().expect("creature");
        def.life_half_life = v.parse().expect("oldage=N frames");
        w.species.set_creature(sp, def);
    }
    // brood=lay (brood lane, 2026-10-23; the owner: "when an ant is ready to be born then its genome should be
    // selected from the selection step"): the game's own life cycle runs in the bed. A fed ant lays when its bank
    // reaches the brood block's bar (`try_bud`, exactly as in the lab: egg 120 J, a larva fed the rest of an adult's
    // price by nestmates and food beside it, a pupa, then the adult), and **selection writes the child's mind at the
    // moment the adult hatches** -- the hatch is the birth, so there is no lag between the pick and the ant. The egg
    // carries its layer's mind until then; with no selection (sel=off) the hatchling keeps it, as in the lab.
    // Selection's own births (quota, refill, hall) are not made: food decides how many are born, selection who.
    // Why: with births made from nothing (kid_grants x 200 J each, ~150-220 a round) and the headcount held at lo..hi,
    // food had no use beyond upkeep, so the nest filled: colsel20's light-head gardens held a median 389 stored cells
    // (~370 kJ, one cell is about one ant's price) and ~87 full grants in each ant's bank.
    // **On by default** (owner, 2026-10-24: "Breeding should be on by default"), with `lay_cap=48:8`, after brood1-1023
    // (64 colonies x 32 rounds from hcs, common gardens from round 15): 84-89 ants, starved 1-2 a round, 0-3 of 64
    // extinct, food home 16.8-24.9k, crater 3-8, nest food at round 31 260 cells against 514 without breeding.
    // `brood=off` is the bed before it: births made from nothing, the headcount held at lo..hi.
    let brood_lay = match arg(&a, "brood") {
        Some("off") => false,
        None | Some("lay") => true,
        Some(v) => panic!("brood={v}: use lay or off"),
    };
    // lay_cap=T:D (with brood=lay): the laying bar doubles for every D ants (adults plus brood) above T, so the colony
    // grows on its surplus to around T and then lays only as fast as its banks outrun the raised bar -- a graded brake,
    // never a stop. The bed's frame cost grows with its ants (a capped 200k round ran ~2.1x a brood=off one). Default
    // 48:8 (with breeding on by default); `lay_cap=off` is the game's bar at any size, which in the pilot boomed to
    // 150-200 ants, starved ~650 adults a round and cost ~5x.
    let lay_cap: Option<(f64, f64)> = match arg(&a, "lay_cap").unwrap_or("48:8") {
        "off" => None,
        v => {
            let (t, d) = v.split_once(':').expect("lay_cap=T:D|off");
            Some((t.parse().unwrap(), d.parse::<f64>().unwrap().max(1.0)))
        }
    };
    // lay_cap_p=P (brood lane, 2026-10-24; Teacher lane's ask, after far64 boomed to ~710 ants in the lab box, which
    // has no lay_cap): the cap's strength, 0..1, scaling its exponent -- the bar is lay_at x 2^(P (heads - T) / D).
    // 1 (default) is lay_cap as it was; 0 is no cap. colsel's `capwean=R0:R1` fades it over rounds, the way `wean=`
    // fades the body rules, so the colony has to learn its own brake (`PP_LAYGENE`) before the scaffold is gone.
    let lay_cap_p = arg(&a, "lay_cap_p").map_or(1.0, |v| v.parse::<f64>().expect("lay_cap_p=P")).clamp(0.0, 1.0);
    // lay_reserve=R (with brood=lay; brood lane, 2026-10-23, a trial before any game rule): laying answers the
    // colony's FREE reserve -- every adult's bank plus the nest's stored food, less what the brood already laid still
    // needs to reach an adult's price -- per head (adults and brood), in full grants. At or above R the bar is the
    // game's; below it the bar grows as (R / free)^2. Why: every brake in the pilot answered today's wealth, and a
    // brood laid today needs 500+ frames and ~920 J each before it works, so the food and store brakes (adults only)
    // overshot and swung 40 <-> 200 ants. Counting the brood's debt is the delay put into the signal.
    let lay_reserve: Option<f64> = arg(&a, "lay_reserve").map(|v| v.parse().unwrap());
    // nocull=1 (with brood=lay): selection removes nobody -- no hard culls, no quota culls, no retiring -- so food and
    // starvation alone set the colony's size; the control for any regulation arm (review, 2026-10-23: culls were the
    // largest death source, 929 against 651 starved in one run, and every culled ant is food that pays for eggs).
    let nocull = num("nocull", 0) > 0;
    // hatch_grant=T: every founder's TRAIT_BIRTH_GRANT is set to T (-1..1; the shipped founding draws it), so each
    // hatchling's price -- and the bank it hatches with -- is grant_fraction(T) of a full grant. Paid by the larva's
    // feeders like the rest of its price: nothing is made. Review: 51-61% of starved adults died before judging, on
    // a ~0.5-grant hatch bank against kid_grants=3 today.
    if let Some(t) = arg(&a, "hatch_grant").map(|v| v.parse::<f32>().unwrap()) {
        inrun::set_trait(&mut w, sp, organism::TRAIT_BIRTH_GRANT, t);
    }
    let base_lay_at = w.species.get(sp).creature.as_ref().and_then(|d| d.brood.as_ref()).map_or(0.0, |b| b.lay_at);
    if brood_lay {
        assert!(base_lay_at > 0.0, "brood=lay: the ant has no brood block");
        w.births_paused = false;
    }
    w.cut_log = Some(Vec::new());
    w.food_log = Some(Vec::new());
    w.digest_log = Some(Vec::new());
    w.death_log = Some(Vec::new());
    let founders = inrun::ants(&w, sp);
    // flat=1: every founder starts on one full grant (the founding deals staggered reserves).
    // grants=G: every founder starts on G full grants (default: the founding's staggered reserves).
    if let Some(g) = arg(&a, "grants") {
        inrun::flat_reserves(&mut w, sp, g.parse().unwrap());
    }
    println!(
        "inrun: seed {seed}, {} founders ({}), {frames} frames, food {dist} cols out, {drip} cells every {every} up to {cap} a side, out {}",
        founders.len(),
        mix.iter().map(|m| format!("{}x{}", m.0, m.2)).collect::<Vec<_>>().join(" "),
        out.display()
    );
    let mut f = std::fs::File::create(out.join("ants.csv")).unwrap();
    writeln!(f, "id,tag").unwrap();
    for (k, id) in founders.iter().enumerate() {
        writeln!(f, "{id},{}", mix[slot(k)].0).unwrap();
    }
    // ---- The in-run selection loop (sel=score|random; off by default) ----
    let sel = arg(&a, "sel").unwrap_or("off").to_string();
    let fnum = |k: &str, d: f64| arg(&a, k).map_or(d, |v| v.parse::<f64>().unwrap());
    let (min_age, judge_every, cooldown) = (num("min_age", 15_000), num("judge", 500), num("cooldown", 5_000));
    let (zb0, zc0, kprice) = (fnum("zbirth", 2.0), fnum("zcut", 2.0), fnum("k", 2.0));
    let (mut lo, mut hi) = (num("lo", 24) as usize, num("hi", 36) as usize);
    // The family cap's band top under brood=lay, where `hi` no longer bounds the colony: a quarter of this or of the
    // colony now, whichever is larger.
    let hi_band = hi;
    // With brood=lay the colony's size is food's to set (and `lay_cap`'s), so the band's top cull is off unless asked.
    if brood_lay && arg(&a, "hi").is_none() {
        hi = 100_000;
    }
    // grow_band=F:HI (review lane, 2026-10-22): every F frames the headcount band moves up by 4 until `hi` reaches HI, so
    // the colony grows through the round and the nest has to grow with it (the dig price otherwise hits its floor once
    // 25 open cells per ant exist, and late generations are bred on bailing spoil). Off by default.
    let grow_band: Option<(u64, usize)> = arg(&a, "grow_band").map(|v| {
        let (f, t) = v.split_once(':').expect("grow_band=F:HI");
        (f.parse().unwrap(), t.parse().unwrap())
    });
    let (kpm, ksigma) = (fnum("kpm", 0.03) as f32, fnum("ksigma", 1.0) as f32);
    // Children's prune and grow. At prune 0 / grow 1 every mutated empty slot became a live link (~60 a child), and
    // with nothing in the score charging for wire, brains went from ~700 links to ~2,200 in 20 generations; the body
    // bills every link a tick, so the evolved lines starved (p8: 13 of 16 runs extinct). Links hold steady when
    // pm * empty * grow = pm * live * prune: with ~2,100 empty and ~700 live, grow 0.1 against prune 0.3.
    let (kprune, kgrow) = (fnum("kprune", 0.3) as f32, fnum("kgrow", 0.1) as f32);
    // mutation=neutral: [`neutral_mutate`] with `kevents` structural changes a child.
    let neutral = arg(&a, "mutation") == Some("neutral");
    let kevents = num("kevents", 10) as usize;
    let kid_grants = fnum("kid_grants", 1.0) as f32;
    // hall=F: an adult that starved stays a candidate parent for F frames, its score multiplied by `dead_w`, and a colony
    // below `lo` refills from the best of the living and these dead together (owner's idea, 2026-10-21: pass on the best
    // ants even if they starved, with a penalty for dying). Without it, a dry store starved 96% of a colony inside ~2k
    // frames in 47 of 108 rounds (p10-p11), and the colony was rebuilt from whichever few were left. 0 = off.
    let (hall_keep, dead_w) = (num("hall", 0), fnum("dead_w", 0.5));
    // Exchange rates between jobs, and a debit per food cell swallowed away from home (default: the ledger's 1,1,1,0).
    let (w_del, w_dig, w_clear, w_field) = (fnum("w_deliver", 1.0), fnum("w_dig", 1.0), fnum("w_clear", 1.0), fnum("w_field", 0.0));
    // Pay by depth: a delivery `d` rows below the datum is worth (1 + w_depth * min(d, 10) / 10) times (owner, 2026-10-21:
    // food dropped at the door is not food stored in the nest).
    let w_depth = fnum("w_depth", 0.0);
    // Pay by how far the spoil went: a pellet let go within `rim` columns of the nest pays nothing for its carry (nor the
    // cutter's half), rising to full `rim_ramp` columns further out. Spoil heaped on the rim rolls back into the pit:
    // after the pit's peak, 764 pellets let go within 30 columns against 198 dropped inside it (p5, 2026-10-21).
    let (rim, rim_ramp) = (fnum("rim", 0.0), fnum("rim_ramp", 10.0).max(1.0));
    // Only nest work pays: a cell cut more than `nest_r` columns from the nest earns no dig credit, and its pellet earns
    // no clear credit wherever it goes. Without this, size-neutral colonies (p11) farmed credit by cutting surface soil
    // 20-40 columns out and dropping it beside the hole: 'out' drops peaked there (8,203 at 20-29 columns, 7,663 at
    // 30-39) while digs were spread evenly over 0-120. 0 = off (pay everywhere, the old rule).
    let nest_r = fnum("nest_r", 20.0) as i32;
    let in_nest = |x: i32| nest_r <= 0 || (x - inrun::NEST).abs() <= nest_r;
    // nest=zone (owner, 2026-10-22, on the Teacher lane's replay page; built by the dig-scoring thread): the nest is the
    // stamped mouth plus `nest_pad` cells of the soil around it, and any original cell dug inside that zone joins it,
    // bringing its own `nest_pad` cells of soil. It never shrinks. Digging in the zone is the only dig that pays
    // (replacing the `nest_r` column band), and a trip cell counts as delivered only when put down in it (replacing the
    // 12-column home band, strict or not). Off by default. Open questions put to the owner (Teacher lane, 2026-10-22):
    // food put down in a part that has since filled in still counts; a tunnel toward the food heap carries the nest
    // out with it; the crater under the mouth stays nest.
    let zone_on = arg(&a, "nest") == Some("zone");
    let nest_pad = num("nest_pad", 2) as i32;
    // nest_top=N (owner, 2026-10-22: "we don't want to dig into a crater"): a cut fewer than N rows below the datum
    // never pays and never grows the zone. 0 = off. The zone itself still takes in the top rows over its cells, as pad:
    // first built as "no top-row cell is ever zone", that also moved delivery out from under the ants -- home food
    // fell 89 -> 9 per colony-round (13 of 13 lower, 8 seeds x 2 rounds), every pellet put down on the crater floor
    // booked as a 'tdrop', while the pit did not shrink (94 -> 98).
    let nest_top = num("nest_top", 0) as i32;
    // zone_debit=F (owner, 2026-10-22: "we could even add a cost for digging outside the nest... I wouldn't want that
    // forever... but top priority is teaching them to dig in the nest and not crater"): an original-ground cut that
    // does not pay under nest=zone -- outside the zone, or in its top `nest_top` rows -- charges F times the cut half.
    // A training-time lever, meant to come off. 0 = off.
    let zone_debit = fnum("zone_debit", 0.0);
    // Rim quarrying (dig-scoring thread, 2026-10-21): a lone ant learns to come down from the surface, cut the first wall
    // cell and go straight back out, and the nest becomes a crater (44 of 49 cuts in the top 4 rows after 12k; 70-80% of
    // the pit is ground the ants cut themselves). `rim_rows=N` pays nothing, neither half, for an original-ground cut
    // fewer than N rows below the datum inside the nest region, and `rim_debit=F` charges F times the cut half instead.
    // Off by default (rim_rows=0) until judged against the controls.
    // Judged 2026-10-22 (64 seeds x 6 rounds, h1 founders, turnover 30k): rim_rows=3 took extinctions from 20 to 34 of
    // 64, nearly all round-1 starvation die-offs; total food unchanged. The rim cutter is the ant that goes up to the
    // surface (in round 1 of 12 seeds, mostly-rim cutters delivered more food than mostly-deep cutters in 9), so paying it
    // nothing selects against going out. Two softer shapes: `rim_share=F` pays a rim cut F of a full cut (both halves)
    // instead of nothing, and `deep_bonus=B` pays a cut rim_rows or more below the datum (1 + B) times.
    let rim_rows = fnum("rim_rows", 0.0) as i32;
    let rim_debit = fnum("rim_debit", 0.0);
    let rim_share = fnum("rim_share", 0.0);
    // cut_join=N (owner, 2026-10-22: a stuck ant that swims through the ground under the nest must not be paid as a
    // nest digger -- "I don't want it to be counted as improved nest digging and be selected for"): a cut of original
    // ground pays only when one of its four neighbours is open space joined to the door below the datum. The joined set
    // is flooded from the door every N frames, and a paying cut joins its own cell at once, so a tunnel dug outward
    // from the nest keeps paying cell by cell while a swimmer, whose way back is packed shut behind it, stops paying at
    // the next flood. An unjoined cut is logged as `unjoined`. 0 = off (every cut pays, the old rule).
    let cut_join = num("cut_join", 0);
    // cut_roof=1 (owner, 2026-10-22: "not digging the top layer near the nest (which turns the nest into a crater)"):
    // a cut pays only when its cell is still nest under the colony rule (scratch/inrun/nestcount.py) once it is cut --
    // some ground (soil, packed soil, spoil, stone) left above it in its column between the datum and the cell, or the
    // open run along the datum row through its column no wider than the 5-cell door mouth. Taking the roof off a column
    // and deepening a column already open to the sky pay nothing, and are logged as `unroofed`; cutting the walls and
    // floor of a chamber under its roof pays as before. Unlike rim_rows (depth only, which starved round-1 colonies)
    // it does not care how deep the cut is. colsel10 (hcs, 48 rounds, food 25): crater 207 against nest 111 at r47,
    // a wide open shaft with the nest along its sides; the crater case alone did not move it. 0 = off.
    let cut_roof = num("cut_roof", 0) > 0;
    // roof_debit=F (owner, 2026-10-23: "should we make that even more expensive"): an `unroofed` cut also charges F
    // times the cut half, as rim_debit does. rim_debit on the top 3 rows took round-1 extinctions 20 -> 34 of 64 with
    // the h1 founders; hcs starves far less. 0 = off.
    let roof_debit = fnum("roof_debit", 0.0);
    let deep_bonus = fnum("deep_bonus", 0.0);
    // A meal at home after work (owner, 2026-10-21: "the nest needs a purpose"): a food cell swallowed at home pays
    // `w_home_eat` once per stretch of work (a trip out, a dig, a carry), so working and coming back to eat pays and
    // sitting at home eating the store does not (the stay-at-homes are who drained the store and starved together in the first pilots).
    let w_home_eat = fnum("w_home_eat", 0.0);
    // strict=1 closes four holes the design doc's adversarial review found (2026-10-22), all off by default so the
    // earlier batches reproduce: (a) a trip cell put down more than 12 columns from the nest is not a delivery (the
    // food log books every trip put-down, home or not: 43 of 674 'deliveries' in two seed-3 runs lay 13+ columns out);
    // (b) the meal at home pays after a trip out only if the trip bit food (a walk out and back was enough); (c) the
    // hungry departure pays at the trip's first field bite, not at the crossing; (d) progress pay is taken back per
    // cell not delivered, not only when none arrives.
    let strict = num("strict", 0) > 0;
    // A floor on turnover: when `turnover` frames pass with no birth, the adult with the lowest z is replaced by a
    // child of the adult with the highest (random arm: both at random). A healthy colony otherwise froze: in the
    // nest-purpose rounds, once the headcount sat in its band no ant cleared the bars and rounds 2-3 had 0-47 births
    // in 200k frames, so selection stopped exactly when the colony was doing well (p6, 2026-10-21).
    let turnover = num("turnover", 0);
    // turn=T (owner, 2026-10-21: "a target turnover and the bar moves based on the distribution of score"): each
    // adult is replaced about once per T frames. The bar is a rank, not a fixed z, so it holds whatever scale the
    // score has after a reweighting. Clear failures (z below -zhard) go on top of the quota; no ant below the median
    // breeds to fill it; the best get up to three children (passes over the top half and quarter); headcount
    // outside lo..hi adds births or culls. Replaces the z bars and `turnover` when set.
    let turn = num("turn", 0);
    let zhard = fnum("zhard", 3.0);
    let shrunk = arg(&a, "rank") == Some("shrunk");
    let lambda = fnum("lambda", 8.0);
    // famprior=1 (with rank=shrunk): shrink toward the mean of the ant's family's OTHER adults instead of the colony
    // median, when it has any. One ant's score predicts its line's at only ~0.3 however long the record (relia.py,
    // 12 colonies: 0.16 at 4k, 0.33 at 8k, 0.29 at 16k), so relatives carry evidence a longer wait does not.
    let famprior = num("famprior", 0) > 0;
    // Parents stay (owner's debate, 2026-10-21), but each child is earned fresh: fresh_kid=1 judges a parent's next
    // child on its record since its last one (at least min_age frames of it), and kid_cap bounds a lifetime's children.
    let fresh_kid = num("fresh_kid", 0) == 1;
    // cross=P (review lane, 2026-10-22): with probability P a quota child is a crossover of its parent with another
    // eligible parent (`train::cross`: each of `mind::crossover_group`'s groups whole from one side), then mutated as
    // usual. The scaffold was fixed so genomes stay slot-aligned for exactly this (creature-direction D4); the harness
    // never crossed. Off by default. Hall refills never cross.
    let cross_p = fnum("cross", 0.0) as f32;
    // pick=lexicase (review lane, 2026-10-22): quota parents are chosen by epsilon-lexicase over five per-ant rates
    // (food home within 8 columns, new-ground cuts, spoil carried out, paid meals at home, hungry departures) instead
    // of by the scalar score: each child shuffles the objectives, keeps the ants within one median absolute deviation
    // of the best on the first, breaks ties on the next, and so on. Specialists on any one job stay in the parent pool
    // with no weights. Culls stay on the scalar score. Off by default.
    let lexicase = arg(&a, "pick") == Some("lexicase");
    // lesson=PATH:F (review lane, 2026-10-22): the slots where the founder differs from the genome at PATH are the
    // lesson the copier wrote (hA1 vs c1 = the hungry-at-home lesson). They mutate at F times the weight-step chance
    // and never take a structural event. The mutation test showed standard mutation wrecks an unselected skill within
    // six rounds (random parents: cuts 201 -> 26, food 270 -> 102) while no-mutation selection starved 0.3 of 24
    // against 7.1 with it, so the starvation the selected lines show reads as damage to the lesson. Off by default.
    let lesson: Option<(Vec<bool>, f32)> = arg(&a, "lesson").map(|v| {
        let (p, f) = v.rsplit_once(':').expect("lesson=PATH:F");
        let reference = train::load_genome(std::path::Path::new(p), creature::mind::GENOME_LEN);
        let founder = raw.iter().find(|r| !r.is_empty()).expect("a founder genome").clone();
        let flags: Vec<bool> = founder.iter().zip(reference.iter()).map(|(x, y)| x != y).collect();
        eprintln!("lesson={p}: {} protected slots at {f} of the mutation rate", flags.iter().filter(|&&b| b).count());
        (flags, f.parse().unwrap())
    });
    // SCRATCH (dig-scoring thread): an evolvable dig head's slots (`PP_DIGHEAD_EVOLVE`) mutate at
    // `PP_DIGHEAD_EVOLVE_F` (default 0.25) of the rate and take no structural event, through the lesson protection.
    // **Why**: every head slot is live and its "when" read-out drives the Dig job straight, so at the full rate a
    // 100k selection run (seed 3099) kept 5 adults of 24 against 18 without it.
    let lesson = match (lesson, creature::mind::scratch_log::head_evolve()) {
        (l, None) => l,
        (l, Some(hl)) => {
            let f: f32 = std::env::var("PP_DIGHEAD_EVOLVE_F").ok().map_or(0.25, |v| v.parse().expect("PP_DIGHEAD_EVOLVE_F"));
            let (mut flags, lf) = l.unwrap_or_else(|| (vec![false; creature::mind::GENOME_LEN], f));
            flags.resize(creature::mind::GENOME_LEN, false);
            flags.extend(std::iter::repeat_n(true, hl));
            eprintln!("dig head evolves: {hl} slots at {lf} of the mutation rate");
            Some((flags, lf))
        }
    };
    let prot = lesson.as_ref().map(|(m, f)| (m.as_slice(), *f));
    let kid_cap = num("kid_cap", 1000) as u32;
    // retire=1: a parent leaves once it has had `kid_cap` children (its last child takes its place). Without it the
    // capped parents, immortal with old age off, sat in the colony unable to breed: 307 of 342 judgements owing a
    // replacement found no eligible parent (p7 seed 4, 2026-10-21) -- the "old ants clogging" the owner and I agreed
    // to watch for.
    let retire = num("retire", 0) == 1;
    let mut retiring: Vec<u32> = Vec::new();
    let mut capped_only = false;
    let mut quota = 0.0f64;
    let mut quota_cut: Vec<u32> = Vec::new();
    // Diagnostics: judgements with a quota due, how many found no eligible parent, and placements that failed.
    let (mut q_due, mut q_noparent, mut birth_fail) = (0u64, 0u64, 0u64);
    // Parents picked, and how many of them plain score would also have picked.
    let mut zs_pick = (0u64, 0u64);
    // Each ant's score as it reaches 30k frames of age (child minus parent at matched age).
    let mut age30 = std::io::BufWriter::new(std::fs::File::create(out.join("age30.csv")).unwrap());
    writeln!(age30, "id,score30").unwrap();
    let mut last_birth = 0u64;
    let mut srng = train::Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xA5A5);
    let kslots = inrun::slots();
    struct Who {
        born: u64,
        credit: f64,
        g2: f64,
        family: String,
        gen: u32,
        genome: Vec<f32>,
        last_kid: u64,
        kids: u32,
        // Worked since its last paid meal at home: out on a trip (head 15+ columns from the nest), a cut of the
        // ground, or a pellet carried out (owner, 2026-10-21: "also reward diggers who eat in the nest").
        tripped: bool,
        // The record at its last child (credit, sum of squares): with `fresh_kid`, the next child is earned on the
        // record since then.
        at_kid: (f64, f64),
        // Lexicase objectives, counts: home deliveries (8 cols), new-ground cuts, spoil out, paid home meals, hungry
        // departures.
        m: [f64; 5],
    }
    let mut book: HashMap<u32, Who> = HashMap::new();
    // brood=lay: who the next hatchlings' minds come from, best first (each judgement's parents, a parent once per
    // child the judgement gave it, round by round: every parent once, then those given two, then three), and how many
    // hatched since the last judgement (that judgement's births, which its quota's culls wait for).
    let mut sires: Vec<u32> = Vec::new();
    let mut sire_at = 0usize;
    let mut hatched_since = 0usize;
    let mut seen_births = 0u64;
    let mut brood_log = brood_lay.then(|| {
        let mut f = std::io::BufWriter::new(std::fs::File::create(out.join("brood.csv")).unwrap());
        writeln!(f, "frame,adults,eggs,larvae,pupae,laid,hatched,larvae_starved,brood_lost,hatch_denied,nursed_kj,ate_kj,lay_at").unwrap();
        f
    });
    // The recently starved adults: (death frame, score at death, record).
    let mut hall: Vec<(u64, f64, Who)> = Vec::new();
    let (home_colony, home_anchor) = founders.first().and_then(|&i| w.organism(i)).map(|s| (s.colony, s.forage_anchor)).unwrap();
    for (k, id) in founders.iter().enumerate() {
        book.insert(*id, Who { born: 0, credit: 0.0, g2: 0.0, family: mix[slot(k)].0.clone(), gen: 0, genome: raw[slot(k)].clone(), last_kid: 0, kids: 0, tripped: false, at_kid: (0.0, 0.0), m: [0.0; 5] });
    }
    let mut csv_ants = std::fs::OpenOptions::new().append(true).open(out.join("ants.csv")).unwrap();
    let mut births = std::io::BufWriter::new(std::fs::File::create(out.join("births.csv")).unwrap());
    // patches.csv: frame,x,material for each jump of `patches=` (header only when it is off).
    let mut patch_log = std::io::BufWriter::new(std::fs::File::create(out.join("patches.csv")).unwrap());
    writeln!(patch_log, "frame,x,material").unwrap();
    writeln!(births, "frame,id,parent,family,gen,kind,links").unwrap();
    let mut sel_log = std::io::BufWriter::new(std::fs::File::create(out.join("sel.csv")).unwrap());
    // The colony's food stocks every 1,000 frames (owner, 2026-10-21: watch the store fall before anyone starves).
    // nest_food: food cells at or below the ground datum within 12 columns of the nest (the heaps reach in to 15
    // columns, and their feet sink below the datum: a 20-column reach counted 5-12 buried heap cells as "stored");
    // door_food: above the datum within 12 columns and 4 rows; field_food: everything else within 45 columns. Tanks are banks in
    // full grants; crop is cells carried in crops.
    let mut stock_log = std::io::BufWriter::new(std::fs::File::create(out.join("stock.csv")).unwrap());
    writeln!(stock_log, "frame,alive,nest_food,door_food,field_food,crop_cells,tank_mean,tank_min,tank_max,nest_corpse").unwrap();
    let mut hungry_probe = std::env::var("PP_HUNGRYPROBE").is_ok_and(|v| v == "1").then(|| {
        let mut f = std::io::BufWriter::new(std::fs::File::create(out.join("hungry.csv")).unwrap());
        writeln!(f, "frame,id,x,y,tank,home,crop,crumbs8,prov8,best_j,jaws,verb,gut,crumb_q,picks").unwrap();
        f
    });
    let mut larder_log = std::io::BufWriter::new(std::fs::File::create(out.join("larder.csv")).unwrap());
    writeln!(larder_log, "frame,prov_cells,prov_kj,crumb_cells,crumb_kj,other_cells,other_kj").unwrap();
    // The general nest score (lab::nestscore, PR #770, HARVESTER profile) every 25,000 frames, printed beside food
    // home: printed only, never paid (owner, 2026-10-21). A read of the grid; it changes nothing in the run.
    // garden.csv (plants= only): the standing plant food every 1,000 frames, by kind, across the whole bed. `live_*`
    // is on a living plant (leaf and blade, flower, fruit still on the stem), `loose_*` lies free (fallen fruit,
    // litter, dead leaf, crumbs, provisions); `seeds` and `plants` count organisms (a waiting seed, a growing plant).
    const GARDEN_KINDS: [&str; 9] = ["leaf", "grassblade", "flower", "fruit", "windfall", "litter", "deadleaf", "crumbs", "provisions"];
    // SCRATCH (plant-food lane, 2026-10-23), the weaning bed's three levers. **`fjoule` and `weed=30` are on by default
    // on a planted bed** (owner, 2026-10-24: "The default should be on unless there is uncertainty"): every planted
    // batch ran both, and without weeding digging falls 246 -> 46 cuts. `fjoule=0` / `weed=0` opt out; neither does
    // anything without `plants=`, so the pile bed is unchanged. `cap_to` stays off (a within-round fall kills the colony
    // once the pile is gone: `Reports/dead-ends.md`).
    // fjoule=1: delivery and progress pay scale by the food's face worth over `joule_ref` (default 960, a provisions
    //   cell), so pay follows energy: a leaf (40) pays 1/24 of a provisions cell and a crop holds 24 times as many.
    //   Per-cell pay (the default) paid a leaf cell like a provisions cell.
    // weed=B: every 500 frames every cell a plant owns within B columns of the door is pulled (above the datum to air,
    //   below it to soil), a scaffold like the pile's top-up: a stand seeds itself over the door within ~20k frames
    //   and an ant cannot cut a root (`jaw_can_cut`: plant tissue is not ground), so digging fell 246 -> 46 cuts.
    // cap_to=C:F: the pile's cap falls linearly from `cap` to C by frame F (then holds), so a colony is weaned off the
    //   pile onto the garden within a round.
    let planted = !garden.plants.is_empty();
    let fjoule = num("fjoule", planted as u64) > 0;
    let joule_ref = fnum("joule_ref", 960.0);
    let weed = num("weed", if planted { 30 } else { 0 }) as i32;
    let cap_to: Option<(usize, u64)> = arg(&a, "cap_to").map(|v| {
        let (c, f) = v.split_once(':').expect("cap_to=C:F");
        (c.parse().unwrap(), f.parse().unwrap())
    });
    let soil_m = w.materials.id_of("soil").expect("soil");
    let mut weeded = 0u64;
    let mut garden_log = (!garden.plants.is_empty()).then(|| {
        let mut f = std::io::BufWriter::new(std::fs::File::create(out.join("garden.csv")).unwrap());
        writeln!(f, "frame,plants,seeds,{},{}", GARDEN_KINDS.map(|k| format!("live_{k}")).join(","), GARDEN_KINDS.map(|k| format!("loose_{k}")).join(",")).unwrap();
        f
    });
    let garden_ids: Vec<Option<pixel_physics::sim::material::MaterialId>> = GARDEN_KINDS.iter().map(|k| w.materials.id_of(k)).collect();
    let mut nest_log = std::io::BufWriter::new(std::fs::File::create(out.join("nest.csv")).unwrap());
    writeln!(nest_log, "frame,alive,food_home,nest_score,dug,chambers,niches,tunnel_cells,pit,per_cell,door_open,zone,zone_reach").unwrap();
    let mut food_home = 0u64;
    writeln!(sel_log, "frame,alive,adults,births,removals,bias,median,food_price,dig_price,clear_price,ghosts").unwrap();
    let mut bias = 0.0f64;
    // Ants culled this frame whose death record has not come in yet. `slay` frees the slot at once and the record lands
    // a frame later, so a child born in the same judgement can take the culled ant's id; without this, that record
    // struck the child out of the book and it lived on unjudged (a "ghost": up to 16 of 40 ants in the first pilots).
    let mut culled_pending: std::collections::HashSet<u32> = Default::default();
    // The prices (the ledger's): food from delivered / eaten worth over 5,000 frames; dig from open nest cells per
    // ant against 25; clear from pellets let go inside, net of re-cuts, per ant.
    let mut delivered: std::collections::VecDeque<(u64, f64)> = Default::default();
    let mut eaten: std::collections::VecDeque<(u64, f64)> = Default::default();
    let (mut cell_worth, mut n_field) = (0.0f64, 0u64);
    let mut loose_n = 0i64;
    let mut dig_price = 1.0 + kprice;
    // fprice=tank: food is priced by how full the colony's tanks are, 1 + ktank * shortfall of the mean bank below `tank`
    // full grants, so a delivery pays least while everyone is fat and most as the tanks drain (owner, 2026-10-21). The
    // flow price (delivered over eaten, 5k frames) sat at 2.83-2.91 of its cap of 3 because pile bites count as eaten,
    // so it never told a fat colony from a starving one: die-offs came with every ant at home on ~17x a full tank,
    // draining in lockstep, none going out until all were empty (p12, 29 die-offs).
    let tank_price = arg(&a, "fprice").is_some_and(|v| v == "tank");
    let (tank_full, ktank) = (fnum("tank", 3.0), fnum("ktank", 4.0));
    let start_e = w.species.get(sp).creature.as_ref().map_or(1.0, |d| d.start_energy) as f64;
    let mut tank_p = 1.0;
    // store=S: food is also priced by the larder (owner, 2026-10-23: "I don't love that the nest is 100% full of
    // food"). Delivery and progress pay are scaled by 1 - nest_food / (S * living ants), floored at 0, refreshed with
    // the stock census every 1,000 frames: a delivery pays in full into an empty store and nothing once it holds S
    // cells an ant. colsel20's rules-off gardens held a median 389 stored cells for 24 ants, each on ~87 full grants:
    // tank_p sat at 1 the whole game because the tanks were never short, so nothing ever said "enough".
    let store_cap = fnum("store", 0.0);
    let mut store_p = 1.0f64;
    // w_hungry=W: pay W when an ant leaves home (head from within 12 columns of the nest to 15+ out) while its own bank
    // is below `hungry` full grants, at most once per `hungry_gap` frames (owner, 2026-10-21). Rewards the habit at the
    // moment it is needed; the delivery it leads to comes far later and, at ~1 per 30k-frame life, too rarely to select on.
    let (w_hungry, hungry_at, hungry_gap) = (fnum("w_hungry", 0.0), fnum("hungry", 2.0), num("hungry_gap", 3000));
    // Per ant: (was last seen home, frame of its last paid hungry departure).
    let mut leave: HashMap<u32, (bool, u64)> = HashMap::new();
    let mut hungry_paid = 0u64;
    // progress=1: food is paid half as it moves home and half on arrival (research lane, PR #768 s3: one delivery is
    // one rare lump, about one per 30k-frame life, which selection cannot see). While an ant carries trip food (crop
    // cells from a bite away from home), each 100 frames it is paid 0.5 * the delivery value * cells * (the columns it
    // closed on the nest beyond its best so far / the columns it started from). The pay is an advance: if the crop
    // empties with nothing delivered (eaten on the way), the trip's progress pay is taken back.
    let progress = num("progress", 0) > 0;
    // Per carrier: (start columns, best columns, progress paid, cells delivered this trip).
    let mut carry: HashMap<u32, (f64, f64, f64, u32)> = HashMap::new();
    // strict: the most cells each carrier held this trip, who has bitten food on this trip out, and hungry departures
    // waiting for their first field bite (departure frame).
    let mut carried: HashMap<u32, u32> = HashMap::new();
    let mut bit: std::collections::HashSet<u32> = Default::default();
    let mut hungry_due: HashMap<u32, u64> = HashMap::new();
    // starve_at=F:G (a probe, not training): at frame F every ant's bank is set to G full grants, to see which brains
    // leave home when hungry (die-offs: nobody did).
    let starve_at: Option<(u64, f64)> = arg(&a, "starve_at").map(|v| {
        let (f, g) = v.split_once(':').expect("starve_at=F:G");
        (f.parse().unwrap(), g.parse().unwrap())
    });
    let mut base_open: Option<i64> = None;
    // tick=lab [labrain=R] (environment review, 2026-10-27): step the bed with the lab's own tick (`lab::tick_world`:
    // `frame::step`, then the timeline, then the mister) instead of the harness's subset (`inrun::step`: the sweep,
    // active sites, fields, scent). The subset leaves out liquid bodies, chunk bodies, particles and blasts, and the
    // mister's `rain::tick` -- which also drains the box every tick even at Off. So a `plants=` bed's `rain=` never fell
    // here. `labrain=R` is the mister's setting on the lab tick (`Rain::from_index`: 0 off, 1 light -- the lab's shipped
    // default -- 2 steady, 3 heavy); default 0, the bed's own `rain` setting. Unset: the harness's step, the run unchanged.
    let mut lab_tick = (arg(&a, "tick") == Some("lab")).then(|| {
        let mut spec = sc.bed.clone();
        spec.rain = pixel_physics::lab::rain::Rain::from_index(num("labrain", 0) as u8);
        (spec, pixel_physics::sim::particle::ParticleSystem::new(), pixel_physics::sim::explosion::Blasts::new(), pixel_physics::sim::player::Tuning::default())
    });
    // The datum and the ground below it at frame 0.
    let (bw, bh) = (inrun::bed_w(), 128i32); // the training bed (`bed_timed`; `PP_BED_W` widens it eastward)
    let surf: Vec<i32> = (0..bw).map(|x| inrun::surface(&w, &sc, x)).collect();
    let mut f = std::fs::File::create(out.join("surface.txt")).unwrap();
    for s in &surf {
        writeln!(f, "{s}").unwrap();
    }
    let mut orig = vec![false; (bw * bh) as usize];
    let mut zone = vec![false; (bw * bh) as usize];
    let grow = |zone: &mut Vec<bool>, x: i32, y: i32| {
        for yy in (y - nest_pad).max(0)..=(y + nest_pad).min(bh - 1) {
            for xx in (x - nest_pad).max(0)..=(x + nest_pad).min(bw - 1) {
                zone[(yy * bw + xx) as usize] = true;
            }
        }
    };
    for x in 0..bw {
        for y in surf[x as usize]..bh {
            orig[(y * bw + x) as usize] = inrun::ground(&w, x, y);
        }
    }
    // The stamp: the door's own surface cell and every open cell at or below the datum within 6 columns of it at
    // frame 0 (the bed's founding cut), each with its pad.
    if zone_on {
        grow(&mut zone, inrun::NEST, surf[inrun::NEST as usize]);
        for x in inrun::NEST - 6..=inrun::NEST + 6 {
            for y in surf[x as usize]..bh {
                if !inrun::ground(&w, x, y) {
                    grow(&mut zone, x, y);
                }
            }
        }
        eprintln!("nest=zone pad {nest_pad}: {} cells at frame 0", zone.iter().filter(|&&z| z).count());
    }
    let mut ev = std::io::BufWriter::new(std::fs::File::create(out.join("events.csv")).unwrap());
    writeln!(ev, "frame,id,kind,x,y,v,cutter").unwrap();
    // Who cut the pellet each ant holds; pellets let go inside, by spot; field food each ant holds.
    // Each pellet carries (cutter, whether it was cut in the nest region).
    // A pellet's cutter and its worth: 0 outside the nest region, rim_share at the rim, 1 + deep_bonus below it.
    let mut held: HashMap<u32, (u32, f64)> = HashMap::new();
    let mut loose: HashMap<(i32, i32), (u32, f64)> = HashMap::new();
    let t0 = std::time::Instant::now();
    let trace = num("trace", 0);
    let mut trace_log = (trace > 0).then(|| std::io::BufWriter::new(std::fs::File::create(out.join("trace.csv")).unwrap()));
    if let Some(tl) = trace_log.as_mut() {
        writeln!(tl, "frame,id,x,y,tank,crop,trip,spoil,since_nest,still,scent_a,scent_b").unwrap();
    }
    let mut scent_log = (trace > 0).then(|| std::io::BufWriter::new(std::fs::File::create(out.join("scent.csv")).unwrap()));
    if let Some(sl) = scent_log.as_mut() {
        writeln!(sl, "frame,laid_a,laid_b,cells_a,sum_a,cells_b,sum_b").unwrap();
    }
    let phero_dump = num("phero_dump", 0);
    let film = fnum("film", 0.0) as u64;
    let mut film_r = pixel_physics::render::Renderer::new();
    let film_p = pixel_physics::sim::ParticleSystem::new();
    let mut film_buf: Vec<u8> = Vec::new();
    let mut joined = vec![false; (bw * bh) as usize];
    let join_ground: Vec<_> = ["soil", "packedsoil", "spoil", "stone"].iter().filter_map(|n| w.materials.id_of(n)).collect();
    let mut spiders = Spiders::new(&a, &mut w, seed);
    if let Some(s) = spiders.as_mut() {
        s.fill(&mut w, 0, &mut ev);
    }
    for fr in 1..=frames {
        if cut_join > 0 && (fr - 1) % cut_join == 0 {
            joined.iter_mut().for_each(|j| *j = false);
            // Passable = anything but ground (soil, packed soil, spoil, stone): food stored on a chamber floor, bodies and
            // the door markers do not cut a chamber off. First built on inrun::open (empty or a body), which left every
            // floor cut under stored food unpaid -- the old ant lost 62 of 232 near-door cuts, nearly all in its larder.
            let open_below = |x: i32, y: i32| (0..bw).contains(&x) && (0..bh).contains(&y) && y >= surf[x as usize] && !join_ground.contains(&w.get(x, y).material);
            let mut stack: Vec<(i32, i32)> = (0..bh).filter(|&y| open_below(inrun::NEST, y)).map(|y| (inrun::NEST, y)).collect();
            while let Some((x, y)) = stack.pop() {
                let i = (y * bw + x) as usize;
                if joined[i] {
                    continue;
                }
                joined[i] = true;
                for (ex, ey) in [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)] {
                    if open_below(ex, ey) && !joined[(ey * bw + ex) as usize] {
                        stack.push((ex, ey));
                    }
                }
            }
        }
        if let Some((spec, particles, blasts, tuning)) = lab_tick.as_mut() {
            pixel_physics::lab::tick_world(&mut w, particles, blasts, tuning, spec, Some(&sc));
        } else {
            inrun::step(&mut w, &sc);
        }
        if fr % every == 1 || every == 1 {
            let cap_now = cap_to.map_or(cap, |(c, f)| if fr >= f { c } else { (cap as f64 + (c as f64 - cap as f64) * fr as f64 / f as f64).round() as usize });
            // On a bed widened by PP_BED_W the west wall is still 128 out: a west heap that would fall past it is
            // dropped 6 columns beyond the east one instead, so food farther than the wall allows is one-sided.
            if let Some((t, lo, hi)) = patches {
                if fr >= patch_next {
                    patch_next = fr + t;
                    let d = lo + patch_rng.below((hi - lo + 1) as usize) as i32;
                    let east = patch_rng.unit() < 0.5;
                    let x = if east { inrun::NEST + d } else { inrun::NEST - d };
                    // Kept on the bed: a west heap past the wall goes east instead, as `dist=` does.
                    patch_at = if x >= 8 && x < bw - 8 { (x, if east { food_mats.0 } else { food_mats.1 }) } else { (inrun::NEST + d, food_mats.0) };
                    if food_all {
                        patch_at.1 = FOOD_ALL[food_rng.below(FOOD_ALL.len())];
                    }
                    writeln!(patch_log, "{fr},{},{}", patch_at.0, patch_at.1).unwrap();
                }
                inrun::top_up_with(&mut w, &sc, patch_at.0, cap_now, drip as usize, spread, patch_at.1);
            } else {
                let west = west_at(dist);
                for (x, mat, east) in [(inrun::NEST + dist, food_mats.0, true), (west, food_mats.1, false)] {
                    if food_flip.is_some_and(|f| east != ((fr < f) == flip_east_first)) {
                        continue;
                    }
                    inrun::top_up_with(&mut w, &sc, x, cap_now, drip as usize, spread, mat);
                }
            }
        }
        if food_flip == Some(fr) && !flip_keep {
            // The first heap's loose food goes at the flip, so the side that fed the colony is empty, not slowly eaten out.
            let west = west_at(dist);
            let (x, mat) = if flip_east_first { (inrun::NEST + dist, food_mats.0) } else { (west, food_mats.1) };
            let m = w.materials.id_of(mat).expect("the drip's food");
            let mut cleared = 0;
            for cx in (x - 8 - spread).max(0)..=(x + 8 + spread).min(bw - 1) {
                for cy in 0..bh {
                    let c = w.get(cx, cy);
                    if c.material == m && c.organism_id() == 0 {
                        w.set(cx, cy, pixel_physics::sim::Cell::EMPTY);
                        cleared += 1;
                    }
                }
            }
            println!("foodflip at frame {fr}: {cleared} cells cleared from the first heap; deliveries so far {}", w.creature_stats.deliveries);
        }
        if weed > 0 && fr % 500 == 0 {
            for x in (inrun::NEST - weed).max(0)..=(inrun::NEST + weed).min(bw - 1) {
                for y in 0..bh {
                    let c = w.get(x, y);
                    let o = c.organism_id();
                    if o == 0 || !w.organism(o).is_some_and(|s| w.species.get(s.species).creature.is_none()) {
                        continue;
                    }
                    w.loss_context = Some(organism::DeathCause::Culled);
                    w.set(x, y, if y >= surf[x as usize] { pixel_physics::sim::Cell::new(soil_m, c.shade) } else { pixel_physics::sim::Cell::EMPTY });
                    w.loss_context = None;
                    weeded += 1;
                }
            }
        }
        let frame = w.frame - f0;
        let cuts = std::mem::take(w.cut_log.as_mut().unwrap());
        for (x, y, _, who) in cuts {
            let i = (y * bw + x) as usize;
            if (0..bw).contains(&x) && (0..bh).contains(&y) && orig[i] {
                orig[i] = false;
                let depth = y - surf[x as usize];
                let mut worth = if zone_on {
                    if zone[i] && depth >= nest_top {
                        grow(&mut zone, x, y);
                        // A paid zone cut, for judging the zone (which cuts pay is otherwise not in the log).
                        writeln!(ev, "{frame},{who},zpay,{x},{y},{depth},{who}").unwrap();
                        1.0
                    } else {
                        if zone_debit > 0.0 {
                            if let Some(b) = book.get_mut(&who) {
                                let g = -zone_debit * 0.5 * w_dig * dig_price;
                                b.credit += g;
                                b.g2 += g * g;
                            }
                        }
                        0.0
                    }
                } else if in_nest(x) {
                    1.0
                } else {
                    0.0
                };
                if cut_join > 0 {
                    let nb = [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)];
                    if nb.iter().any(|&(ex, ey)| (0..bw).contains(&ex) && (0..bh).contains(&ey) && joined[(ey * bw + ex) as usize]) {
                        joined[i] = true;
                    } else {
                        writeln!(ev, "{frame},{who},unjoined,{x},{y},{depth},{who}").unwrap();
                        worth = 0.0;
                    }
                }
                if worth > 0.0 && cut_roof && y >= surf[x as usize] {
                    let ground = |ex: i32, ey: i32| join_ground.contains(&w.get(ex, ey).material);
                    let roofed = (surf[x as usize]..y).any(|ey| ground(x, ey));
                    let gap_open = |ex: i32| (0..bw).contains(&ex) && !ground(ex, surf[ex as usize]);
                    let (mut l, mut r) = (x, x);
                    while l - x > -8 && gap_open(l - 1) {
                        l -= 1;
                    }
                    while r - x < 8 && gap_open(r + 1) {
                        r += 1;
                    }
                    // Unroofed means the datum cell of this column is open too, so the run below contains x.
                    if !roofed && r - l + 1 > 5 {
                        writeln!(ev, "{frame},{who},unroofed,{x},{y},{depth},{who}").unwrap();
                        worth = 0.0;
                        if roof_debit > 0.0 {
                            if let Some(b) = book.get_mut(&who) {
                                let g = -roof_debit * 0.5 * w_dig * dig_price;
                                b.credit += g;
                                b.g2 += g * g;
                            }
                        }
                    }
                }
                if worth > 0.0 && rim_rows > 0 {
                    if depth < rim_rows {
                        worth = rim_share;
                        if let Some(b) = book.get_mut(&who) {
                            let g = -rim_debit * 0.5 * w_dig * dig_price;
                            b.credit += g;
                            b.g2 += g * g;
                        }
                    } else {
                        worth = 1.0 + deep_bonus;
                    }
                }
                held.insert(who, (who, worth));
                writeln!(ev, "{frame},{who},dig,{x},{y},{depth},{who}").unwrap();
                if worth == 0.0 {
                    continue;
                }
                if let Some(b) = book.get_mut(&who) {
                    b.tripped = true;
                    b.m[1] += 1.0;
                    let g = worth * 0.5 * w_dig * dig_price;
                    b.credit += g;
                    b.g2 += g * g;
                }
            } else {
                let (mut tag, mut nest) = (0, 0.0);
                'find: for r in 0..=2 {
                    for dy in -r..=r {
                        for dx in -r..=r {
                            if let Some((t, n)) = loose.remove(&(x + dx, y + dy)) {
                                (tag, nest) = (t, n);
                                break 'find;
                            }
                        }
                    }
                }
                held.insert(who, (tag, nest));
                writeln!(ev, "{frame},{who},recut,{x},{y},0,{tag}").unwrap();
                if tag != 0 {
                    loose_n -= 1;
                }
            }
        }
        let mut food_rows = std::mem::take(w.food_log.as_mut().unwrap());
        if let Some(s) = spiders.as_mut() {
            s.ate(&food_rows);
            food_rows.retain(|r| !s.ids.contains(&r.who));
        }
        for r in food_rows {
            let (x, y) = r.at;
            let home = if zone_on {
                (0..bw).contains(&x) && (0..bh).contains(&y) && zone[(y * bw + x) as usize]
            } else {
                (x - inrun::NEST).abs() <= 12
            };
            match r.kind {
                creature::FOOD_SWALLOW if r.flag => {
                    if let Some(b) = book.get_mut(&r.who) {
                        if b.tripped && w_home_eat > 0.0 {
                            b.tripped = false;
                            bit.remove(&r.who);
                            b.credit += w_home_eat;
                            b.m[3] += 1.0;
                            b.g2 += w_home_eat * w_home_eat;
                        }
                    }
                    writeln!(ev, "{frame},{},homeeat,{x},{y},{},0", r.who, r.worth).unwrap()
                }
                creature::FOOD_SWALLOW if !r.flag => {
                    if strict {
                        bit.insert(r.who);
                        if let Some(at) = hungry_due.remove(&r.who) {
                            hungry_paid += 1;
                            if let Some(b) = book.get_mut(&r.who) {
                                b.credit += w_hungry;
                                b.g2 += w_hungry * w_hungry;
                            }
                            writeln!(ev, "{frame},{},hungryout,{x},0,{at},0", r.who).unwrap();
                        }
                    }
                    n_field += 1;
                    cell_worth += (r.worth as f64 - cell_worth) / n_field as f64;
                    if let Some(b) = book.get_mut(&r.who) {
                        b.credit -= w_field;
                        b.g2 += w_field * w_field;
                    }
                    writeln!(ev, "{frame},{},field,{x},{y},{},0", r.who, r.worth).unwrap()
                }
                creature::FOOD_DROP if r.flag && (strict || zone_on) && !home => writeln!(ev, "{frame},{},tdrop,{x},{y},0,0", r.who).unwrap(),
                creature::FOOD_DROP if r.flag => {
                    food_home += 1;
                    delivered.push_back((frame, if n_field > 0 { cell_worth } else { 1.0 }));
                    if let Some(b) = book.get_mut(&r.who) {
                        if (x - inrun::NEST).abs() <= 8 {
                            b.m[0] += 1.0;
                        }
                        let got: f64 = delivered.iter().map(|d| d.1).sum();
                        let ate: f64 = eaten.iter().map(|d| d.1).sum();
                        let deep = 1.0 + w_depth * ((y - surf[x.clamp(0, bw - 1) as usize]).clamp(0, 10) as f64 / 10.0);
                        let fpr = if tank_price { tank_p } else { 1.0 + kprice * (1.0 - if ate > 0.0 { got / ate } else { 1.0 }).max(0.0) };
                        // fjoule: the cell just put down, by its face worth (the field mean if it is already gone).
                        let jf = if fjoule {
                            let v = creature::food_value(&w, w.get(x, y)) as f64;
                            (if v > 0.0 { v } else { cell_worth.max(1.0) }) / joule_ref
                        } else {
                            1.0
                        };
                        let g = jf * deep * w_del * fpr * store_p * if progress { 0.5 } else { 1.0 };
                        if let Some(c) = carry.get_mut(&r.who) {
                            c.3 += 1;
                        }
                        b.credit += g;
                        b.g2 += g * g;
                    }
                    // v: the cell's face worth as put down (`food_value`; 960 for a whole provisions, fruit or windfall
                    // cell, less for a crumb), so a reader can count food home in energy, not cells (selection lane,
                    // 2026-10-27). It was always 0, and no reader used it.
                    let worth = creature::food_value(&w, w.get(x, y)).round() as i64;
                    writeln!(ev, "{frame},{},deliver,{x},{y},{worth},0", r.who).unwrap()
                }
                creature::FOOD_DROP if home => writeln!(ev, "{frame},{},churn,{x},{y},0,0", r.who).unwrap(),
                // SCRATCH (foraging-loop thread): food put down in the field, so a load's fate closes.
                creature::FOOD_DROP => writeln!(ev, "{frame},{},fdrop,{x},{y},{},0", r.who, r.worth).unwrap(),
                _ => {}
            }
        }
        let digested: Vec<_> = w.digest_log.as_mut().unwrap().drain(..).collect();
        for (who, worth, _) in digested {
            if let Some(s) = spiders.as_mut().filter(|s| s.ids.contains(&who)) {
                s.digested(who, worth);
                continue;
            }
            // x,y: the eater's head now (nest lane, 2026-10-27; was 0,0), so a reader can tell food eaten at home --
            // the larder going down -- from food eaten in the field. Read only.
            let (ex, ey) = w.organism(who).and_then(|s| s.chain.first().copied()).unwrap_or((0, 0));
            writeln!(ev, "{frame},{who},eat,{ex},{ey},{worth},0").unwrap();
            eaten.push_back((frame, worth as f64));
        }
        if let Some(s) = spiders.as_mut() {
            s.watch(&w, frame, &mut ev);
        }
        for r in w.death_log.as_mut().unwrap().drain(..) {
            if let Some(s) = spiders.as_mut() {
                s.died(&r, frame, &mut ev);
            }
            if r.species == sp {
                writeln!(ev, "{frame},{},death,0,0,{},0", r.id, r.cause.index()).unwrap();
                held.remove(&r.id);
                if !culled_pending.remove(&r.id) {
                    if let Some(b) = book.remove(&r.id) {
                        let age = frame - b.born;
                        if hall_keep > 0 && age >= min_age && r.cause == organism::DeathCause::Starved {
                            hall.push((frame, b.credit / (age as f64 / 1000.0), b));
                        }
                    }
                }
            }
        }
        if let Some(s) = spiders.as_mut() {
            s.fill(&mut w, frame, &mut ev);
        }
        // brood=lay: an adult just hatched is born now, and selection writes its mind (see `brood_lay`). Read after
        // the death log, so an id a death freed this frame is out of the book before a hatchling can take it.
        if brood_lay && w.creature_stats.births != seen_births {
            seen_births = w.creature_stats.births;
            let mut fresh: Vec<u32> = inrun::ants(&w, sp).into_iter().filter(|i| !book.contains_key(i)).collect();
            fresh.sort_unstable();
            for cid in fresh {
                // No selection: the hatchling keeps the mind its egg took from its layer, as in the lab.
                if sel == "off" {
                    let g = creature::mind::genome_of(&w, cid).map(|g| g.w.clone()).unwrap_or_default();
                    writeln!(births, "{frame},{cid},0,layer,0,hatch,{}", g.iter().filter(|v| **v != 0.0).count()).unwrap();
                    writeln!(csv_ants, "{cid},layer").unwrap();
                    book.insert(cid, Who { born: frame, credit: 0.0, g2: 0.0, family: "layer".into(), gen: 1, genome: g, last_kid: 0, kids: 0, tripped: false, at_kid: (0.0, 0.0), m: [0.0; 5] });
                    hatched_since += 1;
                    continue;
                }
                // The queue's next parent still alive and under its cap, cycling the queue again when it runs out
                // (the judgement's ranking stands until the next one); with no queued parent alive, the egg's own
                // layer, as in the lab; then the best of the hall; with nobody, the founder line. Review (2026-10-23):
                // a first build fell back to the best living adult by rate, which skips min_age, cooldown, the bars
                // and the family cap -- 25-42% of hatchlings took a mind that way, mostly from ants a few hundred
                // frames old, and it ran under sel=random too.
                let mut pid = None;
                let mut how = "hatch";
                for _ in 0..2 {
                    while sire_at < sires.len() {
                        let s = sires[sire_at];
                        sire_at += 1;
                        if book.get(&s).is_some_and(|b| b.kids < kid_cap) {
                            pid = Some(s);
                            break;
                        }
                    }
                    if pid.is_some() || sires.is_empty() {
                        break;
                    }
                    sire_at = 0;
                    how = "hatch-cycle";
                }
                if pid.is_none() {
                    pid = w.organism(cid).map(|s| s.parent).filter(|p| book.contains_key(p));
                    how = "hatch-layer";
                }
                let (base, family, gen) = match pid {
                    Some(i) => (book[&i].genome.clone(), book[&i].family.clone(), book[&i].gen),
                    None => {
                        how = "hatch-hall";
                        match hall.iter().max_by(|x, y| x.1.partial_cmp(&y.1).unwrap()) {
                            Some(h) => (h.2.genome.clone(), h.2.family.clone(), h.2.gen),
                            None => (raw[slot(0)].clone(), mix[slot(0)].0.clone(), 0),
                        }
                    }
                };
                let mut child = if neutral {
                    neutral_mutate(&base, &kslots, kpm, ksigma, kevents, prot, &mut srng)
                } else {
                    train::mutate(&base, &kslots, kpm, ksigma, kprune, kgrow, &[], &mut srng)
                };
                lay_mutate(&mut child, &mut srng);
                creature::mind::give(&mut w, cid, inrun::mind(&child));
                if let Some(pb) = pid.and_then(|i| book.get_mut(&i)) {
                    pb.last_kid = frame;
                    pb.kids += 1;
                    pb.at_kid = (pb.credit, pb.g2);
                    if retire && pb.kids >= kid_cap {
                        retiring.push(pid.unwrap());
                    }
                }
                writeln!(births, "{frame},{cid},{},{family},{},{how},{}", pid.unwrap_or(0), gen + 1, child.iter().filter(|v| **v != 0.0).count()).unwrap();
                writeln!(csv_ants, "{cid},{family}").unwrap();
                book.insert(cid, Who { born: frame, credit: 0.0, g2: 0.0, family, gen: gen + 1, genome: child, last_kid: 0, kids: 0, tripped: false, at_kid: (0.0, 0.0), m: [0.0; 5] });
                hatched_since += 1;
                last_birth = frame;
            }
        }
        // A pellet let go: where the head is now.
        // Sorted: `held` is a HashMap, and the order pellets are handled in moves `loose_n` (an out-pellet's clear price
        // reads it) and the order credits are summed. Unsorted, 2 of 64 seeds diverged between two runs of one build.
        let mut gone: Vec<u32> = held.keys().copied().filter(|&id| w.organism(id).is_none_or(|s| s.spoil.is_none())).collect();
        gone.sort_unstable();
        for id in gone {
            let (tag, worth) = held.remove(&id).unwrap();
            let Some(&(x, y)) = w.organism(id).and_then(|s| s.chain.first()) else { continue };
            if y < surf[x.clamp(0, bw - 1) as usize] {
                writeln!(ev, "{frame},{id},out,{x},{y},0,{tag}").unwrap();
                let alive = book.len().max(1) as f64;
                let far = ((((x - inrun::NEST).abs() as f64) - rim) / rim_ramp).clamp(0.0, 1.0);
                let far = if rim > 0.0 { far } else { 1.0 };
                let far = far * worth;
                let pc = far * w_clear * (1.0 + kprice * (loose_n as f64 / (2.0 * alive)).clamp(0.0, 1.0));
                if let Some(b) = book.get_mut(&id) {
                    b.tripped = true;
                    b.credit += pc;
                    if far > 0.0 {
                        b.m[2] += 1.0;
                    }
                    b.g2 += pc * pc;
                }
                if tag != 0 {
                    if let Some(b) = book.get_mut(&tag) {
                        let g = far * 0.5 * w_dig * dig_price;
                        b.credit += g;
                        b.g2 += g * g;
                    }
                }
            } else {
                loose.insert((x, y), (tag, worth));
                loose_n += 1;
                writeln!(ev, "{frame},{id},in,{x},{y},0,{tag}").unwrap();
            }
        }
        if let Some((f, g)) = starve_at {
            if fr == f {
                inrun::flat_reserves(&mut w, sp, g as f32);
            }
        }
        if progress && fr % 100 == 0 {
            let fpr_now = store_p * if tank_price { tank_p } else { 1.0 + kprice };
            let mut ids: Vec<u32> = book.keys().copied().collect();
            ids.sort_unstable();
            for id in ids {
                let st = w.organism(id).map(|s| (s.chain.first().copied(), s.trip_cells, s.crop.map_or(0, |c| c.cells), s.crop.map_or(0.0, |c| c.unit)));
                let Some((Some((x, _)), trip, cells, unit)) = st else { continue };
                let jf = if fjoule { unit as f64 / joule_ref } else { 1.0 };
                if trip > 0 && cells > 0 {
                    let d = (x - inrun::NEST).abs() as f64;
                    let c = carry.entry(id).or_insert((d.max(1.0), d, 0.0, 0));
                    let most = carried.entry(id).or_insert(0);
                    *most = (*most).max(cells as u32);
                    if d < c.1 {
                        let g = jf * 0.5 * w_del * fpr_now * cells as f64 * (c.1 - d) / c.0;
                        c.1 = d;
                        c.2 += g;
                        let b = book.get_mut(&id).unwrap();
                        b.credit += g;
                        b.g2 += g * g;
                    }
                } else if let Some(c) = carry.remove(&id) {
                    let most = carried.remove(&id).unwrap_or(0).max(1) as f64;
                    if strict && c.2 > 0.0 {
                        let back = c.2 * (1.0 - (c.3 as f64 / most).min(1.0));
                        let b = book.get_mut(&id).unwrap();
                        b.credit -= back;
                    } else if c.3 == 0 && c.2 > 0.0 {
                        let b = book.get_mut(&id).unwrap();
                        b.credit -= c.2;
                    }
                }
            }
        }
        if fr % 100 == 0 {
            if let Some((f, top)) = grow_band {
                if fr % f == 0 && hi < top {
                    lo += 4;
                    hi += 4;
                }
            }
            if tank_price {
                let es: Vec<f64> = inrun::ants(&w, sp).iter().filter_map(|&i| w.organism(i)).map(|s| s.energy as f64).collect();
                let mean = es.iter().sum::<f64>() / es.len().max(1) as f64;
                tank_p = 1.0 + ktank * (1.0 - mean / (tank_full * start_e)).clamp(0.0, 1.0);
            }
            if w_hungry > 0.0 {
                let mut ids: Vec<u32> = book.keys().copied().collect();
                ids.sort_unstable();
                for id in ids {
                    let Some(s) = w.organism(id) else { continue };
                    let Some(&(x, _)) = s.chain.first() else { continue };
                    let dx = (x - inrun::NEST).abs();
                    let e = s.energy as f64;
                    let st = leave.entry(id).or_insert((true, 0));
                    if dx <= 12 {
                        st.0 = true;
                        hungry_due.remove(&id);
                    } else if dx >= 15 && st.0 {
                        st.0 = false;
                        if e < hungry_at * start_e && (st.1 == 0 || frame - st.1 >= hungry_gap) {
                            st.1 = frame;
                            if strict {
                                hungry_due.insert(id, frame);
                                continue;
                            }
                            hungry_paid += 1;
                            let b = book.get_mut(&id).unwrap();
                            b.credit += w_hungry;
                            b.m[4] += 1.0;
                            b.g2 += w_hungry * w_hungry;
                            writeln!(ev, "{frame},{id},hungryout,{x},0,{e:.0},0").unwrap();
                        }
                    }
                }
            }
            for (id, b) in book.iter_mut() {
                if let Some(&(x, _)) = w.organism(*id).and_then(|s| s.chain.first()) {
                    if (x - inrun::NEST).abs() >= 15 && (!strict || bit.contains(id)) {
                        b.tripped = true;
                    }
                }
            }
        }
        while delivered.front().is_some_and(|d| d.0 + 5000 < frame) {
            delivered.pop_front();
        }
        while eaten.front().is_some_and(|d| d.0 + 5000 < frame) {
            eaten.pop_front();
        }
        if fr % 1000 == 0 || fr == 1 {
            let mut open_n = 0i64;
            for x in (inrun::NEST - 100)..=(inrun::NEST + 100) {
                for y in surf[x as usize].max(40)..bh {
                    if matches!(inrun::cell_char(&w, x, y), '.' | 'a') {
                        open_n += 1;
                    }
                }
            }
            let b0 = *base_open.get_or_insert(open_n);
            let per = (open_n - b0) as f64 / book.len().max(1) as f64;
            dig_price = 1.0 + kprice * (1.0 - per / 25.0).max(0.0);
        }
        if sel != "off" && fr % judge_every == 0 {
            // In id order: `book` is a HashMap, and its order is not the run's (a read, not a choice -- only the
            // file's row order moved when this was sorted).
            let mut aged: Vec<(&u32, &Who)> = book.iter().collect();
            aged.sort_unstable_by_key(|(id, _)| **id);
            for (id, b) in aged {
                let age = frame - b.born;
                if age >= 30_000 && age < 30_000 + judge_every {
                    writeln!(age30, "{id},{:.4}", b.credit / 30.0).unwrap();
                }
            }
            // Each ant's rate per 1,000 frames alive, against the adults' median, in units of its own standard error:
            // a sum of credits is Poisson-like, so its error is sqrt(sum of squares), floored at what the colony's
            // average ant would have shown over the same age (an ant with no events is not certain).
            let ids: Vec<u32> = { let mut v: Vec<u32> = book.keys().copied().collect(); v.sort_unstable(); v };
            let n = ids.len();
            let (sg2, st): (f64, f64) = ids.iter().map(|i| { let b = &book[i]; (b.g2, (frame - b.born) as f64 / 1000.0) }).fold((0.0, 0.0), |a, c| (a.0 + c.0, a.1 + c.1));
            let rate_g2 = sg2 / st.max(1e-9);
            let adults: Vec<u32> = ids.iter().copied().filter(|i| frame - book[i].born >= min_age).collect();
            let mut births_n = std::mem::take(&mut hatched_since);
            let (mut q_now, mut hard_n) = (0usize, 0usize);
            let mut removals_n = 0;
            let mut med = f64::NAN;
            // One adult is enough to judge by once `turn` is on: a colony crashed to two ants by a dry store could
            // otherwise never refill (seed 4: 24 -> 2 at 120k, then no judgement for the rest of the round).
            if adults.len() >= if turn > 0 { 1 } else { 3 } {
                if n < lo {
                    bias = (bias - 0.05).max(-1.5);
                } else if n > hi {
                    bias = (bias + 0.05).min(1.5);
                }
                let score = |b: &Who| b.credit / ((frame - b.born) as f64 / 1000.0).max(0.1);
                let mut sc: Vec<f64> = adults.iter().map(|i| score(&book[i])).collect();
                sc.sort_by(|a, b| a.partial_cmp(b).unwrap());
                med = sc[sc.len() / 2];
                let z = |b: &Who| {
                    let t = ((frame - b.born) as f64 / 1000.0).max(0.1);
                    (score(b) - med) / (b.g2.max(rate_g2 * t).sqrt() / t).max(1e-6)
                };
                let (zb, zc) = (zb0 + bias, zc0 - bias);
                // The family cap: no founder line past a quarter of the colony.
                let mut fam: HashMap<String, usize> = HashMap::new();
                for i in &ids {
                    *fam.entry(book[i].family.clone()).or_default() += 1;
                }
                // A quarter of the band's top, not of the colony now: as a quarter of `n` it shut every family out
                // once a colony was small (n 7: cap 2), so a starving colony could not refill (p7, 2026-10-21).
                let cap_f = if brood_lay { (hi_band.max(n) / 4).max(2) } else { (hi / 4).max(2) };
                // Who the score picks: parents with their kid counts, and removals.
                let mut parents: Vec<(u32, u32)> = Vec::new();
                // Every eligible parent this judgement, for `cross=`'s second parent.
                let mut mates: Vec<u32> = Vec::new();
                for i in &adults {
                    let b = &book[i];
                    let zi = z(b);
                    if zi > zb && frame - b.last_kid >= cooldown && (b.last_kid > 0 || frame - b.born >= min_age) {
                        parents.push((*i, ((zi - zb).floor() as u32 + 1).min(3)));
                    }
                }
                let mut cut: Vec<u32> = adults.iter().copied().filter(|i| z(&book[i]) < -zc).collect();
                if turnover > 0 && parents.is_empty() && frame - last_birth >= turnover {
                    let zs: Vec<(u32, f64)> = adults.iter().map(|i| (*i, z(&book[i]))).collect();
                    let best = zs.iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap().then(b.0.cmp(&a.0))).unwrap().0;
                    let worst = zs.iter().min_by(|a, b| a.1.partial_cmp(&b.1).unwrap().then(a.0.cmp(&b.0))).unwrap().0;
                    if best != worst {
                        parents.push((best, 1));
                        if !cut.contains(&worst) {
                            cut.push(worst);
                        }
                    }
                }
                if turn > 0 {
                    // rank=shrunk (research lane, PR #768): rank by the rate shrunk toward the median with `lambda`
                    // thousand frames of prior, (credit + lambda * median) / (T + lambda) - median, instead of z. z
                    // divides by the ant's own squared credits, so a rare large credit (a delivery) counts against
                    // it: z and plain score agreed on only 44 of 133 parents (smoke9). The hard cull stays on z.
                    let mut fsum: HashMap<&str, (f64, usize)> = HashMap::new();
                    if famprior {
                        for i in &adults {
                            let e = fsum.entry(book[i].family.as_str()).or_default();
                            e.0 += score(&book[i]);
                            e.1 += 1;
                        }
                    }
                    let prior = |b: &Who| -> f64 {
                        match fsum.get(b.family.as_str()) {
                            Some(&(s, n)) if n >= 2 => {
                                let own = if frame - b.born >= min_age { score(b) } else { 0.0 };
                                let k = if frame - b.born >= min_age { n - 1 } else { n };
                                (s - own) / k as f64
                            }
                            _ => med,
                        }
                    };
                    let rv = |b: &Who| -> f64 {
                        if shrunk {
                            let t = (frame - b.born) as f64 / 1000.0;
                            (b.credit + lambda * prior(b)) / (t + lambda) - med
                        } else {
                            z(b)
                        }
                    };
                    let mut zs: Vec<(u32, f64)> = adults.iter().map(|i| (*i, rv(&book[i]))).collect();
                    zs.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap().then(a.0.cmp(&b.0)));
                    // Owed replacements carry over until a child is actually born (capped at 5, so a dry spell does
                    // not end in a burst): taken off at the moment of the birth, they were lost whenever no parent was
                    // eligible (58 of 146 due judgements in a 200k test).
                    quota = (quota + adults.len() as f64 * judge_every as f64 / turn as f64).min(5.0);
                    let q = quota.floor() as usize;
                    q_now = q;
                    cut = zs.iter().filter(|p| z(&book[&p.0]) < -zhard).map(|p| p.0).collect();
                    let hard = cut.len();
                    hard_n = hard;
                    let extra_cut = n.saturating_sub(hi);
                    for p in &zs {
                        if cut.len() >= hard + extra_cut || p.1 >= 0.0 {
                            break;
                        }
                        if !cut.contains(&p.0) {
                            cut.push(p.0);
                        }
                    }
                    // The quota's removals wait for its births: each is made only once a child has taken its place,
                    // so a colony that cannot breed is not also emptied (p7: 36 culls against 30 births, 24 -> 7).
                    quota_cut = zs.iter().filter(|p| p.1 < 0.0 && !cut.contains(&p.0)).take(q).map(|p| p.0).collect();
                    // With the hall on, refilling below `lo` is its job (below), not the living-only quota's.
                    let short = if hall_keep > 0 { 0 } else { lo.saturating_sub(n) };
                    let want = (hard + quota_cut.len() + short) as u32;
                    // brood=lay: as many picks as there are adults, the next judgement's hatchlings' parents.
                    let want = if brood_lay { want.max(adults.len() as u32) } else { want };
                    // Breeding z: lifetime, or with fresh_kid the record since the last child.
                    let zb_of = |i: u32| -> Option<f64> {
                        let b = &book[&i];
                        if b.kids >= kid_cap || frame - b.last_kid < cooldown {
                            return None;
                        }
                        if !fresh_kid || b.kids == 0 {
                            return Some(rv(b));
                        }
                        let since = frame - b.last_kid;
                        if since < min_age {
                            return None;
                        }
                        let t = since as f64 / 1000.0;
                        let (c, g2) = (b.credit - b.at_kid.0, b.g2 - b.at_kid.1);
                        if shrunk {
                            return Some((c + lambda * prior(b)) / (t + lambda) - med);
                        }
                        Some((c / t - med) / (g2.max(rate_g2 * t).sqrt() / t).max(1e-6))
                    };
                    // Parents whose family is at its cap are passed over here, so their children go to the next in line
                    // rather than not being born (the cap otherwise stopped most births once a line did well).
                    if std::env::var("INRUN_DEBUG").is_ok() && frame % 20000 == 0 {
                        for p in &zs {
                            let b = &book[&p.0];
                            eprintln!("dbg f{frame} n{n} id{} z{:.2} zb{:?} kids{} since{} fam{}={} cap{cap_f}", p.0, p.1, zb_of(p.0), b.kids, frame - b.last_kid, b.family, fam.get(&b.family).copied().unwrap_or(0));
                        }
                    }
                    let mut okz: Vec<(u32, f64)> = zs
                        .iter()
                        .filter_map(|p| zb_of(p.0).map(|v| (p.0, v)))
                        .filter(|p| lexicase || p.1 > 0.0 || (n < lo && p.1 >= 0.0))
                        .collect();
                    // The family cap steers births to other lines; it never stops them. When every eligible parent is
                    // in a capped line (a colony down to one family: seed 4, all nine ants m20), they breed anyway.
                    let open: Vec<(u32, f64)> = okz.iter().copied().filter(|p| fam.get(&book[&p.0].family).copied().unwrap_or(0) < cap_f).collect();
                    capped_only = open.is_empty();
                    // A colony below its band with no eligible parent breeds from its best ant by lifetime record:
                    // the evidence still picks which ant, the bar is not lowered to "anyone". Seed 4 sat at two ants
                    // for 120k frames because the better one's record since its last child was below the median.
                    if okz.is_empty() && n < lo {
                        if let Some(best) = zs.last() {
                            okz.push(*best);
                        }
                    }
                    if !capped_only {
                        okz = open;
                    }
                    okz.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap().then(a.0.cmp(&b.0)));
                    let ok: Vec<u32> = okz.into_iter().map(|p| p.0).collect();
                    let mut kids = vec![0u32; ok.len()];
                    if lexicase && !ok.is_empty() {
                        let rate = |i: u32, k: usize| {
                            let b = &book[&i];
                            b.m[k] / ((frame - b.born) as f64 / 1000.0).max(0.1)
                        };
                        for _ in 0..want {
                            let mut order: Vec<usize> = (0..5).collect();
                            for k in (1..order.len()).rev() {
                                let j = srng.below(k + 1);
                                order.swap(k, j);
                            }
                            let mut pool: Vec<usize> = (0..ok.len()).collect();
                            for &obj in &order {
                                if pool.len() <= 1 {
                                    break;
                                }
                                let rs: Vec<f64> = pool.iter().map(|&p| rate(ok[p], obj)).collect();
                                let best = rs.iter().copied().fold(f64::MIN, f64::max);
                                let mut sorted = rs.clone();
                                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
                                let med = sorted[sorted.len() / 2];
                                let mut dev: Vec<f64> = rs.iter().map(|r| (r - med).abs()).collect();
                                dev.sort_by(|a, b| a.partial_cmp(b).unwrap());
                                let mad = dev[dev.len() / 2];
                                pool = pool.iter().zip(rs.iter()).filter(|(_, r)| **r >= best - mad).map(|(p, _)| *p).collect();
                            }
                            kids[pool[srng.below(pool.len())]] += 1;
                        }
                    } else {
                        let mut given = 0;
                        'passes: for span in [ok.len(), ok.len().div_ceil(2), ok.len().div_ceil(4)] {
                            for k in kids.iter_mut().take(span) {
                                if given >= want {
                                    break 'passes;
                                }
                                *k += 1;
                                given += 1;
                            }
                        }
                    }
                    mates = ok.clone();
                    if want > 0 {
                        q_due += 1;
                        if ok.is_empty() {
                            q_noparent += 1;
                        }
                    }
                    parents = ok.into_iter().zip(kids).filter(|p| p.1 > 0).collect();
                    // Research lane's check (b): would plain score have picked the same parents? z divides by the
                    // ant's own squared credits, so rare large credits (a delivery) count against it.
                    if !parents.is_empty() {
                        let k = parents.len();
                        let mut by_s: Vec<(u32, f64)> = adults.iter().filter(|i| frame - book[i].last_kid >= cooldown).map(|i| (*i, score(&book[i]))).collect();
                        by_s.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap().then(a.0.cmp(&b.0)));
                        let same = parents.iter().filter(|p| by_s.iter().take(k).any(|q| q.0 == p.0)).count();
                        zs_pick.0 += k as u64;
                        zs_pick.1 += same as u64;
                    }
                }
                let room = (hi + 4).saturating_sub(n);
                let floor_room = n.saturating_sub(lo.saturating_sub(8));
                cut.truncate(floor_room);
                if sel == "random" {
                    // Same counts, chosen blind.
                    let kids: u32 = parents.iter().map(|p| p.1).sum();
                    let pool: Vec<u32> = adults.iter().copied().filter(|i| frame - book[i].last_kid >= cooldown || book[i].last_kid == 0).collect();
                    mates = pool.clone();
                    parents = (0..kids).filter(|_| !pool.is_empty()).map(|_| (pool[srng.below(pool.len())], 1)).collect();
                    let mut ad = adults.clone();
                    let mut c2 = Vec::new();
                    for _ in 0..cut.len().min(ad.len()) {
                        c2.push(ad.swap_remove(srng.below(ad.len())));
                    }
                    cut = c2;
                    let mut qc = Vec::new();
                    for _ in 0..quota_cut.len().min(ad.len()) {
                        qc.push(ad.swap_remove(srng.below(ad.len())));
                    }
                    quota_cut = qc;
                }
                if brood_lay {
                    sires.clear();
                    sire_at = 0;
                    for round in 1..=parents.iter().map(|p| p.1).max().unwrap_or(0) {
                        sires.extend(parents.iter().filter(|p| p.1 >= round).map(|p| p.0));
                    }
                    parents.clear();
                }
                if nocull {
                    cut.clear();
                    quota_cut.clear();
                    retiring.clear();
                }
                for &id in &cut {
                    if inrun::cull(&mut w, id) {
                        culled_pending.insert(id);
                        removals_n += 1;
                        writeln!(births, "{frame},{id},0,{},{},cull", book[&id].family, book[&id].gen).unwrap();
                        book.remove(&id);
                    }
                }
                for (pid, k) in parents {
                    for _ in 0..k {
                        if births_n >= room {
                            break;
                        }
                        let Some(p) = book.get(&pid) else { break };
                        let f = p.family.clone();
                        if fam.get(&f).copied().unwrap_or(0) >= cap_f && !(turn > 0 && capped_only) {
                            break;
                        }
                        let mut crossed = false;
                        let base: Vec<f32> = if cross_p > 0.0 && mates.len() >= 2 && srng.unit() < cross_p {
                            let others: Vec<u32> = mates.iter().copied().filter(|&m| m != pid && book.contains_key(&m)).collect();
                            if others.is_empty() {
                                p.genome.clone()
                            } else {
                                crossed = true;
                                let q = others[srng.below(others.len())];
                                train::cross(&p.genome, &book[&q].genome, &mut srng)
                            }
                        } else {
                            p.genome.clone()
                        };
                        let mut child = if neutral {
                            neutral_mutate(&base, &kslots, kpm, ksigma, kevents, prot, &mut srng)
                        } else {
                            train::mutate(&base, &kslots, kpm, ksigma, kprune, kgrow, &[], &mut srng)
                        };
                        lay_mutate(&mut child, &mut srng);
                        let gen = p.gen + 1;
                        let placed = inrun::birth(&mut w, sp, inrun::mind(&child), kid_grants);
                        if placed.is_none() {
                            birth_fail += 1;
                        }
                        if let Some(cid) = placed {
                            births_n += 1;
                            last_birth = frame;
                            *fam.entry(f.clone()).or_default() += 1;
                            let pb = book.get_mut(&pid).unwrap();
                            pb.last_kid = frame;
                            pb.kids += 1;
                            pb.at_kid = (pb.credit, pb.g2);
                            if retire && pb.kids >= kid_cap {
                                retiring.push(pid);
                            }
                            writeln!(births, "{frame},{cid},{pid},{f},{gen},{},{}", if crossed { "cross" } else { "birth" }, child.iter().filter(|v| **v != 0.0).count()).unwrap();
                            writeln!(csv_ants, "{cid},{f}").unwrap();
                            book.insert(cid, Who { born: frame, credit: 0.0, g2: 0.0, family: f, gen, genome: child, last_kid: 0, kids: 0, tripped: false, at_kid: (0.0, 0.0), m: [0.0; 5] });
                        }
                    }
                }
                for id in std::mem::take(&mut retiring) {
                    if book.contains_key(&id) && inrun::cull(&mut w, id) {
                        culled_pending.insert(id);
                        removals_n += 1;
                        writeln!(births, "{frame},{id},0,{},{},retire", book[&id].family, book[&id].gen).unwrap();
                        book.remove(&id);
                    }
                }
                let refill = if hall_keep > 0 { 0 } else { lo.saturating_sub(n) };
                let paid = births_n.saturating_sub(refill + hard_n).min(q_now);
                quota -= paid as f64;
                for id in std::mem::take(&mut quota_cut).into_iter().take(births_n.saturating_sub(refill + hard_n)) {
                    if book.contains_key(&id) && inrun::cull(&mut w, id) {
                        culled_pending.insert(id);
                        removals_n += 1;
                        writeln!(births, "{frame},{id},0,{},{},cull", book[&id].family, book[&id].gen).unwrap();
                        book.remove(&id);
                    }
                }
            }
            if hall_keep > 0 {
                hall.retain(|h| frame - h.0 <= hall_keep);
                let n_now = book.len();
                if n_now < lo && !brood_lay {
                    // Plain score per 1k frames for both; a dead ant's is cut by `dead_w` (a loss grows by 1/dead_w).
                    let pen = |v: f64| if v > 0.0 { v * dead_w } else { v / dead_w.max(1e-6) };
                    let mut ids: Vec<u32> = book.keys().copied().collect();
                    ids.sort_unstable();
                    // (value, Some(living id) | None, hall index)
                    let mut cand: Vec<(f64, Option<u32>, usize)> = ids
                        .iter()
                        .filter(|i| frame - book[i].born >= min_age && book[i].kids < kid_cap)
                        .map(|i| (book[i].credit / ((frame - book[i].born) as f64 / 1000.0), Some(*i), 0))
                        .collect();
                    cand.extend(hall.iter().enumerate().filter(|h| h.1 .2.kids < kid_cap).map(|(k, h)| (pen(h.1), None, k)));
                    if sel == "random" {
                        for k in (1..cand.len()).rev() {
                            cand.swap(k, srng.below(k + 1));
                        }
                    } else {
                        cand.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap().then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
                    }
                    // Graded as the quota's births are: one each down the list, then again down the top half, the top
                    // quarter.
                    let need = lo - n_now;
                    let mut kids = vec![0u32; cand.len()];
                    let mut given = 0;
                    'hp: for span in [cand.len(), cand.len().div_ceil(2), cand.len().div_ceil(4)] {
                        for k in kids.iter_mut().take(span) {
                            if given >= need {
                                break 'hp;
                            }
                            *k += 1;
                            given += 1;
                        }
                    }
                    for (c, k) in cand.iter().zip(kids) {
                        for _ in 0..k {
                            let (genome, family, gen, pid) = match c.1 {
                                Some(i) => (book[&i].genome.clone(), book[&i].family.clone(), book[&i].gen, i),
                                None => {
                                    let h = &hall[c.2];
                                    (h.2.genome.clone(), h.2.family.clone(), h.2.gen, 0)
                                }
                            };
                            let mut child = if neutral {
                                neutral_mutate(&genome, &kslots, kpm, ksigma, kevents, prot, &mut srng)
                            } else {
                                train::mutate(&genome, &kslots, kpm, ksigma, kprune, kgrow, &[], &mut srng)
                            };
                            lay_mutate(&mut child, &mut srng);
                            let Some(cid) = inrun::birth_in(&mut w, sp, home_colony, home_anchor, inrun::mind(&child), kid_grants) else {
                                birth_fail += 1;
                                continue;
                            };
                            births_n += 1;
                            last_birth = frame;
                            match c.1 {
                                Some(i) => {
                                    let pb = book.get_mut(&i).unwrap();
                                    pb.kids += 1;
                                    pb.last_kid = frame;
                                    pb.at_kid = (pb.credit, pb.g2);
                                }
                                None => hall[c.2].2.kids += 1,
                            }
                            let kind = if c.1.is_some() { "refill" } else { "hall" };
                            writeln!(births, "{frame},{cid},{pid},{family},{},{kind},{}", gen + 1, child.iter().filter(|v| **v != 0.0).count()).unwrap();
                            writeln!(csv_ants, "{cid},{family}").unwrap();
                            book.insert(cid, Who { born: frame, credit: 0.0, g2: 0.0, family, gen: gen + 1, genome: child, last_kid: 0, kids: 0, tripped: false, at_kid: (0.0, 0.0), m: [0.0; 5] });
                        }
                    }
                }
            }
            let got: f64 = delivered.iter().map(|d| d.1).sum();
            let ate: f64 = eaten.iter().map(|d| d.1).sum();
            let fp = if tank_price { tank_p } else { 1.0 + kprice * (1.0 - if ate > 0.0 { got / ate } else { 1.0 }).max(0.0) };
            let cp = 1.0 + kprice * (loose_n as f64 / (2.0 * book.len().max(1) as f64)).clamp(0.0, 1.0);
            // Live ants the book does not know (must stay 0).
            let ghosts = inrun::ants(&w, sp).iter().filter(|i| !book.contains_key(i)).count();
            writeln!(sel_log, "{frame},{},{},{births_n},{removals_n},{bias:.2},{med:.3},{fp:.2},{dig_price:.2},{cp:.2},{ghosts}", book.len(), adults.len()).unwrap();
        }
        if brood_lay && fr % 25_000 == 0 {
            write_nestmap(&w, &out.join(format!("nestmap_f{fr:07}.txt")));
            write_occupants(&w, &out.join(format!("occupants_f{fr:07}.txt")));
        }
        if fr % 25_000 == 0 {
            let door = w.nest_sites.first().map_or(inrun::NEST, |s| s.x);
            let (ns, _) = pixel_physics::lab::nestscore::of_world(&w, &sc.bed, door, &pixel_physics::lab::nestscore::HARVESTER);
            // door_open (dig thread, PR #780): the nest's largest piece joins the air; -1 before there is a nest.
            let door_open = pixel_physics::lab::nestscore::door_open(&w, &sc.bed, door, 3, 20).map_or(-1, |o| o as i32);
            writeln!(nest_log, "{fr},{},{food_home},{:.2},{},{},{},{},{},{:.3},{door_open},{},{}", inrun::ants(&w, sp).len(), ns.score, ns.dug, ns.chambers.len(), ns.niches.len(), ns.tunnel_cells, ns.pit, ns.per_cell(), zone.iter().filter(|&&z| z).count(), (0..zone.len()).filter(|&i| zone[i]).map(|i| (i as i32 % bw - inrun::NEST).abs()).max().unwrap_or(0)).unwrap();
        }
        // brood=lay: the size brake (`lay_cap`) every 250 frames, and the brood's census every 1,000.
        if brood_lay && fr % 250 == 0 {
            let mut stages = [0u32; 3];
            let mut debt = 0.0f64;
            for id in w.live_brood_ids() {
                if let Some(s) = w.organism(id) {
                    if let Some(b) = s.brood {
                        stages[b.stage as usize] += 1;
                        if b.stage != organism::BroodStage::Pupa {
                            debt += (b.target - s.energy).max(0.0) as f64;
                        }
                    }
                }
            }
            let ant_ids = inrun::ants(&w, sp);
            let adults_n = ant_ids.len();
            let heads = (adults_n + stages.iter().sum::<u32>() as usize) as f64;
            let mut lay_at = match lay_cap {
                Some((t, d)) => base_lay_at as f64 * 2f64.powf(lay_cap_p * (heads - t).max(0.0) / d),
                None => base_lay_at as f64,
            };
            if let Some(r) = lay_reserve {
                let banks: f64 = ant_ids.iter().filter_map(|&i| w.organism(i)).map(|s| s.energy as f64).sum();
                let food = w.materials.id_of("provisions");
                let mut store = 0.0f64;
                for x in (inrun::NEST - 12).max(0)..=(inrun::NEST + 12).min(bw - 1) {
                    for y in surf[x as usize]..bh {
                        let c = w.get(x, y);
                        if c.organism_id() == 0 && Some(c.material) == food {
                            store += w.materials.get(c.material).food_energy as f64;
                        }
                    }
                }
                let free = (banks + store - debt) / heads.max(1.0) / start_e;
                if free < r {
                    lay_at *= (r / free.max(r / 64.0)).powi(2);
                }
            }
            let mut def = w.species.get(sp).creature.clone().expect("creature");
            if let Some(b) = def.brood.as_mut() {
                if (b.lay_at as f64 - lay_at).abs() > 0.5 {
                    b.lay_at = lay_at as f32;
                    w.species.set_creature(sp, def);
                }
            }
            if let Some(bl) = brood_log.as_mut().filter(|_| fr % 1000 == 0) {
                let cs = &w.creature_stats;
                writeln!(bl, "{fr},{adults_n},{},{},{},{},{},{},{},{},{:.1},{:.1},{lay_at:.0}", stages[0], stages[1], stages[2], cs.eggs_laid, cs.births, cs.larvae_starved, cs.brood_lost, cs.hatches_denied, (cs.brood_nursed_j + cs.brood_crop_fed_j + cs.brood_shared_j) / 1000.0, cs.brood_ate_j / 1000.0).unwrap();
            }
        }
        // PP_HUNGRYPROBE=1: every 250 frames, each ant below a quarter grant -- where it is, what its crop holds, what
        // food touches its head, and the verb it last chose. Asks why colonies starve beside a nest full of crumbs.
        if let Some(hp) = hungry_probe.as_mut().filter(|_| fr % 250 == 0) {
            let probe_def = w.species.get(sp).creature.clone().expect("creature");
            let (prov_m, crumb_m) = (w.materials.id_of("provisions"), w.materials.id_of("crumbs"));
            for id in inrun::ants(&w, sp) {
                let Some(s) = w.organism(id) else { continue };
                let tank = s.energy as f64 / start_e;
                let Some(&(x, y)) = s.chain.first().filter(|_| tank < 0.25) else { continue };
                let home = (x - inrun::NEST).abs() <= 12 && y >= surf[x as usize];
                let crop = s.crop.map_or("-".to_string(), |c| format!("{}:{:.0}", w.materials.get(c.material).name, c.worth()));
                let (mut crumbs, mut prov, mut best) = (0, 0, 0f32);
                for (dx, dy) in [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)] {
                    let c = w.get(x + dx, y + dy);
                    let m = Some(c.material);
                    crumbs += (m == crumb_m) as u32;
                    prov += (m == prov_m) as u32;
                    if m == crumb_m || m == prov_m {
                        best = best.max(creature::food_value(&w, c));
                    }
                }
                let jaws = s.spoil.map_or("-", |p| if p.store { "store" } else { "pellet" });
                let gut = s.traits[organism::TRAIT_GUT_BIAS];
                let q = crumb_m.map_or(0.0, |m| creature::diet_quality(&w, m, gut));
                let mut offered = Vec::new();
                creature::options::build(&w, id, &probe_def, &mut offered);
                let picks = offered.iter().filter(|o| o.verb == creature::options::Verb::PickUp).count();
                let verb = creature::mind::last_verb_of(&w, id).map_or("-".to_string(), |v| format!("{v:?}"));
                writeln!(hp, "{fr},{id},{x},{y},{tank:.2},{},{crop},{crumbs},{prov},{best:.0},{jaws},{verb},{gut:.2},{q:.3},{picks}", home as u8).unwrap();
            }
        }
        if fr % 1000 == 0 {
            if let Some(gl) = garden_log.as_mut() {
                let (mut live, mut loose) = ([0u32; GARDEN_KINDS.len()], [0u32; GARDEN_KINDS.len()]);
                for y in 0..bh {
                    for x in 0..bw {
                        let c = w.get(x, y);
                        if let Some(k) = garden_ids.iter().position(|&m| m == Some(c.material)) {
                            if c.organism_id() != 0 { live[k] += 1 } else { loose[k] += 1 }
                        }
                    }
                }
                let (mut plants, mut seeds) = (0u32, 0u32);
                for id in w.live_organism_ids() {
                    let Some(st) = w.organism(id) else { continue };
                    if w.species.get(st.species).creature.is_some() {
                        continue;
                    }
                    if pixel_physics::lab::census::is_waiting_seed(&w, id, st) { seeds += 1 } else { plants += 1 }
                }
                let j = |v: &[u32]| v.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(",");
                writeln!(gl, "{fr},{plants},{seeds},{},{}", j(&live), j(&loose)).unwrap();
            }
            let (mut nest_f, mut door_f, mut field_f, mut nest_c) = (0u32, 0u32, 0u32, 0u32);
            let corpse_m = w.materials.id_of("corpse");
            // larder.csv: the nest's stored food split by what it is and what it is worth (kJ). Most of a packed
            // garden nest's "food" cells are crumbs -- part-eaten provisions put back, each worth only what is left
            // of it -- so a cell count is not a larder.
            let (prov_m, crumb_m) = (w.materials.id_of("provisions"), w.materials.id_of("crumbs"));
            let mut larder = [(0u32, 0f64); 3];
            for x in (inrun::NEST - 45).max(0)..=(inrun::NEST + 45).min(bw - 1) {
                let dx = (x - inrun::NEST).abs();
                let sy = surf[x as usize];
                for y in 0..bh {
                    let c = w.get(x, y);
                    if c.organism_id() != 0 || w.materials.get(c.material).food_energy <= 0.0 {
                        continue;
                    }
                    // Corpses are edible but are not stored food: at seed 1's die-off every "food" cell in the
                    // nest was a dead ant (or crumbs off one), none provisions.
                    if Some(c.material) == corpse_m {
                        if y >= sy && dx <= 12 {
                            nest_c += 1;
                        }
                        continue;
                    }
                    if y >= sy && dx <= 12 {
                        nest_f += 1;
                        let k = if Some(c.material) == prov_m { 0 } else if Some(c.material) == crumb_m { 1 } else { 2 };
                        larder[k].0 += 1;
                        larder[k].1 += creature::food_value(&w, c) as f64 / 1000.0;
                    } else if y < sy && dx <= 12 && sy - y <= 4 {
                        door_f += 1;
                    } else {
                        field_f += 1;
                    }
                }
            }
            let ants = inrun::ants(&w, sp);
            let tanks: Vec<f64> = ants.iter().filter_map(|&i| w.organism(i)).map(|s| s.energy as f64 / start_e).collect();
            let crop: u32 = ants.iter().filter_map(|&i| w.organism(i)).map(|s| s.crop.map_or(0, |c| c.cells as u32)).sum();
            let n = tanks.len().max(1) as f64;
            let (mn, mx) = tanks.iter().fold((f64::MAX, 0.0f64), |a, &t| (a.0.min(t), a.1.max(t)));
            writeln!(stock_log, "{fr},{},{nest_f},{door_f},{field_f},{crop},{:.2},{:.2},{:.2},{nest_c}", tanks.len(), tanks.iter().sum::<f64>() / n, if tanks.is_empty() { 0.0 } else { mn }, mx).unwrap();
            writeln!(larder_log, "{fr},{},{:.1},{},{:.1},{},{:.1}", larder[0].0, larder[0].1, larder[1].0, larder[1].1, larder[2].0, larder[2].1).unwrap();
            if store_cap > 0.0 {
                store_p = (1.0 - nest_f as f64 / (store_cap * tanks.len().max(1) as f64)).clamp(0.0, 1.0);
            }
        }
        // `trace=N` (read only): every N frames, one row per living ant, for tracing individuals through a die-off.
        if let Some(tl) = trace_log.as_mut().filter(|_| fr % trace == 0) {
            for id in inrun::ants(&w, sp) {
                if let Some(s) = w.organism(id) {
                    let (x, y) = s.chain.first().copied().unwrap_or((0, 0));
                    let crop = s.crop.map_or(0, |c| c.cells);
                    let (sa, sb) = (w.pheromone_at(pheromone::Channel::A, x, y), w.pheromone_at(pheromone::Channel::B, x, y));
                    writeln!(tl, "{fr},{id},{x},{y},{:.2},{crop},{},{},{},{},{sa},{sb}", s.energy as f64 / start_e, s.trip_cells, u8::from(s.spoil.is_some()), s.since_nest, s.still_ticks).unwrap();
                }
            }
            // The trail planes over the bed's middle: what the minds have laid, and how much of it stands.
            let laid = w.minds.as_ref().map_or([0.0; 3], |m| m.counts.laid);
            let (mut na, mut sa, mut nb, mut sb) = (0u32, 0u64, 0u32, 0u64);
            for x in (inrun::NEST - 100)..=(inrun::NEST + 100) {
                for y in 0..bh {
                    let (a, b) = (w.pheromone_at(pheromone::Channel::A, x, y), w.pheromone_at(pheromone::Channel::B, x, y));
                    na += u32::from(a > 0);
                    sa += u64::from(a);
                    nb += u32::from(b > 0);
                    sb += u64::from(b);
                }
            }
            writeln!(scent_log.as_mut().unwrap(), "{fr},{:.0},{:.0},{na},{sa},{nb},{sb}", laid[0], laid[1]).unwrap();
        }
        // SCRATCH (evolvable-pheromones lane, 2026-10-25): `phero_dump=N` -- every N frames, planes A and B over the
        // bed's middle 201 columns, all rows, as `phero_fNNNNNNN.u16` (A then B, little-endian, 201 x bh each), for
        // pictures of where each scent stands. Read only.
        if phero_dump > 0 && fr % phero_dump == 0 {
            let mut bytes = Vec::with_capacity(2 * 201 * bh as usize * 2);
            for ch in [pheromone::Channel::A, pheromone::Channel::B] {
                for y in 0..bh {
                    for x in (inrun::NEST - 100)..=(inrun::NEST + 100) {
                        bytes.extend_from_slice(&w.pheromone_at(ch, x, y).to_le_bytes());
                    }
                }
            }
            std::fs::write(out.join(format!("phero_f{fr:07}.u16")), bytes).unwrap();
        }
        // `film=N`: every N frames, the game's own renderer over the bed, as film/fNNNNNNN.png (for a video).
        if film > 0 && fr % film == 0 {
            let (fw, fh) = (pixel_physics::app::WIDTH, pixel_physics::app::HEIGHT);
            if film_buf.is_empty() {
                film_buf = vec![0u8; (fw * fh * 4) as usize];
                std::fs::create_dir_all(out.join("film")).unwrap();
            }
            film_r.draw(&w, &film_p, &pixel_physics::sim::fxhash::ChunkSet::default(), &mut film_buf, (fw, fh), true);
            // film_hw=H: the crop's half-width about the door (default 80; 128 is the whole bed, a garden's plants included).
            let hw = num("film_hw", 80) as i32;
            let (x0, y0) = ((inrun::NEST - hw).max(0) as u32, 30u32);
            let (cw, ch) = ((2 * hw as u32 + 1).min(fw - x0).min(bw as u32 - x0), (bh as u32).saturating_sub(y0).min(fh - y0));
            let mut img = image::RgbaImage::new(cw, ch);
            for y in 0..ch {
                for x in 0..cw {
                    let i = (((y0 + y) * fw + x0 + x) * 4) as usize;
                    img.put_pixel(x, y, image::Rgba([film_buf[i], film_buf[i + 1], film_buf[i + 2], 255]));
                }
            }
            img.save(out.join(format!("film/f{fr:07}.png"))).unwrap();
            // The same crop's materials (u16 LE ids, row-major) and which organism owns each cell, for hover readouts.
            if fr == film {
                let mut names = std::fs::File::create(out.join("film/materials.txt")).unwrap();
                for k in 0..w.materials.len() {
                    writeln!(names, "{k} {}", w.materials.get(pixel_physics::sim::material::MaterialId(k as u16)).name).unwrap();
                }
            }
            let mut mats = Vec::with_capacity((cw * ch * 2) as usize);
            let mut owners = String::new();
            for y in 0..ch {
                for x in 0..cw {
                    let c = w.get((x0 + x) as i32, (y0 + y) as i32);
                    mats.extend_from_slice(&c.material.0.to_le_bytes());
                    if c.organism_id() != 0 {
                        owners.push_str(&format!("{} {} ", y * cw + x, c.organism_id()));
                    }
                }
            }
            std::fs::write(out.join(format!("film/m{fr:07}.bin")), mats).unwrap();
            std::fs::write(out.join(format!("film/o{fr:07}.txt")), owners).unwrap();
        }
        if fr % window == 0 {
            let alive = inrun::ants(&w, sp).len();
            println!("frame {fr:7}  alive {alive:3}  {:.1}s", t0.elapsed().as_secs_f64());
            let mut pf = std::fs::File::create(out.join(format!("ants_f{fr:07}.txt"))).unwrap();
            for id in inrun::ants(&w, sp) {
                if let Some(st) = w.organism(id) {
                    let (x, y) = st.chain.first().copied().unwrap_or((0, 0));
                    // Columns 6-7: the ledger's credit so far and the founder line (for reliability by record length).
                    let (cr, fam) = book.get(&id).map_or((0.0, "-"), |b| (b.credit, b.family.as_str()));
                    writeln!(pf, "{id} {x} {y} {:.1} {} {cr:.3} {fam}", st.energy, u8::from(st.spoil.is_some())).unwrap();
                }
            }
            if maps {
                let (x0, x1, y0, y1) = (inrun::NEST - 100, inrun::NEST + 100, 40, bh - 1);
                let mut m = std::fs::File::create(out.join(format!("map_f{fr:07}.txt"))).unwrap();
                writeln!(m, "{x0} {y0} {} {}", x1 - x0 + 1, y1 - y0 + 1).unwrap();
                for y in y0..=y1 {
                    let row: String = (x0..=x1).map(|x| inrun::cell_char(&w, x, y)).collect();
                    writeln!(m, "{row}").unwrap();
                }
            }
        }
    }
    ev.flush().unwrap();
    if let Some(tl) = trace_log.as_mut() {
        tl.flush().unwrap();
    }
    if let Some(sl) = scent_log.as_mut() {
        sl.flush().unwrap();
    }
    if let Some(gl) = garden_log.as_mut() {
        gl.flush().unwrap();
    }
    if sel != "off" {
        sel_log.flush().unwrap();
        births.flush().unwrap();
        let dir = out.join("final");
        std::fs::create_dir_all(&dir).unwrap();
        let mut f = std::fs::File::create(dir.join("index.csv")).unwrap();
        writeln!(f, "id,family,gen,born,kids,score").unwrap();
        let frame = w.frame - f0;
        for (id, b) in &book {
            train::save_genome(&dir.join(format!("{id}.raw.txt")), &b.genome).unwrap();
            writeln!(f, "{id},{},{},{},{},{:.3}", b.family, b.gen, b.born, b.kids, b.credit / ((frame - b.born) as f64 / 1000.0).max(0.1)).unwrap();
        }
    }
    if sel != "off" {
        println!("selection: hungry departures paid {hungry_paid}, quota due {q_due}, no eligible parent {q_noparent}, placements failed {birth_fail}; parents {} of which plain score agrees {}", zs_pick.0, zs_pick.1);
        age30.flush().unwrap();
    }
    write_nestmap(&w, &out.join("nestmap.txt"));
    write_occupants(&w, &out.join("occupants.txt"));
    creature::mind::scratch_log::copy_flush();
    println!("{}", creature::mind::scratch_log::carry_report());
    if creature::mind::scratch_log::oldlab_on() {
        eprintln!("{}", creature::mind::scratch_log::oldlab_report());
    }
    if weed > 0 {
        println!("weed: {weeded} plant cells pulled within {weed} columns of the door");
    }
    if let (Some(s), Some(d)) = (spiders.as_ref(), arg(&a, "spider_out")) {
        let total = match arg(&a, "spider_score") {
            // total by default: the owner's pick, 2026-10-24 ("Total food eaten in its life").
            Some("life") => false,
            None | Some("total") => true,
            Some(o) => panic!("spider_score={o}: life or total"),
        };
        s.save(std::path::Path::new(d), w.frame - f0, total);
    }
    if let Some(s) = spiders.as_ref() {
        let k = w.minds.as_ref().map_or(Default::default(), |m| m.counts);
        println!(
            "spiders: {} placed, {} died ({} killed by ants, {} starved), {} spider cells bitten off; ants killed {}, bites {} (all minds), flesh swallowed {}",
            s.placed, s.deaths, s.killed, s.starved, s.wounds, s.ants_killed, k.bites, k.flesh
        );
    }
    // The bite and the alarm with or without spiders (`options::attack_on`, on by default since 2026-10-24): whether
    // either fired at all on a bed where no stranger lives (Teacher lane's question before the default landed).
    let k = w.minds.as_ref().map_or(Default::default(), |m| m.counts);
    println!("mind acts: bites {} kills {} flesh {} | scent laid A {:.0} B {:.0} alarm {:.1}", k.bites, k.kills, k.flesh, k.laid[0], k.laid[1], k.laid[2]);
    println!("done: {frames} frames in {:.1}s ({:.0} frames/s)", t0.elapsed().as_secs_f64(), frames as f64 / t0.elapsed().as_secs_f64());
    println!("home_eat_forced {}", creature::options::HOME_EAT_FORCED.load(std::sync::atomic::Ordering::Relaxed));
}
