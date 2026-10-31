//! Scratch dump for the ant-look design: renders the lab box with the real
//! renderer and writes, per frame, the RGBA buffer plus every animal's chain,
//! heading and load, so a render-only sprite layer can be mocked up offline.
//! Built for `Reports/ant-sprite-layer-design-2026-10-31.md`.
//!
//! ```text
//! cargo run --release --example antlook_dump -- start=45000 frames=150 out=/tmp/antlook
//! ```
use pixel_physics::lab::scenario::Scenario;
use pixel_physics::lab::{Lab, HEIGHT, WIDTH};
use std::io::Write;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args().skip(1).find_map(|a| a.strip_prefix(&format!("{key}=")).and_then(|v| v.parse().ok()))
}

fn main() {
    let scenario: String = arg("scenario").unwrap_or_else(|| "played_bed".into());
    let seed: u64 = arg("seed").unwrap_or(1);
    let start: u64 = arg("start").unwrap_or(20_000);
    let frames: u64 = arg("frames").unwrap_or(240);
    let zoom: i32 = arg("zoom").unwrap_or(1);
    let out: String = arg("out").expect("out=dir");
    std::fs::create_dir_all(&out).unwrap();
    let mut sc = Scenario::load(&scenario).unwrap();
    sc.bed.seed = seed;
    let mut lab = Lab::new(sc.bed.clone());
    lab.show_help = false;
    lab.load_scenario(sc);
    if lab.stats.showing() {
        lab.stats.toggle();
    }
    for _ in 0..start {
        lab.tick_for_harness();
    }
    // Centre on the densest 64x40 window of ants, unless center= is given.
    let ants = |lab: &Lab| -> Vec<(i32, i32)> {
        lab.world.live_organism_ids().into_iter().filter_map(|id| {
            let s = lab.world.organism(id)?;
            lab.world.species.get(s.species).creature.as_ref()?;
            s.chain.first().copied()
        }).collect()
    };
    let heads = ants(&lab);
    let center: (i32, i32) = arg::<String>("center").map(|s| {
        let v: Vec<i32> = s.split(',').map(|p| p.parse().unwrap()).collect();
        (v[0], v[1])
    }).unwrap_or_else(|| {
        let mut best = (0usize, (0, 0));
        for &(x, y) in &heads {
            let n = heads.iter().filter(|&&(a, b)| (a - x).abs() < 32 && (b - y).abs() < 20).count();
            if n > best.0 { best = (n, (x, y)); }
        }
        best.1
    });
    let bounds = lab.world.bounds();
    for _ in 1..zoom { lab.renderer.adjust_zoom(1); }
    let (sx, sy) = lab.renderer.visible_span((WIDTH, HEIGHT));
    lab.renderer.set_camera(center.0 - sx / 2, center.1 - sy / 2, (WIDTH, HEIGHT), bounds);
    eprintln!("antlook_dump: {scenario} seed={seed} start={start} ants={} world={}x{} centre={center:?} zoom={zoom} cam=({},{}) span={sx}x{sy}",
        heads.len(), lab.spec.width, lab.spec.height, lab.renderer.camera_x, lab.renderer.camera_y);
    let mut meta = std::fs::File::create(format!("{out}/ants.jsonl")).unwrap();
    writeln!(meta, "{{\"w\":{WIDTH},\"h\":{HEIGHT},\"zoom\":{},\"cam\":[{},{}],\"world\":[{},{}]}}",
        lab.renderer.zoom, lab.renderer.camera_x, lab.renderer.camera_y, lab.spec.width, lab.spec.height).unwrap();
    let mut buf = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
    let mut raw = std::fs::File::create(format!("{out}/frames.rgba")).unwrap();
    for _ in 0..frames {
        lab.tick_for_harness();
        let touched = lab.world.take_touched_chunks();
        lab.renderer.draw(&lab.world, &lab.particles, &touched, &mut buf, (WIDTH, HEIGHT), true);
        raw.write_all(&buf).unwrap();
        let mut line = format!("{{\"f\":{},\"a\":[", lab.world.frame);
        let mut first = true;
        for id in lab.world.live_organism_ids() {
            let Some(s) = lab.world.organism(id) else { continue };
            let sp = lab.world.species.get(s.species);
            if sp.creature.is_none() || s.chain.is_empty() { continue; }
            let chain: Vec<String> = s.chain.iter().map(|(x, y)| format!("[{x},{y}]")).collect();
            let crop = s.crop.as_ref().map(|c| lab.world.materials.get(c.material).name.clone()).unwrap_or_default();
            let spoil = s.spoil.as_ref().map(|c| lab.world.materials.get(c.cell.material).name.clone()).unwrap_or_default();
            if !first { line.push(','); }
            first = false;
            line.push_str(&format!("{{\"id\":{},\"sp\":\"{}\",\"c\":[{}],\"h\":{},\"crop\":\"{}\",\"spoil\":\"{}\",\"fly\":{},\"e\":{:.0}}}",
                id, sp.name, chain.join(","), s.heading, crop, spoil, s.flight.is_some(), s.energy));
        }
        line.push_str("]}");
        writeln!(meta, "{line}").unwrap();
    }
}
