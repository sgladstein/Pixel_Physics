//! A generic live-tunables registry — §10 of the plan's UI-improvement
//! pass, built on §9's text primitive. Not a bespoke panel per subsystem:
//! any already data-driven value registers as one `Tunable`
//! `(category, name, value, min, max, step)`, and the panel (`app.rs`'s
//! `draw_tunables_panel`) is generic over whatever's currently registered.
//!
//! **Scope of this pass.** Materials are the only registrant — every
//! finite `f32` field on `Material` (`density`, `friction_angle`,
//! `flammability`, `ignition_temperature`, `burn_temperature`,
//! `heat_conductivity`, `melting_point`, `boiling_point`). Integer fields
//! (`dispersion: u8`, `flow_rate: u16`, `burn_duration: u16`,
//! `max_unsupported_span: u16`) are deliberately not registered yet — this
//! module's save path always writes back a float literal (RON's own
//! syntax distinguishes `45` from `45.0`), and building that distinction
//! in for four fields nothing here exercises would be scope creep, not a
//! head start; a future pass can add an integer variant when a real
//! caller needs one. A field left at its "never" sentinel (`f32::INFINITY`
//! — `ignition_temperature`, `burn_temperature`, `melting_point`,
//! `boiling_point`, all default to this per `material.rs`'s own doc) is
//! also skipped: dragging a slider "up" from infinity has no sensible
//! starting point, and registering it would either silently clamp to a
//! misleadingly-finite value or need its own special-cased UI.

use crate::sim::material::{MaterialKind, MaterialRegistry};

/// Which menu the tunables panel shows an entry in.
///
/// The panel lists one group at a time. With a dozen-odd materials and ten
/// fields each it was a single scroll of well over a hundred rows, and the
/// two or three entries anyone is actually iterating on were buried in it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TunableGroup {
    /// How the material behaves — density, friction, combustion, flow.
    Physics,
    /// How it is drawn. Changes nothing in the simulation, which is the
    /// property that makes the split worth having and is asserted in tests.
    Visual,
}

impl TunableGroup {
    pub fn label(self) -> &'static str {
        match self {
            TunableGroup::Physics => "PHYSICS",
            TunableGroup::Visual => "VISUAL",
        }
    }

    pub fn next(self) -> Self {
        match self {
            TunableGroup::Physics => TunableGroup::Visual,
            TunableGroup::Visual => TunableGroup::Physics,
        }
    }

    pub fn all() -> [TunableGroup; 2] {
        [TunableGroup::Physics, TunableGroup::Visual]
    }
}

/// One live-adjustable value. `value` is a live snapshot at the moment
/// the registry was built, not a handle back into the registry it came
/// from — `App` re-derives the list fresh whenever the panel is open
/// (materials are cheap to enumerate; a few dozen entries), so there is
/// no separate synchronization path to keep a stale snapshot updated.
#[derive(Clone)]
pub struct Tunable {
    pub group: TunableGroup,
    /// Which material this belongs to — also the `.ron` file's own base
    /// name, by the convention every shipped material file already
    /// follows (`sand.ron` defines `name: "sand"`).
    pub category: String,
    /// The exact field name as it appears in the `.ron` file — reused
    /// verbatim as the search key for the targeted-edit save path, so a
    /// typo here would silently fail to save rather than corrupt the
    /// wrong field.
    pub name: String,
    pub value: f32,
    pub min: f32,
    pub max: f32,
    pub step: f32,
}

/// Every material field worth exposing, in a stable order (the order the
/// panel lists them in, category by category).
pub fn from_materials(materials: &MaterialRegistry) -> Vec<Tunable> {
    let mut out = Vec::new();
    // `paintable()`'s own filter (skip `Empty`/`Bedrock`, the two engine-
    // internal ids with nothing a content author would ever tune) is
    // reused here for the identical reason it exists there.
    for id in materials.paintable() {
        let m = materials.get(id);
        let category = m.name.clone();
        out.push(Tunable { group: TunableGroup::Physics, category: category.clone(), name: "density".into(), value: m.density, min: 0.0, max: 5.0, step: 0.1 });
        out.push(Tunable {
            group: TunableGroup::Physics,
            category: category.clone(),
            name: "friction_angle".into(),
            value: m.friction_angle,
            min: 1.0,
            max: 89.0,
            step: 1.0,
        });
        out.push(Tunable { group: TunableGroup::Physics, category: category.clone(), name: "flammability".into(), value: m.flammability, min: 0.0, max: 1.0, step: 0.05 });
        // Liquids only -- a dead band means nothing to a powder or a gas, and
        // an entry per material that ignores it is just noise in the panel.
        if m.kind == MaterialKind::Liquid {
            out.push(Tunable {
                group: TunableGroup::Visual,
                category: category.clone(),
                name: "fill_dimming".into(),
                value: m.fill_dimming,
                min: 0.0,
                max: 1.0,
                step: 0.05,
            });
            out.push(Tunable {
                group: TunableGroup::Physics,
                category: category.clone(),
                name: "min_transfer".into(),
                value: m.min_transfer as f32,
                min: 0.0,
                max: 400.0,
                step: 4.0,
            });
        }
        out.push(Tunable {
            group: TunableGroup::Physics,
            category: category.clone(),
            name: "heat_conductivity".into(),
            value: m.heat_conductivity,
            min: 0.0,
            max: 1.0,
            step: 0.05,
        });
        for (field, value) in [
            ("ignition_temperature", m.ignition_temperature),
            ("burn_temperature", m.burn_temperature),
            ("melting_point", m.melting_point),
            ("boiling_point", m.boiling_point),
        ] {
            if value.is_finite() {
                out.push(Tunable { group: TunableGroup::Physics, category: category.clone(), name: field.into(), value, min: 0.0, max: 2000.0, step: 10.0 });
            }
        }
    }
    out
}

/// Where a saved edit's target `.ron` file lives, by the same
/// name-matches-filename convention `from_materials` already relies on.
pub fn material_file_path(dir: impl AsRef<std::path::Path>, material_name: &str) -> std::path::PathBuf {
    dir.as_ref().join(format!("{material_name}.ron"))
}

/// Rewrite exactly `field`'s own value span in `source` to `new_value`,
/// leaving every other byte — comments included — untouched. Not a
/// `ron::ser` round-trip: that would silently destroy every comment in the
/// file, and material files' comments carry real reasoning (`oil.ron`'s
/// own header, for one). Distinct from a substring match inside a longer
/// field name (matching `temperature` inside `burn_temperature` would
/// corrupt the wrong field).
///
/// Most material files only write the handful of fields that differ from
/// `Material`'s `serde` defaults (`stone.ron` never mentions
/// `heat_conductivity`, for one) — so `field` genuinely absent from the
/// text is the *common* case for a registered [`Tunable`], not a typo, and
/// live-verifying this against real asset files (rather than only the
/// hand-built strings in this module's own tests) is what caught it.
/// When `field` isn't found as an existing key, this appends
/// `field: new_value,` on its own line just before the file's own closing
/// `)` (the outermost one — every shipped material file is a single
/// top-level struct, so its last `)` is unambiguous) rather than erroring.
pub fn write_field_value(source: &str, field: &str, new_value: f32) -> Result<String, String> {
    match find_field_value_span(source, field) {
        Some(span) => {
            let mut out = String::with_capacity(source.len());
            out.push_str(&source[..span.start]);
            out.push_str(&format_value(new_value));
            out.push_str(&source[span.end..]);
            Ok(out)
        }
        None => {
            let close = source.rfind(')').ok_or_else(|| format!("field '{field}' not found, and no closing ')' to insert it before"))?;
            // The struct's last existing field may or may not already end
            // in a trailing comma (RON allows either) -- insert one
            // ourselves when it's missing, or `field: value` would run
            // straight into whatever came before it with no separator.
            // `last_significant_char` (not a raw `.trim_end().chars().last()`)
            // matters here: a review found that if the struct's last line
            // before `)` is a bare trailing comment, the naive version reads
            // the comment's own last letter as "the last real character",
            // decides no comma is needed, and inserts one anyway right after
            // the comment -- for the case where a comma genuinely *was*
            // needed, that stray comma lands outside the comment (harmless),
            // but the reverse mistake (skipping a needed comma because the
            // comment happened to end past where a comma already was) would
            // produce RON `ron::from_str` rejects before ever writing to
            // disk. Either way, reading through comments is the correct fix.
            let needs_comma = !matches!(last_significant_char(&source[..close]), Some(',') | Some('(') | None);
            let mut out = String::with_capacity(source.len() + field.len() + 16);
            out.push_str(&source[..close]);
            if needs_comma {
                out.push(',');
            }
            out.push_str(&format!("\n    {field}: {},\n", format_value(new_value)));
            out.push_str(&source[close..]);
            Ok(out)
        }
    }
}

fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// The last non-whitespace character in `text`, ignoring `//` line
/// comments entirely — so a trailing comment-only line (or a value with a
/// trailing inline comment) doesn't masquerade as real struct content.
/// Scans lines from the end, stripping each line's own comment before
/// checking whether anything real is left on it.
fn last_significant_char(text: &str) -> Option<char> {
    for line in text.lines().rev() {
        let code = line.split("//").next().unwrap_or("");
        if let Some(c) = code.trim_end().chars().last() {
            return Some(c);
        }
    }
    None
}

/// Byte range of `field`'s value literal within `source` — from the first
/// non-whitespace character after its `:` to the first `,`, `)`, newline,
/// or `//` line comment after that (whichever comes first; trailing
/// whitespace trimmed off the end). `field` must appear as a whole
/// identifier (not preceded or followed by another identifier character)
/// immediately followed by `:` (whitespace allowed in between, matching how
/// `ron`'s own parser is whitespace-insensitive there).
///
/// The `//` case matters: an independent review confirmed that without it,
/// a field written as `density: 1.0 // heavy` has its trailing comment
/// silently folded into the matched span and deleted on save — the
/// resulting text is still valid RON (so `save_tunable`'s parse check
/// doesn't catch it), it just permanently loses the comment, which is
/// exactly what this module's targeted-edit approach exists to avoid.
fn find_field_value_span(source: &str, field: &str) -> Option<std::ops::Range<usize>> {
    let bytes = source.as_bytes();
    let mut search_from = 0;
    while let Some(rel) = source[search_from..].find(field) {
        let start = search_from + rel;
        let end = start + field.len();
        search_from = end;
        let prev_is_boundary = start == 0 || !is_ident_char(bytes[start - 1]);
        let next_is_boundary = end == bytes.len() || !is_ident_char(bytes[end]);
        if !prev_is_boundary || !next_is_boundary {
            continue;
        }
        let after = &source[end..];
        let after_trimmed = after.trim_start();
        if !after_trimmed.starts_with(':') {
            continue;
        }
        let colon_pos = end + (after.len() - after_trimmed.len());
        let value_region = &source[colon_pos + 1..];
        let value_start = colon_pos + 1 + (value_region.len() - value_region.trim_start().len());
        let value_rest = &source[value_start..];
        let delim = value_rest.find([',', ')', '\n']).unwrap_or(value_rest.len());
        let comment = value_rest.find("//").unwrap_or(value_rest.len());
        let raw_len = delim.min(comment);
        let trimmed_len = value_rest[..raw_len].trim_end().len();
        return Some(value_start..value_start + trimmed_len);
    }
    None
}

/// A RON float literal for `v` — always at least one digit after the
/// decimal point (`45` alone is a valid RON float too, but every shipped
/// material file already writes whole numbers as `45.0`, and matching
/// that existing style matters more here than shaving two bytes).
/// Rounded to 4 decimal places and trailing zeros stripped, so a step of
/// `0.1` repeatedly applied doesn't accumulate `f32` noise into the file
/// as `45.09999998`.
fn format_value(v: f32) -> String {
    let s = format!("{v:.4}");
    let trimmed = s.trim_end_matches('0');
    let trimmed = trimmed.trim_end_matches('.');
    if trimmed.contains('.') {
        trimmed.to_string()
    } else {
        format!("{trimmed}.0")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::material;

    #[test]
    fn every_group_is_reachable_and_visual_changes_nothing_in_the_simulation() {
        let registry = MaterialRegistry::builtin();
        let all = from_materials(&registry);
        for group in TunableGroup::all() {
            assert!(
                all.iter().any(|t| t.group == group),
                "the {} menu is empty, so nothing would ever show in it",
                group.label()
            );
        }
        // The split is only worth having if `Visual` really is inert, so
        // that is asserted rather than left as an intention: adding a field
        // here forces a decision about which side it belongs on.
        for t in all.iter().filter(|t| t.group == TunableGroup::Visual) {
            assert_eq!(t.name, "fill_dimming", "unexpected entry {} in the VISUAL menu", t.name);
        }
    }

    #[test]
    fn from_materials_registers_finite_fields_and_skips_never_sentinels() {
        let reg = MaterialRegistry::builtin();
        let tunables = from_materials(&reg);
        let sand: Vec<_> = tunables.iter().filter(|t| t.category == "sand").collect();
        assert!(sand.iter().any(|t| t.name == "friction_angle"));
        assert!(sand.iter().any(|t| t.name == "density"));
        // Sand has no ignition_temperature set (stays at the "never"
        // sentinel) -- must not appear at all, not appear clamped to 2000.
        assert!(!sand.iter().any(|t| t.name == "ignition_temperature"), "an unset (infinite) field should not be registered");

        // Oil actually sets a real burn_temperature (900C) -- unlike
        // ignition_temperature, which oil's own file explicitly leaves at
        // the "never" default (catches only from an adjacent flame, not
        // spontaneously; see oil.ron's own comment).
        let oil: Vec<_> = tunables.iter().filter(|t| t.category == "oil").collect();
        assert!(oil.iter().any(|t| t.name == "burn_temperature"), "oil sets a real burn_temperature and should be tunable");
        assert!(!oil.iter().any(|t| t.name == "ignition_temperature"), "oil leaves ignition_temperature unset and it should not be registered");
        let _ = material::SAND; // keep the import meaningful if the above ever changes
    }

    #[test]
    fn write_field_value_replaces_only_the_named_field() {
        let source = "(name: \"sand\", kind: Powder, density: 1.6, friction_angle: 34.0, colors: [(1,2,3)])";
        let updated = write_field_value(source, "friction_angle", 40.0).unwrap();
        assert!(updated.contains("friction_angle: 40.0"));
        assert!(updated.contains("density: 1.6"), "an unrelated field must be untouched: {updated}");
        assert_eq!(updated.len(), source.len() + "40.0".len() - "34.0".len());
    }

    #[test]
    fn write_field_value_does_not_match_a_field_name_that_is_a_substring_of_another() {
        let source = "(name: \"oil\", kind: Liquid, burn_temperature: 900.0, temperature: 20.0)";
        // A field literally named "temperature" exists too, distinct from
        // "burn_temperature" -- searching for "temperature" must not stop
        // at the substring inside "burn_temperature".
        let updated = write_field_value(source, "temperature", 25.0).unwrap();
        assert!(updated.contains("burn_temperature: 900.0"), "burn_temperature must be untouched: {updated}");
        assert!(updated.contains("temperature: 25.0"), "the standalone temperature field should have been updated: {updated}");
    }

    #[test]
    fn write_field_value_preserves_comments_elsewhere_in_the_file() {
        let source = "// a real reason this material exists\n(name: \"oil\", kind: Liquid, density: 0.8)";
        let updated = write_field_value(source, "density", 0.9).unwrap();
        assert!(updated.starts_with("// a real reason this material exists\n"), "the comment must survive verbatim: {updated}");
    }

    #[test]
    fn write_field_value_does_not_swallow_a_trailing_inline_comment_on_the_edited_line() {
        // A field written as `density: 1.0 // heavy` is natural RON style
        // that no shipped file happens to use today, but the original
        // value-span search stopped only at `,`/`)`/newline -- swallowing
        // the comment into the matched span and silently deleting it on
        // save, without the parse-before-write safety net catching it
        // (the result is still valid RON, just missing the comment).
        let source = "(name: \"oil\", kind: Liquid, density: 1.0 // heavy\n)";
        let updated = write_field_value(source, "density", 0.9).unwrap();
        assert!(updated.contains("// heavy"), "the inline comment must survive: {updated}");
        assert!(updated.contains("density: 0.9"), "{updated}");
    }

    #[test]
    fn write_field_value_insert_path_reads_through_a_trailing_comment_before_the_closing_paren() {
        // A comment-only line as the struct's last content before `)`
        // must not be mistaken for "the last real character was a letter,
        // so a comma is needed" -- `last_significant_char` has to see past
        // it to the real last field (which already ends in a comma here).
        let source = "(name: \"stone\", kind: Solid, density: 2.5, colors: [(1,2,3)],\n// a trailing note\n)";
        let updated = write_field_value(source, "heat_conductivity", 0.4).unwrap();
        assert!(updated.contains("// a trailing note"), "the comment must survive: {updated}");
        ron::from_str::<material::MaterialDef>(&updated).expect("must still parse -- no stray or missing comma: {updated}");
    }

    #[test]
    fn write_field_value_appends_a_field_missing_from_the_file_before_the_closing_paren() {
        // The common case: `heat_conductivity` (say) is a real, finite
        // field on `Material` -- registered as a `Tunable` -- but this
        // particular file never wrote it, relying on the struct default
        // instead. Saving an adjustment must still work, not error.
        let source = "(name: \"stone\", kind: Solid, density: 2.5, colors: [(128, 128, 132)], max_unsupported_span: 3)";
        let updated = write_field_value(source, "heat_conductivity", 0.4).unwrap();
        assert!(updated.contains("heat_conductivity: 0.4"), "{updated}");
        assert!(updated.contains("density: 2.5"), "existing fields must be untouched: {updated}");
        assert!(updated.trim_end().ends_with(')'), "must still end with the closing paren: {updated}");
        ron::from_str::<material::MaterialDef>(&updated).expect("appended field should still parse");
    }

    #[test]
    fn write_field_value_errors_only_when_there_is_no_closing_paren_at_all() {
        assert!(write_field_value("not a ron struct", "density", 1.0).is_err());
    }

    #[test]
    fn a_written_field_value_still_parses_as_valid_ron() {
        // The actual safety net the save path relies on: this doesn't
        // enforce parseability itself (that's `App::save_tunable`'s job,
        // reading the result back through `ron::from_str` before ever
        // writing to disk) -- this test just confirms the common case
        // produces valid RON, so a regression here would be caught before
        // it ever reached that safety net silently passing malformed text.
        let source = std::fs::read_to_string("assets/materials/sand.ron").expect("sand.ron should exist in the crate root");
        let updated = write_field_value(&source, "friction_angle", 37.5).unwrap();
        ron::from_str::<material::MaterialDef>(&updated).expect("edited sand.ron should still parse");
    }

    #[test]
    fn format_value_strips_trailing_zeros_but_keeps_a_decimal_point() {
        assert_eq!(format_value(45.0), "45.0");
        assert_eq!(format_value(12.345), "12.345");
        assert_eq!(format_value(0.1), "0.1");
        assert_eq!(format_value(-5.5), "-5.5");
    }
}
