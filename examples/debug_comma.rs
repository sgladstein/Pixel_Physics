//! TEMPORARY — delete after use. Reproduces the tunables-panel save error.
use pixel_physics::sim::material::{self, MaterialDef};
use pixel_physics::tunables;

fn main() {
    let path = tunables::material_file_path(material::ASSET_DIR, "water");
    let source = std::fs::read_to_string(&path).expect("read water.ron");
    for (field, value, integral) in [("min_transfer", 60.0f32, true), ("flow_rate", 800.0, true), ("density", 1.0, false)] {
        let updated = tunables::write_field_value(&source, field, value, integral).expect("write");
        match ron::from_str::<MaterialDef>(&updated) {
            Ok(_) => println!("{field:>14} = {value}  -> parses OK"),
            Err(e) => {
                println!("{field:>14} = {value}  -> FAILS: {e}");
                let line = updated.lines().find(|l| l.contains(field)).unwrap_or("?");
                println!("{:>16}wrote: {}", "", line.trim());
            }
        }
    }
}
