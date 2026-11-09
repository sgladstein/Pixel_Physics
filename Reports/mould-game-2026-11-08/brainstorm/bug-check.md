# Mould prototype: bug check (2026-11-08)

One independent pass over `src/mould/{mod,render,lesson,play}.rs` and
`src/bin/mould.rs` at branch `claude/project-thread-cek3ly`, head `60f8e59c`.
Nothing in the repo was edited. The model-side findings were run in a scratch
copy of the mould module (model files only, built with overflow checks on), at
`/tmp/claude-0/-home-user-Pixel-Physics/fb694d0e-9c94-59b9-bea3-b6eeccf0da0f/scratchpad/mcheck/examples/x.rs`.

**Summary.** Two lesson bugs are real and a new player can hit them: a slow
player's lesson starves to death, and spending salt or snips early makes the
lesson impossible to finish. There is also one input bug where a stroke is
left live. I found no panics, no overflow and no determinism break.

Ranked by how likely a player is to hit it.

---

## 1. The lesson runs out of food. A slow player's mould starves to nothing and the lesson can't be finished

- **Where:** `lesson.rs:85-98` (`Lesson::dish`) together with `mod.rs:1097-1106` (eating) and `PILE_FOOD = 20_000`.
- **What goes wrong:** the lesson has no clock, but the piles still get eaten. A pile with 40 or more walkers on it loses 1.2 food per step (`table 40 x eat 0.03`). That empties a 20,000 pile in about **139 s**. A scored dish ends at 98 s, so this never shows up there. The lesson has no end time, and a new player reading banners takes longer than that. Once the joined piles are empty the whole mould starves: no walkers are left and nothing can bring them back. The Join step can then never pass, the storm never comes, and only R gets out.
- **Proved** (scratch run, the probe's own scripted lesson hand plus N seconds of "reading" at the start of each step):

| reading per step | where it ends | score |
|---|---|---|
| 0 s | Done at 50 s | 3 stars, 4/4 |
| 15 s | Done at 105 s | 2 stars (tough lost) |
| 30 s | Done at 162 s; home pile already **empty** at the storm | reach star lost, 3/4 |
| 60 s | Done at 283 s; **20 walkers left** | 0 stars, 1/4, after 0 |
| nobody plays | 3/4 joined at 30-120 s, then **0 walkers from 150 s on**, for good | lesson stuck on step 1 |

- **Smallest fix:** keep the dish piles full until the Storm step. For example, at the top of `Lesson::update`, when `self.step < Step::Storm` (or simply `!= Storm/Done`), run `for p in d.piles.iter_mut().filter(|p| !p.oats) { p.food = p.full; }`. A larger `food` set in `Lesson::dish` only moves the cliff further out.

## 2. Salt or snips spent before their lesson step make that step impossible

- **Where:** `lesson.rs:134-135` (`Snip` passes on `snips_used`, `Salt` on `salt_cells`) together with `mod.rs:668-677` (`begin_stroke` refuses at 0 stock). `SALT_STOCK = 1` and `SNIP_STOCK = 3`, and neither ever comes back.
- **What goes wrong:** the briefing card lists all four tools before step 1, so trying key 3 early is natural. One salt stroke during steps 1-4 uses up the only salt. At step 5 ("PRESS 3 AND DRAG A SALT WALL") `begin_stroke` returns false, the HUD says "NO SALT LEFT", and the step never passes. The storm is still held at `u64::MAX`, so the dish then goes on to starve (finding 1). Three snips spent before step 4 do the same thing to step 4.
- **Proved:** the same scripted hand, with one salt stroke made during step 1: `salt step: begin_stroke -> false, salt_left 0` and then `STUCK on Salt for 200 s (t=218s, home food 0, joined 0/4, walkers 0)`.
- **Smallest fix:** in `Lesson::next`, top up the stock for the step being entered: `Step::Snip => d.snips_left = d.snips_left.max(1)` and `Step::Salt => d.salt_left = d.salt_left.max(1)`.
- **Related (traced, low):** the Snip step passes on *any* click with the snip tool, even on bare agar with no tube near it. The lesson's own note says a step "passes because the mould did the thing", so this contradicts it. Passing on `counts.snipped > 0` (walkers killed by snip) would match the banner.

## 3. A salt or snip stroke stays live after the mouse is released over the HUD strip, and the next press of any tool keeps drawing it

- **Where:** `bin/mould.rs:144-156` (`mouse`), with `:277-283` (the cursor is set to `None` whenever it is below the dish, `y >= H`).
- **What goes wrong:** `mouse(false)` returns at `let Some((x, y)) = self.cursor else { return };` *before* the `(false, _) => end_stroke()` arm. If a drag ends over the HUD rows or outside the window, the stroke keeps its unused length. The next time the button is pressed with any tool (lamp, oats) and the mouse moves, `CursorMoved` sees `pressed && stroke_active()` and calls `drag_stroke`. That lays salt, or cuts tube and kills walkers, in a straight line from the old end point toward the cursor, using up whatever length was left.
- **Steps:** press 3, drag a salt line downward past the bottom of the dish into the HUD, release there. Press 1, hold the lamp somewhere and move: salt appears from the bottom edge toward the lamp.
- **Sure:** traced (the window can't be driven headlessly, but each call on the path is plain code).
- **Same family, traced:** pressing H mid-drag and releasing while the card is up also skips `end_stroke` (the brief branch `:137-143` returns early). Pressing a digit key mid-drag switches the tool but keeps drawing the old stroke, and if the new tool is the lamp, lights it too.
- **Smallest fix:** in `mouse()`, do `if !down { self.dish.end_stroke(); }` first, above both the brief check and the cursor check, and call `self.hold()` before each early return.

## 4. The window clips the HUD on screens shorter than about 760 px, and the minimum size allows a clipped window

- **Where:** `bin/mould.rs:237-248`. `pixels 0.17.2` uses the default `ScalingMode::PixelPerfect`, and its scale is `floor(...).max(1.0)` (`renderers.rs:328-330`), so it never scales *down*. The buffer is 1024x726. In any smaller window the image is drawn 1:1, centred and cropped top and bottom. `with_min_inner_size(512x320)` explicitly allows that.
- **What goes wrong:** on a 1366x768 laptop at 100% (the inner area is about 700 px after the title bar and taskbar), or in any window resized smaller, the bottom HUD lines are cut off: the score lines, the "NEW FOOD" and "STARS" lines, and with more clipping the timer and the hint. The banner at the top is cut too.
- **Sure:** suspected; it depends on the display and window manager. The pixels code path is read, not run.
- **Smallest fix:** build with `PixelsBuilder::new(w, h, surface).scaling_mode(ScalingMode::Fill)`, or set the minimum inner size to the frame size.

## 5. The lamp can keep shining after the mouse is let go

- **Where:** `bin/mould.rs:137-143` and `:160-163`.
- **What goes wrong:** press H while holding the lamp. `key()` shows the card and then `hold()` leaves the lamp on. A release while the card is up sets `pressed = false` but doesn't call `hold()`. Dismissing the card with a key also returns before `hold()`. The dish resumes with `dish.lamp` still `Some`, pushing the mould at the old spot with no button held, until the mouse next moves.
- **Also (suspected, not handled):** alt-tabbing away while holding the button. `WindowEvent::Focused(false)` isn't handled, so if the release goes elsewhere, `pressed` stays true and the lamp stays on.
- **Sure:** traced.
- **Fix:** the same as in 3 (call `hold()`/`end_stroke` on every path), and clear `pressed`, then `hold()`, on `Focused(false)`.

## 6. The score shows "THRIFT 0.00X, BAR 1.15X" when fewer than two piles are joined

- **Where:** `mod.rs:376-382` (`ratio()` returns 0 when `shortest == 0`) and `render.rs:497-503`.
- **What goes wrong:** a dish that joins only home (seed 7 with no play does) reports a thrift of 0.00x. That reads as better than the bar, next to a "-" for the thrift star. It is wrong in the player's own words.
- **Sure:** traced.
- **Fix:** print `THRIFT --` when `s.reach < THRIFT_MIN_JOINED`.

## 7. R and N keep fast and paused

- **Where:** `bin/mould.rs:116-124`; `restart` doesn't touch `view.fast` or `view.paused`.
- **What goes wrong:** after a fast dish, N starts the next dish at 4x, so 90 s goes by in about 22 s. A paused dish restarts paused. Both are shown in the HUD corner, so this is low. It may be intended.
- **Sure:** traced.
- **Fix:** add `self.view.fast = false; self.view.paused = false;` in `restart`.

## 8. Salt, snip and the lamp still work during the storm watch and after the score; oats don't

- **Where:** `begin_stroke` (`mod.rs:668`) and `set_lamp` have no phase check, and `place_oats` does (`:644`). The strike's `self.lamp = None` (`:1228`) is undone by the window's next `hold()` while the button is held.
- **What goes wrong:** this is inconsistent, but mostly harmless. A player can snip their own network during the 8 s watch and lose toughness. Strokes after "Over" just spend stock.
- **Sure:** traced.
- **Fix, if wanted:** add `self.phase != Phase::Growing` to `begin_stroke`, and to the window's `hold()` for the lamp.

## 9. Toughness is free if the storm finds nothing to aim at

- **Where:** `mod.rs:1379-1413` and `:1242-1246`.
- **What goes wrong:** the aim skips every tube cell within `r + 12` of *any* pile, oats included. If no candidate is left, `storm_line` is `None`, nothing is cut, and the star is earned whenever `reach >= 2`.
- **Sure:** suspected only, and probably unreachable in practice. I tried dropping the oats in hand onto the best piece's candidate cells 2 s before the aim, on 8 seeds with full networks (reach 6). The candidates shrank (for example 178 -> 56) but the storm struck every time. The no-cut case did appear once (seed 7 with no play), but only at reach 1, where no star is given.
- **Fix, if wanted:** fall back to aiming without the near-pile filter when `cand` is empty.

## 10. The lesson and "dish 1" are the same dish

- **Where:** `LESSON_SEED = 1` (`lesson.rs:37`), and `KeyN => restart(seed + 1)` from seed 0 goes to `Dish::new(1)`.
- **What goes wrong:** the lesson's last line "N: DISH 1" sends the player to the layout they just learned on, this time with the storm and the late food. This is a design note, not a crash. Fix: a different lesson seed, or have N skip to 2.

## 11. Very low: walkers left standing on new salt; snip marks barely visible

- A salt dab converts a disc of *cells* (offsets up to (1,1)) but only kills walkers within 1.5 of the stroke point. 0-7 walkers per stroke are left standing on salt (5 seeds). They walk off within 10 s, and nothing panics. After the storm, 0 walkers were left on the scar (5 seeds), so the `expect("walker inside the dish")` calls are safe.
- The snip mark (`render.rs:244-255`) is a single screen pixel per dab inside a 4x4-pixel cell, so the "fading mark" is close to invisible at scale 4.

---

## Checked and found clean

- **`storm_at = u64::MAX`:** every use either compares it or uses `saturating_sub` (`phase_tick`, `seconds_left`, the HUD's lesson branch). There is no overflow, including the `t + 1 >= storm_at` paths. `Lesson::next(Storm)` aims straight away and scores normally.
- **Panics:** none found. Indexing is bounds-checked or uses positions inside the dish. `Rng::below(0)` is never called. There is no division by zero (`ratio`, `p.food / p.full`, `drag_stroke` returns when `d < 0.5`). A dish with zero walkers runs without trouble (checked past 300 s).
- **Text at scale 4 (1024 px):** every glyph used exists in the crisp table, including `' ! + * [ ] / :`. Every line fits. The widest is HUD line 3 with the oats hint plus the keys, at 1018 px. The storm hint (1042 px with keys) correctly drops the keys text via the `hx + w + 6 < fw` check. The storm label's clamps are valid at scales 1, 2 and 4.
- **Determinism:** the RNG is our own splitmix, and the BTreeSet and sort orders are stable. Same seed gives the same dish within a build. Across operating systems the platform's f32 `sin`/`cos`/`ln` may differ, which is outside the "same build" rule.
- **Lesson steps 1-3 and 6:** passable in the scripted run (Lamp at 4 s, Oats at 8 s, Join at 14 s, Storm at 50 s, 3 stars), apart from findings 1 and 2.
