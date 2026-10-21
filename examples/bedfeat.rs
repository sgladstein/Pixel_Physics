//! Scratch (not for main): each hold-out layout's dig-bed features, one CSV row per layout.
use pixel_physics::train::{challenges::DigBed, holdout_layouts};
fn main() {
    let n: usize = std::env::args().nth(1).and_then(|v| v.parse().ok()).unwrap_or(192);
    println!("seed,k,depth,lamp,lens,lens_x,lens_y,lens_w,lens_h,step,step_x,step_up,side");
    for l in holdout_layouts(n) {
        let b = DigBed::of(l);
        let (lx, ly, lw, lh) = b.lens.unwrap_or((0, 0, 0, 0));
        let (sx, su) = b.step.unwrap_or((0, 0));
        println!("{},{},{},{},{},{lx},{ly},{lw},{lh},{},{sx},{su},{}", l.seed, l.k, b.depth, b.lamp, u8::from(b.lens.is_some()), u8::from(b.step.is_some()), b.side);
    }
}
