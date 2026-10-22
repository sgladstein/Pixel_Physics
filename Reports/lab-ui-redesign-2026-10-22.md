# Lab interface redesign: review and plan

## Status (edited in place)

**Step 1 shipped on `claude/project-thread-vclgr3`, 2026-10-22: the window rule.** Measured with `labui split=1`'s new per-tile window census (`render::paint_owner`, read off the pixels; a positive control is `paint_owner_counts_one_window_painted_over_another`):

| | before | after |
|---|---|---|
| tiles with any two windows on one pixel | 71 of 73 | **0 of 70** |
| pixels painted by one window over another, summed | 627,757 | **0** |

What it is: pages dock on the left, BATTLE then the cell page and BIOSPHERE on the right, all between the clock line (`ui::DOCK_TOP`) and the tab strip (`ui::dock_bottom`). When two would still touch, the one opened or clicked last stays and the other folds to a title strip you click to bring back. Every window has the same `X`. Hover explanations and the LOOK readout search for a clear spot (`ui::place_clear`) and show their first sentence until the pointer rests. The notice line moved into the tab strip. Also: BIOSPHERE and BATTLE stop clicks; ESC closes the window opened last and quits only on a second press with nothing open; REBUILD asks twice; the wheel scrolls the window under it and zooms only over the box; BIOSPHERE opens shut, and a short welcome card replaced the 30-line key list as the first screen; a page and BIOSPHERE may now be up together. Not done in step 1: "+N MORE" is still not scrollable on the generic pages (step 4 replaces most of those lists with charts).

Steps 2 to 5 below are the plan as reviewed; each gets its line here when it lands.

2026-10-22. Scope: the evolution lab (`cargo run --release --bin lab`): every window, page, menu and the bar. Nothing here changes the simulation; every step is checked for bit-identical results.

**The short version.** The lab's interface grew one page at a time, and each page placed itself. So windows land on top of each other, clicks and the scroll wheel go straight through some of them into the box, a few close only by key, the menu is two flat lists of 31 items, and the pages that matter most show their data as long tables or walls of numbers. The plan fixes it in five steps: (1) one layout rule for every window, so nothing overlaps, nothing lets a click through, and everything closes the same way; (2) a test of crisp high-resolution text, the biggest single step towards "modern", decided on its measured cost before anything is laid out twice; (3) a bar with direct page buttons and a menu grouped by task; (4) charts and short summaries in place of the long lists, with the full table one click away; (5) one consistent look for every window.

An independent adversarial review checked this plan against the code before you saw it. It confirmed the bugs, caught three it had missed (the scroll wheel, a permanent false warning, R rebuilding on one press), and showed that four charts needed data the lab does not keep. All of that is folded in below.

## 1. What is wrong today

Measured on today's build with the `labui` contact sheet (73 page states) and the real app under a virtual display. Pictures are in `today/`.

### Windows overlap
Every floating thing places itself, and only two of them know about each other.

| What lands on what | Why | Picture |
|---|---|---|
| The hover card (what the cursor is over) covers the top of a page | it sits at a fixed spot top-left and dodges only the CELL window | `today/hover-card-over-page.png` |
| BIOSPHERE covers the BATTLE panel | both are anchored top-right and neither knows about the other | `today/stats-over-battle.png` |
| A hover explanation, CELL and BATTLE pile up over WHAT HAPPENED | explanations go beside the page they explain, whatever is already there | `today/pileup.png` |
| Tall pages (CELL, SIDE BY SIDE) run into the clock at the top | a page grows upward from the bar | `today/cell-genome.png`, `today/side-by-side.png` |
| The centre notice ("TOOL LOOK") prints over a page's bottom row | drawn last, at a fixed spot | |
| A roster's last column runs under CELL | the roster draws text past its own box | `today/every-animal.png` |

### Windows that do not close, or do not stop the mouse
- **Clicks go through BIOSPHERE and BATTLE into the box.** Confirmed in the real app: a click on BIOSPHERE opened a CELL window for the empty cell behind it (`today/click-through-stats.png`). Brushes start on the press, so with a brush armed it pours through the panel.
- **The scroll wheel always zooms the box**, even over a page, so no window can be scrolled with it. Pages that run out of room drop their last rows with "+N MORE" and no way to reach them.
- **BIOSPHERE prints "TAB CLOSE" but clicking it does nothing**, and it opens by itself on a fresh lab, hidden behind the key list.
- **BATTLE has no close button**, only "F9 HIDES".
- **PLANTS, ANTS and THE BOX have no clickable close**; F1 to F3 or opening another page closes them.
- **Esc quits the whole lab**, even while the key list is showing. Most programs use Esc to close the top window.
- **R throws the box away on one press**, with no confirmation.
- **"PIXELS X4 -> X1 (WINDOW)" shows permanently** at default settings: the default asks for a pixel budget the default window cannot give. It is a warning about nothing, sitting where a page title goes.

### Long lists and dense tables
| Page | Today | What a player wants from it |
|---|---|---|
| EVERY ANIMAL / EVERY PLANT | a 9-column table, 14 rows a page, often 50 to several hundred rows | how the population is spread, and which few are worth a look |
| HISTORY | one row per family line, can run to hundreds | how long lines last and what ends them |
| WHAT HAPPENED | a log of hundreds of lines | is anything unusual happening, and the last few events |
| CELL → GENOME | 30+ signed numbers in a column | how this one differs from its colony |
| SIDE BY SIDE | two columns of 30+ numbers | where the two differ |
| COLONY TRAINING → best ants | 25 dense rows | is selection working round over round |
| BIOSPHERE | already has a strip, a band and a histogram, but between 15 lines of text | four headline numbers and whether they are rising |
| PARAMETERS | "- 3.300 + 0 TO 40" per row | where a value sits in its range, and whether it was changed |
| Hover explanations | median 27 words, longest 121, all capitals | one sentence, with the rest on demand |
| The opening key list | 25 lines of bindings over the whole box | how to start |

### Confusing menus
- **MENU is 31 items in two flat columns.** Main pages (PLANTS, ANTS) sit beside their own sub-pages (EVERY PLANT, FOOD) with nothing to say which belongs to which. ANT BRAIN, which changes the simulation, is filed as a page.
- **The VIEW column mixes four kinds of thing**: overlays, clock settings, a one-off action (SAVE CHRONICLE NOW) and five magnifier knobs only a developer needs. Rain, zoom and the pixel budget are keyboard-only and appear nowhere.
- **Jargon names**: THE RACK, THE SHELF, SIDE BY SIDE, REACTION ON EVENT, DISPLAY FLOOR. "MOUSE" printed in the key column says nothing.
- **Seven speed buttons take a third of the bar's second row**, which is why the main pages could not fit on the bar, and RESET (which throws the box away) sits beside HELP.

## 2. The proposal

### Step 1: one layout rule for every window (bug fixes)
Fixed regions, and every window lives in one:
- **Clock**: top-left, one line, as today. Its extra warning lines move to the right of an open page instead of under it.
- **Left dock** (at most 210 of 512 px): the page you opened. Always the same place, from under the clock down to the tab strip.
- **Right column** (at most 180 px): BATTLE at the top when it is on, then the inspector (CELL / one individual) or OVERVIEW below it. CELL and OVERVIEW can both be open; OVERVIEW then folds to its four headline tiles.
- With both sides open, **at least 120 px (a quarter) of the box stays visible**. The test sheet prints the visible width per picture.
- **The hover card keeps a fixed home** (the code chose that on purpose: a card under the pointer covers what it describes), but picks the first of a few fixed spots that no window covers. **Explanations and notices** are placed the same way, against every window drawn that frame.

Every window behaves the same:
- a title bar with an **X close button**, and page buttons (BACK, TABLE, SAVE) beside it;
- **no click and no scroll passes through a window**; the wheel scrolls the window under the cursor, and "+N MORE" becomes a scrollable list;
- **Esc closes the top window**; with nothing open, a second Esc quits. **R asks for a second press** the same way;
- OVERVIEW starts closed; a short welcome card replaces the key list on a fresh lab (`mockups/m6-numbers-welcome.png`);
- the false PIXELS warning goes (the default budget will match what the default window can give).

**How "nothing overlaps" is checked without trusting the new code.** Each painted pixel is tagged with the window that painted it (test builds only), and the sheet counts pixels painted by two windows. This reads the pixels, not the layout list, so a window that forgot to register still counts. It is run once with an overlap deliberately put back, to prove it can see one. The sheet gains the tiles it lacks today: COLONY TRAINING, SCENARIOS, BATTLE beside a page, the key list, SIDE BY SIDE with two ants.

### Step 2: crisp text, tested before anything is redrawn
`mockups/m7-text-compare.png` (left today, right the proposal). The 5x7 capitals drawn at 512x320 and blown up are the main reason the lab looks dated. The interface can be drawn on its own layer at 2x (1024x640) while the box stays pixel art. This reverses a choice recorded in the code (`render::Hud`): text was kept blocky on purpose so it reads as part of the same square-celled picture as the world. The comparison picture is how to judge whether that still holds.
- **Same layout**: a sharper font on the same character grid at 2x, so no page has to be laid out again. Capitals first; mixed case later and only for prose, because every string in the code is written in capitals and converting them automatically produces things like "Worth A look".
- **Cost, honestly**: BIOSPHERE alone measured 1.11 ms per frame at 1x in the held-world game, and 2x is four times the pixels. Two full docks could cost several milliseconds. The layer can skip repainting when nothing on it changed, which would also end today's forced full repaint of the box whenever the mouse is in the window, so the net could be a saving.
- **What it touches**: a second texture and a small shader in the lab binary only, and the screenshot hooks and test sheet must merge the two layers.
- **Decision rule**: built as a spike behind a switch, measured with `ascii` and the lab benchmark. It ships on only if a settled box with both docks open costs no more than today. If it fails, steps 3 to 5 go ahead at 1x unchanged.

### Step 3: the bar and the menus
`mockups/m4-menu.png`, `mockups/m4b-view.png`.
- **Direct page buttons on the bar**: OVERVIEW, PLANTS, ANTS, HISTORY, LAB. They fit because the seven speed buttons collapse to one cell showing both the speed asked for and the speed achieved ("ASK 64X / GOT 61X"), with < and > beside it. Keys 1 to 7 still jump to a speed; clicking the cell opens the full ladder. RESET moves into the menu.
- **Each page is a window with tabs**: ANTS = Summary | Food | Every ant; PLANTS = Summary | Every plant; HISTORY = Lines | What happened; LAB = The box | Numbers | Chambers | Scenarios | Colony training. Sub-pages appear only as tabs, never twice.
- **MENU becomes a grouped index** (Watch, Individuals, Experiment) with Rebuild, Save chronicle and Quit along the bottom. ANT BRAIN becomes a labelled switch under Experiment showing its state.
- **VIEW holds every display toggle**, grouped: Paint over the box, Panels, Clock, Weather (rain), Zoom and pixels, Magnifier (five knobs behind one row).
- **Plain names**: THE RACK → Chambers, THE SHELF → Kept jars, REACTION ON EVENT → On an event, DISPLAY FLOOR → Lowest frame rate. Keys and behaviour do not change.

### Step 4: charts and summaries instead of long lists
`mockups/m2-ants.png`, `mockups/m3-history.png`, `mockups/m3b-what-happened.png`, `mockups/m5-training.png`, `mockups/m1-overview.png`.
Every list keeps its full table one click away (TABLE), because picking one individual is still a real job. What changes is what opens first. Time axes are world frames over the whole run, never minutes, because the speed dial runs from 1x to 1024x.

| Page | Opens on | Data |
|---|---|---|
| ANTS summary (PLANTS likewise) | a bar of what they are doing; age against energy with 3 px dots, hover names the nearest and click follows it; energy histogram with median, low count and best; three "worth a look" rows | roster, as today |
| HISTORY → Lines | how lines ended as one stacked bar; how long lines lasted, as a histogram stacked by cause (scales to thousands of lines, unlike a row per line); lines alive over the run; the four longest | lineage records, as today |
| HISTORY → What happened | births and deaths over the whole run as a strip; then the newest lines with ALL / DEATHS / BIRTHS / YOU filters, wheel-scrollable | births/deaths from the stats samples; lines from the log |
| CELL → GENES | the 12 genes where this ant differs most from its colony, as bars from the colony mean, each scaled to its gene's range, biggest first | genome + a colony mean, computed when the page is open |
| SIDE BY SIDE | the same bars for the two ants, biggest differences first | genomes |
| COLONY TRAINING | headline tiles, a round progress bar, food home, alive and nest score round over round, then the best five | round summaries, as today. Dots per ant per round need one small new list per round |
| OVERVIEW | four headline tiles (plants, ants, births, biomass) with change and sparkline; population over the run; what the ants are doing; generations | stats samples, as today |
| NUMBERS | a slider per value, a mark where the shipped default sits, changed values highlighted | parameters, as today |
| Hover explanations | the first sentence; the rest after the cursor rests for a second | as today |

### Step 5: one look, and feel
A title bar with an accent line, consistent padding, section headings with a rule, one palette (green good, amber fair, red poor, blue for links and ants), tabs and filter chips, a shadow so windows separate from the box. And feel, since that is the project's first rule: buttons that visibly press, hover highlights on rows and dots, charts that update live, and a selected ant marked in the box at the same moment it is picked on a chart.

### What is shared with the held-world game
BIOSPHERE's painter is also used by the held world (`druid`). The new OVERVIEW is lab-only code; the old page stays for the held world, and the close button added to it is switched on only by the lab.

## 3. Order of work and how each step is checked
1. **Layout rule and close bugs**, with the pixel-tag overlap count (target 0, positive control first), click- and scroll-through tests, and visible box width per picture.
2. **Crisp text spike**, kept only if its measured frame cost passes the rule above.
3. **Bar and menus**, checked by the sheet's existing click-every-button pass and the bar width trace.
4. **Charts**, page by page in order of use: ANTS, HISTORY, CELL genes, COLONY TRAINING, OVERVIEW, the rest, with before and after pictures for each.
5. **Look and feel**, alongside 4.

Every step: the same seeded box run with and without the change must give the same world, and frame cost is quoted from `ascii` and the lab benchmark for each.

## 4. Choices made for you, which you can overrule
- **Docks rather than draggable windows.** At 512x320 there is no room to arrange windows by hand, and dragging is how they end up on top of each other.
- **Esc and R need a second press** to quit or rebuild.
- **The full tables stay** behind TABLE.
- **The speed buttons collapse to one cell**; keys 1 to 7 are unchanged.
- **Crisp text ships only if it measures free**; otherwise the rest goes ahead at today's resolution.
