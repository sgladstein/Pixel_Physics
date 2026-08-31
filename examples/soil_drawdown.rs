//! **What does one plant do to the whole bed's water?**
//!
//! The owner's report, 2026-08-31, from the moisture overlay: the lab starts
//! at 62% everywhere and holds there; the moment a single plant goes in, the
//! surface starts drying, the dryness spreads *sideways* across the whole
//! bed — faster left than right — and settles at 18%, while the bottom of
//! the bed climbs to 100%. Seeds then will not germinate.
//!
//! 62 / 18 / 100 are `SOIL_FIELD_CAPACITY`, `SOIL_WILTING_POINT` and
//! `SOIL_SATURATED`, so the question is whether the profile is being pumped
//! from one end of that range to the other.
//!
//! ```text
//! cargo run --release --example soil_drawdown                 # one plant
//! cargo run --release --example soil_drawdown -- founders=0   # the control: nothing alive
//! cargo run --release --example soil_drawdown -- frames=24000 every=4000
//! ```
//!
//! # What it reports, and why these three views
//!
//! - **A horizontal profile along the rooting row.** The claim is that
//!   dryness spreads sideways from one plant to the whole bed, so the
//!   instrument has to be able to see *distance from the plant* — a single
//!   column cannot, and neither can a total.
//! - **A vertical profile down the middle.** The claim is that the bottom
//!   saturates while the top empties. That is a redistribution, not a loss,
//!   and only a column shows it.
//! - **The bed's total water, and how it splits above and below field
//!   capacity.** This is what separates the two candidate stories: a *sink*
//!   (total falls — the plant and the sun really are consuming it) from a
//!   *ratchet* (total holds, but the distribution polarises to the ends of
//!   the range). They look identical on the overlay and want opposite fixes.
//!
//! # `founders=0` is the control and is not optional
//!
//! An empty bed must hold flat at field capacity. The owner reports that it
//! does, and this arm is what makes every figure in the other one mean
//! something: if the bed drifts with nothing alive in it, the plant is not
//! the cause and the rest of the run is measuring the wrong thing.

use pixel_physics::lab::scene::LabBox;
use pixel_physics::lab::Lab;
use pixel_physics::sim::material;
use pixel_physics::sim::update;
use pixel_physics::sim::world::World;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix(&format!("{key}=")).map(|v| v.parse().ok().expect("parses")))
}

fn tick(lab: &mut Lab) {
    pixel_physics::sim::frame::step(
        &mut lab.world,
        &mut lab.particles,
        &mut lab.blasts,
        pixel_physics::sim::player::PlayerInput::default(),
        &pixel_physics::sim::player::Tuning::default(),
    );
}

/// Mean soil moisture over the cells of one row that actually hold water.
fn row_mean(world: &World, spec: &LabBox, y: i32, x0: i32, x1: i32) -> Option<u16> {
    let (mut sum, mut n) = (0u64, 0u64);
    for x in x0.max(0)..x1.min(spec.width) {
        let c = world.get(x, y);
        if world.materials.get(c.material).water_capacity > 0 {
            sum += update::soil_moisture(c) as u64;
            n += 1;
        }
    }
    (n > 0).then(|| (sum / n) as u16)
}

fn main() {
    let founders: usize = arg("founders").unwrap_or(1);
    let frames: u64 = arg("frames").unwrap_or(24_000);
    let every: u64 = arg("every").unwrap_or(4_000);
    let seed: u64 = arg("seed").unwrap_or(1);
    let spec = LabBox { founders, colonies: 0, seed, ..LabBox::default() };
    println!(
        "soil_drawdown: founders={founders} frames={frames} every={every} seed={seed} \
         (field capacity {}, wilting point {}, saturated {})",
        material::SOIL_FIELD_CAPACITY,
        material::SOIL_WILTING_POINT,
        material::SOIL_SATURATED
    );

    let mut lab = Lab::new(spec.clone());
    let start_bank = lab.world.atmospheric_bank;
    let start_total = {
        let w = &lab.world;
        let mut t = 0u64;
        for y in spec.ground_y..(spec.ground_y + spec.soil_depth) {
            for x in 0..spec.width {
                let c = w.get(x, y);
                if w.materials.get(c.material).water_capacity > 0 {
                    t += update::soil_moisture(c) as u64;
                }
            }
        }
        t
    };
    // Where the founder actually is, so "distance from the plant" is read off
    // the scene rather than assumed.
    let planted: Vec<i32> = spec.founder_columns();
    println!("  founder column(s): {planted:?}\n");

    for f in 0..=frames {
        if f % every == 0 {
            let w = &lab.world;
            // Horizontal: the row a root drinks from, in eighths of the bed.
            let row = spec.ground_y + 2;
            let step = spec.width / 8;
            let across: Vec<String> = (0..8)
                .map(|i| {
                    let x0 = i * step;
                    match row_mean(w, &spec, row, x0, x0 + step) {
                        Some(m) => format!("{m:>4}"),
                        None => "   -".to_string(),
                    }
                })
                .collect();
            // Vertical: down the middle of the bed.
            let down: Vec<String> = (0..8)
                .map(|i| {
                    let y = spec.ground_y + i * (spec.soil_depth / 8);
                    match row_mean(w, &spec, y, 0, spec.width) {
                        Some(m) => format!("{m:>4}"),
                        None => "   -".to_string(),
                    }
                })
                .collect();
            // Totals, split by which side of field capacity they sit on --
            // a ratchet moves water between these two without changing the
            // sum, a sink shrinks the sum.
            let (mut total, mut below) = (0u64, 0u64);
            for y in spec.ground_y..(spec.ground_y + spec.soil_depth) {
                for x in 0..spec.width {
                    let c = w.get(x, y);
                    if w.materials.get(c.material).water_capacity == 0 {
                        continue;
                    }
                    let m = update::soil_moisture(c) as u64;
                    total += m;
                    below += (material::SOIL_FIELD_CAPACITY as u64).saturating_sub(m);
                }
            }
            // **Which door the water left by.** `evaporation::tick` credits
            // `atmospheric_bank`; `plant::transpire` credits nothing at all.
            // So bank growth is the sun's take and the rest of the shortfall
            // is the plant's -- the two are otherwise indistinguishable on an
            // overlay, and they want completely different fixes.
            let banked = (w.atmospheric_bank - start_bank) * material::LIQUID_FULL as f64;
            let lost = start_total as f64 - total as f64;
            println!(
                "  frame {f:>6}  across [{}]  down [{}]  total {total:>9}  deficit {below:>8}  \
                 lost {lost:>9.0} = sun {banked:>9.0} + plant {:>9.0}",
                across.join(" "),
                down.join(" "),
                lost - banked
            );
        }
        if f < frames {
            tick(&mut lab);
        }
    }
}
