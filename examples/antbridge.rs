//! **The double bridge with real ants** (Goss et al. 1989; Deneubourg et
//! al. 1990): level 2 of the bridge test, the ants of `bridgebox`'s walker
//! result on the game's body and engine.
//!
//! A stone box: a nest chamber on the left, a food chamber on the right
//! with `food=` rows of `crumbs` on its floor, and between them one neck from
//! each chamber to a fork, then two branches `gap=` rows apart. `long=<rows>`
//! makes the `longside=` branch (lower by default) dip that many rows and
//! back, so it is `2 x rows` cells longer; `opens=<frame>` keeps the other
//! (short) branch shut at both ends until then -- Goss's "short branch added
//! later", whose colonies stayed on the long one (16/18).
//!
//! `ant=textbook` (default) is the textbook ant's slice B
//! (`creature::textbook::ForageCfg`): it picks up a crumb it touches, carries
//! it home on its home vector, puts it down in the nest chamber, and walks by
//! Deneubourg's `(k + C)^n` on the food trail it lays. `mute=1` lays the
//! trail and never reads it. `ant=game` is the shipped ant in the same box.
//! `ant=walk` is the new ant (`creature::needs`, `WalkParts::NEW_ANT`) from
//! frame `needsat=` (1). Both colony ants need `room=18` and a fed floor
//! below 1 (`fed=0.5`): see `chamber_h` and `crumb` for why.
//!
//! **The new walk, 2026-10-15** (`ant=walk room=18 fed=0.5`, the textbook
//! grid otherwise, 4 seeds a mirror, game plane `brho=0.03 bdiff=0.05`):
//! 0.95-1.00 of crossings on the short branch in both mirrors, **and the
//! same with the trail wiped every pass (`brho=1`)**. Opened late, the short
//! branch takes over at once (stayed on long 0/8). Hungry ants walk out on
//! the way-out pull and eat at the feeder; none brings a crumb home
//! (delivered 0). So it passes Goss's majority bar by its pull, not by its
//! trail, and fails the lock-in bar.
//!
//! **Demography is fixed** in every arm: births paused, `halflife` long, and
//! every `500` frames each ant is topped up to `fed=` of its start energy
//! (1.0 by default; the textbook ant does not eat, so this is the stopgap
//! that keeps it alive; the game ant needs hunger to forage, so give it
//! less). Crumbs put down in the nest are taken away every 500 frames and
//! the food floor is restocked: the colony neither fills nor runs dry.
//!
//! Read: crossings of the two branches at the middle column, by branch and
//! by laden, per window; per seed the short branch's share of all crossings
//! over the whole run and the last half, and whether it is the majority or
//! over 80% (Goss's two bars).
//!
//! **What it found, 2026-10-15** (`tunnel=2 release=50 brho=0.0002 bdiff=0
//! lay=0.1 k=2 long=40`, 30k frames, 8 seeds, numbers in
//! `textbook::ForageCfg`): the textbook ant passes Goss's bars in both
//! mirrors at 150 ants (short majority 14/16, over 80% 11/16; stays on
//! the long branch 16/16 when the short one opens late), and breaks
//! symmetry on equal branches with either side winning. `trace=1` found
//! why 3-cell tunnels failed: every empty ant walked the neck's floor row
//! (the row reached first, climbing from the chamber below) and the floor
//! leads down the shaft, so the lower branch won whatever its length.
//! Four harness biases came first: vectors aimed at the chamber floors
//! (below the fork) pulled every choice down; a trail at the engine
//! plane's 255 cap reads the same on both branches; the plane's blend
//! carried the neck's trail into both arms of the fork; and the 3-cell
//! tunnel above.
//! `ant=game` does not forage in this box (0-3 crossings a run); its
//! bridge is `trailfollow`'s.
//!
//! **The dead branch, 2026-10-16** (`feeders=2 room=44 gap=18`: each branch
//! ends in its own feeder room, the two the same shape under gravity;
//! `moves=<frame>` moves the food to the other room; scored as the fed
//! branch's share, and the switch as the first of three windows running
//! that give the newly fed branch the majority, after a window over 80% on
//! the old one). The bar is Dussutour et al. 2009 (*J. Exp. Biol.*
//! 212:2337, *Pheidole megacephala*): every colony moved to the new branch
//! (15/15), after 20.8 ± 5.9 min, 1.46 lifetimes of their foraging trail.
//! Textbook ant, 150 ants, 2 x 4 seeds, the trail's lifetime 10k frames
//! (`brho=0.0012`), food moved at 30k:
//! - **No fade (`brho=0.0002`): never, 0/8.** Nothing in the ant lets go.
//! - **Fade alone (laid only when laden, `layout=0`): 3/8, at ~3
//!   lifetimes.** Traced: the old trail stands at 8x the choice constant
//!   `k`, so it takes ln 8 ~ 2 lifetimes just to fall to it.
//! - **Crowding lays less (`crowd=0.03`, Czaczkes 2013): 8/8, but at
//!   ~2.5 lifetimes**, and Goss's late-opened short branch then wins
//!   (stays on long ~1/8): it lowers every trail, the one that should hold
//!   too.
//! - **The no-entry mark (`giveup=600 search=24 noentry=0.1`, Robinson
//!   2005): 8/8 at 0.9-1.5 lifetimes with `layout=0`** -- the paper's
//!   1.0-1.9. On Goss's bridge at the same settings, a late short branch
//!   opened once the colony is established is ignored 8/8, but the short
//!   branch reaches 80% on 1/8 (Goss 11/14). Laid both ways (`layout=1`),
//!   Goss passes and the switch drops to 3/8: the trail on the dead branch
//!   is renewed by every ant that walks it.
//!
//! **Two scents, the paper's own rule (2026-10-16 afternoon;
//! `explore=0.1 explorec=1 k=6 kn=1 explorek=10 explorelay=0.3
//! exploremax=10 layout=0 giveup=300`, food moved at 31,500 and back at
//! 63,000, Dussutour's Fig. 10 timeline at 700 frames a minute):** first
//! switch 8/8 at 1.12-1.54 lifetimes, with the figure's gradual shape and
//! 80/20 floor, about 7 min late; the switch back is no faster (1.26-1.68,
//! 7/8; paper 0.85). The same setting loses Goss's late-opened short branch
//! (0/16 stay on the long one) and his 80% (about 3/16). Results and the
//! curve in `emergence-sandbox/dead-branch/lessons-dead-branch-2026-10-16.md`
//! section 4c.
//!
//! Four harness traps came first. A remembered feeder that is now empty
//! pinned every ant that reached it (the empty room filled), so the ant
//! gives up (`giveup=`); counted over the whole trip, ants gave up on a
//! long busy branch and marked the food they had not reached, so it counts
//! only near the remembered site (`search=`). A shelf-split chamber put one
//! room's crumbs in its own doorway. And an outbound vector aimed between
//! two rooms sits nearer one room's food: it pulls along the row only.

use pixel_physics::sim::creature::{self, textbook};
use pixel_physics::sim::parallel;
use pixel_physics::sim::pheromone::Channel;
use pixel_physics::sim::rng::Rng;
use pixel_physics::sim::{Cell, Rect, World};

const KEYS: &[&str] = &[
    "seed",
    "seeds",
    "frames",
    "ants",
    "ant",
    "gap",
    "len",
    "neck",
    "tunnel",
    "long",
    "longside",
    "opens",
    "release",
    "k",
    "n",
    "lay",
    "layout",
    "vector",
    "mute",
    "brho",
    "bdiff",
    "fed",
    "halflife",
    "food",
    "png",
    "every",
    "trace",
    "needsat",
    "room",
    "worth",
    "feeders",
    "moves",
    "start",
    "vout",
    "giveup",
    "search",
    "sense",
    "outfrac",
    "crowd",
    "noentry",
    "repel",
    "markfork",
    "arho",
    "drop",
    "roomh",
    "explore",
    "explorec",
    "explorek",
    "kn",
    "back",
    "restock",
    "explorelay",
    "exploremax",
];

const FEED_EVERY: u64 = 500;

fn check_args() {
    let mut seen = std::collections::HashSet::new();
    for a in std::env::args().skip(1) {
        let key = a.split_once('=').map_or(a.as_str(), |(k, _)| k).to_string();
        assert!(
            KEYS.contains(&key.as_str()),
            "antbridge: unknown argument `{a}` (known: {})",
            KEYS.join(", ")
        );
        assert!(seen.insert(key.clone()), "antbridge: `{key}` given twice");
    }
}

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| {
        a.strip_prefix(&format!("{key}=")).map(|v| {
            v.parse()
                .ok()
                .unwrap_or_else(|| panic!("antbridge: `{a}` does not parse"))
        })
    })
}

/// The box, in cells.
struct Scene {
    nest: (i32, i32, i32, i32),
    food: (i32, i32, i32, i32),
    mid: i32,
    /// Rows at or above this are the upper branch.
    split_y: i32,
    neck_y: i32,
    plug: Vec<(i32, i32)>,
    short_upper: bool,
    /// `feeders=2`: the upper and lower feeder rooms, each `(x0, x1, floor)`:
    /// the span the crumbs are laid over, and `floor` the room's lowest open
    /// row.
    rooms: Option<[(i32, i32, i32); 2]>,
}

const CH_W: i32 = 16;
/// `feeders=2`: rows from a branch's mouth down to its feeder room's floor.
const FEEDER_DROP: i32 = 12;
const MARGIN: i32 = 4;

/// **A chamber's height**: `room=`, or room for both branches' mouths and
/// ten rows. `room=18` (at `gap=10 tunnel=2`) keeps every cell of a chamber
/// within `under_cover`'s 20 rows of its ceiling: the default 24-row chamber
/// has a floor the game ant and the new walk read as open ground ("outside"),
/// so a hungry new-walk ant on it searched (`Seek`, 254,817 decisions in
/// 30k frames) instead of taking the way out, and delivered nothing.
fn chamber_h(gap: i32, th: i32) -> i32 {
    let room = arg("room").unwrap_or(gap + 2 * th + 10);
    assert!(
        room >= gap + 2 * th + 4,
        "room={room}: a chamber must reach down to the lower branch's floor"
    );
    room
}

/// **A feeder crumb, worth `worth=`** (100 by default) in `aux`, as a crumb
/// cut from a meal carries its worth (`Carried::into_cell`). A bare crumb
/// (`aux` 0) is priced at `food_energy` 40, and through the ant's gut that
/// is under `EAT_YIELD_THRESHOLD`: the new walk's food sense skipped every
/// one (0 eat decisions in 30k frames, 2026-10-15) while the game ant took
/// them. The textbook ant does not eat, so its rows are unchanged.
fn crumb(crumbs: pixel_physics::sim::material::MaterialId) -> Cell {
    Cell::new(crumbs, 0).with_aux(arg("worth").unwrap_or(100))
}

/// The box's width and height for these arguments.
fn size(gap: i32, len: i32, th: i32, long: i32) -> (i32, i32) {
    (
        MARGIN + CH_W + len + CH_W + MARGIN,
        MARGIN + long + chamber_h(gap, th) + MARGIN,
    )
}

#[allow(clippy::too_many_arguments)]
fn build(
    world: &mut World,
    gap: i32,
    len: i32,
    neck: i32,
    th: i32,
    long: i32,
    long_lower: bool,
    food_rows: i32,
    feeders: bool,
) -> Scene {
    let stone = world.materials.id_of("stone").expect("stone");
    let crumbs = world.materials.id_of("crumbs").expect("crumbs");
    let (ch_w, margin) = (CH_W, MARGIN);
    let ch_h = chamber_h(gap, th);
    let up_room = if long_lower { 0 } else { long };
    let (w, h) = size(gap, len, th, long);
    for y in 0..h {
        for x in 0..w {
            world.set(x, y, Cell::new(stone, 0).with_attached(true));
        }
    }
    let ch_y0 = margin + up_room;
    let nest = (margin, ch_y0, margin + ch_w - 1, ch_y0 + ch_h - 1);
    let food = (
        margin + ch_w + len,
        ch_y0,
        margin + ch_w + len + ch_w - 1,
        ch_y0 + ch_h - 1,
    );
    let carve = |world: &mut World, x0: i32, y0: i32, x1: i32, y1: i32| {
        for y in y0..=y1 {
            for x in x0..=x1 {
                world.set(x, y, Cell::EMPTY);
            }
        }
    };
    carve(world, nest.0, nest.1, nest.2, nest.3);
    carve(world, food.0, food.1, food.2, food.3);
    // `feeders=2` centres the branches in the chambers (`gap=18 room=44`
    // fits both feeder rooms).
    let up_y = ch_y0 + if feeders { (ch_h - gap - 2 * th) / 2 } else { 4 };
    let lo_y = up_y + th + gap;
    // The neck meets the shaft halfway between the branches, so neither
    // branch is the neck's straight continuation.
    let neck_y = (up_y + lo_y) / 2;
    let (x_a, x_b) = (nest.2 + 1 + neck, food.0 - 1 - neck);
    // Necks along each chamber's lower part, then a shaft up to both
    // branches. `feeders=2` has no food-side neck: each branch runs on into
    // its own feeder room.
    let necks: &[(i32, i32, i32)] = if feeders {
        &[(nest.2 + 1, x_a + th - 1, x_a)]
    } else {
        &[
            (nest.2 + 1, x_a + th - 1, x_a),
            (x_b - th + 1, food.0 - 1, x_b - th + 1),
        ]
    };
    for &(x0, x1, shaft) in necks {
        carve(world, x0, neck_y, x1, neck_y + th - 1);
        carve(world, shaft, up_y, shaft + th - 1, lo_y + th - 1);
    }
    let mid = (nest.2 + food.0) / 2;
    let (a, b) = (nest.2 + 1 + len / 4, nest.2 + 1 + 3 * len / 4);
    for (y, is_long, dir) in [(up_y, !long_lower, -1), (lo_y, long_lower, 1)] {
        if is_long && long > 0 {
            let far = y + dir * long;
            let (ylo, yhi) = (y.min(far), y.max(far));
            carve(world, x_a, y, a + th - 1, y + th - 1);
            carve(world, a, ylo, a + th - 1, yhi + th - 1);
            carve(world, a, far, b + th - 1, far + th - 1);
            carve(world, b, ylo, b + th - 1, yhi + th - 1);
            carve(world, b, y, x_b, y + th - 1);
        } else {
            carve(world, x_a, y, if feeders { food.0 - 1 } else { x_b }, y + th - 1);
        }
    }
    // **`feeders=2`: two feeder rooms the same shape under gravity.** The
    // food chamber is stone again except two rooms, each entered by its
    // branch three rows below the room's ceiling, with the floor (and the
    // crumbs) `FEEDER_DROP` rows below the mouth. A first version split the
    // chamber with a shelf instead: the upper room's crumbs lay just under
    // its mouth, the crowd at the food blocked the way in, and the upper
    // branch's trail held a third of the lower's (2026-10-16, crowding on:
    // 1.0 against 3.3 deposits), which read as a lower-branch bias.
    let drop: i32 = arg("drop").unwrap_or(FEEDER_DROP);
    let rooms = feeders.then(|| {
        assert!(
            lo_y - (arg::<i32>("roomh").unwrap_or(FEEDER_DROP + 3) - drop) > up_y + drop + 1,
            "feeders=2 needs gap >= {}",
            FEEDER_DROP + 3
        );
        assert!(
            lo_y + drop <= food.3,
            "feeders=2 needs room >= {}",
            lo_y + drop - ch_y0 + 1
        );
        for y in food.1..=food.3 {
            for x in food.0..=food.2 {
                world.set(x, y, Cell::new(stone, 0).with_attached(true));
            }
        }
        // `drop=1` (with `tunnel=2`): the room's floor is the branch's
        // floor, a flat arena as in Dussutour's, so a laden ant never has
        // to climb to its door. With the door 12 rows up, 15-35 laden ants
        // sat on the new room's floor under its door for the first trail
        // lifetime after the food moved (census, 2026-10-16), on every
        // setting. The room keeps its height (headroom above the door
        // instead of a drop below it): a 5-row flat room jammed with 76
        // ants. The crumbs go in the room's far half only, off the doorway
        // (the shelf version's lesson above). **Measured worse** (`room=52`,
        // fed-only laying with the mark, 2026-10-16): before the food moved
        // empty ants split about evenly between the rooms (16/12 crossings
        // a window), 75 of 150 sat searching the unfed room, and the colony
        // never met the 0.8 the switch is scored from (1/8). Kept as an
        // option; the default room is the one the results stand on.
        // `roomh=`: the room's height above its floor, for the colony ants,
        // which read a room over about ten rows as open ground (`room=18`).
        let head = arg::<i32>("roomh").unwrap_or(FEEDER_DROP + 3) - drop;
        assert!(up_y - head > ch_y0, "feeders=2 drop={drop} needs a taller room=");
        for y in [up_y, lo_y] {
            carve(world, food.0, y - head, food.2, y + drop);
        }
        let x0 = if drop < FEEDER_DROP { food.0 + CH_W / 2 } else { food.0 };
        [(x0, food.2, up_y + drop), (x0, food.2, lo_y + drop)]
    });
    // Food: rows of crumbs on the food chamber's floor (or the starting
    // room's).
    let stock = rooms.map_or((food.0, food.2, food.3), |r| {
        r[usize::from(arg::<String>("start").as_deref() == Some("lower"))]
    });
    for y in stock.2 - food_rows + 1..=stock.2 {
        for x in stock.0..=stock.1 {
            world.set(x, y, crumb(crumbs));
        }
    }
    let short_y = if long_lower { up_y } else { lo_y };
    let (p_a, p_b) = (x_a + th, x_b - th - 1);
    let plug = [p_a, p_a + 1, p_b, p_b + 1]
        .into_iter()
        .flat_map(|x| (short_y..short_y + th).map(move |y| (x, y)))
        .collect();
    Scene {
        nest,
        food,
        mid,
        split_y: (up_y + th + lo_y) / 2,
        neck_y,
        plug,
        short_upper: long_lower,
        rooms,
    }
}

fn main() {
    check_args();
    creature::mutation_off_for_measuring();
    let seeds: Vec<u64> = match arg::<u64>("seed") {
        Some(s) => vec![s],
        None => (1..=arg::<u64>("seeds").unwrap_or(8)).collect(),
    };
    let frames: u64 = arg("frames").unwrap_or(60_000);
    let n_ants: usize = arg("ants").unwrap_or(60);
    let kind: String = arg("ant").unwrap_or_else(|| "textbook".into());
    assert!(
        kind == "textbook" || kind == "game" || kind == "walk",
        "antbridge: ant={kind}: textbook, game or walk"
    );
    let gap: i32 = arg("gap").unwrap_or(10);
    let len: i32 = arg("len").unwrap_or(80);
    let neck: i32 = arg("neck").unwrap_or(10);
    let th: i32 = arg("tunnel").unwrap_or(3);
    let long: i32 = arg("long").unwrap_or(0);
    let long_lower = arg::<String>("longside").as_deref() != Some("upper");
    let opens: u64 = arg("opens").unwrap_or(0);
    let release: u64 = arg("release").unwrap_or(0);
    let fed: f32 = arg("fed").unwrap_or(1.0);
    let food_rows: i32 = arg("food").unwrap_or(2);
    let every: u64 = arg("every").unwrap_or(frames / 10);
    let feeders = arg::<u8>("feeders").unwrap_or(1) == 2;
    let moves: u64 = arg("moves").unwrap_or(0);
    assert!(moves == 0 || feeders, "moves= needs feeders=2");
    // `back=<frame>`: the food goes back to the first room (Dussutour's
    // third phase, at 120 min against the first move's 45).
    let back: u64 = arg("back").unwrap_or(0);
    assert!(back == 0 || back > moves && moves > 0, "back= needs moves= before it");
    assert!(
        arg::<f32>("explore").unwrap_or(0.0) == 0.0 || arg::<f32>("noentry").unwrap_or(0.0) == 0.0,
        "explore= and noentry= both ride channel A"
    );
    assert!(!feeders || long == 0, "feeders=2 runs equal branches");
    println!(
        "antbridge: ant={kind} ants={n_ants} gap={gap} len={len} neck={neck} tunnel={th} long={long} longside={} opens={opens} release={release} fed={fed} frames={frames}",
        if long_lower { "lower" } else { "upper" }
    );
    let mut rows = Vec::new();
    for &seed in &seeds {
        rows.push(run(
            seed, frames, n_ants, &kind, gap, len, neck, th, long, long_lower, opens, release, fed, food_rows, every,
            feeders, moves,
        ));
    }
    if feeders {
        // **Dussutour et al. 2009's moved food**: per seed, the frames from
        // the move to the first window in which the newly fed branch
        // carried the majority of crossings, and the same in trail
        // lifetimes (`12 / brho` frames, the engine plane's fade).
        let life = 12.0 / arg::<f32>("brho").unwrap_or(0.001);
        let switched: Vec<Option<u64>> = rows.iter().map(|r| r.5).collect();
        if arg::<u64>("back").unwrap_or(0) > 0 {
            let b: Vec<Option<u64>> = rows.iter().map(|r| r.6).collect();
            println!(
                "SUMMARY moved back: switch after [{}] frames = [{}] trail lifetimes; switched {} of {} (Dussutour 2009: 15/15, 12.2 +- 7.7 min = 0.85 lifetimes, against 20.8 the first time)",
                b.iter().map(|s| s.map_or("never".into(), |v| v.to_string())).collect::<Vec<_>>().join(" "),
                b.iter().map(|s| s.map_or("-".into(), |v| format!("{:.2}", v as f32 / life))).collect::<Vec<_>>().join(" "),
                b.iter().filter(|s| s.is_some()).count(),
                rows.len()
            );
        }
        let n = switched.iter().filter(|s| s.is_some()).count();
        println!(
            "SUMMARY moved food at {moves}: switch after [{}] frames = [{}] trail lifetimes ({life:.0} frames); switched {n} of {} (Dussutour 2009: 15/15, 1.0-1.9 lifetimes)",
            switched.iter().map(|s| s.map_or("never".into(), |v| v.to_string())).collect::<Vec<_>>().join(" "),
            switched.iter().map(|s| s.map_or("-".into(), |v| format!("{:.2}", v as f32 / life))).collect::<Vec<_>>().join(" "),
            rows.len()
        );
    }
    let maj = rows.iter().filter(|r| r.0 > 0.5).count();
    let what = if feeders { "fed" } else { "short" };
    let over = rows.iter().filter(|r| r.0 >= 0.8).count();
    println!(
        "SUMMARY {what}-branch share, last half (share/crossings): [{}]; majority {maj} of {}, over 80% {over} of {}",
        rows.iter()
            .map(|r| format!("{:.2}/{}", r.0, r.1))
            .collect::<Vec<_>>()
            .join(" "),
        rows.len(),
        rows.len()
    );
    println!(
        "SUMMARY whole run: [{}]; delivered per seed [{}]",
        rows.iter()
            .map(|r| format!("{:.2}/{}", r.2, r.3))
            .collect::<Vec<_>>()
            .join(" "),
        rows.iter().map(|r| r.4.to_string()).collect::<Vec<_>>().join(" ")
    );
}

#[allow(clippy::too_many_arguments)]
fn run(
    seed: u64,
    frames: u64,
    n_ants: usize,
    kind: &str,
    gap: i32,
    len: i32,
    neck: i32,
    th: i32,
    long: i32,
    long_lower: bool,
    opens: u64,
    release: u64,
    fed: f32,
    food_rows: i32,
    every: u64,
    feeders: bool,
    moves: u64,
) -> (f32, u64, f32, u64, u64, Option<u64>, Option<u64>) {
    let back: u64 = arg("back").unwrap_or(0);
    // `restock=<frames>`: how often the nest's crumbs are eaten and the
    // feeder refilled. Dussutour's feeder is 3 ml of sucrose, never short;
    // at `FEED_EVERY` a crowd of empty ants waits at the bare floor between
    // refills, and an ant with food in reach never gives up.
    let restock: u64 = arg("restock").unwrap_or(FEED_EVERY);
    let (w, h) = size(gap, len, th, long);
    let mut world = World::new(Rect::new(0, 0, w - 1, h - 1));
    world.seed = seed;
    let sc = build(&mut world, gap, len, neck, th, long, long_lower, food_rows, feeders);
    // Which room is fed: 0 upper, 1 lower.
    let mut fed_room = usize::from(arg::<String>("start").as_deref() == Some("lower"));
    let mut switched: Option<u64> = None;
    let mut switched_back: Option<u64> = None;
    let mut chose_before = false;
    let mut chose_back = false;
    let mut streak = 0u32;
    let stone = world.materials.id_of("stone").expect("stone");
    let crumbs = world.materials.id_of("crumbs").expect("crumbs");
    if opens > 0 {
        for &(x, y) in &sc.plug {
            world.set(x, y, Cell::new(stone, 0).with_attached(true));
        }
    }
    world
        .pheromones
        .set_channel_rho(Channel::B, arg("brho").unwrap_or(0.001));
    world
        .pheromones
        .set_channel_diffuse(Channel::B, arg("bdiff").unwrap_or(0.05));
    // Channel A carries the textbook ant's no-entry mark (`noentry=`): it
    // fades `arho=` a pass (by default 2.4 times slower than the trail,
    // Robinson et al. 2009) and does not spread.
    world.pheromones.set_channel_rho(
        Channel::A,
        arg("arho").unwrap_or_else(|| arg::<f32>("brho").unwrap_or(0.001) / 2.4),
    );
    world.pheromones.set_channel_diffuse(Channel::A, 0.0);
    for _ in 0..100 {
        parallel::step(&mut world);
    }
    let id = world.species.id_of("ant").expect("ant ships");
    if let Some(c) = world.species.get_mut(id).creature.as_mut() {
        c.life_half_life = arg("halflife").unwrap_or(10_000_000);
    }
    world.births_paused = true;
    // Both vectors aim at the neck's level, so at a fork they favour neither
    // branch; the drop zone is the nest chamber.
    let home = ((sc.nest.0 + sc.nest.2) / 2, sc.nest.3);
    let aim_home = ((sc.nest.0 + sc.nest.2) / 2, sc.neck_y);
    let feeder = ((sc.food.0 + sc.food.2) / 2, sc.neck_y);
    if kind != "textbook" {
        // The game ant's home is its birth cell (`forage_anchor`), but the
        // new walk reads home from a founded site (`nearest_nest_site`), and
        // a built nest has none: without this its ants wander (smoke run
        // 2026-10-15: nest_sites=0, 226 crossings, none laden).
        world.register_nest_site((sc.nest.0 + sc.nest.2) / 2, sc.nest.3, (sc.nest.2 - sc.nest.0) / 2);
    }
    if kind == "textbook" {
        let mute = arg::<u8>("mute").unwrap_or(0) == 1;
        let cfg = textbook::TextbookCfg {
            p_move: 1.0,
            turn: 0.125,
            dt: 4.0,
            drop_base: 0.025,
            drop_per: 0.11,
            pick: 0.029,
            tau: 0.0,
            template: 2.0,
            source: crumbs,
            n_rock: false,
            pick_around: true,
            base_always: true,
            build_on_box: false,
            forage: Some(textbook::ForageCfg {
                food: crumbs,
                home,
                home_r: (sc.nest.2 - sc.nest.0) / 2 + 1,
                aim_home,
                feeder,
                vector: arg("vector").unwrap_or(1.0),
                vector_out: arg("vout").unwrap_or_else(|| arg("vector").unwrap_or(1.0)),
                k: arg("k").unwrap_or(20.0),
                n: arg("n").unwrap_or(2),
                lay: arg("lay").unwrap_or(1.0),
                lay_out: arg::<u8>("layout").unwrap_or(1) == 1,
                out_frac: arg("outfrac").unwrap_or(1.0),
                read: !mute,
                give_up: arg("giveup").unwrap_or(0),
                search_r: arg("search").unwrap_or(0),
                sense: arg("sense").unwrap_or(0),
                out_flat: sc.rooms.is_some(),
                crowd: arg("crowd").unwrap_or(0.0),
                no_entry: arg("noentry").unwrap_or(0.0),
                repel: arg("repel").unwrap_or(1.0),
                mark_fork: arg::<u8>("markfork").unwrap_or(0) == 1,
                mark_to_fork: arg::<u8>("markfork").unwrap_or(0) == 2,
                explore: arg("explore").unwrap_or(0.0),
                explore_c: arg("explorec").unwrap_or(0.1),
                explore_k: arg("explorek").unwrap_or(1.0),
                explore_lay: arg("explorelay").unwrap_or(0.0),
                explore_max: arg("exploremax").unwrap_or(0.0),
                kn: arg::<u8>("kn").unwrap_or(0) == 1,
            }),
            sort: None,
            nest: None,
        };
        world.textbook = Some(Box::new(textbook::Textbook::new(id, cfg)));
    }
    let mut rng = Rng::new(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xa27b);
    let mut colony = None;
    let mut placed = 0usize;
    let mut plant = |world: &mut World, placed: &mut usize, rng: &mut Rng| {
        for _ in 0..40 {
            let x = sc.nest.0 + 1 + rng.below((sc.nest.2 - sc.nest.0 - 1) as u32) as i32;
            if let Some(site) = creature::plant_founder_in(world, x, sc.nest.3, "ant", colony, None) {
                if colony.is_none() {
                    colony = creature::colony_of_site(world, &site);
                }
                world.schedule_active_site(site);
                *placed += 1;
                return;
            }
        }
    };
    if release == 0 {
        while placed < n_ants {
            let before = placed;
            plant(&mut world, &mut placed, &mut rng);
            if placed == before {
                break;
            }
        }
    }
    // `ant=walk`: the new ant (`creature::needs`, parts `WalkParts::NEW_ANT`),
    // handed every ant at frame `needsat=` (1 by default: the nest here is
    // built, not dug, so there is no founding to wait out) as trailfollow's
    // `needs=walk` does. Coordinator's "Both ants" row, 2026-10-15.
    let needs_at: u64 = arg("needsat").unwrap_or(1);
    let mut last_x: std::collections::HashMap<u32, i32> = std::collections::HashMap::new();
    // `trace=1`: for every empty ant leaving the nest-side neck into the
    // shaft, the row it walked the neck on (ceiling, middle, floor), the
    // trail one cell above and below the junction, and the branch it next
    // crossed the middle on. Printed as a table at the end.
    let trace = arg::<u8>("trace").unwrap_or(0) == 1;
    let shaft_x = sc.nest.2 + 1 + neck;
    let mut pending: std::collections::HashMap<u32, (i32, u16, u16)> = std::collections::HashMap::new();
    // [neck row 0..th][branch short/long] counts, empty ants, last half.
    let mut fork = vec![[0u64; 2]; th as usize];
    let mut fork_trail = vec![[0f64; 2]; th as usize];
    // [short, long] crossings: whole run, last half, this window.
    let (mut whole, mut late, mut win) = ([0u64; 2], [0u64; 2], [[0u64; 2]; 2]);
    // Empty ants crossing the middle *away from the nest* only, this
    // window: Dussutour's count (outbound ants, 1 cm past the choice
    // point). `win[0]` also holds empty ants walking home, and after a move
    // those are the ants giving up on the emptied branch.
    let mut outbound = [0u64; 2];
    let mut delivered = 0u64;
    // Crumbs taken out of the nest chamber: delivery for the ants that are
    // not the textbook ant (which counts its own).
    let mut eaten = 0u64;
    let short_lane = |y: i32| -> usize {
        let upper = y < sc.split_y;
        usize::from(upper != sc.short_upper)
    };
    for f in 1..=frames {
        if release > 0 && placed < n_ants && f % release == 0 {
            plant(&mut world, &mut placed, &mut rng);
        }
        if moves > 0 && (f == moves || f == back) {
            // Take the food away from one room and put it in the other.
            if let Some(rooms) = sc.rooms {
                let (x0, x1, floor) = rooms[fed_room];
                for y in floor - food_rows + 1..=floor {
                    for x in x0..=x1 {
                        if world.get(x, y).material == crumbs && world.get(x, y).organism_id() == 0 {
                            world.set(x, y, Cell::EMPTY);
                        }
                    }
                }
                fed_room = 1 - fed_room;
                println!(
                    "seed {seed} f {f:>7}: FOOD MOVED to the {} room",
                    ["upper", "lower"][fed_room]
                );
            }
        }
        if opens > 0 && f == opens {
            for &(x, y) in &sc.plug {
                world.set(x, y, Cell::EMPTY);
            }
        }
        if kind == "walk" && f == needs_at {
            let mut walk = creature::needs::NeedsWalk::new(creature::needs::NeedsMode::Walk, world.frame);
            walk.parts = creature::needs::WalkParts::NEW_ANT;
            world.needs = Some(Box::new(walk));
            println!(
                "  NEEDS seed {seed}: frame {f} parts={} nest_sites={} shaft={}",
                creature::needs::WalkParts::NEW_ANT.label(),
                world.nest_sites.len(),
                world.nest_sites.iter().any(|n| n.shaft.is_some())
            );
        }
        parallel::step(&mut world);
        world.step_active_sites();
        world.step_fields();
        world.step_pheromones();
        if f % restock == 0 {
            feed(&mut world, fed);
            // Eat what came home; restock the feeder.
            for y in sc.nest.1..=sc.nest.3 {
                for x in sc.nest.0..=sc.nest.2 {
                    if world.get(x, y).material == crumbs && world.get(x, y).organism_id() == 0 {
                        eaten += 1;
                        world.set(x, y, Cell::EMPTY);
                    }
                }
            }
            let (x0, x1, floor) = sc.rooms.map_or((sc.food.0, sc.food.2, sc.food.3), |r| r[fed_room]);
            for y in floor - food_rows + 1..=floor {
                for x in x0..=x1 {
                    if world.is_empty(x, y) {
                        world.set(x, y, crumb(crumbs));
                    }
                }
            }
        }
        for oid in world.live_organism_ids() {
            let Some(st) = world.organism(oid) else { continue };
            if st.species != id {
                continue;
            }
            let Some(&(hx, hy)) = st.chain.first() else { continue };
            // The textbook ant carries its crumb as a pellet; the game ant
            // and the new walk carry food in the crop (trailfollow's
            // `carrying_larder`).
            let laden = usize::from(st.spoil.is_some() || st.crop.is_some_and(|c| c.material == crumbs));
            if trace && laden == 0 {
                if let Some(&px) = last_x.get(&oid) {
                    // Stepping from the neck into the shaft, heading out.
                    if px < shaft_x && hx >= shaft_x && hx < shaft_x + th && (sc.neck_y..sc.neck_y + th).contains(&hy) {
                        let up = world.pheromone_at(Channel::B, hx, sc.neck_y - 1);
                        let down = world.pheromone_at(Channel::B, hx, sc.neck_y + th);
                        pending.insert(oid, (hy - sc.neck_y, up, down));
                    }
                }
            }
            if let Some(&px) = last_x.get(&oid) {
                if (px < sc.mid) != (hx < sc.mid) && hx > sc.nest.2 && hx < sc.food.0 {
                    // `feeders=2`: lane 0 is the branch to the fed room.
                    let lane = if sc.rooms.is_some() {
                        usize::from((hy < sc.split_y) != (fed_room == 0))
                    } else {
                        short_lane(hy)
                    };
                    if let Some((row, up, down)) = pending.remove(&oid) {
                        if f > frames / 2 && laden == 0 && px < sc.mid {
                            fork[row as usize][lane] += 1;
                            fork_trail[row as usize][0] += f64::from(up);
                            fork_trail[row as usize][1] += f64::from(down);
                        }
                    }
                    whole[lane] += 1;
                    win[laden][lane] += 1;
                    if laden == 0 && px < sc.mid && hx >= sc.mid {
                        outbound[lane] += 1;
                    }
                    if f > frames / 2 {
                        late[lane] += 1;
                    }
                }
            }
            last_x.insert(oid, hx);
        }
        if f % every == 0 {
            let all = win[0][0] + win[0][1] + win[1][0] + win[1][1];
            let fed_share = (win[0][0] + win[1][0]) as f32 / all.max(1) as f32;
            // A switch needs a choice to switch from: the window before the
            // move must have had over 80% on the then-fed branch (a colony
            // splitting 50:50, as the muted one does, never chose).
            if moves > 0 && f == moves {
                chose_before = fed_share > 0.8;
            }
            if back > 0 && f == back {
                chose_back = fed_share > 0.8;
                streak = 0;
            }
            // ...and the switch is the end of the first three windows in a row
            // that each give the new branch the majority, so one window's
            // flicker past half is not a switch.
            if moves > 0 && f > moves {
                streak = if all >= 20 && fed_share > 0.5 { streak + 1 } else { 0 };
                if back > 0 && f > back {
                    if chose_back && switched_back.is_none() && streak >= 3 {
                        switched_back = Some(f - back);
                    }
                } else if chose_before && switched.is_none() && streak >= 3 {
                    switched = Some(f - moves);
                }
            }
            let counts = world.textbook.as_ref().map(|t| t.counts);
            let d = counts.map_or(eaten, |c| c.delivered);
            println!(
                "seed {seed} f {f:>7}: crossings {}/other empty {}/{} laden {}/{} | alive {} | picked {} delivered {}",
                if sc.rooms.is_some() { "fed" } else { "short" },
                win[0][0],
                win[0][1],
                win[1][0],
                win[1][1],
                world.live_organism_ids().len(),
                counts.map_or(0, |c| c.food_picks),
                d
            );
            delivered = d;
            if sc.rooms.is_none() && counts.is_some_and(|c| c.no_entry_steps > 0) {
                println!("  no-entry steps {}", counts.map_or(0, |c| c.no_entry_steps));
            }
            if sc.rooms.is_some() {
                // The food trail along each branch's middle half, in deposits
                // (`SCALE`): what the fork's `(k + C)^n` reads.
                let mean_on = |ch: Channel, y: i32| {
                    let xs = sc.nest.2 + 1 + len / 4..sc.food.0 - len / 4;
                    let n = xs.len().max(1) as f32;
                    xs.map(|x| {
                        (0..th)
                            .map(|dy| f32::from(world.pheromone_at(ch, x, y + dy)))
                            .sum::<f32>()
                            / th as f32
                    })
                    .sum::<f32>()
                        / n
                        / f32::from(pixel_physics::sim::pheromone::SCALE)
                };
                let mean = |y: i32| mean_on(Channel::B, y);
                let (up, lo) = (mean(sc.split_y - th - gap / 2), mean(sc.split_y + gap / 2));
                // The exploration scent (`explore=`) on the same stretch.
                let (eu, el) = (
                    mean_on(Channel::A, sc.split_y - th - gap / 2),
                    mean_on(Channel::A, sc.split_y + gap / 2),
                );
                let (e_fed, e_other) = if fed_room == 0 { (eu, el) } else { (el, eu) };
                let (fed_c, other_c) = if fed_room == 0 { (up, lo) } else { (lo, up) };
                let met = counts.map_or(0.0, |c| c.met_sum / c.crowd_rolls.max(1) as f64);
                let marks = |y: i32| {
                    (sc.nest.2 + 1..sc.food.0)
                        .map(|x| f32::from(world.pheromone_at(Channel::A, x, y)))
                        .fold(0.0f32, f32::max)
                        / f32::from(pixel_physics::sim::pheromone::SCALE)
                };
                // Where the ants are: nest chamber, the nest-side neck and
                // shaft, a branch, a feeder room.
                let mut at = [0u32; 5];
                let mut room_trip = [(0u64, 0u32, 0u32); 2];
                for oid in world.live_organism_ids() {
                    let Some(&(hx, hy)) = world.organism(oid).and_then(|s| s.chain.first()) else {
                        continue;
                    };
                    let i = if hx <= sc.nest.2 {
                        0
                    } else if hx >= sc.food.0 {
                        if hy < sc.split_y {
                            3
                        } else {
                            4
                        }
                    } else if hx <= sc.nest.2 + 1 + neck + th {
                        1
                    } else {
                        2
                    };
                    at[i] += 1;
                    if i >= 3 {
                        let trip = world.textbook.as_ref().and_then(|t| t.trips.get(&oid).copied());
                        room_trip[i - 3].0 += trip.map_or(0, |t| u64::from(t.0));
                        room_trip[i - 3].1 += u32::from(trip.is_some_and(|t| t.1));
                        room_trip[i - 3].2 += u32::from(world.organism(oid).is_some_and(|s| s.spoil.is_some()));
                    }
                }
                println!(
                    "  rooms upper/lower: mean give-up count {:.0}/{:.0}, given up {}/{}, laden {}/{}",
                    room_trip[0].0 as f64 / f64::from(at[3].max(1)),
                    room_trip[1].0 as f64 / f64::from(at[4].max(1)),
                    room_trip[0].1,
                    room_trip[1].1,
                    room_trip[0].2,
                    room_trip[1].2
                );
                println!(
                    "  ants nest {} neck {} branches {} upper room {} lower room {}",
                    at[0], at[1], at[2], at[3], at[4]
                );
                let (mu, ml) = (marks(sc.split_y - th - gap / 2), marks(sc.split_y + gap / 2));
                let (mark_f, mark_o) = if fed_room == 0 { (mu, ml) } else { (ml, mu) };
                println!(
                    "  trail fed/other {fed_c:.1}/{other_c:.1} deposits, exploration fed/other {e_fed:.1}/{e_other:.1}, no-entry peak fed/other {mark_f:.1}/{mark_o:.1}, fed share {fed_share:.2}, mean encounters at a laying step {met:.1}, no-entry steps {}",
                    counts.map_or(0, |c| c.no_entry_steps)
                );
            }
            if let Some(walk) = world.needs.as_ref() {
                let c = walk.counts;
                println!(
                    "  walk: decisions {:?} took_forage {} quit_forage {} gave_up {} lay_walks {}",
                    c.decisions, c.took_forage, c.quit_forage, c.gave_up, c.lay_walks
                );
            }
            if sc.rooms.is_some() {
                println!("  outbound fed/other {}/{}", outbound[0], outbound[1]);
            }
            win = [[0; 2]; 2];
            outbound = [0; 2];
        }
    }
    if trace {
        let upper_is = if sc.short_upper { 0 } else { 1 };
        for (row, counts) in fork.iter().enumerate() {
            let n = counts[0] + counts[1];
            println!(
                "TRACE seed {seed} neck row {row} ({}): {n} empty ants out of the neck, upper branch {} | trail above the junction {:.0} below {:.0} (mean raw)",
                ["ceiling", "middle", "floor"].get(row).unwrap_or(&"?"),
                counts[upper_is],
                fork_trail[row][0] / n.max(1) as f64,
                fork_trail[row][1] / n.max(1) as f64
            );
        }
    }
    if let Some(dir) = arg::<String>("png") {
        let _ = std::fs::create_dir_all(&dir);
        let img = draw(&world, w, h);
        img.save(format!("{dir}/s{seed}.png")).expect("png");
    }
    let share = |a: [u64; 2]| a[0] as f32 / (a[0] + a[1]).max(1) as f32;
    (
        share(late),
        late[0] + late[1],
        share(whole),
        whole[0] + whole[1],
        delivered,
        switched,
        switched_back,
    )
}

fn feed(world: &mut World, floor: f32) {
    use pixel_physics::sim::world::Account;
    for id in world.live_organism_ids() {
        let Some(st) = world.organism(id) else { continue };
        let Some(start) = world.species.get(st.species).creature.as_ref().map(|c| c.start_energy) else {
            continue;
        };
        let short = floor * start - st.energy;
        if short <= 0.0 {
            continue;
        }
        let colony = world.colony_of(id);
        world.set_organism_energy(id, floor * start);
        world.book(colony, Account::Granted, f64::from(short));
    }
}

/// Stone grey, open dark, crumbs tan, ants yellow, and the food trail as red
/// over open cells.
fn draw(wd: &World, w: i32, h: i32) -> image::RgbaImage {
    const PX: u32 = 3;
    let stone = wd.materials.id_of("stone");
    let crumbs = wd.materials.id_of("crumbs");
    let mut img = image::RgbaImage::new(w as u32 * PX, h as u32 * PX);
    for y in 0..h {
        for x in 0..w {
            let c = wd.get(x, y);
            let s = (f32::from(wd.pheromone_at(Channel::B, x, y)) / 65535.0).sqrt();
            let col = if c.organism_id() != 0 {
                [250, 220, 60, 255]
            } else if Some(c.material) == stone {
                [90, 90, 96, 255]
            } else if Some(c.material) == crumbs {
                [200, 150, 90, 255]
            } else {
                [(24.0 + 220.0 * s) as u8, 22, 30, 255]
            };
            for dy in 0..PX {
                for dx in 0..PX {
                    img.put_pixel(x as u32 * PX + dx, y as u32 * PX + dy, image::Rgba(col));
                }
            }
        }
    }
    img
}
