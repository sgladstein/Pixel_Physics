//! **Does a tunnel dug in soil survive?** — the measurement behind the
//! evolution lab's digging question, and a direct check on a claim
//! `wiki/ants.md` makes today: *"Turn a colony loose on a soil bank and it
//! hollows it out, leaving the stone beneath untouched."*
//!
//! `Reports/evolution-lab-design-guide-2026-08-30.md` §2b records the owner's
//! decision to decline collapsing tunnels, and reads the cost of that as the
//! structural scheduler's 16%. It then says a repose angle is harmless —
//! *"a dug wall that slumps a little is available and free; a roof that falls
//! in is what was declined."* **That is the thing this harness tests**, and
//! it is testable because the two mechanisms are separable: the structural
//! scheduler is not what closes a hole in a powder. `update_powder`'s
//! straight-down rule is, and it runs in the CA sweep whether or not any
//! structural code is linked.
//!
//! Three arms, each a bed with the same excavation cut into it:
//!
//! | arm | bed |
//! |---|---|
//! | `soil` | the lab's own bed — `soil`, a `Powder` |
//! | `sand` | `sand`, the loosest shipped powder — the *negative* control, expected worse |
//! | `stone` | `stone`, a `Solid` — the **positive control** |
//! | `lined` | `soil`, with the excavation's wall worked into `packedsoil` — what an ant now leaves behind |
//! | `flooded` | `lined`, with the shaft filled with water — the wall wets from the inside |
//! | `watertable` | `lined`, dug into a bank already at `SOIL_SATURATED` — the wall wets from the outside |
//!
//! **The last three exist to keep the mechanic from being a binary.**
//! `CLAUDE.md`'s first law is that an outcome is a distribution, not a
//! switch, so a lining that could never fail would be the same defect as a
//! tunnel that always does. `packedsoil` reverts to `soil` above
//! `material::SOIL_FIELD_CAPACITY`, and these two arms are the wet halves
//! of that: one soaks the wall from the void, one from the bank.
//!
//! **Two columns per void, and the second one is why the wet arms are
//! readable at all.** `open` is *materially empty*, which a flooded shaft
//! is not — so on `flooded` the shaft reads 0% open on frame 0, before a
//! single tick, purely because there is water standing in it. That is
//! `CLAUDE.md`'s "ask what your number counts when nothing is wrong"
//! exactly. `caved` counts the void's cells that now hold **ground** (a
//! `Powder` or a `Solid`), which is the thing actually being claimed, and
//! it stays 0 for water. Read `caved`, not `open`, on any arm with liquid
//! in it.
//!
//! **The positive control is the point.** `CLAUDE.md`: *a null looks the same
//! whether the mechanism is quiet or the probe never reached it*, and *run the
//! positive control — construct the case whose answer you know is non-zero and
//! check the instrument reports it.* A tunnel in stone must read 100%
//! surviving at every frame, or this harness is measuring its own scene
//! construction and not the physics.
//!
//! The excavation is what an ant would actually dig, not an abstract cavity:
//! a vertical shaft from the surface, a horizontal gallery off it, and a
//! chamber at the end. Each is censused separately, because they fail for
//! different reasons and a single pooled number would hide that — a shaft is
//! a vertical face (the repose rule), a gallery has a roof (the straight-down
//! rule), and a chamber is both with a longer span.
//!
//! # The `colony` arm — the one that is not a hand-carved cavity
//!
//! Every arm above answers *does a lined tunnel stand*. It cannot answer *do
//! ants produce one*, and those need separating: a hand-written lining is a
//! claim about `update_powder`, and a colony is a claim about
//! `creature::line_burrow` and about whether digging reaches soil at all.
//! `arms=colony` puts 55 ants on a soil bank over stone and censuses the
//! **standing void inside the bank** — the quantity a player would call a
//! nest — with `digs` and `packed` printed beside it every time, because a
//! bank with no holes in it and a colony that never dug are the same picture
//! (`CLAUDE.md`: "did it fire at all" needs a counter, not a picture).
//!
//! **Its baseline is the same binary with the lining switched off**, which is
//! what `PIXEL_PHYSICS_BURROW_LINING=off` is for. A standing quantity has no
//! baseline of its own; run both and read the pair:
//!
//! ```text
//! cargo run --release --example burrow_probe -- arms=colony seeds=4
//! PIXEL_PHYSICS_BURROW_LINING=off cargo run --release --example burrow_probe -- arms=colony seeds=4
//! ```
//!
//! ```text
//! cargo run --release --example burrow_probe
//! cargo run --release --example burrow_probe -- frames=3600 seeds=4
//! cargo run --release --example burrow_probe -- arms=soil width=256
//! ```

mod common;

use common::PlantScene;
use pixel_physics::sim::explosion::Blasts;
use pixel_physics::sim::particle::ParticleSystem;
use pixel_physics::sim::weather::Pin;
use pixel_physics::sim::material::MaterialKind;
use pixel_physics::sim::{frame, material, player, Cell, World};

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| {
        a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses"))
    })
}

/// One dug void, censused on its own. `cells` is what was carved; the census
/// counts how many of them are still materially empty.
struct Void {
    name: &'static str,
    cells: Vec<(i32, i32)>,
}

impl Void {
    /// **Raw material equality, not `Cell::is_empty`.** `is_empty` is
    /// managed-aware and answers "is this position available", which is a
    /// different question from "is there material here" (`CLAUDE.md`'s
    /// gotcha). A tunnel refilled with soil must not read as empty.
    fn open(&self, world: &World) -> usize {
        self.cells.iter().filter(|(x, y)| world.get(*x, *y).material == material::EMPTY).count()
    }

    /// How many of the carved cells have **ground** standing in them --
    /// anything the sweep treats as a `Powder` or a `Solid`.
    ///
    /// `open` alone cannot carry the wet arms. A shaft full of water is not
    /// materially empty, so it reads 0% open on frame 0 with nothing having
    /// happened; a shaft whose roof fell in reads 0% open too, and those are
    /// the opposite finding. This column separates them, and it is the one
    /// to quote whenever there is liquid in the scene.
    fn caved(&self, world: &World) -> usize {
        self.cells
            .iter()
            .filter(|(x, y)| {
                matches!(
                    world.materials.kind(world.get(*x, *y).material),
                    MaterialKind::Powder | MaterialKind::Solid
                )
            })
            .count()
    }
}


/// **Is the standing void one gallery, or a scatter of bites?** — a
/// connected-component pass over the empty cells in the bank footprint.
///
/// `roofed` says *how much* enclosed void there is and cannot distinguish
/// 130 cells of tunnel from 130 separate nibbles at an open face. Those are
/// the same number and opposite findings — `CLAUDE.md`'s *a mean over events
/// is not a mean over the thing you care about*, one level up: a gallery is a
/// long connected run, quarrying is a scatter of singletons.
///
/// **Eight-connected, because the digger is.** `creature.rs`'s dig steps the
/// cell in the ant's `heading`, and headings are the eight compass
/// directions, so two diagonally-touching void cells are one passage to the
/// animal that made them. `CLAUDE.md`: *a traversal must use the same
/// neighbourhood the writer used* — a four-neighbour pass here would report a
/// diagonal tunnel as a row of singletons and manufacture the very finding
/// this measurement exists to test for.
///
/// Each component carries its **roofed** count as well as its size, because
/// the two answer different halves. The quarried corner of a bank is one
/// enormous component with almost no roof over it; a gallery is a smaller
/// component that is nearly all roof. Reading size alone would score the
/// erosion case as the best tunneller in the run.
///
/// Returned largest-first.
/// **Ground that is not attached to anything standing on the floor** — the
/// count behind *"there is dirt floating in the sky"*.
///
/// Returns (pieces, cells).
///
/// **A flood fill, because the obvious census answers a different question.**
/// Counting ground cells with nothing directly beneath them reads **49-62**
/// on the shipped behaviour and 94-100 with spoil hauling, and almost none of
/// either is visible: the roof of a gallery is unsupported by definition, and
/// so is every cell of an arch. What a player sees as floating is a *piece*
/// with no path down to the ground at all, and only a connected-component
/// pass can tell the two apart. `CLAUDE.md`'s "ask what your number counts
/// when nothing is wrong" -- the per-cell version counts working tunnels.
///
/// 8-connected, the neighbourhood the digger and the sweep both use. A piece
/// is grounded when it reaches the bottom row, which in every scene here is
/// bedrock or the stone floor.
fn floating_ground(world: &World, w: i32, h: i32) -> (usize, usize) {
    let idx = |x: i32, y: i32| (y as usize) * (w as usize) + (x as usize);
    let ground: Vec<bool> = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .map(|(x, y)| matches!(world.materials.kind(world.get(x, y).material), MaterialKind::Powder | MaterialKind::Solid))
        .collect();

    let mut seen = vec![false; ground.len()];
    let mut stack: Vec<(i32, i32)> = Vec::new();
    let (mut pieces, mut cells) = (0usize, 0usize);
    for sy in 0..h {
        for sx in 0..w {
            if !ground[idx(sx, sy)] || seen[idx(sx, sy)] {
                continue;
            }
            seen[idx(sx, sy)] = true;
            stack.push((sx, sy));
            let (mut size, mut grounded) = (0usize, false);
            while let Some((cx, cy)) = stack.pop() {
                size += 1;
                grounded |= cy == h - 1;
                for (dx, dy) in [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)] {
                    let (nx, ny) = (cx + dx, cy + dy);
                    if nx < 0 || nx >= w || ny < 0 || ny >= h || seen[idx(nx, ny)] || !ground[idx(nx, ny)] {
                        continue;
                    }
                    seen[idx(nx, ny)] = true;
                    stack.push((nx, ny));
                }
            }
            if !grounded {
                pieces += 1;
                cells += size;
            }
        }
    }
    (pieces, cells)
}

fn void_components(
    world: &World,
    (x0, x1): (i32, i32),
    (y0, y1): (i32, i32),
) -> Vec<Component> {
    let (w, h) = ((x1 - x0) as usize, (y1 - y0) as usize);
    let idx = |x: i32, y: i32| (y - y0) as usize * w + (x - x0) as usize;

    // Roofed is a per-column prefix over the *whole* column, not just the
    // footprint: a cell is roofed when ground stands above it anywhere,
    // including the bank's own untouched cap above `y0`.
    let mut empty = vec![false; w * h];
    let mut roofed = vec![false; w * h];
    for x in x0..x1 {
        let mut above = 0usize;
        for y in 0..y1 {
            let m = world.get(x, y).material;
            if y >= y0 && m == material::EMPTY {
                empty[idx(x, y)] = true;
                roofed[idx(x, y)] = above > 0;
            }
            if matches!(world.materials.kind(m), MaterialKind::Powder | MaterialKind::Solid) {
                above += 1;
            }
        }
    }

    let mut seen = vec![false; w * h];
    let mut out = Vec::new();
    let mut stack: Vec<(i32, i32)> = Vec::new();
    for sy in y0..y1 {
        for sx in x0..x1 {
            if !empty[idx(sx, sy)] || seen[idx(sx, sy)] {
                continue;
            }
            seen[idx(sx, sy)] = true;
            stack.push((sx, sy));
            let mut comp = Component::default();
            while let Some((cx, cy)) = stack.pop() {
                comp.cells += 1;
                if roofed[idx(cx, cy)] {
                    comp.roofed += 1;
                }
                for (dx, dy) in [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)] {
                    let (nx, ny) = (cx + dx, cy + dy);
                    if nx < x0 || nx >= x1 || ny < y0 || ny >= y1 {
                        continue;
                    }
                    if empty[idx(nx, ny)] && !seen[idx(nx, ny)] {
                        seen[idx(nx, ny)] = true;
                        stack.push((nx, ny));
                    }
                }
            }
            out.push(comp);
        }
    }
    out.sort_unstable_by(|a, b| b.cells.cmp(&a.cells).then(b.roofed.cmp(&a.roofed)));
    out
}

/// One connected run of standing void: how big it is, and how much of it has
/// ground overhead.
#[derive(Default, Clone, Copy)]
struct Component {
    cells: usize,
    roofed: usize,
}

/// **What shape is the cavity?** — the columns `roofed` structurally cannot
/// carry, and the reason this arm was extended.
///
/// `roofed` is a **volume**. Toffin et al. (*PNAS* 2009), which is the
/// mechanism `(Crowding, Dig, w)` was built from, is entirely about **shape**: a
/// colony digs a round cavity first, and the cavity sprouts tunnels once it
/// outgrows the workers digging it. A round chamber and a ramified warren of
/// the same size are the same `roofed` count and the opposite finding —
/// `larder_probe`'s lesson in a new costume, and the same trap the
/// `void`-versus-`roofed` repair already caught once in this very file.
///
/// Two numbers, because the transition has two halves and neither substitutes
/// for the other — a shape can stretch without budding (an oval) and can bud
/// without stretching much (a disc with three short spurs):
///
/// * **`circ`** — the isoperimetric quotient `4*pi*A / P^2` over the
///   4-connected boundary. This is the transition itself: round-to-ramified
///   is a fall in it. **It measures compactness, not circleness**, and the
///   difference is not pedantry: the perimeter is a count of grid faces, so a
///   curve is a staircase and pays for every step, and a digital disc reads
///   **0.572** against an axis-aligned square's **0.785**. Both sit far above
///   a gallery (0.134) and a five-toothed comb (0.057), which is the
///   separation the column is for.
/// * **`buds`** — how many distinct runs of the cavity stand further out than
///   `BUD_K` times its own inscribed radius. This is the count Toffin reports
///   rising. **Its domain is "a cavity with things coming off it"**, which is
///   the phrase §14e asks for and is also its limit: no radial cut from one
///   centre can separate the teeth of a comb, which are joined along their
///   roots. A five-toothed comb reads **3**, not 5. A shape with no chamber in
///   it is read on `circ` instead — `inradius` beside it is what says which case you
///   are looking at.
///
/// **A grid is not a plane, so neither number's ceiling is the textbook one.**
/// A digital disc does *not* read `circ` 1.0. `arms=selftest` puts synthetic
/// shapes through this exact function and prints what each reads, so the table
/// is read against measured references rather than against a remembered
/// constant — and it is the positive control `CLAUDE.md` demands before any of
/// these numbers is quoted about a colony.
#[derive(Default, Clone, Copy)]
struct Shape {
    cells: usize,
    perimeter: usize,
    circularity: f64,
    /// Radius of the largest disc that fits inside the cavity, in cells. The
    /// scale-free reading of "how big is the chamber", and the column the
    /// colony-size control is read on: a cavity that grows with the colony
    /// grows *this*, where `roofed` also grows when ants merely dig more
    /// tunnel.
    inradius: f64,
    buds: usize,
}

/// How far past the inscribed disc a cell has to stand before it counts as a
/// protrusion rather than as the cavity's own rim.
///
/// **A square's corners are the constraint, and they set this number.** For a
/// square of side `2a+1` the inscribed radius is `a+1` and a corner stands at
/// `a*sqrt(2)`, so the ratio climbs toward `sqrt(2)` from below: at or under
/// 1.414 a plain square reads as four buds. 1.6 clears that with headroom and
/// still catches a spur about a third of the cavity's radius long. Set from
/// the geometry rather than from a run, and `arms=selftest` is what says
/// whether it separates the shapes it has to separate. `budk=` overrides it.
const BUD_K: f64 = 1.6;

/// The smallest protrusion that counts, in cells. Eight, matching the `ge8`
/// column beside it: below that a bud is one ant's single bite.
const MIN_BUD: usize = 8;

/// Shape statistics for one connected void, over a boolean mask.
///
/// Takes a mask rather than a `World` **so the selftest can hand it a shape
/// whose answer is known**. A metric that can only be run on the thing it is
/// measuring cannot be checked against a case where nothing is wrong, which is
/// the failure this repo has hit six times.
fn shape_of(mask: &[bool], w: i32, h: i32, bud_k: f64) -> Shape {
    let idx = |x: i32, y: i32| (y as usize) * (w as usize) + (x as usize);
    let inside = |x: i32, y: i32| x >= 0 && x < w && y >= 0 && y < h && mask[idx(x, y)];
    let cells: Vec<(i32, i32)> =
        (0..h).flat_map(|y| (0..w).map(move |x| (x, y))).filter(|&(x, y)| mask[idx(x, y)]).collect();
    if cells.is_empty() {
        return Shape::default();
    }

    // **Perimeter is 4-connected, where the components are 8-connected**, and
    // the mismatch is deliberate rather than an oversight. Connectivity asks
    // "can the digger walk from here to there", and the digger steps the eight
    // compass headings. Perimeter asks "how much wall does this cavity have",
    // and a wall is a shared *face*. A diagonal contact is not a face.
    //
    // Out of bounds counts as not-cavity, so a cavity reaching the edge of the
    // census window is bounded there -- the honest reading for a bank
    // footprint whose edges are more bank.
    let mut perimeter = 0usize;
    for &(x, y) in &cells {
        for (dx, dy) in [(0, -1), (0, 1), (-1, 0), (1, 0)] {
            if !inside(x + dx, y + dy) {
                perimeter += 1;
            }
        }
    }

    // **The nearest non-cavity cell is always on the rim**, so the inner
    // distance transform is exact when brute-forced over the rim alone: walk
    // the straight line from any cavity cell toward any non-cavity cell and the
    // first non-cavity cell it meets is 4-adjacent to a cavity one. That turns
    // an O(cells^2) census into O(cells * rim) with no chamfer approximation to
    // explain away, which matters because the number it produces is quoted.
    let (rw, rh) = (w + 2, h + 2);
    let mut on_rim = vec![false; (rw * rh) as usize];
    let mut rim: Vec<(i32, i32)> = Vec::new();
    for &(x, y) in &cells {
        for (dx, dy) in [(0, -1), (0, 1), (-1, 0), (1, 0)] {
            let (nx, ny) = (x + dx, y + dy);
            if inside(nx, ny) {
                continue;
            }
            let slot = ((ny + 1) * rw + (nx + 1)) as usize;
            if !on_rim[slot] {
                on_rim[slot] = true;
                rim.push((nx, ny));
            }
        }
    }

    // The largest inscribed disc, and where it sits.
    let dist2_to_rim = |x: i32, y: i32| -> f64 {
        rim.iter()
            .map(|&(rx, ry)| {
                let (dx, dy) = ((rx - x) as f64, (ry - y) as f64);
                dx * dx + dy * dy
            })
            .fold(f64::INFINITY, f64::min)
    };
    let mut best = 0.0f64;
    for &(x, y) in &cells {
        best = best.max(dist2_to_rim(x, y));
    }
    let inradius = best.sqrt();

    // **The tie is not rare, it is the normal case, and taking the first
    // maximum in scan order got a known shape wrong.** Every cell along the
    // middle of a uniform gallery is equally far from the wall, so a
    // first-wins rule puts the "centre" of a 64-cell bar at its left *end* --
    // and the bud count then reads 1 where the shape plainly has two ends.
    // Caught by `arms=selftest`, which is what it is for.
    //
    // So: the centre is the cell of the widest set that lies nearest that
    // set's own centroid. Deterministic, geometrically meaningful ("the middle
    // of the widest part"), and it needs no sort -- which also keeps it clear
    // of `CLAUDE.md`'s unstable-sort tie-order gotcha.
    let widest: Vec<(i32, i32)> = cells.iter().copied().filter(|&(x, y)| dist2_to_rim(x, y) >= best - 1e-9).collect();
    let n = widest.len().max(1) as f64;
    let (cx, cy) = (
        widest.iter().map(|&(x, _)| x as f64).sum::<f64>() / n,
        widest.iter().map(|&(_, y)| y as f64).sum::<f64>() / n,
    );
    let mut centre = widest.first().copied().unwrap_or(cells[0]);
    let mut nearest = f64::INFINITY;
    for &(x, y) in &widest {
        let d = (x as f64 - cx).powi(2) + (y as f64 - cy).powi(2);
        if d < nearest {
            nearest = d;
            centre = (x, y);
        }
    }

    // Everything standing further out than `bud_k * inradius`, split into
    // 8-connected runs -- the same neighbourhood the digger uses, so a
    // diagonal tunnel is one bud rather than a row of singletons.
    let threshold2 = (bud_k * inradius) * (bud_k * inradius);
    let far = |x: i32, y: i32| -> bool {
        if !inside(x, y) {
            return false;
        }
        let (dx, dy) = ((x - centre.0) as f64, (y - centre.1) as f64);
        dx * dx + dy * dy > threshold2
    };
    let mut seen = vec![false; (w * h) as usize];
    let mut stack: Vec<(i32, i32)> = Vec::new();
    let mut buds = 0usize;
    for &(sx, sy) in &cells {
        if seen[idx(sx, sy)] || !far(sx, sy) {
            continue;
        }
        seen[idx(sx, sy)] = true;
        stack.push((sx, sy));
        let mut size = 0usize;
        while let Some((cx, cy)) = stack.pop() {
            size += 1;
            for (dx, dy) in [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)] {
                let (nx, ny) = (cx + dx, cy + dy);
                if !far(nx, ny) || seen[idx(nx, ny)] {
                    continue;
                }
                seen[idx(nx, ny)] = true;
                stack.push((nx, ny));
            }
        }
        if size >= MIN_BUD {
            buds += 1;
        }
    }

    let (a, p) = (cells.len() as f64, perimeter as f64);
    Shape {
        cells: cells.len(),
        perimeter,
        circularity: if p > 0.0 { 4.0 * std::f64::consts::PI * a / (p * p) } else { 0.0 },
        inradius,
        buds,
    }
}

/// The shape of the **largest roofed-void run** inside `(x0..x1, y0..y1)`.
///
/// **Roofed, not all standing void, and that filter is the whole reason this
/// number can be trusted.** `dead-ends.md` records this arm scoring a build
/// that leaves no roof at all at 788 against 472 for one that builds galleries
/// -- exactly backwards -- because a quarried open face is standing void too.
/// A quarry is also, geometrically, one enormous ragged component, so handing
/// it to `shape_of` would report the *bank's eroded face* as the colony's
/// chamber. Roofed void is the half erosion cannot manufacture, and it is
/// what a player would call a room.
fn roofed_void_shape(world: &World, (x0, x1): (i32, i32), (y0, y1): (i32, i32), bud_k: f64) -> Shape {
    let (w, h) = (x1 - x0, y1 - y0);
    let idx = |x: i32, y: i32| ((y - y0) * w + (x - x0)) as usize;
    let mut mask = vec![false; (w * h) as usize];
    for x in x0..x1 {
        let mut above = 0usize;
        for y in 0..y1 {
            let m = world.get(x, y).material;
            if y >= y0 && m == material::EMPTY && above > 0 {
                mask[idx(x, y)] = true;
            }
            if matches!(world.materials.kind(m), MaterialKind::Powder | MaterialKind::Solid) {
                above += 1;
            }
        }
    }

    // Largest 8-connected run only. The colony digs several holes; the
    // question Toffin asks is about the shape of *a cavity*, and pooling
    // every pocket in the bank into one figure would average a chamber
    // together with the nibbles around it.
    let mut seen = vec![false; mask.len()];
    let mut stack: Vec<(i32, i32)> = Vec::new();
    let mut best: Vec<(i32, i32)> = Vec::new();
    for sy in y0..y1 {
        for sx in x0..x1 {
            if !mask[idx(sx, sy)] || seen[idx(sx, sy)] {
                continue;
            }
            seen[idx(sx, sy)] = true;
            stack.push((sx, sy));
            let mut run: Vec<(i32, i32)> = Vec::new();
            while let Some((cx, cy)) = stack.pop() {
                run.push((cx, cy));
                for (dx, dy) in [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)] {
                    let (nx, ny) = (cx + dx, cy + dy);
                    if nx < x0 || nx >= x1 || ny < y0 || ny >= y1 || seen[idx(nx, ny)] || !mask[idx(nx, ny)] {
                        continue;
                    }
                    seen[idx(nx, ny)] = true;
                    stack.push((nx, ny));
                }
            }
            if run.len() > best.len() {
                best = run;
            }
        }
    }

    let mut only = vec![false; mask.len()];
    for &(x, y) in &best {
        only[idx(x, y)] = true;
    }
    shape_of(&only, w, h, bud_k)
}

/// **The instrument's positive control**, and it runs in the same binary.
///
/// `CLAUDE.md`: *construct the case whose answer you know is non-zero and check
/// the instrument reports it* — and the other half, *put the fault back and
/// watch it go red*. Six synthetic shapes with known answers go through the
/// same `shape_of` the colony table calls, and every claim the table makes
/// about a number is asserted here:
///
/// * a disc and a square must read **0 buds** — the false-positive half, and
///   the one `BUD_K` is set from;
/// * a disc with three tunnels must read **3** and a plain gallery **2** — the
///   sensitivity half, without which a `buds 0` in the colony table means
///   nothing at all;
/// * a compact shape must out-read a ramified one on `circ` by a wide margin —
///   the transition the column exists to see.
///
/// **It has already earned its keep twice.** Written first with a first-wins
/// tie-break on the inscribed-disc centre, it put the centre of a uniform
/// gallery at the gallery's *end* and counted one bud where there are two; and
/// it disproved this function's own doc comment, which claimed `circ` ranks a
/// disc above a square. It does not, and the reason is in the assertions.
///
/// It also **prints** what each shape reads, because the digital ceiling is
/// not the textbook one and the table's reader needs the reference values
/// rather than a remembered `1.0`.
fn selftest_arm(bud_k: f64) {
    let (w, h) = (81i32, 81i32);
    let idx = |x: i32, y: i32| (y * w + x) as usize;
    let disc = |r: f64| -> Vec<bool> {
        let mut m = vec![false; (w * h) as usize];
        for y in 0..h {
            for x in 0..w {
                let (dx, dy) = ((x - 40) as f64, (y - 40) as f64);
                if (dx * dx + dy * dy).sqrt() <= r {
                    m[idx(x, y)] = true;
                }
            }
        }
        m
    };

    let mut shapes: Vec<(&str, Vec<bool>)> = Vec::new();
    shapes.push(("disc r=14", disc(14.0)));

    let mut square = vec![false; (w * h) as usize];
    for y in 26..=54 {
        for x in 26..=54 {
            square[idx(x, y)] = true;
        }
    }
    shapes.push(("square 29x29", square));

    // A chamber with three tunnels off it -- Toffin's second phase, built by
    // hand so the count it must report is known rather than inferred.
    let mut spurs = disc(12.0);
    for t in 0..18 {
        for k in -1..=1 {
            spurs[idx((40 + 12 + t).min(w - 1), (40 + k).clamp(0, h - 1))] = true;
            spurs[idx((40 + k).clamp(0, w - 1), (40 + 12 + t).min(h - 1))] = true;
            spurs[idx((40 - 12 - t).max(0), (40 + k).clamp(0, h - 1))] = true;
        }
    }
    shapes.push(("disc + 3 tunnels", spurs));

    // Five teeth off a spine: fully ramified, no chamber at all.
    let mut comb = vec![false; (w * h) as usize];
    for x in 10..70 {
        for y in 39..=41 {
            comb[idx(x, y)] = true;
        }
    }
    for i in 0..5 {
        let x = 14 + i * 12;
        for y in 42..62 {
            for k in 0..3 {
                comb[idx(x + k, y)] = true;
            }
        }
    }
    shapes.push(("comb, 5 teeth (no chamber)", comb));

    // A plain gallery: no chamber, and both ends stand off the tiny inscribed
    // disc, so it reads two buds. Recorded rather than tuned away -- it is the
    // honest answer for a shape that is all tunnel.
    let mut bar = vec![false; (w * h) as usize];
    for x in 8..72 {
        for y in 39..=41 {
            bar[idx(x, y)] = true;
        }
    }
    shapes.push(("bar 64x3 (no chamber)", bar));

    shapes.push(("empty", vec![false; (w * h) as usize]));

    println!("=== arm selftest ===  the shape columns, on shapes whose answer is known");
    println!("  `circ` ceiling on a grid is not the textbook 1.0 -- these are the reference values.");
    println!("{:>26}  {:>7}  {:>9}  {:>6}  {:>10}  {:>5}", "shape", "cells", "perimeter", "circ", "inradius", "buds");
    let mut read: Vec<(&str, Shape)> = Vec::new();
    for (name, mask) in &shapes {
        let s = shape_of(mask, w, h, bud_k);
        println!("{name:>26}  {:>7}  {:>9}  {:>6.3}  {:>10.2}  {:>5}", s.cells, s.perimeter, s.circularity, s.inradius, s.buds);
        read.push((name, s));
    }
    let get = |name: &str| read.iter().find(|(n, _)| *n == name).map(|(_, s)| *s).expect("shape");

    let (disc, square) = (get("disc r=14"), get("square 29x29"));
    let (spurs, comb, bar) = (get("disc + 3 tunnels"), get("comb, 5 teeth (no chamber)"), get("bar 64x3 (no chamber)"));

    // The false-positive half: a compact shape has nothing off it, and neither
    // its staircase rim nor its corners may be counted as one.
    assert_eq!(disc.buds, 0, "a plain disc is a chamber with nothing off it; BUD_K must not manufacture buds from its own rim");
    assert_eq!(square.buds, 0, "a square's four corners are what BUD_K is set to clear");
    // The sensitivity half, and without it a `buds 0` in the colony table
    // means nothing at all (`CLAUDE.md`: a null looks the same whether the
    // mechanism is quiet or the probe never reached it).
    assert_eq!(spurs.buds, 3, "three tunnels were drawn off one cavity and three must be counted -- this is the regime the column is for");
    assert_eq!(bar.buds, 2, "a uniform gallery has two ends and both stand off its own inscribed disc");

    // **The stated blind spot, pinned rather than hidden.** "Protrusions off
    // the main cavity" is undefined when there is no main cavity: a comb is
    // five teeth on a spine, every part of it is three cells wide, and no
    // radial cut from one centre can separate teeth that are joined along
    // their roots. It reads 2, not 5. That is not a bug to tune out -- it is
    // the definition's domain, and the column that reads this shape is `circ`
    // (0.057 here, the lowest in the table, against a disc's 0.572).
    // Asserted so the behaviour is pinned and a later reader is not left
    // guessing whether 2 was intended.
    assert_eq!(comb.buds, 3, "a comb has no main cavity, so the bud count is out of its definition's domain -- five teeth read 3, and `circ` is the column for this shape");
    assert!(comb.inradius < 3.0, "and the column that says so is `inradius`: a shape nowhere wider than a gallery has no chamber in it");

    // `circ` separates compact from ramified, which is the transition. It does
    // **not** rank a disc above a square, and that is a property of the grid
    // rather than a defect: the perimeter is a count of 4-connected faces, so
    // a curve is a staircase and pays for every step. A digital disc reads
    // 0.572 against an axis-aligned square's 0.785. The column is
    // *compactness*; the textbook 1.0 for a circle is unreachable here and
    // must not be quoted. Asserted in the direction it really goes, so that
    // anyone who "fixes" this ordering finds out why it is like this.
    assert!(
        square.circularity > disc.circularity,
        "a staircase perimeter costs a curve more than a straight edge, and the reference values depend on it: square {:.3} must exceed disc {:.3}",
        square.circularity,
        disc.circularity
    );
    assert!(
        disc.circularity > bar.circularity * 3.0 && disc.circularity > comb.circularity * 3.0,
        "compact must read far above ramified or the column cannot see the transition: disc {:.3} against bar {:.3} and comb {:.3}",
        disc.circularity,
        bar.circularity,
        comb.circularity
    );
    assert!(
        (disc.inradius - 14.0).abs() < 1.0,
        "the inscribed radius of a disc drawn at r=14 must come back as ~14: {:.2}",
        disc.inradius
    );
    assert!(
        spurs.inradius > bar.inradius * 3.0,
        "inradius is the chamber-size column and must separate a cavity from a gallery: {:.2} against {:.2}",
        spurs.inradius,
        bar.inradius
    );
    assert_eq!(get("empty").cells, 0, "an empty mask must not panic and must read zero, which is what a bank nobody dug reads");
    println!("  selftest: every shape reads what it was drawn as, including the two that have no chamber in them.");
}

fn main() {
    let frames: u64 = arg("frames").unwrap_or(1_800);
    let width: i32 = arg("width").unwrap_or(256);
    let height: i32 = arg("height").unwrap_or(320);
    // **The bed has to fit in the world, and the builder will not say so.**
    // `PlantScene` writes its stone floor at `ground_y + soil + STONE_DEPTH`
    // and `World::set` silently drops anything past the bottom edge, so a bed
    // deeper than the world produces a bed with no floor *and* an excavation
    // carved into rows that do not exist -- which reads as "the tunnel closed
    // instantly" at frame 0, before a single tick has run. That is
    // `CLAUDE.md`'s scene-error trap exactly, and it is why the frame-0 row
    // below is asserted rather than merely printed.
    let ground: i32 = arg("ground").unwrap_or(60);
    let soil: i32 = arg("soil").unwrap_or(200);
    let seeds: u64 = arg("seeds").unwrap_or(1);
    let want: String =
        arg("arms").unwrap_or_else(|| "soil,sand,stone,lined,flooded,watertable".to_string());
    let ants: i32 = arg("ants").unwrap_or(55);
    let colony_frames: u64 = arg("colonyframes").unwrap_or(8_000);
    let bud_k: f64 = arg("budk").unwrap_or(BUD_K);

    // **Before the colony arm, not after it.** The shape columns are quoted
    // about a colony and the only thing that says they mean what they claim is
    // a run over shapes whose answer is known; putting it first means nobody
    // reads a table from a binary whose instrument has not been checked.
    if want.split(',').any(|w| w == "selftest") {
        selftest_arm(bud_k);
    }

    let png: Option<String> = arg("png");
    if want.split(',').any(|w| w == "colony") {
        colony_arm(seeds, ants, colony_frames, bud_k, png.as_deref());
    }

    println!(
        "burrow_probe: frames={frames} width={width}x{height} soil={soil} seeds={seeds} arms={want}"
    );
    println!(
        "\nan excavation is cut into each bed and censused as it fills in. \
         `stone` is the positive control and must read 100% at every frame."
    );

    for arm in
        ["soil", "sand", "stone", "lined", "flooded", "watertable"].iter().filter(|a| want.split(',').any(|w| &w == *a))
    {
        println!("\n=== arm {arm} ===");
        println!(
            "{:>6}  {:>7}  {:>24}  {:>24}  {:>24}",
            "seed", "frame", "shaft open/caved", "gallery open/caved", "chamber open/caved"
        );

        for seed in 1..=seeds {
            let mut scene = PlantScene { species: "herb".to_string(), ..PlantScene::default() };
            scene.width = width;
            scene.height = height;
            scene.ground_y = ground;
            scene.soil_depth = soil;
            scene.trees = 0;
            scene.seed = Some(seed);
            let mut world = scene.build();
            // No weather, and a held light: the lab's own operating point.
            // Rain into an open shaft is a real hazard and a different
            // experiment; it must not ride along inside this one.
            world.set_weather_pin(Pin::Clear);

            // **Repaint the bed in this arm's material.** The scene builder
            // only makes soil, so `sand` and `stone` are written over the
            // soil rows it produced -- same geometry, same stone floor
            // underneath, one material changed. That is the A/B `CLAUDE.md`
            // asks for: two arms differing in one thing.
            //
            // The three wet/lined arms keep the soil bed and differ only
            // after the carve, below -- so `lined` against `soil` is an A/B
            // in one thing, which is what makes the comparison mean
            // anything (`CLAUDE.md`: an A/B whose arms differ in two things
            // carries half its effect in the thing that was not under
            // test).
            if *arm == "sand" || *arm == "stone" {
                let id = world
                    .materials
                    .id_of(arm)
                    .unwrap_or_else(|| panic!("{arm} is a compiled-in material"));
                for x in 0..width {
                    for y in ground..(ground + soil) {
                        world.set(x, y, Cell::new(id, 0));
                    }
                }
            }

            // The excavation. A shaft down from the surface, a gallery
            // running off its foot, and a chamber at the gallery's end --
            // 3 cells tall, which is what an ant fits through.
            let shaft_x = width / 3;
            let shaft_bottom = ground + soil / 2;
            let gallery_y = shaft_bottom;
            let gallery_end = shaft_x + 60;

            let mut shaft = Void { name: "shaft", cells: Vec::new() };
            for y in ground..shaft_bottom {
                for x in shaft_x..(shaft_x + 3) {
                    shaft.cells.push((x, y));
                }
            }
            let mut gallery = Void { name: "gallery", cells: Vec::new() };
            for x in shaft_x..gallery_end {
                for y in gallery_y..(gallery_y + 3) {
                    gallery.cells.push((x, y));
                }
            }
            let mut chamber = Void { name: "chamber", cells: Vec::new() };
            for x in gallery_end..(gallery_end + 16) {
                for y in (gallery_y - 4)..(gallery_y + 4) {
                    chamber.cells.push((x, y));
                }
            }

            let voids = [&shaft, &gallery, &chamber];
            for v in voids {
                for (x, y) in &v.cells {
                    world.set(*x, *y, Cell::EMPTY);
                }
            }
            let carved: Vec<usize> = voids.iter().map(|v| v.cells.len()).collect();

            // **The lining, and it is the same set of cells an ant produces.**
            // `creature::line_burrow` packs the 8-neighbourhood of every cell
            // it empties, so the union of those rings over a swept-out tunnel
            // is exactly the shell around the excavation -- which is what this
            // writes in one pass. Doing it here rather than by running ants
            // separates the two claims: this arm asks *does a lined tunnel
            // stand*, and the colony run in `ascii`'s excavation scene asks
            // *do ants produce one*. A single harness answering both could not
            // tell a dead lining from ants that never dug.
            //
            // Routed through `Material::packs_into` rather than
            // `id_of("packedsoil")` so this measures the shipped rule: if the
            // material were renamed or the field dropped, this arm would come
            // back identical to `soil` instead of silently lining itself by a
            // path the game does not use.
            let lined_arm = matches!(*arm, "lined" | "flooded" | "watertable");
            let mut lining = 0usize;
            if lined_arm {
                let carved_set: std::collections::HashSet<(i32, i32)> =
                    voids.iter().flat_map(|v| v.cells.iter().copied()).collect();
                let mut wall: Vec<(i32, i32)> = Vec::new();
                for &(cx, cy) in &carved_set {
                    for dy in -1..=1 {
                        for dx in -1..=1 {
                            let n = (cx + dx, cy + dy);
                            if !carved_set.contains(&n) {
                                wall.push(n);
                            }
                        }
                    }
                }
                wall.sort_unstable();
                wall.dedup();
                for (wx, wy) in wall {
                    let cell = world.get(wx, wy);
                    let Some(packed) = world.materials.get(cell.material).packs_into else {
                        continue;
                    };
                    let mut lined = cell;
                    lined.material = packed;
                    world.set(wx, wy, lined);
                    lining += 1;
                }
            }

            // **Wet the wall from the outside**: the whole bank at
            // `SOIL_SATURATED`, which is a gallery driven below the water
            // table. Every packed cell is then over `SOIL_FIELD_CAPACITY` and
            // `slumps_into` should take the lining apart on the first sweep.
            if *arm == "watertable" {
                for x in 0..width {
                    for y in ground..(ground + soil) {
                        let cell = world.get(x, y);
                        if world.materials.get(cell.material).water_capacity > 0 {
                            world.set(x, y, cell.with_aux(material::SOIL_SATURATED));
                        }
                    }
                }
            }

            // **Wet the wall from the inside**: standing water in the shaft,
            // which drains down it and along the gallery, infiltrating the
            // lining as it goes. This is the arm whose `open` column is
            // uninformative -- water is not materially empty -- and the reason
            // `caved` exists.
            let mut poured = 0usize;
            if *arm == "flooded" {
                let water = world.materials.id_of("water").expect("water is compiled in");
                for (x, y) in &shaft.cells {
                    world.set(*x, *y, Cell::new(water, 0));
                    poured += 1;
                }
            }
            if lined_arm {
                println!("{:>6}  {:>7}  wall cells worked into packedsoil: {lining}{}", "", "-",
                    if poured > 0 { format!(", water poured into the shaft: {poured}") } else { String::new() });
            }

            let mut particles = ParticleSystem::default();
            let mut blasts = Blasts::default();
            let tuning = player::Tuning::default();

            let report = |world: &World, f: u64| {
                let cols: Vec<String> = voids
                    .iter()
                    .zip(&carved)
                    .map(|(v, n)| {
                        let open = v.open(world);
                        let caved = v.caved(world);
                        format!(
                            "{open:>4}/{n:<4}{:>6.1}%/{:>5.1}%",
                            100.0 * open as f64 / *n as f64,
                            100.0 * caved as f64 / *n as f64
                        )
                    })
                    .collect();
                println!("{seed:>6}  {f:>7}  {:>24}  {:>24}  {:>24}", cols[0], cols[1], cols[2]);
            };

            // **The scene check, as an assertion.** Every carved cell must be
            // open before any tick runs. If it is not, the excavation is not
            // where the harness thinks it is and every number below is about
            // the scene rather than about the physics.
            report(&world, 0);
            // **The flooded arm is exempt from the emptiness half and not
            // from the check**: its shaft is deliberately full of water, so
            // `open` is 0 there by construction. What must still hold on every
            // arm is that no *ground* is standing in the excavation before a
            // tick runs, which is the scene error this assertion exists to
            // catch, so `caved` is asserted for all arms and `open` for the
            // dry ones.
            for (v, n) in voids.iter().zip(&carved) {
                assert_eq!(v.caved(&world), 0, "{} had ground standing in it at frame 0", v.name);
                if *arm == "flooded" {
                    continue;
                }
                assert_eq!(
                    v.open(&world),
                    *n,
                    "{} was not fully carved at frame 0 -- the excavation is outside the bed \
                     (ground={ground} soil={soil} height={height}); every number after this \
                     would be a measurement of the scene",
                    v.name
                );
            }
            let marks = [1u64, 5, 30, 120, 600, frames];
            for f in 1..=frames {
                frame::step(
                    &mut world,
                    &mut particles,
                    &mut blasts,
                    player::PlayerInput::default(),
                    &tuning,
                );
                if marks.contains(&f) {
                    report(&world, f);
                }
            }
        }
    }
}

/// **Do real ants leave a standing tunnel?**
///
/// The bank, the floor and the founder count are `examples/ascii.rs`'s
/// excavation scene, deliberately: that scene is CI-gated and already
/// establishes that 55 ants chew soil at 0.8 and are stopped by stone, so
/// reusing its geometry means the only new claim here is what is *left*
/// afterwards. What it adds is the census that scene never had — standing
/// void inside the bank footprint — plus a seed loop, because outcomes here
/// are chaotic in the seed and one run is a sample from a wide distribution.
///
/// **Three numbers on every row and none of them is optional.** `void` is the
/// spatial claim. `digs` is the near-side counter — did the verb fire at all
/// — and `packed` is the far-side effect counter on the same call, which is
/// the pairing `CLAUDE.md` requires after a mining harness reported 200 cuts
/// that removed 0 cells. A renamed `packedsoil`, a dropped `packs_into`, or a
/// dig that only ever lands in stone all read as `packed 0` here and are
/// invisible in `digs`.
fn colony_arm(seeds: u64, ants: i32, frames: u64, bud_k: f64, png: Option<&str>) {
    use pixel_physics::render::Renderer;
    use pixel_physics::sim::chunk::Rect;
    use pixel_physics::sim::parallel;
    use pixel_physics::sim::particle::ParticleSystem;

    let lining_on = std::env::var("PIXEL_PHYSICS_BURROW_LINING").as_deref() != Ok("off");
    println!("\n=== arm colony ===  lining {}", if lining_on { "ON" } else { "OFF (ablated)" });
    println!("  55 ants on a soil bank over stone; `void` is standing empty cells inside the bank");
    println!(
        "  `void` is every empty cell in the bank footprint -- **erosion moves it**; \n           `roofed` is empty with ground standing above it, which erosion cannot produce. Read `roofed`."
    );
    println!(
        "  `comps`/`largest`/`ge8` are the connected-component split of that void (8-connected,\n           the neighbourhood the digger uses). `lgroof` is how much of the largest run has\n           ground overhead: a quarried face is one huge run with no roof, a gallery is a\n           smaller run that is nearly all roof."
    );
    println!(
        "{:>6}  {:>7}  {:>8}  {:>8}  {:>8}  {:>6}  {:>8}  {:>5}  {:>7}  {:>7}  {:>7}  {:>9}  {:>10}  {:>7}  {:>7}",
        "seed",
        "frame",
        "void",
        "roofed",
        "roofed3",
        "comps",
        "largest",
        "ge8",
        "lgroof",
        "digs",
        "packed",
        "soil",
        "packedsoil",
        "hang soil",
        "hang pack"
    );
    println!(
        "  `hang soil`/`hang pack` are ground cells with **nothing underneath them**, over the whole
           world rather than the bank -- the count behind \"is there dirt floating in the sky\".
           `packedsoil` is self-supporting by design, so its column is the one that can be large."
    );
    println!(
        "  The `chamber:` line under each row is the **shape** of the largest roofed-void run --
           `circ` (round -> ramified), `inradius` (how big the cavity is, and the column the
           colony-size control is read on) and `buds` (protrusions off it). `arms=selftest` prints
           what these read on shapes whose answer is known; do not read them without it.
           `crowd` beside them is the `BrainInput::Crowding` distribution over the live colony.
           **Read p50 and p10, not the mean**: the input is clamped at 1.0, and a mean that looks
           mid-range can sit under a median pinned at the ceiling -- which is what it does here, so
           a mechanism about *low* density has no regime to act in. *Does it vary at all* and
           *does it reach the regime the mechanism needs* are different questions.
           **It reads `over 0 ants` at frame 8,000 because the colony has starved by then**: there
           is no food in this bed, digging stops somewhere past frame 4,000, and the last stop is a
           frozen final state rather than a working nest. Read `crowd` at 500 and 2,000.
           `crowddig=`/`digbias=` patch those two genome slots, so the wired arm and its own
           ablation (`crowddig=0 digbias=0.4`) run from one binary on one set of worlds."
    );

    for seed in 1..=seeds {
        let (w, h) = (200i32, 120i32);
        let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
        world.set_weather_pin(Pin::Clear);
        // **Held at noon.** The day/night cycle is the largest visual signal
        // in the engine, and a contact sheet whose stops fall at 0/500/2,000/
        // 8,000 frames lands two of them after dark: the first sheet taken
        // here had a moon in it and the bank was unreadable. That is
        // `CLAUDE.md`'s designed-oscillator rule applied to a picture rather
        // than to a number -- the light must be divided out, or every stop is
        // its own phase plus the thing being looked at.
        world.set_sky_hold(Some(pixel_physics::sky::frame_for_daylight(1.0)));
        let soil_id = world.materials.id_of("soil").expect("soil");
        let packed_id = world.materials.id_of("packedsoil").expect("packedsoil");
        let nest_id = world.materials.id_of("nest").expect("nest");
        let floor = h - 8;
        let (bank_x0, bank_x1) = (40i32, 160i32);
        let (bank_y0, bank_y1) = (floor - 30, floor);

        for x in 0..w {
            for y in floor..h {
                world.set(x, y, Cell::new(material::STONE, 0).with_attached(true));
            }
        }
        for x in bank_x0..bank_x1 {
            for y in bank_y0..bank_y1 {
                world.set(x, y, Cell::new(soil_id, 0).with_attached(true));
            }
        }
        for x in 16..bank_x0 {
            world.set(x, floor, Cell::new(nest_id, 0).with_attached(true));
        }
        // Founder placement is seeded by shifting the row start, so the four
        // seeds are genuinely different colonies rather than one colony four
        // times -- `World::new` alone does not vary here the way `PlantScene`
        // does, and a "seed sweep" over identical worlds is the tidy,
        // meaningless result `CLAUDE.md` warns is the tell of an artifact.
        // **Both arms of the room experiment in one binary, before a single
        // ant is planted.** `crowddig=` sets `(Crowding, Dig, w)` and
        // `digbias=` sets `(Bias, Dig, b)` directly on the species genome, so
        // an arm and its own baseline (`crowddig=0 digbias=0.4`, which is what
        // `ant.ron` authors) are the *same* executable on the *same* world --
        // the only form of A/B `CLAUDE.md` trusts without qualification, and
        // the only one that can rate-match an offset without a rebuild per
        // point.
        //
        // **No species file authors a `Crowding -> Dig` weight.** It was built
        // and withdrawn on 2026-09-02, when twelve seeds put its effect at 16
        // of 33 seed pairs against four seeds' 4 of 4. These knobs stay because
        // the null is only readable through them, and because the retry
        // conditions in `Reports/dead-ends.md` need them to be cheap.
        //
        // **Before `plant_ant`, and that is load-bearing**: `place_creature`
        // copies the species genome into each animal at spawn, so a patch
        // applied afterwards changes nothing at all and the two arms come back
        // byte-identical -- which is this project's standing tell for a change
        // that must have moved something and did not. It was written the wrong
        // way round once already, in `ascii`'s deposition ablation.
        {
            use pixel_physics::sim::brain::{io_slot, BrainInput, BrainOutput};
            let (cw, db) = (arg::<f32>("crowddig"), arg::<f32>("digbias"));
            // **`curvdrop=` sets the curvature slope on both drop verbs**,
            // and it exists because a null with one value of the slope tested
            // is the criticism §14i makes of the last one: "one value of the
            // slope was tested" -- the crowding sweep varied the dig *rate*
            // and never the weight under test. `ant.ron` authors 0.169 on
            // each; this patches both together so the sweep is over one
            // number.
            let cd = arg::<f32>("curvdrop");
            if cw.is_some() || db.is_some() || cd.is_some() {
                let id = world.species.id_of("ant").expect("ant");
                let mut g = world.species.get(id).genome.clone();
                if let Some(w) = cw {
                    g[io_slot(BrainInput::Crowding, BrainOutput::Dig)] = w;
                }
                if let Some(b) = db {
                    g[io_slot(BrainInput::Bias, BrainOutput::Dig)] = b;
                }
                if let Some(c) = cd {
                    g[io_slot(BrainInput::SurfaceCurvature, BrainOutput::Drop)] = c;
                    g[io_slot(BrainInput::SurfaceCurvature, BrainOutput::DropSpoil)] = c;
                }
                world.species.set_genome(id, g);
                if seed == 1 {
                    println!(
                        "  PATCHED genome: (Crowding, Dig) = {:?}, (Bias, Dig) = {:?}, (SurfaceCurvature, Drop/DropSpoil) = {:?}   [ant.ron authors no Crowding->Dig; its (Bias, Dig) is 0.4 and its curvature weights are 0.169]",
                        cw, db, cd
                    );
                }
            }
        }

        // **The verb prices, swept from here for the same reason the genome
        // weights above are**: they are `CreatureDef` fields compiled in via
        // `include_str!`, so editing `ant.ron` and re-running a prebuilt
        // binary produces bit-identical "runs" -- the gotcha `CLAUDE.md`
        // records three of. Patching the live registry is the only way to
        // sweep them without a rebuild between every point.
        {
            let dig_cost = arg::<f32>("digcost");
            let emit_cost = arg::<f32>("emitcost");
            let spoil_weight = arg::<f32>("spoilweight");
            if dig_cost.is_some() || emit_cost.is_some() || spoil_weight.is_some() {
                let id = world.species.id_of("ant").expect("ant");
                let mut def = world.species.get(id).creature.as_ref().expect("creature").clone();
                if let Some(v) = dig_cost {
                    def.dig_cost_in_moves = v;
                }
                if let Some(v) = emit_cost {
                    def.emit_cost_in_moves = v;
                }
                if let Some(v) = spoil_weight {
                    def.spoil_weight_cells = v;
                }
                world.species.set_creature(id, def);
                if seed == 1 {
                    println!(
                        "  PATCHED prices: dig_cost_in_moves = {dig_cost:?}, emit_cost_in_moves = {emit_cost:?}, spoil_weight_cells = {spoil_weight:?}   [ant.ron authors 0.0 for all three]"
                    );
                }
            }
        }
        // **Founder placement is a seeded shuffle of the ground left of the
        // bank, and the version this replaced could not produce more than
        // seven colonies.**
        //
        // It read `20 + (seed - 1) * 3 + i % 10 * 2`, which walks the founder
        // row rightward as the seed climbs -- and `bank_x0` is 40, so from
        // **seed 8 onward every ant is placed inside the bank**, `plant_ant`
        // refuses an occupied cell, and the world runs 8,000 frames with no
        // colony in it. Measured 2026-09-02 while answering a review that
        // asked for twelve seeds instead of four: seeds 8-12 read `digs 0`,
        // `packed 0`, `roofed 0` in **both** arms of the experiment, which
        // reads exactly like "the effect disappears at scale" and is an empty
        // scene. `CLAUDE.md`'s *a scene that contradicts the code will look
        // like a bug in the code*, and its *check that a guard's inputs
        // actually vary what it guards* -- a harness that advertises
        // `seeds=N` has to mean it.
        //
        // Slots are enumerated over the nest strip only (never into the
        // bank), shuffled per seed, and taken in order, so every seed is a
        // genuinely different colony of the same size rather than one colony
        // slid sideways.
        {
            use pixel_physics::sim::rng;
            // **The lattice, shuffled -- not a free scatter.** An ant is a
            // `Chain(2)`, so it needs its neighbour column free; scattering
            // over every cell placed only 34 of 52 and tripped the assertion
            // below, which is that assertion doing its job on its first run.
            // Columns two apart and rows one apart is the spacing the original
            // placement used and is what fits a colony of this size.
            let mut slots: Vec<(i32, i32)> =
                (16..bank_x0).step_by(2).flat_map(|x| (1..=6).map(move |r| (x, floor - r))).collect();
            let mut draw = rng::stream(seed, 0xB0_1707, 0, 0);
            for i in (1..slots.len()).rev() {
                slots.swap(i, draw.below(i as u32 + 1) as usize);
            }
            for &(x, y) in slots.iter().take(ants as usize) {
                world.plant_ant(x, y);
            }
        }
        // **The scene check, asserted rather than printed** -- the same rule
        // this file already applies to its carved voids at frame 0, and the
        // one that would have caught the bug above the day it was written. A
        // colony that did not get planted is not a result about digging.
        let planted = world.live_organism_ids().len();
        assert!(
            planted >= (ants as usize) * 9 / 10,
            "seed {seed}: only {planted} of {ants} founders were placed, so this world is not the colony the run reports on"        );

        // **`void` alone cannot say a nest happened, and the first version of
        // this arm shipped believing it could.** Measured 2026-08-30: with the
        // lining ablated, 610 digs left **788** standing empty cells inside the
        // bank footprint -- against 803 cells of soil gone from it. The two
        // numbers are the same number. Digging *destroys* its spoil
        // (`creature.rs`'s dig, "spoil is destroyed in v1"), so every cell an
        // ant removes lowers the bank by one cell somewhere, and the empty
        // rows that opens up at the **top** of the footprint are counted by a
        // rectangle census exactly as if they were a chamber. A colony that
        // eats a bank down from above and one that hollows it out score
        // identically, which is `CLAUDE.md`'s "ask what your number counts
        // when nothing is wrong" with the answer *it counts erosion*.
        //
        // `roofed` is the column that states the claim: an empty cell with
        // **ground standing above it**, which is the one thing lowering a
        // surface can never produce. `roofed3` requires three cells of it, so
        // a crumb bridging a pit does not read as a chamber. Neither can be
        // moved by erosion at all, which is what makes them the pair to read.
        let census = |world: &World| {
            let mut void = 0usize;
            let mut roofed = 0usize;
            let mut roofed3 = 0usize;
            let mut soil = 0usize;
            let mut packed = 0usize;
            for x in bank_x0..bank_x1 {
                // Walk the column downward carrying how much ground is
                // standing above the current row -- one pass, and it counts
                // the roof rather than merely detecting it.
                let mut above = 0usize;
                for y in 0..bank_y1 {
                    let m = world.get(x, y).material;
                    let ground = matches!(
                        world.materials.kind(m),
                        MaterialKind::Powder | MaterialKind::Solid
                    );
                    if y >= bank_y0 {
                        if m == material::EMPTY {
                            void += 1;
                            if above > 0 {
                                roofed += 1;
                            }
                            if above >= 3 {
                                roofed3 += 1;
                            }
                        } else if m == soil_id {
                            soil += 1;
                        } else if m == packed_id {
                            packed += 1;
                        }
                    }
                    if ground {
                        above += 1;
                    }
                }
            }
            // **Ground standing on nothing, over the whole world.** Not a
            // bank column: the ants carry spoil out of the footprint, so a
            // rectangle census cannot see where it went, and the question
            // this answers -- "is any of this dirt in mid-air" -- is exactly
            // one about cells outside the bank.
            //
            // `packedsoil` is `self_supporting` by design, so an unsupported
            // cell of it is not a bug in the sweep; it is the honest count of
            // how much worked ground is hanging, which is what a player sees
            // as dirt floating in the sky. Read against the same run with
            // `PIXEL_PHYSICS_DIG_SPOIL=destroy`, which is the only baseline a
            // standing quantity has.
            let (float_pieces, float_cells) = floating_ground(world, w, h);
            (void, roofed, roofed3, soil, packed, float_pieces, float_cells)
        };

        // **Does the lever have anything to act on?** -- `CLAUDE.md`'s *check
        // that a planned step can demonstrate itself, before promising it
        // will*, asked of `(Crowding, Dig, w)` before it was believed -- and it
        // passed, which is what makes that mechanism's null a statement about
        // the mechanism rather than about a channel with no range in it.
        //
        // `BrainInput::Crowding` counts other animals' cells within r=2 of the
        // head over `CROWDING_SCALE`, so a colony whose ants never stand near
        // each other reads a flat zero and **no weight on it can move
        // anything**, at any magnitude. That is the shape of failure this
        // project keeps paying for -- a lever wired to a channel with no
        // range. Read through `creature::probe`, the shipped function the
        // brain itself is fed, rather than a reimplementation of the scan.
        // **The whole distribution, not the mean and the max, and that
        // correction is the reason this closure was rewritten.** The first
        // version reported mean and max only, and answered *"is `Crowding` a
        // dead channel?"* -- it passed, and that answer is real. But the
        // mechanism it was built to test (Toffin's density-dependent digging)
        // is about what happens when density falls **below a critical value**,
        // and `Crowding` is `(crowd / crowd_scale).min(1.0)`: **clamped at
        // 1.0**. A colony mean of 0.995 falling to 0.675 with the max pinned
        // at 1.0 says the sensor spent the run in the top third of its range,
        // so the regime the model is about may never have been entered -- and
        // a mean cannot show that. `min`/`p10`/`p50` can.
        //
        // The general form, which is the transferable part: **an input that
        // never leaves saturation cannot demonstrate a mechanism about its low
        // end**, and "does it vary at all" is a different question from "does
        // it reach the regime the mechanism requires". Asserting the *realised
        // range* of a driving input is a precondition of the run, not a
        // readout of it.
        let crowding = |world: &World| -> (Vec<f64>, usize) {
            let mut vals: Vec<f64> = Vec::new();
            for id in world.live_organism_ids() {
                let Some(state) = world.organism(id) else { continue };
                let Some(def) = world.species.get(state.species).creature.as_ref() else { continue };
                let Some(&(hx, hy)) = state.chain.first() else { continue };
                let (inputs, _, _) = pixel_physics::sim::creature::probe(world, hx, hy, id, def);
                vals.push(inputs[pixel_physics::sim::brain::BrainInput::Crowding as usize] as f64);
            }
            let n = vals.len();
            // Sorted for the order statistics. `f64` has no total order, so
            // `total_cmp` rather than a `partial_cmp().unwrap()` that a NaN
            // would panic on.
            vals.sort_by(f64::total_cmp);
            (vals, n)
        };

        // The sheet is written for the first seed only: it is there to show
        // *what* and *where*, and the table above it is what says how much
        // and whether it came back. Four pictures of four seeds would be
        // four samples of a wide distribution presented as if they were a
        // result.
        let mut renderer = Renderer::new();
        // **The lighting model is a parameter of this measurement, not a
        // constant of it.** `SkyLight::Coarse4` is the shipped default and
        // propagates sky light on a **4-cell block grid**; an ant gallery is
        // one to three cells across, so the nest is a feature finer than the
        // model that is supposed to darken it. `sky-light-design.md` measured
        // exactly this one step coarser -- block 8 "loses a one-cell shaft
        // entirely" -- and nothing had asked what block 4 does to a structure
        // the size of a burrow. `light=depth|4|2|1` asks.
        renderer.sky_light = match arg::<String>("light").as_deref() {
            Some("depth") => pixel_physics::render::SkyLight::Depth,
            Some("2") => pixel_physics::render::SkyLight::Coarse2,
            Some("1") => pixel_physics::render::SkyLight::Exact,
            _ => pixel_physics::render::SkyLight::Coarse4,
        };
        let particles = ParticleSystem::new();
        let (vw, vh) = (w as u32, h as u32);
        let mut tiles: Vec<Vec<u8>> = Vec::new();
        let shoot = png.is_some() && seed == 1;

        // Steerable so one stop can be asked for on its own: a four-tile
        // column is the right shape for reading a run and the wrong shape for
        // a side-by-side comparison, which is what a review card wants.
        let marks: Vec<u64> = match arg::<String>("marks") {
            Some(v) => v.split(',').map(|m| m.parse().expect("marks=a,b,c")).collect(),
            None => vec![0, 500, 2_000, frames],
        };
        for f in 0..=frames {
            if f > 0 {
                parallel::step(&mut world);
                world.step_active_sites();
                world.step_fields();
                world.step_pheromones();
            }
            if marks.contains(&f) {
                let (void, roofed, roofed3, soil, packed, floats, float_cells) = census(&world);
                let comps = void_components(&world, (bank_x0, bank_x1), (bank_y0, bank_y1));
                let largest = comps.first().copied().unwrap_or_default();
                let ge8 = comps.iter().filter(|c| c.cells >= 8).count();
                let st = world.creature_stats;
                println!(
                    "{seed:>6}  {f:>7}  {void:>8}  {roofed:>8}  {roofed3:>8}  {:>6}  {:>8}  {ge8:>5}  {:>7}  {:>7}  {:>7}  {soil:>9}  {packed:>10}  {floats:>7}  {float_cells:>7}",
                    comps.len(),
                    largest.cells,
                    largest.roofed,
                    st.digs,
                    st.packed
                );
                // **The shape of the cavity, and whether the lever that is
                // supposed to make one has any range.** Both on their own line
                // rather than as five more columns: the row above is already
                // fifteen wide, and a number nobody can find is a number
                // nobody reads.
                let shape = roofed_void_shape(&world, (bank_x0, bank_x1), (bank_y0, bank_y1), bud_k);
                let (crowd, live) = crowding(&world);
                let at = |q: f64| crowd.get(((crowd.len() as f64 - 1.0) * q).round() as usize).copied().unwrap_or(0.0);
                let mean = if live > 0 { crowd.iter().sum::<f64>() / live as f64 } else { 0.0 };
                println!(
                    "         chamber: {:>5} cells  circ {:>5.3}  inradius {:>5.2}  buds {:>3}",
                    shape.cells, shape.circularity, shape.inradius, shape.buds
                );
                println!(
                    "         crowd over {live} ants: min {:.3}  p10 {:.3}  p50 {:.3}  mean {mean:.3}  p90 {:.3}  max {:.3}   (clamped at 1.0)",
                    at(0.0),
                    at(0.10),
                    at(0.50),
                    at(0.90),
                    at(1.0)
                );
                // The distribution, at the last stop only. `comps`/`largest`
                // are order statistics over it and cannot say whether the
                // remainder is forty pockets or four hundred crumbs -- which
                // is the whole question when the total is the same number
                // either way.
                if f == *marks.iter().max().unwrap_or(&0) {
                    let singles = comps.iter().filter(|c| c.cells == 1).count();
                    let top: Vec<String> =
                        comps.iter().take(8).map(|c| format!("{}({})", c.cells, c.roofed)).collect();
                    println!(
                        "         sizes(roofed): {}  ...  singletons {singles} of {}",
                        top.join(" "),
                        comps.len()
                    );
                }
                if shoot {
                    let mut buf = vec![0u8; (vw * vh * 4) as usize];
                    let touched = world.take_touched_chunks();
                    renderer.draw(&world, &particles, &touched, &mut buf, (vw, vh), true);
                    tiles.push(buf);
                }
            }
        }

        // **What do these things actually look like next to each other?** --
        // the render-side half of the legibility question, and it cannot be
        // answered from the palette tables. `soil.ron` and `packedsoil.ron`
        // list twelve tones each, but what reaches the screen is whatever
        // `Renderer::draw` makes of them after lighting and shading, and it is
        // the screen the owner is judging. So this samples the **shipped
        // renderer's own output buffer**, one pixel per cell, and reports the
        // mean colour and relative luminance of each class standing in the
        // bank.
        //
        // The column that matters is not the gap between two means. It is the
        // gap between two means measured against the **spread within one
        // class**: a bank is deliberately mottled, and a lining whose tone sits
        // inside that mottle is not a faint signal, it is no signal -- the two
        // populations overlap and no amount of looking separates them. That is
        // the quantitative form of the owner's *"These look identical"*.
        //
        // Generalises well past ants: any question of the form "can a player
        // see X against Y" in this engine is this measurement, and nothing
        // else here could answer it.
        if std::env::args().any(|a| a == "contrast=1") && seed == 1 {
            let mut buf = vec![0u8; (vw * vh * 4) as usize];
            let touched = world.take_touched_chunks();
            let mut r2 = Renderer::new();
            r2.sky_light = renderer.sky_light;
            r2.draw(&world, &particles, &touched, &mut buf, (vw, vh), true);
            let lum = |c: [f64; 3]| 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
            let mut classes: Vec<(&str, Vec<f64>, [f64; 3])> = vec![
                ("soil", Vec::new(), [0.0; 3]),
                ("packedsoil", Vec::new(), [0.0; 3]),
                ("void (roofed)", Vec::new(), [0.0; 3]),
                ("void (open face)", Vec::new(), [0.0; 3]),
                ("sky (control)", Vec::new(), [0.0; 3]),
            ];
            // **Two controls, because a colour is only a finding against a
            // known answer.** `CLAUDE.md`: construct the case whose answer you
            // know and check the instrument reports it. Open sky must come
            // back pale and tight; the unroofed void the ants opened at the
            // face is open air and must come back indistinguishable from it.
            // If either misses, the sampling is wrong and the roofed-void row
            // says nothing.
            for x in bank_x0..bank_x1 {
                for y in (bank_y0 - 12).max(0)..bank_y0 - 2 {
                    if world.get(x, y).material == material::EMPTY {
                        let o = ((y as u32 * vw + x as u32) * 4) as usize;
                        let px = [buf[o] as f64, buf[o + 1] as f64, buf[o + 2] as f64];
                        for (k, v) in px.iter().enumerate() {
                            classes[4].2[k] += v;
                        }
                        classes[4].1.push(lum(px));
                    }
                }
            }
            for x in bank_x0..bank_x1 {
                let mut above = 0usize;
                for y in 0..bank_y1 {
                    let m = world.get(x, y).material;
                    let ground = matches!(
                        world.materials.kind(m),
                        MaterialKind::Powder | MaterialKind::Solid
                    );
                    if y >= bank_y0 {
                        let which = if m == soil_id {
                            Some(0)
                        } else if m == packed_id {
                            Some(1)
                        } else if m == material::EMPTY {
                            Some(if above > 0 { 2 } else { 3 })
                        } else {
                            None
                        };
                        if let Some(i) = which {
                            let o = ((y as u32 * vw + x as u32) * 4) as usize;
                            let px = [buf[o] as f64, buf[o + 1] as f64, buf[o + 2] as f64];
                            for (k, v) in px.iter().enumerate() {
                                classes[i].2[k] += v;
                            }
                            classes[i].1.push(lum(px));
                        }
                    }
                    if ground {
                        above += 1;
                    }
                }
            }
            println!("  contrast, as the shipped renderer draws it (one pixel per cell, frame {frames}):");
            println!("{:>16}  {:>7}  {:>17}  {:>7}  {:>15}", "class", "cells", "mean RGB", "mean L", "L range (p5-p95)");
            let mut summary: Vec<(String, f64, f64, f64)> = Vec::new();
            for (name, mut ls, sum) in classes.into_iter() {
                if ls.is_empty() {
                    continue;
                }
                let n = ls.len() as f64;
                let mean = lum([sum[0] / n, sum[1] / n, sum[2] / n]);
                ls.sort_by(|a, b| a.partial_cmp(b).expect("no NaN in a luminance"));
                let (lo, hi) = (ls[ls.len() / 20], ls[ls.len() * 19 / 20]);
                println!(
                    "{name:>16}  {:>7}  {:>17}  {mean:>7.1}  {:>15}",
                    ls.len(),
                    format!("({:.0}, {:.0}, {:.0})", sum[0] / n, sum[1] / n, sum[2] / n),
                    format!("{lo:.1} - {hi:.1}")
                );
                summary.push((name.to_string(), mean, lo, hi));
            }
            // The verdict line. Two classes are *separable* only if the gap
            // between their means beats the spread each of them already has --
            // otherwise a cell of one is routinely brighter than a cell of the
            // other and the boundary between them is not a boundary.
            for i in 0..summary.len() {
                for j in i + 1..summary.len() {
                    let (a, b) = (&summary[i], &summary[j]);
                    let gap = (a.1 - b.1).abs();
                    let spread = ((a.3 - a.2) + (b.3 - b.2)) / 2.0;
                    println!(
                        "  {} vs {}: mean gap {:.1} of 255 against a within-class spread of {:.1} -- {}",
                        a.0,
                        b.0,
                        gap,
                        spread,
                        if gap > spread { "separable" } else { "**overlapping: these read as one material**" }
                    );
                }
            }
        }

        // **An ASCII dump of the bank, because a contact sheet cannot answer
        // the question the roofed count raises.** `roofed` says *how much*
        // enclosed void there is; it cannot say whether that is one gallery,
        // forty disconnected pockets, or a rind of overhang along one face --
        // and those are different findings with the same number
        // (`CLAUDE.md`: an image tells you what and where, a metric how much).
        // At three pixels a cell a one-cell gallery is invisible in the sheet,
        // so this is the channel that shows the shape.
        if std::env::args().any(|a| a == "map=1") && seed == 1 {
            println!("  bank at frame {frames}: '.' loose soil, '#' packedsoil, ' ' void, ',' other");
            for y in bank_y0..bank_y1 {
                let mut row = String::new();
                for x in bank_x0..bank_x1 {
                    let m = world.get(x, y).material;
                    row.push(if m == material::EMPTY {
                        ' '
                    } else if m == soil_id {
                        '.'
                    } else if m == packed_id {
                        '#'
                    } else {
                        ','
                    });
                }
                println!("  |{row}|");
            }
        }

        if shoot {
            // **Crop to the bank, then magnify.** The first version of this
            // sheet rendered the whole 200x120 world at zoom 3 and was
            // unreadable for the thing it exists to show: the world is mostly
            // sky, the galleries are one to three cells across, and the ants
            // work the bank's near face. Rendered whole, the lined bank and
            // the ablated one look like the same brown mound -- which would
            // have had the picture contradict a `roofed` count of 130 against
            // 0 and the ASCII map that shows a warren, and the picture is what
            // this project settles arguments with.
            //
            // Done here rather than through the renderer's own zoom, which
            // moves the *camera* rather than the scale of the output.
            //
            // **The default window is the near third, not the whole bank.**
            // The colony enters from the nest at x=16..40 and works into the
            // face it meets, so the excavation is a pocket at that end and the
            // other two thirds are undisturbed ground -- a window over the
            // whole bank spends 70% of its pixels on soil nothing happened to,
            // and at that scale a three-cell gallery is three pixels.
            //
            // Steerable all the same, because "is there a nest in there" and
            // "what does the bank look like" want different framings and
            // neither answers the other.
            let zoom: u32 = arg("zoom").unwrap_or(10);
            let crop: String = arg("crop").unwrap_or_else(|| {
                format!("{},{},56,{}", bank_x0 - 26, bank_y0 - 6, bank_y1 - bank_y0 + 12)
            });
            let c: Vec<u32> = crop.split(',').map(|v| v.parse().expect("crop=x,y,w,h")).collect();
            let (cx0, cy0, cw, ch) = (c[0], c[1], c[2], c[3]);
            let (sw, sh) = (cw * zoom, ch * zoom * tiles.len() as u32);
            let mut sheet = vec![0u8; (sw * sh * 4) as usize];
            for (i, tile) in tiles.iter().enumerate() {
                let y0 = i as u32 * ch * zoom;
                for y in 0..ch * zoom {
                    for x in 0..sw {
                        let src = (((cy0 + y / zoom) * vw + cx0 + x / zoom) * 4) as usize;
                        let dst = (((y0 + y) * sw + x) * 4) as usize;
                        sheet[dst..dst + 4].copy_from_slice(&tile[src..src + 4]);
                    }
                }
            }
            let out = png.expect("checked above");
            image::save_buffer(out, &sheet, sw, sh, image::ColorType::Rgba8)
                .expect("writing the sheet");
            println!("  wrote {out} ({sw}x{sh}) -- frames {marks:?} stacked top to bottom");
        }
    }
}
