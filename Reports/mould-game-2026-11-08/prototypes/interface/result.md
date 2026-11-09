# Mould: interface and graphics

## Round 2 — the picked look, built (2026-11-08)

### Review 1's fixes, carried into the built look

The bug check and review of the challenge dishes changed five things the player sees. All five are now in the built look:

- **Star ladder.** Keeping the network short, and losing nothing to the storm, now count only once all the food is joined. The briefing card's stars now read "Join all the food", "All joined, and the network short", and "All joined, nothing lost to the storm" (`built-g-brief.png`). The lesson's last line comes from the game, so it says "then" by itself.
- **Escape.** The first Esc on a dish no longer quits. For the next 2 s a gold line, "Press Esc again to quit", sits above the tool bar (`built-p-esc-armed.png`). A second Esc in that time quits.
- **Withheld tools.** Each dish now starts on the first tool it offers, and keys for withheld tools do nothing. The built look already leaves a withheld tool's slot empty and marked "Not here", read from the same list the keys check. It agrees on all three dishes (`built-dish2-ruler-brief.png`, `built-dish3-dry-start.png`).
- **The mould died.** Running Dry left alone now ends the moment the last mould dies. The card reads "The mould died at 1:05", then "Running dry: 0 stars" (`built-o-dish-died.png`). "Not reached: only starving strays got there" is too wide for the big lettering beside the stars, so a line that long drops to the normal size (`built-dish1-wall-plan-end-s2.png`: Behind the Wall, the plan hand, seed 2).
- **The square glow round the food.** This was the band of moving food, not the glow. The game measures how far food has travelled in four directions only, so the distances around a pile form diamonds. Every band therefore left the pile as a pale square of 4-pixel blocks. Within 12 cells of a pile, the band now measures straight from the pile's edge, so it leaves as a ring. Further out it hands back to the game's own distances, so a cut still stops it. The band's front edge also ramps up over about four cells instead of jumping, which takes away the hard diagonal steps along every tube (the "patchy in a still frame" noted below). Compare `built-b-lamp-held.png` and `built-p-esc-armed.png` with the earlier copies.
  - Tried first: drawing the food glow itself pixel by pixel. It changed 838 pixels by at most 2 levels and cost about 0.2 ms, so it was taken out.
- **Cost after the fixes:** built 3.07 ms, old look 1.43 ms (+1.64 ms; 99th percentile 5.04 against 2.30). The run before the fixes, on the same machine, gave built 3.14 against 1.36, so the band change costs nothing measurable.
- **Ship bar:** still met on five seeds after the merge. Doing nothing scores 0 on all three dishes. Random scores at most 1 (Behind the Wall: one seed in five got 1). The intended hand scores 3 on every seed. Dish 1 with the intended hand costs 4.19 ms a frame, simulation and drawing together.

**What this is:** the look the owner picked is now the game's own. It uses B's dark culture: gold tubes glowing on dark agar, the lamp lighting the dish, mixed-case lettering, and the big storm count. It also takes C's labelled tool tiles (name, key, stock pips, "+1 in 5s", "3 left") and C's timeline to the storm, with late food marked on it. The window opens in it. F2 still cycles it, then the old look, then C, B and A, so any of them can be brought back. Its corner letter is **N** (for night). The blind letters A, B and C are unchanged.

**Where it is going:** this is the look the remaining work will be drawn in. The challenge dishes, the dish picker and the Wanderer are already drawn in it.

**What it costs:** **+1.5 ms a frame on average** over the old look. The 99th percentile is +2.5 ms. Measured on the same dish state frame by frame: 6,060 frames at window scale, with the oats preview live.

| | Draw mean | 99th percentile | Over the old look (mean / p99) |
|---|---|---|---|
| old look (`current`) | 1.29 ms | 2.18 ms | — |
| **built** | **2.79 ms** | **4.71 ms** | **+1.50 / +2.53 ms** |
| C | 2.57 ms | 4.18 ms | +1.28 / +2.00 |
| B | 2.90 ms | 4.86 ms | +1.61 / +2.68 |
| A | 3.64 ms | 6.24 ms | +2.35 / +4.06 |

The simulation adds 1.36 ms mean to every row, so a whole frame averages 4.2 ms against a 16.7 ms budget. The first working version cost 4.23 ms, +2.9 ms over the old look. These cuts brought it down (they also made B and C cheaper than in round 1):

- The glow is worked out on a half-size grid and read back smoothly. A glow is soft anyway, so nothing blocky shows. 0.70 → 0.21 ms.
- The glow is then reused for a second frame. A glow 1/60 s old cannot be seen.
- Each food pile's own glow now rides in the same pass instead of being painted pixel by pixel. Piles went 0.61 → 0.23 ms.
- The tool bar is redrawn only when something on it changes. The mould count goes on top each frame.
- The rim's storm flash is kept ready, like the bare dish.
- The corner chips are solid, not see-through.

### How to see it

- **Window:** `cargo run --release --bin mould`. It opens in the built look. F2 cycles the looks; `-- 0 B` still opens the lesson in B.
- **Pictures:** `cargo run --release --example mouldprobe -- looks=DIR seeds=1` writes the `built-*` files here, and every other look's as well. The probe's other picture modes now draw in the game's look unless told otherwise with `style=`. For example, `level=all look=DIR` draws the challenge dishes and `menu=DIR` draws the picker.

### The pictures

`built-sheet.png` shows all sixteen moments; each also has its own full-size `built-<letter>-<name>.png`. `built-storm.gif` plays the storm's last 4 s, the strike, 4 s of watching, then the score, at real speed (2.2 MB). `built-wander-pour.gif` plays the Wanderer's pour from Go to arrival (4.4 MB). The counts are in `built-counts.txt`.

| | Moment | Counts at that frame |
|---|---|---|
| a | Lesson step 2, oats preview | t 4.5 s, food joined 1/4, mould 758 |
| b | Lamp held beside the trunk | t 25.5 s, food joined 4/6, mould 1,239, joins flashed so far 9 |
| c | Storm in 3 s | t 87 s, food joined 6/6, mould 1,800, wound cells opened so far 442 |
| d | Score | 3 of 6 still joined after the storm, stars: join and short. Why line: "The storm cut off 3 piles: a loop holds" |
| e | Salt wall mid-drag | salt cells 246 |
| f | Snip held over a tube | snips used 0 |
| g | Briefing card | t 0 |
| h | Oats held 44 cells away | "Too far to pull: 44 cells" |
| i | **A second after a snip across the trunk**: the cut is still open and dark red | wound cells 145, food joined 4 → 3 |
| j | **Two pieces meeting**: the join flashes | t 10.1 s, joins flashed 6 |
| k | **The Wanderer's card** | t 0, mould 400 |
| l | **Go open**: fed mould in the crack. The crack beats, and the bar says how much would come | joined to the crack 1,258 of 1,265 |
| m | **3 s into the pour** | in line 507, through 532, died in line 194 |
| n | **The arrival card**: "Mabel arrived with 926 of 1,387" | through 926, died in line 307 |
| o | **Running Dry left alone**: "The mould died at 1:05" | t 65.1 s, mould 0, starved 1,382 |
| p | **The first Esc on a dish**: "Press Esc again to quit" | t 25 s, mould 1,266 |

The challenge dishes and the picker, drawn in the built look (from `mouldprobe level=all look=DIR` and `menu=DIR`):

- `built-dish1-wall-brief.png`, `built-dish2-ruler-brief.png`, `built-dish3-dry-brief.png`: each dish's own card. It shows the goal, the dish's one twist, and what each of the three stars needs. It lists only the tools the dish gives, with how many of each.
- `built-dish*-start.png` shows each dish as it opens. `built-dish*-intended-end.png` shows the end after the scripted three-star hand. Beat the Ruler's end shows the ruler, dotted from rim to rim, with the best possible answer faint beside it: "Best: 0.87x the ruler".
- `built-window-dish2.png` is the real window opening Beat the Ruler, in the built look.
- `built-menu-fresh.png` is the picker on a first start; `built-menu-some.png` is the picker part-played.
- The ship bar is unchanged on seed 1. Doing nothing scores 0 stars on all three dishes, random 0, and the intended hand 3. Dish 1 with the intended hand costs 3.74 ms a frame, simulation and drawing together.

### What the built look draws

Everything the old drawing draws now, in the new style:

- **Food seen moving:** a pale band rides out along every fed tube. It uses the same numbers as the old look, so a starving branch's beat stops at the cut.
- **Joins:** where two pieces of mould meet, a burst flashes and fades over 0.8 s, with a gold glow around it.
- **Snips:** an open snip shows dark wine while open and fades back to agar over its last second.
- **The Wanderer:**
  - The crack glows teal and beats once mould is in it.
  - "The way on" chip sits above the crack. It brightens when Go is open.
  - Her bar shows the dish number, mould now and best, and the food left as a draining bar (her clock). Below that is the Go line: "G: go. 1,258 of 1,265 would come", then the pour's own numbers.
  - Her briefing card and the arrival or "didn't make it" card.
- **Challenge dishes:**
  - "Bell in 0:41" replaces "Storm in" (these dishes have no storm).
  - What each star needs sits top left while the dish runs.
  - A tool the dish withholds keeps its slot, empty, marked "Not here".
  - The dish's own oats stock and regrowth are shown.
  - The rule's own number replaces the food pips where it reads something else: "All joined: 0.88x the ruler", or "Eating 2 piles".
  - The score card leads with the amount ("400 mould alive at the bell"), then the dish and its stars, then the why line at full brightness.
- **Score card on a scored dish:** the why line replaces the network-length line. It names the ratio itself when shape cost the star.
- **The picker:** cards at the same places the window's click test reads, with best stars, "Done" on the lesson, and what opens a locked dish.
- Fed yellow and starving grey remain the main readout throughout.

### What changed from round 1's B and C, and what didn't work

- The dot font's comma looked like a full stop at this size, so "1,265" read as "1.265". The mixed font now has a comma with a tail.
- "+1 in 13s" overflowed its tile in the mixed font. Tile text starts 8 px further left.
- The picker's cards were 56 px tall, and the mixed-case lines touched. They are now 64 px. The click test reads the same rectangles, so clicks still land.
- Their capital lines ("REACHED THE FOOD AT 0:11") are set in sentence case. A ratio keeps a small x ("0.88x"), and a line that opens on a number starts small ("400 mould alive").
- Not done: A, B and C in a Wanderer dish use the built look's tile bar, because they were drawn before she existed. In A and B a challenge dish shows all four tools. Only the built look and C leave a withheld tool's slot empty.
- The pulse band had hard cell-sized edges, as in the old look. It read as moving food at play speed but looked patchy in a still frame. Fixed in review 1 (above).

### Code and checks

- `src/mould/look.rs`: `Style::Night` and `Style::GAME`.
- `View::default()` stays the old look, so the Mould Organ, which paints over the old frame, is untouched.
- `src/bin/mould.rs`: the window opens in `Style::GAME`; one line changed.
- `render.rs` (the picker's card height): one constant changed.
- Merged in the thread branch (79a886f1), the challenge dishes (aeae6d29) and the review's fixes (1fbd739f). Conflicts kept both sides.
- Checks:
  - `cargo test --release --lib mould`: 35 passed, 3 ignored (after the review merge). The glyph test now covers every dish, picker and Wanderer string.
  - Clippy with `-D warnings` on lib and tests, `--bin mould`, `--bin organ`, `--example mouldprobe` and `--example organprobe`: clean.
  - `docscheck`: clean.

---

## Round 1 — three looks

**What this is:** the same dish drawn three ways, A, B and C. All three are real code that runs in the game window. The pictures are the window's own frames from one scripted run. Each look covers the whole list: a tool bar, the status, previews under the cursor, the briefing card, the lesson banner, the score, the dish itself, and the font. Today's look is `current-*` for comparison. The simulation is untouched, so every look is the same dish, step for step.

## The three, by feel

- **A**: warm and toy-like. Ink outlines, chunky sticker lettering, big coloured buttons on a wooden bench, red-and-white tape for the storm line, and a burning fuse for the time left.
- **B**: dark and alive. The tubes glow gold in the dark, the lamp lights up the agar around it, the buttons are round, the storm is a dial, and the text is in mixed case.
- **C**: clean and exact. A glass dish on graph paper, glossy raised tubes, labelled tool tiles with stock pips, and a timeline bar to the storm with the late food marked on it.

## How to see them

- Window: `cargo run --release --bin mould`, then **F2** cycles current → C → B → A (the letter shows in the bottom-right corner). `-- 0 B` opens the lesson in look B.
- Pictures: `cargo run --release --example mouldprobe -- looks=DIR seeds=1` writes every file in this folder. `style=A|B|C` draws any other probe picture mode in a look.

## The pictures

Each sheet (`A-sheet.png`, `B-sheet.png`, `C-sheet.png`, `current-sheet.png`) has the same eight moments in the same order: top row a b c d, bottom row e f g h. Each moment also has a full-size PNG (`A-c-storm-warning.png` and so on). `*-storm.gif` plays the last 4 s before the storm, the strike and 4 s of the watch, then holds on the score, at real speed (A 2.4 MB, B 2.3 MB, C 3.0 MB).

| | Moment | Counts at that frame (identical in every look) |
|---|---|---|
| a | Lesson step 2, oats preview under the cursor | t 4.5 s, food joined 1/4, mould 758, oats placed 0 |
| b | Mid-dish, lamp held beside the trunk | t 25.5 s, food joined 4/6, mould 1,199, oats placed 5, salt cells 159 |
| c | Storm in 3 s | t 87.0 s, food joined 6/6, mould 1,800, snips 3, storm-killed 0 |
| d | Score | t 101 s, joined 6/6 before the storm, **3/6 after**, storm-killed 6, stars 2 (join, short) |
| e | Salt wall mid-drag | 26 of 60 cells drawn, salt cells 246 |
| f | Snip held over a tube, not yet pressed | snips used 0 |
| g | Briefing card | t 0, mould 400 |
| h | Oats held 44 cells from the nearest mould | oats placed 5 |

The full counts are in `counts.txt`.

## What every look does (shared, so it is not a reason to pick one)

- **Tool bar** with an icon, key number, stock and the selected tool for each tool. Oats show their regrowth as the next pip filling (or a bar in A), and "+1 in 7 s".
- **Status**: storm countdown, food joined as one pip per pile (filled when joined; pending piles dotted), and mould size. **In the last 5 s** a big flashing number appears at the top of the dish, the dish rim flashes in the storm colour, and the timer goes red or violet. (The old look showed a single line of text.)
- **Previews** draw only what the code knows:
  - The **lamp** shows its real reach (16 cells) with arrows pointing outward, because the push carries the mould outward.
  - **Oats** show the ghost flakes where they will land and a thread to the nearest mould. The words grade the reach: "It will reach these" (within 28 cells), "It may reach these, slowly" (28–36), or "Too far to pull: N cells". Spots where oats can't go, or none left, show a red X and say why ("No oats: next in 7 s").
  - **Salt and snip** while dragging show a ring of how far the stroke can still go, plus "Wall left: 34 of 60". On hover, the **snip** paints exactly the tube cells a press would clear and says "Drag to cut this tube" or "No tube here".
  - **The pull reach is measured, not guessed** (`mouldprobe pull=1`, 5 seeds × 8 directions). By distance from the nearest mould, oats were reached within 5 s in:

    | Distance | Reached | Time |
    |---|---|---|
    | Up to 28 cells | 213 of 213 | 0.06–0.87 s |
    | 28–36 cells | 15 of 18 | about 2 s |
    | 36–40 cells | 2 of 7 | |
    | 40–48 cells | 0 of 4 | |
- **Score leads with an amount**: "3 of 6 food still joined", then "Joined 6 of 6 before the storm struck", then the network length against its bar, with the stars beside. Each pile on the dish gets a ✓ or ✗ for whether it ended joined, so the score points at the dish rather than covering it.
- **Dish**: tubes have smooth edges instead of 4×4 staircases. Fed yellow and starving grey stay the main readout in all three looks. Food piles carry a gauge of the food left, bright while being eaten. Oats are drawn as flakes with a time-left gauge. Late food shows a ring with a countdown for 5 s before it lands. A snip leaves a mark as wide as the cut. The storm line and the scar are labelled.
- **Hint and keys** sit in the dish's bottom corners, so the lesson banner and the storm count have the top of the dish.

## Frame cost

Measured on the same dish state frame by frame: the scripted fresh player's whole dish, 6,060 frames at scale 4 (the window), with the oats preview live. Times are for drawing only; the simulation adds the same 1.34 ms mean to every look.

| | Draw mean | 99th percentile | Extra over current (mean / p99) |
|---|---|---|---|
| current | 1.28 ms | 2.35 ms | — |
| A | 3.85 ms | 7.67 ms | **+2.6 / +5.3 ms** |
| B | 3.91 ms | 7.87 ms | **+2.6 / +5.5 ms** |
| C | 3.02 ms | 6.86 ms | **+1.7 / +4.5 ms** |

**All three cost more than the ~2 ms worst-frame allowance**, at the 99th percentile. The single worst frame is machine noise here: today's look alone has measured 7 to 42 ms worst on different passes, so the 99th percentile is the honest figure. A whole frame stays under 6 ms mean against a 16.7 ms budget.

Where the cost goes, measured per section at 40 s and at 87 s (`cargo test --release --lib mould::look::cost -- --ignored --nocapture`):

- **The dish pass** costs 1.1–2.3 ms. B is highest because of its glow.
- **The piles** cost 0.5–1.2 ms.
- **A's wooden bar** cost 1.5 ms before it was cheapened.

Already cut by about a third:
- per-pixel logarithms replaced by a lookup table;
- `hypot` replaced by a plain square root;
- rings now visit only their own pixels;
- rounded boxes fill their middles without measuring distance;
- the bare dish and rim are worked out once per look.

**Next cuts if a look is picked:**
- draw each pile's sprite once, and again only when its food or eaters change;
- redraw the bar only when a number on it changes;
- in B, blur the glow every other frame.

Each of these is an estimated 0.5–1 ms.

## What didn't work, or isn't done

- A's first tubes were three-tone cel shading; it read as speckle, so A now has one flat tone with a light and a dark edge. The trail's cell-by-cell grain also read as "carpet" in every look, so drawing now uses a softened copy of the trail (the simulation still reads the raw one).
- A's first agar had paper grain. It made A's GIF 10 MB, so A's agar is flat.
- B's glow is worked out per cell, so its halo is slightly blocky at close range.
- The previews show reach and stroke length, not the tube the mould will grow. That would need simulating ahead, which the brief ruled out ("cheap and honest").
- Not built: the pulse, snap flash and open wounds going into `render.rs` on the owner's branch. Each look will need its own version of those once they land.
- Code: `src/mould/look.rs` is new. `render.rs` gained one `View::style` field and a three-line hand-off at the top of `draw`. `mod.rs` gained `Dish::stroke_state()`, a read-only accessor. `bin/mould.rs` gained F2. The probe gained `looks=`, `pull=` and `style=`. `cargo test --release --lib mould`: 12 passed, 1 ignored (the timing). `cargo clippy --release --locked --bin mould --example mouldprobe -- -D warnings`: clean.

## Designer's recommendation

**B**, with two things from C. B makes the mould look alive at rest, and that is what the owner liked about it ("looks cool"). Gold glow against grey dim is the clearest fed-or-starving contrast of the three at phone size, and the dark dish makes every preview line, the storm line and the countdown stand out without boxes. Its round buttons have the least text on them, so I would take C's labelled tool tiles, or at least put the tool names under the buttons, and C's timeline with the late-food marks, which says more than the dial. A is the most charming and the easiest for a newcomer to read, but its pale agar makes the grey of a starving branch the weakest of the three. If A is picked, the starving tube needs a darker grey. C is the cheapest to draw by 0.9 ms and the safest, but it reads as an instrument rather than a pet.
