//! How noisy a rendered region is, as a number, so two strips can be
//! compared without squinting at them.
//!
//! Written the moment a render change that measurably fired failed to change
//! the picture: `render.rs`'s grain was cut to **zero** at depth and deep
//! rock still read as television static, which means the static was never
//! the grain. An image says *what and where* and cannot apportion a texture
//! between two mechanisms that produce the same picture; a paired difference
//! of local variance can.
//!
//! Reports **mean absolute deviation from the 3x3 neighbourhood mean**,
//! rather than plain variance: the quantity the eye reads as "speckle" is
//! per-pixel departure from its immediate surroundings, and a smooth
//! large-scale gradient (a strata band, the depth ramp) must not count as
//! noise. A region that is a clean ramp scores near zero however much its
//! ends differ.
//!
//! ```text
//! cargo run --release --example pixel_stat -- a.png b.png crop=0,0,900,200
//! ```

fn main() {
    let mut files: Vec<String> = Vec::new();
    let mut crop: Option<(u32, u32, u32, u32)> = None;
    for arg in std::env::args().skip(1) {
        match arg.split_once('=') {
            Some(("crop", v)) => {
                let n: Vec<u32> = v.split(',').map(|t| t.parse().expect("crop=x,y,w,h")).collect();
                assert_eq!(n.len(), 4, "crop=x,y,w,h");
                crop = Some((n[0], n[1], n[2], n[3]));
            }
            Some((k, _)) => panic!("unknown argument {k:?}"),
            None => files.push(arg),
        }
    }
    assert!(!files.is_empty(), "give at least one png");

    for path in &files {
        let img = image::open(path).unwrap_or_else(|e| panic!("{path}: {e}")).to_rgb8();
        let (iw, ih) = img.dimensions();
        let (x0, y0, w, h) = crop.unwrap_or((0, 0, iw, ih));
        assert!(x0 + w <= iw && y0 + h <= ih, "{path}: crop is outside the {iw}x{ih} image");

        let lum = |x: u32, y: u32| {
            let p = img.get_pixel(x0 + x, y0 + y).0;
            // Rec. 601 luma: what is being compared is brightness speckle,
            // not hue -- chroma is measured separately below.
            p[0] as f32 * 0.299 + p[1] as f32 * 0.587 + p[2] as f32 * 0.114
        };

        // Chroma alongside luma, because the two mechanisms that can
        // produce speckle here are distinguishable by it and by nothing
        // else: a *tone* jump inside one palette family moves brightness
        // only, while a *family* dither swaps grey for sandstone and moves
        // hue. Reporting one number would have left the apportionment to
        // whoever was looking at the picture, which is the thing this
        // instrument exists to stop.
        let chroma = |x: u32, y: u32| {
            let p = img.get_pixel(x0 + x, y0 + y).0;
            p[0] as f32 - p[2] as f32
        };
        let mut csum = 0.0f64;
        let mut sum = 0.0f64;
        let mut n = 0u64;
        let mut worst = 0.0f32;
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let mut mean = 0.0;
                for dy in 0..3u32 {
                    for dx in 0..3u32 {
                        mean += lum(x + dx - 1, y + dy - 1);
                    }
                }
                mean /= 9.0;
                let d = (lum(x, y) - mean).abs();
                sum += d as f64;
                worst = worst.max(d);
                let mut cmean = 0.0;
                for dy in 0..3u32 {
                    for dx in 0..3u32 {
                        cmean += chroma(x + dx - 1, y + dy - 1);
                    }
                }
                csum += (chroma(x, y) - cmean / 9.0).abs() as f64;
                n += 1;
            }
        }
        println!(
            "{path}  crop {x0},{y0},{w}x{h}:  luma MAD {:.3}  chroma MAD {:.3}  worst {:.1}  (n={n})",
            sum / n as f64,
            csum / n as f64,
            worst
        );
    }
}
