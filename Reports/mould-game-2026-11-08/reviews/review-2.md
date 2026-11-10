# Review 2: bug check and adversarial read of the mould game before main

Reviewer: independent session, 2026-11-08. Scope: `git diff 3b1877edd..754853039 -- src/mould src/bin/mould.rs src/bin/organ.rs examples/mouldprobe.rs examples/organprobe.rs`. The focus is what changed since review 1 (head `aeae6d290`): the built look (`look.rs`, `Style::Night`, the default), its hand-off from `render.rs`, review 1's fixes, the window's input, and the probe.

**How it was checked.** I read the whole of `look.rs`, `src/bin/mould.rs`, `src/bin/organ.rs`, the `render.rs` and `mod.rs` changes since review 1, and the probe's argument handling. Every reproduction ran as a scratch `#[test]` module in a `git archive 754853039` copy under my scratchpad (`src/mould/review2_scratch.rs`). It is copied beside this file as `review-2-pictures/review2_scratch.rs.txt`. Release runs used `cargo test --release`. The overflow was also run in a debug build. Nothing in `/home/user/Pixel_Physics` was edited.

**Tally: 2 high, 2 medium, 5 low, 3 probe-only.**
- Proved: 8. Traced: 3. Suspected: 1.
- Every one of review 1's proved reproductions now fails to reproduce at the exact moment review 1 tested. But finding 1's fix holds only for about the first 1.5 s; see H2.
- The ship bar is still met on seeds 1-5.

Findings are ranked by how likely a player is to hit them.

---

## H1. HIGH (proved). Food still due when a dish ends is drawn with a countdown that underflows. Release builds show an 18-digit number; debug builds panic.

- **Where:** `src/mould/look.rs:1722`, `let secs = (at - d.t).div_ceil(sps).to_string();` in `piles()`, under `if *at <= d.t + 5 * sps`.
- **Why:** `Dish::pending` is promoted to piles only in `phase_tick` while `Phase::Growing`. When a dish leaves Growing with late food still due, those entries stay in `pending` forever. The window keeps stepping a finished dish, so `d.t` passes `at`. That happens on a challenge dish that ends early (the new "mould died" end) and on the Wanderer after Go. Classic `render.rs` has no subtraction there, so only the new default look breaks.
- **How often a player meets it:** on Running Dry, every losing hand.
  - Running Dry's piles come at 20/40/60/80/100 s.
  - The ship-bar run shows idle dying at 1:04-1:17, random at 1:00-1:27, fresh at 1:19-1:29, and plan at 1:29-1:46 on 3 of 5 seeds. All of these are before the 100 s pile, and most are before the 80 s one.
  - On the Wanderer, the careless Leader pressed Go at 24-54 s on all 6 seeds tried (`n1b`). That is before the 55 s pile every time, and before the 30 s one on 2 of the 6.
- **Reproduction:**
  - `n1_pending_after_dish_over_running_dry`: Running Dry with every walker removed at 2 s ends at 2 s with `pending [20,40,60,80,100]`. 23 s later the look computes `(2400 - 3000)`. In a debug build: `panicked at src/mould/look.rs:1722:24: attempt to subtract with overflow`.
  - `n6_pending_pictures` (release), both in `review-2-pictures/`:
    - `n6-dry-idle-over+40s.png`: Running Dry, idle, seed 1, card "The mould died at 1:05". Two dashed "food coming" rings read `153722867280912906` and `153722867280912926`.
    - `n6-wander6-over+20s.png`: the arrival card is up, and the 55 s ring reads `153722867280912915`.
  - Before the wrap, the finished dish also counts down "5, 4, 3, 2, 1" to food that will never land.
- **Minimal fix:** draw pending food only while `d.phase == Phase::Growing`, and use `at.saturating_sub(d.t)`. Better still, `finish()` (level.rs) and `go()` (wander.rs) can clear `self.pending`, which also stops `Status::coming` counting it.

## H2. HIGH (proved). Review 1's finding 1 is fixed only for oats laid within about 1.5 s. From 2 s after the last fed cell, oats dropped on the starving mould are never eaten.

- **Where:** `src/mould/mod.rs:1220` and `:1344` (`fed_within`, `Params::stray_grace = 2.0`).
- **Why:** the grace counts cells whose `fed_at` is within 2 s at the census *after* the oats land. Only oats laid in the first second or so still find a stamp young enough. Review 1's case ("a player who is a little late") is exactly the one still locked out. Meanwhile the oats preview tells the player "It will reach these", because the nearest walker is 0 cells away.
- **Reproduction (`n7_rescue_when_a_little_late`):** Running Dry, nobody playing, seeds 1-3. Nothing is fed from 35 s. Two oats were laid on the walker nearest the mould's centroid, at +0, +1, +2, +3 and +5 s.

  | seed | +0 s | +1 s | +2 s | +3 s | +5 s |
  |---|---|---|---|---|---|
  | 1 | eaten (950 alive) | eaten (763) | **never** (604) | **never** (459) | **never** (280) |
  | 2 | eaten (1026) | eaten (820) | **never** (676) | **never** (533) | **never** (334) |
  | 3 | eaten (966) | eaten (750) | **never** (595) | **never** (470) | **never** (298) |

  So 450-680 walkers are alive with oats under them, and the oats are never eaten. Review 1's `r2` (oats at the very moment of running dry) now passes in both arms. It tests the one moment the fix covers.
- **Minimal fix:** this is a design call. The report's own sweep found that a longer grace lets a random hand earn stars. Options:
  - (a) Exempt player oats from the stray rule. When `p.oats`, set `strays_ok = true`. The leash exists to stop strays *colonising dish food*; oats are the player's deliberate verb. Then re-run `level=all` on 8 seeds, because Running Dry random seed 4 is the case to watch.
  - (b) Measure the grace from when nothing was last fed, not per cell. Keep `last_fed_any: u64` and let `none_fed && t - last_fed_any <= grace` admit any walker-occupied disc. Either way the preview should not promise "It will reach these" where the rule refuses.

## M1. MEDIUM (traced). The Night tool tiles look like buttons, but a click on them does nothing. Pressing on a tile and dragging up onto the dish turns the lamp on.

- **Where:** `src/bin/mould.rs:658` (the cursor is `None` for `y >= H`) and `:426` (`mouse` sets `pressed = true` before `let Some(..) = self.cursor else { return }`). The tiles are `look.rs:2400` (142x70 rounded boxes with key badges, names and stock pips, below the dish).
- **Path:**
  1. The briefing card says "Click or press a key to start", and the bar is a row of tiles, so a new player clicks "2 OATS".
  2. Nothing happens, with no feedback.
  3. `pressed` stays true. Moving up onto the dish with the button still held makes `hold()` light the lamp, because the tool is still Lamp.
- **Minimal fix:**
  - In `mouse(true)`, when the press lands on the HUD strip, hit-test the tile rectangles (`8 + k*150, y0 + 8, 142, 70`) and select the tool if `has_tool`. Share the rectangles from `look.rs`, as `menu_cards` does for the picker.
  - Set `pressed = true` only when the press lands on the dish.

## M2. MEDIUM (proved). The stray grace is off while any cell anywhere is fed. A large piece that has just starved cannot eat oats laid on it while another piece is still eating.

- **Where:** `src/mould/mod.rs:1220`. The comment says the grace applies "**Only while nothing is fed**", which is deliberate. But it brings back review 1's lock whenever the mould has split.
- **Reproduction (`n5_partial_lock_while_another_piece_feeds`):** Running Dry, random hand, scanned every second. Each candidate was a walker on ground fed 1-2 s ago, more than 40 cells from every fed cell. One oat was laid on it, with a paired control at `stray_reach = inf`:
  - Seed 3 at 37 s: 263 mould round the oat and 1130 fed cells elsewhere. Level rule: **never eaten** in 6 s. Control: eaten.
  - Seed 4 at 36 s: 229 mould round it. Level: never. Control: eaten.
  - Seed 4 at 37 s: 185 mould round it. Level: never. Control: eaten.

  This is the Running Dry situation the dish is about: home still eating while a branch out at a new pile ran dry.
- **Minimal fix:** the same as H2 (a). Exempting oats from the leash removes both H2 and M2 without touching the Beat the Ruler case that ruled out a global grace. That case was dish food started round the side.

## L1. LOW (proved). After any challenge dish ends, and on the Wanderer after Go, the oats, salt and snip cursor says "Not now: the storm". Those dishes have no storm.

- **Where:** `src/mould/look.rs:2036` (`preview`: any phase but Growing gives "Not now: the storm").
- **Reproduction (`n9`, `review-2-pictures/n9-wall-over-oats-cursor.png`):** Behind the Wall, over, oats selected. A red X reads "Not now: the storm" (`storm_at` 10800, `storm_line` None).
- **Minimal fix:** word it by phase. Use "Dish over" for `Phase::Over`, "She's on her way" for `Crossing`, and "the storm" only for `Storm(_)`.

## L2. LOW (proved). The half-size glow cache is keyed by the dish's address and step. A different dish at the same address and step reuses the old glow.

- **Where:** `src/mould/look.rs:1188` (`*a == addr && d.t >= *t && d.t < *t + 4`).
- **Reproduction:**
  - `n2`: free dish 1 at 30 s drawn, then Running Dry at 30 s moved into the same variable. **89,754 pixels** differ from a clean draw of the same dish from another address.
  - `n8` is the window's own path. Lesson behind its card (t = 0), then M, then pick a challenge dish, which opens at t = 0 in the same `Handler::dish` field. **41,888 / 50,857 / 48,559 pixels** (dishes 1-3) show the lesson's glow round the card (`n8-reused.png` against `n8-clean.png`). The first frame after the card is put away is clean (0 px), because the step moves past the window.
  - So a player sees it only behind a briefing card. The probe's pictures can also get it when two dishes share a stack slot.
- **Minimal fix:** key the slot by a dish identity rather than its address. For example, add a `Dish::serial: u64` drawn from a global counter in `assemble`, or simply `(seed, level key, stretch)`. Or clear the cache whenever `d.t == 0`.

## L3. LOW (traced). In the Classic look (F2), the armed Escape shows only in the window title, and the title keeps saying "press Esc again to quit" after the 2 s window has lapsed.

- **Where:** `src/bin/mould.rs:520`. The title is reset only by the next key.
- **Minimal fix:** reset the title in `frame()` when `quit_armed` expires. Or draw the chip in `render.rs` too.

## L4. LOW (traced). The Mould Organ window still crops on a short screen and quits on the first Escape.

- **Where:** `src/bin/organ.rs`. `resumed` and `Resized` lack the mould window's `scaling_for` (`ScalingMode::Fill`), and `KeyCode::Escape => event_loop.exit()`.
- Review 1 noted the scaling as an aside to its finding 5, and it is still open. Its fix (`heal_wounds`, release, focus loss) does hold.
- **Minimal fix:** copy `scaling_for` and the two-press Escape from `src/bin/mould.rs`.

## L5. LOW (suspected). Behind the Wall can read "NOT REACHED: THE MOULD CAME WITHIN inf CELLS".

- **Where:** `src/mould/level.rs` `nearest_unjoined_gap`. It now returns `INFINITY` when no unjoined pile is *unreached*.
- **Path:** at the bell, both piles are reached (`eaters > 0`) by separate pieces but not joined, for example after a connecting tube starved or a stray-started far pile. Then `gap = inf`, and the card formats `{gap:.0}` as "inf". I did not build this state.
- **Minimal fix:** when `gap.is_infinite()`, word it "BOTH FOODS EATEN, BUT NOT JOINED".

---

## Probe only

- **P1 (proved). `chip()` panics when a chip is wider than the frame.**
  - At the probe's default `scale=2` in the default look, Running Dry's stars chip ("Stars: 100, 200 and 300 mould at the bell", 506 px in a 512-px frame) panics. The location is `look.rs:1798`, `x.clamp(4, cv.w - w - 4)`, and the message is `min > max. min = 4, max = 2`.
  - Reproduced by `mouldprobe level=3 seeds=1 players=idle look=DIR`: `panicked at src/mould/look.rs:1798:15`.
  - So `level=all look=...`, `gif=` and `stills=` die on dish 3.
  - The `n4` fuzz found 556 such panics at scale 1 and 64 at scale 2 across Night, Bench, Glow and Toy, and **none at scales 3, 4 or 6**. The window is fixed at 4.
  - Fix: `let x = x.min(cv.w - w - 4).max(4);`. Or default the probe's `scale` to 4 for the built look, because its tile bar is laid out in absolute pixels (`616`, `150` per tile) and does not fit a 512-px frame either.
- **P2 (proved). `oats=` is still silently ignored by `pull=1` and `looks=`.** Neither refuses it: both ran past the echo line with `oats=0`, while `lesson=` refused it. The refusal list at `mouldprobe.rs:1698` misses `pull` and `looks`. Fix: add them to `other_mode`, or call `give_oats` there.
- **P3 (traced). Some modes build from the defaults while echoing the parameters.** `lesson=` and the lesson and Running Dry moments of `looks=` use `Lesson::dish()` and `Dish::from_level(..)`, not `a.params`. So `lesson=DIR pieces=0` echoes `pieces: false` and plays `true`. (The organprobe's `core=` with `play=1`, review 1's last minor, is also unchanged.) Fix: build from `a.params`, or refuse non-default params in those modes.

---

## Review 1's proved reproductions, re-run on 754853039

| Review 1 | Then | Now |
|---|---|---|
| 1 Running Dry lock (`r2`) | oats never eaten | **eaten** in both arms (950 alive at 35 s). But see H2: only within about 1.5 s |
| 2 join flash on an oat (`r3`) | 6 flashes | **0 flashes** (3 oats); control 0 |
| 3 free-dish thrift for doing nothing (`r11`, seeds 1-8) | idle 1 star on seeds 1 and 8 | **idle 0, random 0 on all 8 seeds** |
| 4 organ snip never heals (`r1`) | 171 to 171 | **171 to 0** in 30 s |
| 5 organ release over the HUD | traced | traced fixed: `!down` is handled first, plus `Focused(false)` |
| 6 absent tool selected (`r6`) | carried over | traced fixed: `first_tool` on restart, digits gated by `has_tool`; the dish still refuses salt and snip |
| 7 dead dish runs to the bell (`r7`) | ended at 90 s, "within 0 cells" | **ends at 2 s**, "THE MOULD DIED AT 0:02 / NONE LEFT: KEEP SOME OF IT ON FOOD" |
| 8 ruler hops a break (`r8`) | 205.5 with reach 2 | **`None`** with reach 2; whole spokes 205.38 with reach 3 |
| 9 Escape quits at once | traced | traced fixed: the second press within 2 s quits; Escape on the picker only closes it |
| P1-P4 probe | traced | P1 fixed (echo printed in `level=`); P2 fixed (`level_dish` builds from params); P3 and P4 fixed except P2 above |

## The ship bar (`mouldprobe level=all seeds=1,2,3,4,5`)

| dish | idle | random | intended | |
|---|---|---|---|---|
| Behind the Wall | 0 0 0 0 0 | 0 0 0 0 1 | 3 3 3 3 3 | meets the bar |
| Beat the Ruler | 0 0 0 0 0 | 0 0 0 0 0 | 3 3 3 3 3 (0.87-0.88x) | meets the bar |
| Running Dry | 0 0 0 0 0 | 0 0 0 0 0 | 3 3 3 3 3 (354-411 alive) | meets the bar |

**Frame cost, back to back on this machine** (`level=1 players=intended`, steps plus draw at scale 4, two runs each):
- Night: mean 3.94 / 4.06 ms, worst 5.53 / 7.22 ms.
- Classic: mean 3.18 / 2.70 ms, worst 6.11 / 6.53 ms.

The built look costs about 0.9-1.4 ms a frame more on average, which is consistent with its header's "about 1.7 ms". The fixes report's 2.71 / 4.85 ms is from another machine or another load, so do not compare it with these numbers.

## Checked and found clean

- **Drawing never feeds the simulation (`n3`).** Free dish 3 ran with a random hand for 40 s: once with no drawing, then drawn every frame in each of the five looks. Walkers (720), births (1263), joins (3) and a hash of every trail cell were identical in all six runs. `draw` takes `&Dish`. The only shared state is the three draw-only caches in `look.rs`, and no `Cell`, `RefCell` or atomic sits on `Dish`.
- **No out-of-bounds writes at the window's scale (`n4`).** These all draw without a panic at scales 3, 4 and 6:
  - Dishes: the lesson; three challenge dishes at 0 s, at 20 s and over; the free dish at its start, in the storm alarm, in the storm and over; the Wanderer crossing, over, and at stretch 3 with 900 carried.
  - Overlays: plain, brief, picker, Esc armed, banner (with a 54-character line), and the four tool previews with the cursor at the dish corners.
  - Looks: all five.

  Pixel writes all go through bounds-checked `put`, `mix` and `add`, or through `hud::draw_text*`. The one direct slice write (the dish's flat cells) is sized by `frame_size`. Every padded-trail index stays inside its two-cell pad; the crack is agar only inside the ellipse.
- **Text fits at scale 4.** I checked the longest strings: the score lines, the why lines, "Not reached: only starving strays got there" (it drops a size, as coded), the free card's lines, the tile sub-labels, and the Wanderer briefing card (21 rows: 608 px in a 640-px dish). All fit their panels.
- **The tile-bar cache (`hud_cached`).** Its key covers everything `hud_bench` draws except the mould count, which is drawn after it. That includes the tool, the stocks with the regrowth quantised as drawn, the oats wait, the phase words with their seconds, the timeline at pixel resolution, joined, coming, the score words, the stars, the level and the look. `warn_frac` is not in the key, but it is fixed by the level key and the dish length. So R, N, M, W and F2 cannot show a stale bar.
- **The rim flash (`background`).** It depends only on the look and the flash bit, never on the dish, so it cannot carry over between dishes.
- **The picker.** Click rectangles and drawing share `menu_cards`. All six cards sit above the HUD strip (the last ends at y = 578 < 640), so every card can be clicked through the `y < H` cursor filter.
- **Game state drawn in every phase.** Each of these is drawn: lesson banner and "Lesson done"; the three challenge dishes' stars chip, briefing card (only the tools given, "Not here" slots that agree with `has_tool`), ruler and best answer, and per-pile ✓/✗ at the end; the free dish's timeline with late food, storm line and count, cuts and scar chip; score and stars; "The mould died at 1:05"; the Wanderer's crack, label, pour line, bar and arrival or lost card; Esc armed; paused and fast; the picker with locks and best stars.
- **Window input.**
  - Every key was checked in the picker, on the briefing card and on a dish. On the card, any key but Esc and F2 only dismisses it.
  - N goes to the next stretch only once she has left or died, otherwise to the next unlocked dish or the picker. W, G, R, H and P behave as documented.
  - `restart` resets pause, fast, the brief, the picker, `pressed` and the tool.
  - The lamp is let go on the picker, H, the card, focus loss and release.
  - `Style::parse(args.last())` does not mistake `wander 3` or `d2` for a look.
- **Progress.** Bad lines are skipped, stars are capped at 3, and a failed write only prints. The lesson records 1 when done and a dish its card's stars once at Over. The "mould died" end records 0. Free play and the Wanderer are always open.
- **Review 1's fixes in code.** `eaten_pieces` and `piece_of_src` come from the same census, so comparing piece ids is valid. `best_join`'s pinch rule plus the `measure().reach` gate agree, so a 4-neighbour join is now required. The free dish's ladder (`networked && reach == piles`) agrees with `free_card`'s branches.
