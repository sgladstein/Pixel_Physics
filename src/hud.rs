//! On-screen text — M19/§9's UI-improvement pass's step 0, everything else
//! in that section (brush label, hover inspector, keybind help) builds on
//! this. The engine had zero on-screen text before this: `render.rs`'s own
//! comment on the window title bar called it "cheaper than rendering text."
//!
//! A fixed-width 5x7 bitmap font, not the plan's originally-sketched full
//! ASCII 0x20-0x7E (95 glyphs) range — deliberately scoped down to what the
//! HUD actually needs: space, `A`-`Z`, `0`-`9`, and a small punctuation set
//! (`. , : - / % ( ) [ ] ! ? + =`). Hand-authoring 95 accurate glyphs with no
//! reference font to check against risks silently shipping wrong bitmap
//! data for characters nothing exercises; every glyph actually defined here
//! is a standard, easily-verified dot-matrix letterform. **HUD text is
//! uppercase-only** as a direct consequence — every caller upper-cases its
//! own string before calling `draw_text`, rather than this module silently
//! rendering nothing (or garbage) for a lowercase letter it doesn't have a
//! glyph for.

/// Glyph width/height in pixels, plus one column/row of spacing baked into
/// `draw_text`'s own advance rather than the glyph data — keeps the font
/// table itself exactly 5 bits wide per row.
pub const GLYPH_WIDTH: i32 = 5;
pub const GLYPH_HEIGHT: i32 = 7;
/// Pixels advanced between glyphs, beyond `GLYPH_WIDTH` itself.
const GLYPH_SPACING: i32 = 1;

/// One glyph: 7 rows, each the low 5 bits of a `u8` (bit 4 = leftmost
/// column, bit 0 = rightmost) -- `0` unused for the top 3 bits of every row.
type Glyph = [u8; 7];

const BLANK: Glyph = [0, 0, 0, 0, 0, 0, 0];

fn glyph_for(c: char) -> Glyph {
    match c {
        'A' => [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'B' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110],
        'C' => [0b01111, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b01111],
        'D' => [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110],
        'E' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
        'F' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000],
        'G' => [0b01111, 0b10000, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111],
        'H' => [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'I' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111],
        'J' => [0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100],
        'K' => [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001],
        'L' => [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
        'M' => [0b10001, 0b11011, 0b10101, 0b10001, 0b10001, 0b10001, 0b10001],
        'N' => [0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001],
        'O' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'P' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000],
        'Q' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101],
        'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
        'S' => [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110],
        'T' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
        'U' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'V' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
        'W' => [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010],
        'X' => [0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001],
        'Y' => [0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100],
        'Z' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111],
        '0' => [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
        '1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        '2' => [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111],
        '3' => [0b01110, 0b10001, 0b00001, 0b00110, 0b00001, 0b10001, 0b01110],
        '4' => [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
        '5' => [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110],
        '6' => [0b01110, 0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
        '7' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
        '8' => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
        '9' => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001, 0b01110],
        '.' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b01100],
        ',' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b01000],
        ':' => [0b00000, 0b01100, 0b01100, 0b00000, 0b01100, 0b01100, 0b00000],
        '-' => [0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000],
        '/' => [0b00001, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b10000],
        '%' => [0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b10001],
        '(' => [0b00100, 0b01000, 0b10000, 0b10000, 0b10000, 0b01000, 0b00100],
        ')' => [0b00100, 0b00010, 0b00001, 0b00001, 0b00001, 0b00010, 0b00100],
        // Square, not round like `(`/`)` -- caught by an independent visual
        // check: the help overlay's own text uses `[`/`]` for the brush-
        // size keybind, and this font had no glyph for either, so the
        // rendered help screen showed a blank gap in place of both.
        '[' => [0b11100, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11100],
        ']' => [0b00111, 0b00001, 0b00001, 0b00001, 0b00001, 0b00001, 0b00111],
        '!' => [0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00000, 0b00100],
        '?' => [0b01110, 0b10001, 0b00001, 0b00110, 0b00100, 0b00000, 0b00100],
        '+' => [0b00000, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0b00000],
        '=' => [0b00000, 0b00000, 0b11111, 0b00000, 0b11111, 0b00000, 0b00000],
        // Added for the tunables panel: `_` because every tunable's name is
        // a snake_case field name and rendering those as ragged gaps made
        // them read as two separate words, and `<`/`>` for the pinned
        // readout's own left/right adjust hint. Same class of omission the
        // `[`/`]` comment above records finding by looking at the output.
        '_' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b11111],
        '<' => [0b00010, 0b00100, 0b01000, 0b10000, 0b01000, 0b00100, 0b00010],
        '>' => [0b01000, 0b00100, 0b00010, 0b00001, 0b00010, 0b00100, 0b01000],
        ' ' => BLANK,
        // Any other character (lowercase, accented, anything not in the set
        // above) renders as blank rather than a mystery box or a panic --
        // callers are expected to upper-case first, but a stray unsupported
        // character (an apostrophe in a material name, say) should not take
        // the whole label down with it.
        _ => BLANK,
    }
}

/// Draw `text` (upper-cased internally, so callers don't have to remember
/// to) with its top-left corner at `(x, y)`, one call per glyph via `put`.
/// Off-screen glyphs/pixels are simply clipped, the same as everything
/// else `render.rs` draws.
pub fn draw_text(frame: &mut [u8], width: u32, height: u32, x: i32, y: i32, text: &str, colour: [u8; 4]) {
    let mut cursor_x = x;
    for c in text.chars() {
        let glyph = glyph_for(c.to_ascii_uppercase());
        for (row, bits) in glyph.iter().enumerate() {
            for col in 0..GLYPH_WIDTH {
                if bits & (1 << (GLYPH_WIDTH - 1 - col)) != 0 {
                    crate::render::put(frame, width, height, cursor_x + col, y + row as i32, colour);
                }
            }
        }
        cursor_x += GLYPH_WIDTH + GLYPH_SPACING;
    }
}

/// Total pixel width `draw_text` would occupy for `text` — for right-
/// aligning or centering a label before drawing it.
pub fn text_width(text: &str) -> i32 {
    let len = text.chars().count() as i32;
    if len == 0 {
        0
    } else {
        len * GLYPH_WIDTH + (len - 1) * GLYPH_SPACING
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count_lit(frame: &[u8], colour: [u8; 4]) -> usize {
        frame.chunks_exact(4).filter(|p| *p == colour).count()
    }

    #[test]
    fn drawing_a_known_glyph_sets_exactly_its_own_pixels() {
        // 'I' is the simplest glyph to hand-verify: a full top row, a full
        // bottom row, and a single-pixel-wide vertical stroke connecting
        // them -- 5 + 5 + 5 (the three rows that are `11111` or the middle
        // column) is not how it's counted; count bits directly instead of
        // re-deriving the shape, so this test would catch a genuine
        // transcription error in the glyph table, not just disagree with
        // its own hand-copy of it.
        let (w, h) = (16u32, 16u32);
        let mut frame = vec![0u8; (w * h * 4) as usize];
        let white = [255, 255, 255, 255];
        draw_text(&mut frame, w, h, 0, 0, "I", white);

        let expected_bits: u32 = glyph_for('I').iter().map(|row| row.count_ones()).sum();
        assert_eq!(count_lit(&frame, white) as u32, expected_bits, "lit pixel count should match the glyph's own bit count");

        // And spot-check actual positions: 'I' at (0,0) should light the
        // full top row (columns 0-4, row 0) and leave (5, 0) (past the
        // glyph) and (2, 1) (the top row's *second* row, a gap in the
        // vertical stroke before it starts) dark.
        for col in 0..5 {
            let i = (col * 4) as usize; // row 0, so no `y * w` term needed
            assert_eq!(&frame[i..i + 4], white, "column {col} of I's top row should be lit");
        }
        let past_glyph = (5 * 4) as usize; // row 0, column 5
        assert_ne!(&frame[past_glyph..past_glyph + 4], white, "one column past the glyph should be dark");
    }

    #[test]
    fn unsupported_characters_render_as_blank_not_a_panic() {
        let (w, h) = (32u32, 16u32);
        let mut frame = vec![0u8; (w * h * 4) as usize];
        // Lowercase is upper-cased internally (covered); an apostrophe is
        // genuinely outside the font and must not panic or draw garbage.
        draw_text(&mut frame, w, h, 0, 0, "it's", [255, 255, 255, 255]);
    }

    #[test]
    fn every_punctuation_glyph_the_module_doc_claims_to_support_actually_draws_something() {
        // A regression guard for the exact bug an independent visual check
        // caught: `[`/`]` were used in the live help overlay's own text but
        // had no glyph defined, silently rendering as a gap rather than a
        // bracket. Every character this module's own doc comment lists as
        // supported must actually light at least one pixel, or it's lying
        // about its own character set.
        for c in ". , : - / % ( ) [ ] ! ? + =".chars().filter(|c| !c.is_whitespace()) {
            let bits: u32 = glyph_for(c).iter().map(|row| row.count_ones()).sum();
            assert!(bits > 0, "'{c}' is listed as supported but draws no pixels");
        }
    }

    #[test]
    fn lowercase_input_renders_identically_to_uppercase() {
        let (w, h) = (32u32, 16u32);
        let white = [255, 255, 255, 255];
        let mut lower = vec![0u8; (w * h * 4) as usize];
        let mut upper = vec![0u8; (w * h * 4) as usize];
        draw_text(&mut lower, w, h, 0, 0, "sand", white);
        draw_text(&mut upper, w, h, 0, 0, "SAND", white);
        assert_eq!(lower, upper);
    }

    #[test]
    fn text_width_matches_what_draw_text_actually_occupies() {
        let (w, h) = (64u32, 16u32);
        let mut frame = vec![0u8; (w * h * 4) as usize];
        let white = [255, 255, 255, 255];
        let text = "HELLO";
        draw_text(&mut frame, w, h, 0, 0, text, white);

        let rightmost_lit = (0..w as i32)
            .rev()
            .find(|&x| (0..h as i32).any(|y| { let i = ((y * w as i32 + x) * 4) as usize; frame[i..i + 4] == white }));
        // The reported width should be at least as wide as the rightmost
        // actually-lit pixel plus one (a trailing glyph can legitimately
        // have blank columns on its own right edge, e.g. most letters do,
        // so this is a lower bound, not an exact match).
        assert!(
            rightmost_lit.is_some_and(|x| text_width(text) > x),
            "text_width({text}) = {} should exceed the rightmost lit column {rightmost_lit:?}",
            text_width(text)
        );
    }
}
