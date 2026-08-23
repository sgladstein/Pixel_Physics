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
    let mut diff = false;
    for arg in std::env::args().skip(1) {
        match arg.split_once('=') {
            Some(("crop", v)) => {
                let n: Vec<u32> = v.split(',').map(|t| t.parse().expect("crop=x,y,w,h")).collect();
                assert_eq!(n.len(), 4, "crop=x,y,w,h");
                crop = Some((n[0], n[1], n[2], n[3]));
            }
            Some(("diff", v)) => diff = v != "0",
            Some((k, _)) => panic!("unknown argument {k:?}"),
            None => files.push(arg),
        }
    }
    assert!(!files.is_empty(), "give at least one png");

    // **`diff=1` compares two renders cell by cell**, which is a different
    // question from how noisy each is and the one that keeps coming up: did
    // this change actually alter the picture? Added after a before/after
    // pair was judged by eye, called identical, and needed a number to
    // settle it — two images that differ by a few percent over a narrow band
    // look the same and are not.
    //
    // Reports the mean and max absolute luma difference and, when there is
    // one, the column profile of the difference — because an artifact that
    // is a *vertical seam* shows up as a step in that profile and as almost
    // nothing in a whole-image mean.
    if diff {
        assert_eq!(files.len(), 2, "diff=1 needs exactly two images");
        let a = image::open(&files[0]).unwrap_or_else(|e| panic!("{}: {e}", files[0])).to_rgb8();
        let b = image::open(&files[1]).unwrap_or_else(|e| panic!("{}: {e}", files[1])).to_rgb8();
        assert_eq!(a.dimensions(), b.dimensions(), "images differ in size");
        let (iw, ih) = a.dimensions();
        let (x0, y0, w, h) = crop.unwrap_or((0, 0, iw, ih));
        let luma = |p: &image::Rgb<u8>| p.0[0] as f32 * 0.299 + p.0[1] as f32 * 0.587 + p.0[2] as f32 * 0.114;
        let (mut sum, mut max, mut max_at, mut changed) = (0.0f64, 0.0f32, (0u32, 0u32), 0u64);
        let mut cols = vec![0.0f64; w as usize];
        for y in 0..h {
            for x in 0..w {
                let d = (luma(a.get_pixel(x0 + x, y0 + y)) - luma(b.get_pixel(x0 + x, y0 + y))).abs();
                sum += d as f64;
                cols[x as usize] += d as f64;
                if d > 0.5 {
                    changed += 1;
                }
                if d > max {
                    max = d;
                    max_at = (x0 + x, y0 + y);
                }
            }
        }
        let n = (w * h) as f64;
        println!(
            "diff over {w}x{h}: mean {:.3} luma, max {max:.1} at {max_at:?}, {changed} of {} pixels differ by >0.5 ({:.1}%)",
            sum / n,
            n as u64,
            changed as f64 * 100.0 / n
        );
        // Column profile, coarsened to keep it readable: a seam is a step
        // here and invisible in the mean.
        let step = (w / 24).max(1);
        let profile: Vec<String> = (0..w)
            .step_by(step as usize)
            .map(|x| format!("{:.1}", cols[x as usize] / h as f64))
            .collect();
        println!("  per-column mean diff every {step} px from x={x0}: {}", profile.join(" "));
        return;
    }

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
