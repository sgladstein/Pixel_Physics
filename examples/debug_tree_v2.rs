//! Throwaway visual-verification harness for `Reports/tree-rewrite-
//! design.md`'s new Grow/Germinate-driven tree system --
//! `Reports/tree-rewrite-design.md` §11 step 6, the actual gate: does
//! this produce something that reads as a tree, not just something that
//! passes narrow unit tests. Plants via `plant_tree_v2` (the new system)
//! and, for direct comparison, `plant_tree` (the old, untouched
//! TreeState-based system) side by side in the same scene.

use pixel_physics::app::{App, HEIGHT, WIDTH};

fn save_png(app: &mut App, path: &str) {
    let mut frame = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
    // Force a full redraw (an off-screen cursor is enough -- `App::draw`
    // forces `force_full` whenever `cursor.is_some()`, regardless of
    // position) rather than relying on the dirty-rect `touched_chunks`
    // accumulator, since this harness allocates a fresh zeroed frame buffer
    // per checkpoint instead of reusing one persistent buffer across many
    // frames the way the real renderer loop does -- without this, a
    // checkpoint following a quiet stretch (nothing touched since the last
    // save) would paint nothing at all and save a blank frame.
    app.draw(&mut frame, Some((-1000, -1000)));
    image::save_buffer(path, &frame, WIDTH, HEIGHT, image::ColorType::Rgba8).unwrap();

    // A second copy, cropped to the new tree's own neighbourhood and
    // upscaled 4x -- a whole-world 512x320 shot makes a ~30-cell-tall
    // sapling nearly indistinguishable from a smudge; this is the one
    // actually worth reading.
    let (cx0, cy0, cx1, cy1) = (75i32, 0i32, 135i32, 30i32);
    let (cw, ch) = ((cx1 - cx0) as u32, (cy1 - cy0) as u32);
    const ZOOM: u32 = 12;
    let mut cropped = vec![0u8; (cw * ZOOM * ch * ZOOM * 4) as usize];
    for y in 0..ch {
        for x in 0..cw {
            let src = (((cy0 + y as i32) * WIDTH as i32 + (cx0 + x as i32)) * 4) as usize;
            let px = &frame[src..src + 4];
            for zy in 0..ZOOM {
                for zx in 0..ZOOM {
                    let dx = x * ZOOM + zx;
                    let dy = y * ZOOM + zy;
                    let dst = ((dy * cw * ZOOM + dx) * 4) as usize;
                    cropped[dst..dst + 4].copy_from_slice(px);
                }
            }
        }
    }
    let cropped_path = path.replace(".png", "-zoomed.png");
    image::save_buffer(&cropped_path, &cropped, cw * ZOOM, ch * ZOOM, image::ColorType::Rgba8).unwrap();
}

fn main() {
    let mut app = App::new();
    let dir = "docs/screenshots/tree-rewrite-live-verification";
    std::fs::create_dir_all(dir).ok();

    // A floor to plant on, with real open-sky clearance above it but not
    // hugging the world's own hard y=0 boundary. `field.rs`'s light model
    // is a steep "diffuse fast, decay hard" local glow around open sky/fire
    // (README: "a field cell two rows below the sky already reads under 6%
    // of MAX_LIGHT"), which only cares about distance from open sky, not
    // distance from y=0 -- so this doesn't need to hug the top of the world
    // to get light. An earlier version of this harness planted at y=259 in
    // a 320-tall world (germination never fired: light unreachable that
    // deep), then at y=19 (germination fired, but the tree visually grew as
    // a dense round clump, not a tree silhouette -- turned out to be a
    // second, separate test-scene bug: y=19 left only 19 cells of clearance
    // before the world's own hard top boundary at y=0, so the canopy hit
    // that ceiling almost immediately and had nowhere to go but sideways).
    // y=100 keeps the same open-sky exposure while giving ~100 cells of
    // real room to grow upward into. Also clear of `build_terrain`'s own
    // ledges (y=150/200/260, at various x-ranges) so nothing above either
    // seed blocks light that wasn't there on purpose.
    use pixel_physics::sim::cell::Cell;
    use pixel_physics::sim::material;
    let floor_y = 20;
    let seed_y = floor_y - 1;
    for x in 60..460 {
        app.world.set(x, floor_y, Cell::new(material::STONE, 0));
    }
    // A puddle near (but not overlapping) each tree's own seed cell, so
    // hydrotropism/Absorb has something real to respond to -- offset from
    // the seed's own x position, not the same row, so painting it doesn't
    // silently occupy the cell `plant_tree_v2`/`plant_tree` are about to
    // check `is_empty` on (an earlier version of this harness planted the
    // seed directly into the puddle's own row and range, which made both
    // planting calls silent no-ops).
    for x in 70..85 {
        app.world.set(x, seed_y, Cell::new(material::WATER, 0));
    }
    for x in 320..335 {
        app.world.set(x, seed_y, Cell::new(material::WATER, 0));
    }

    // New system, left.
    app.plant_tree_v2(100, seed_y);
    // Old system, right, for direct comparison.
    app.plant_tree(350, seed_y);

    {
        let cell = app.world.get(100, seed_y);
        let field = app.world.field_at(100, seed_y);
        println!(
            "right after planting: material={:?} organism_id={} aux={} light={} moisture={}",
            app.world.materials.get(cell.material).name,
            cell.organism_id(),
            cell.aux(),
            field.light,
            field.moisture
        );
    }

    for n in [200, 1000, 3000, 6000] {
        for _ in 0..n {
            app.update();
        }
        save_png(&mut app, &format!("{dir}/after-{n}-more-ticks.png"));
        let wood = app.world.materials.id_of("wood").unwrap();
        let new_tree_wood_cells = (40..160)
            .flat_map(|x| (0..floor_y + 2).map(move |y| (x, y)))
            .filter(|&(x, y)| app.world.get(x, y).material == wood)
            .count();
        let seed_cell = app.world.get(100, seed_y);
        let (cell_type, resource) = pixel_physics::sim::organism::unpack_aux(seed_cell.aux());
        let field = app.world.field_at(100, seed_y);
        let sky_field = app.world.field_at(100, 3); // field row 0, world y 0..7
        println!(
            "after {n} more ticks: new-tree wood cells nearby = {new_tree_wood_cells}, active sites = {}, seed-pos cell_type={:?} resource={:.2} light={:.3} moisture={:.3}, sky-row light={:.3}, active_chunks={}",
            app.world.active_site_count(),
            cell_type,
            resource,
            field.light,
            field.moisture,
            sky_field.light,
            app.world.active_chunk_count()
        );
    }

    println!("saved to {dir}");
    println!("ok");
}
