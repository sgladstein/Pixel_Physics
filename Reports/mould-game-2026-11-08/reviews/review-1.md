# Review 1: bug check and adversarial read of the 2026-11-08 mould work

Reviewer: independent session, 2026-11-08. Scope: `git diff 3b1877edd..aeae6d290 -- src/mould src/bin/mould.rs src/bin/organ.rs examples/mouldprobe.rs examples/organprobe.rs` (per-piece food, wounds, join flashes, pulse, the Wanderer, the Mould Organ, challenge dishes and picker, the 3-pile toughness rule).

How it was checked: I read the whole diff. Every reproduction ran as a scratch `#[test]` module in a `git archive` copy of `aeae6d290`, built under my scratchpad (`review_scratch.rs`, never committed). Nothing in `/home/user/Pixel_Physics` was edited. I also ran the existing mould tests on that copy: 22 pass, plus the 2 ignored level tests (`--ignored`), which also pass.

**Tally: 1 high, 4 medium, 4 low, plus 4 probe-only.** Proved: 7. Traced: 6.

The findings are ranked by how likely a player is to hit them.

---

## 1. HIGH (proved). Running Dry: once every bit of mould is grey, food can never be started again. That includes oats dropped right on it and the new piles that turn up under it.

- **Where:** `src/mould/mod.rs:1195-1207` (`census`, `strays_ok`) and `fed_within` (`mod.rs:1302`). The rule only fires with a finite `Params::stray_reach`, which every hand-made dish has (`level::STRAY_REACH = 24`).
- **Why:** a walker on unfed ground is a "stray". A stray can start a pile only if *currently fed* mould lies within `r + 24` of that pile. When the last fed pile runs dry, no cell is fed anywhere. From then on, every walker is a stray and every pile fails the test. The mould is unrecoverable while the clock still runs. Running Dry is the only dish whose piles run out, and its whole premise is "the food runs out". So a player who is a little late meets this on a first attempt.
- **Reproduction (`r2_running_dry_dead_mould_cannot_start_oats`):** Running Dry, seed 1, nobody playing.
  - At 35 s no cell is fed. 950 walkers are alive, 5 oats are in hand, and the three home piles are at 0.
  - Two oats laid on a walker in the middle of the mould were **never eaten in 8 s**. Mould fell to 145 and fed cells stayed at 0.
  - Paired control, the same state with `stray_reach = inf`: the oats were eaten and 232 walkers were alive after 8 s.
  - The dish then runs 85 more seconds that cannot be won. The card says "MOVE ON SOONER" when the real answer is that nothing could be done.
- **Proposed fix (minimal):** count *recently* fed mould as fed for this test. `fed_at` already stamps it. In `fed_within`, accept a cell with `self.t.saturating_sub(self.fed_at[i]) <= STRAY_GRACE` (about 5 s), not only `fed[i] != u16::MAX`. This keeps the leash, since a stray far from any recent network still can't colonise. It also lets just-starved mould be rescued by oats on top of it, which is the verb the player is reaching for. Re-run `mouldprobe level=all`: doing nothing should still score 0 (doing nothing never reaches new food anyway).

## 2. MEDIUM (proved). A join flash fires when an oat is eaten by the network already there, with no bridge made, and it is drawn away from the oat.

- **Where:** `src/mould/mod.rs:1308-1343` (`join_pieces`; the flash test is at `:1329`).
- **Why:** the flash fires for any union of two pile roots whose pair is not in `last_joined`. An oat reached for the first time has no pair entry. When it is eaten by an existing piece, its feeding search meets the pile's search at the boundary between their two regions. That boundary is usually halfway between the oat and the pile, and that is where the flash is drawn. So the "little bridges where there wasn't even food" event also counts plain feeding, and `counts.snaps` counts oats laid.
- **Reproduction (`r3_oat_on_fed_tube_flashes_a_join`):** free dish 1 at 25 s. Oats were laid on fed tube 25-50 cells from food and more than 15 cells from any pile rim.
  - 3 oats gave **6 join flashes** within 2 s.
  - **0** of those flashes were within 8 cells of the oat.
  - The same dish without the oat: **0 flashes**.
- **Proposed fix:** flash only when both piles were reached at the *previous* census and were then in different pieces. Keep last census's `(key -> piece)` map and skip the flash when either key is new or the two keys already shared a piece. A newly reached pile then never flashes on its first census. A real bridge between two established pieces still does, including one to an oat that strays started on their own.

## 3. MEDIUM (proved). Free dish: doing nothing still earns the thrift star on some seeds, and per-piece food made it a little more frequent.

- **Where:** `src/mould/mod.rs` `phase_tick`: `networked && s.ratio() <= THRIFT_BAR`, with `networked = reach >= THRIFT_MIN_JOINED` (3).
- **Reproduction (`r11_free_dish_bar` and `r11b_..._shared_store`):** the Idle hand on seeds 1-8, played to the end.
  - Per-piece food: **1 star on seeds 1 and 8** (thrift: reach 4/6, ratio 1.13 and 1.14).
  - Shared store, paired: 1 star on seed 8 only.
  - So the hole predates tonight, and per-piece food widens it from 1/8 to 2/8.
  - Random reached 2 stars on seed 5 (thrift and toughness) with 5/6 joined.
  - Tonight's toughness change (needs 3 piles) did close the toughness half: idle got toughness on 0/8.
- **Proposed fix:** make the free dish's stars tiers, as the levels already are. Thrift and toughness count only when `reach == piles`, or at least `reach >= piles - 1`. Then "short" means short for the whole job. Check with `mouldprobe` across 8+ seeds (an order statistic, not 5 seeds).

## 4. MEDIUM (proved). Mould Organ: a snip never heals, leaving a permanent invisible wall.

- **Where:** `src/mould/organ.rs:197` (`organ::step`). It copies `Dish::step`'s sub-steps but not `heal_wounds` (or `flow_out`). Snips are on the organ's tool bar, and `Params::default()` has `wound = 4.0`.
- **Reproduction (`r1_organ_snip_never_heals`):** one 30-cell snip on organ dish 1.
  - **171 wound cells, still 171 after 30 s** of `organ::step`.
  - The same snip under `Dish::step`: 171 → 0.
  - After 1 s, `cell_colour` draws a wound with no time left as plain agar. So the player sees bare agar that the mould and the pulse can never cross. Every geodesic, and so every note, routes round an invisible wall.
- **Proposed fix:** add `d.heal_wounds();` to `organ::step` after `d.economy()`. Or set `wound: 0.0` in `organ::new_dish`'s params, as was done for `pieces`, until someone re-measures the organ with wounds.

## 5. MEDIUM (traced). Organ window: releasing the mouse over the HUD strip or outside the window leaves a salt or snip stroke live. The next lamp press then drags it.

- **Where:** `src/bin/organ.rs:115-129`. `mouse()` returns at `let Some((x, y)) = self.cursor else { return };` *before* the `(false, _) => d.end_stroke()` arm. A release with the cursor off the dish (the cursor is `None` for `y >= H`) never ends the stroke.
- **Path:** drag salt and let go over the HUD, so the stroke keeps its remaining length. Press 1 (lamp), then press and move. `CursorMoved` sees `pressed && stroke_active()` and keeps laying salt along the lamp's path. This is the exact bug `src/bin/mould.rs` documents as fixed ("It used to return first when the cursor was over the HUD strip..."). The organ copied the pre-fix shape. It also lacks the mould window's `Focused(false)` let-go and the `ScalingMode::Fill` fix for short screens.
- **Proposed fix:** handle `!down` first, as `src/bin/mould.rs:mouse` does: `if !down { self.pressed = false; self.organ.dish.end_stroke(); self.hold(); return; }`. Then add the `Focused(false)` arm.

## 6. LOW-MEDIUM (traced, plus R6). The selected tool carries into challenge dishes that don't offer it, and keys 3 and 4 select tools that are not on the bar.

- **Where:** `src/bin/mould.rs:234` (`restart` never touches `view.tool`) and `:456-457` (`Digit3/Digit4` are not checked against `has_tool`).
- **Path:** leave free play with snip selected, or press 3 or 4 on Behind the Wall. `hud_lines` hides tools 3 and 4 (`has_tool`), so nothing on the bar is highlighted. A click does nothing: R6 shows `begin_stroke(Salt/Snip)` returns false with salt and snip both absent. The hint line says "NO SALT LEFT" on a dish that never had salt. The player can recover with 1 or 2, but the dish looks broken.
- **Proposed fix:** in `restart`, `if !self.dish.has_tool(self.view.tool) { self.view.tool = Tool::Lamp }`. In `key`, ignore a digit whose tool `!self.dish.has_tool(t)`.

## 7. LOW (proved). A challenge dish with no mould left runs to the bell, and its card then says "THE MOULD CAME WITHIN 0 CELLS".

- **Where:** `src/mould/level.rs:598` (`nearest_unjoined_gap` returns 0.0 when there are no walkers), and `level_tick`, which waits for the bell whatever happens.
- **Reproduction (`r7_dead_level_runs_to_the_bell`):** Behind the Wall with every walker removed at 2 s. The dish ended at **90 s**, card `"NOT REACHED: THE MOULD CAME WITHIN 0 CELLS"`. The amount reads as a near miss when it was a wipe-out. (In play, a lamp held on home or a starved Running Dry gets there.)
- **Proposed fix:** in `level_tick`, end the dish when `self.walkers.is_empty()`, with the card "THE MOULD DIED". In `nearest_unjoined_gap`, return `f32::INFINITY` with no walkers, and word the card on that.

## 8. LOW (proved). Beat the Ruler scores "all joined" across a break that `measure()` calls unjoined.

- **Where:** `src/mould/level.rs:438-490` (`tube_distance`). The 16-neighbour steps, including knight's moves `(2,1,224)`, check only the landing cell. So a path hops a one-cell break.
- **Reproduction (`r8_ruler_hops_a_gap`):** three hand-laid spokes from the meeting point give `best_join` 205.2 and reach 3. A one-cell-thick break across one spoke (3 cells zeroed) gives **`best_join` 205.5 (still "joined") while `measure().reach` = 2**. The stars come from `best_join` only, so the HUD can say "ALL JOINED: 0.9xX" and pay stars while the food count says 2/3. A thin tube with a dip under the threshold at the bell is enough.
- **Proposed fix:** use the organ's own rule (`organ.rs:396-401`). A diagonal needs one of its two corner cells to be tube, and a knight's move needs both cells it crosses. Or require `measure().reach == piles` before scoring the ratio.

## 9. LOW (traced). Escape quits the game immediately, one key press after Escape closes the picker.

- **Where:** `src/bin/mould.rs` `key`: Escape closes the menu, then the next Escape reaches `event_loop.exit()`, including from the briefing card. This predates tonight; the picker makes a double Escape likely. Progress is saved on record, so nothing is lost. **Fix:** make Escape on a dish open the picker, and quit only from the picker, or not at all.

---

## Probe only (`examples/mouldprobe.rs`, `examples/organprobe.rs`)

These are the "a measurement that silently ignores its arguments" class, the same as the `pieces=0` arm found earlier tonight.

- **P1 (traced). `level=` never echoes its parameters.** `main` returns into `levels()` at `mouldprobe.rs:1062`, *before* the `println!("mouldprobe: ... params {:?}")`. A `level=all pieces=0` run prints nothing that says pieces was off. CLAUDE.md asks that a harness echo its own parameters. That rule was broken exactly where tonight's wrong arm ran. **Fix:** print the params line first, or in `levels()`.
- **P2 (traced). `level=`'s frame-cost block and its `look=` stills ignore every parameter.** `mouldprobe.rs:1010` and `:936` use `Dish::from_level(...)`, not `from_level_with(..., params)`. So `level=2 pieces=0` reports the frame cost of pieces=1. The same is true of `wander_cost` (`:1455`): `Dish::new(seed)` and `Dish::wanderer(seed, ...)` ignore `a.params`, while the journeys above it use them. **Fix:** build both from `a.params` (with the level's `stray_reach`).
- **P3 (traced). `wander=1 players=...` ignores an explicit list of exactly 7 hands.** `mouldprobe.rs:1360` tests `a.players.len() == 7` to mean "not given". `players_given` exists. **Fix:** `if !a.players_given`.
- **P4 (traced). `oats=N` is honoured only by the scored-dish `run()`** (`:220`). `level=`, `wander=` and `cut=` ignore it silently. **Fix:** apply it, or panic when given with those modes. Minor: `cut=` calls the hand every step rather than every `STEPS_PER_FRAME` steps, so its plan hand acts twice as often as in the game.
- (organprobe, minor) `core=` is used by the step0 and bend rulers but not by `play=1`, which plays at `organ::CORE_WEIGHT`. The header echoes `core=` regardless. It is documented on the field; say so in the echo, or reject `core` with `play=1`.

---

## Checked and found clean

- **Index safety.** `fed_src`, `piece_of_src` and `census_keys` are written together in one census. `piece_of` uses `.get`. `economy_by_piece` finds a pile's census index by `pile_key`, so a late pile inserted mid-list (`phase_tick`'s `insert(dish_piles().count(), ..)`) or an expired oat between censuses cannot misindex. `strays_ok` is built from the current piles. The `swap_remove` in the death loop keeps `piece_of` in step with `remove_walker`. After Go, `fed.fill(MAX)` makes every `piece_of` `None`. Two oats clicked on the same cell share a `pile_key`. That is harmless, because they always fall in one piece.
- **No panics under fuzz** (`r13`): a random hand on all three challenge dishes × 4 seeds. The Wanderer was led on 4 seeds (all arrived: 926/1387, 283/1587, 1270/2108, 1038/1583), then stretch 2 with those carries under a random hand for 2 min. Whole scored dishes with snips and oats (`r4`, `r11`) also ran clean.
- **Food is never created** (`r4`): the plan hand on seeds 1-3. The largest one-step rise of (all walkers' shares + all piles' food) was 0.0000, once oats placed and late piles are accounted for. Joins pool shares, splits keep them, and the `max(0)` clamp only forgives debt. Divisions are guarded (`size.max(1)`, `walkers.len().max(1)`, `of.max(1)`, `ruler().max(1)`). Food *is* lost by design when a snip kills walkers holding shares, and when a whole piece dies. The doc's "never lost by a split" is true of a split, not of the snip that makes it.
- **Frame cost** (`r5`): `economy_by_piece` takes 0.010 ms a call against 0.004 ms for the shared store (600 walkers). The whole step is 0.6-0.7 ms either way. Its four small Vec allocations a step are immaterial (about 1% of a step). The pulse is a few flops per fed cell. `best_join` (3 Dijkstras over tube cells) runs every 0.5 s on Beat the Ruler only.
- **Determinism:** no HashMap anywhere in scope. `last_joined` is a BTreeMap. The wander probe's threads write to a Mutex and the results are sorted by (seed, hand) before printing. Each job owns its dish.
- **Wounds:** overlapping snips extend correctly (`until[i] <= t` check). A storm scar over a wound is not reverted to agar. Salt, oats, births, lamp push and arrivals all refuse non-agar, so nothing stands on a wound. `spread` treats a wound as a wall for trail and scent. The control in R1 heals 171 → 0 under `Dish::step`.
- **Save file:** a missing, unreadable, non-UTF-8 or garbled `mould-progress.txt` parses to a fresh start (lossy decode; bad lines skipped; stars capped at 3). A failed write only prints. Unlock logic: free play and the Wanderer are always open, so a player can never be locked out of play.
- **Dish flow:** every challenge dish ends at its bell. N, R, M and W reset pause, fast, the brief, `pressed` and the stroke. The lamp is let go on the picker, H, focus loss and release. Wanderer N does nothing until she has left or died. R retries a stretch with its carry.
- **Regressions:** the existing 22 mould tests (lesson, free dish, wander, organ, level) pass, and so do the 2 ignored level tests (`--ignored`: doing nothing scores 0 on Running Dry; the intended hand takes 3 stars on Ruler and Dry). The free dish with `wander=false` draws the same numbers as before (`same_seed_same_dish`). The toughness change gave idle 0/8 toughness stars.
