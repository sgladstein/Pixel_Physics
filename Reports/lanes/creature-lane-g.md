# Lane G — the colony panel

*Branch `claude/creature-lane-g-panel`, cut from `main` at `6d5cbcf`.
Written 2026-08-30.*

## What landed

**The colony is now readable in the game.** `SHIFT+Y` opens a panel down the
left of the screen with the world still visible beside it: how many ants are
alive and whether that is climbing, what they are eating, carrying and
digging, how far short of breeding they are, and what is left of the family
lines. Play-facing account in `wiki/ants.md` §*Reading the colony*.

**Where it sits.** Every number this project tracks about creatures lived in
`examples/creature_probe.rs` and its siblings — logs the owner never runs — so
on screen a colony was fifty dots and no way to tell a thriving one from a
dying one. This closes that: the same quantities, in the running game, in the
words `wiki/ants.md` uses. It is the readout half of the creature line; it
does not change a single thing an ant does.

Cost fork: **built**. Everything the panel needed was reachable from
`World`, so Lane D's files were not touched. The one thing that was not
reachable is named under *What is missing* below.

## The key

`SHIFT+Y`. Every plain letter was already bound — `main.rs`'s own `KeyY` arm
calls `Y` "the last free letter" — and a modifier on the key that *founds* a
colony is the one binding nobody has to be told twice. Shift is `held.grab`,
the single source of truth the gnome's tree-grip already reads. `Shift+Y` no
longer founds a colony; plain `Y` still does.

The help page (`/`) had no room for it: both its columns held exactly 30 rows
and both were full, so `HELP_MARGIN` came down from 20 to 14 to buy a 31st.
`the_help_page_fits_inside_its_own_panel` reads that constant, so the row
budget stays in one place.

## What is on it, and why that and not more

Two questions before any detail, because a panel of twenty counters is a wall:

| section | answers |
|---|---|
| headline, trend strip, placed/born/died, births+deaths per 1k | **is this colony doing well** |
| ENERGY: hungry count, histogram, low/mid/high, the bud gauge | **can it afford to keep going** |
| OUT AND ABOUT: carrying, airborne, excursion depth, work rates | **what is it doing right now** |
| LINEAGE: generation, lines, top share, one row per trait slot | **is there anything left to select on** |

Three rules decided the content:

- **Distributions, not means.** Energy is a histogram; energy, excursion depth
  and every trait are low / middle / high. A colony half starving and half
  comfortable has the same mean as one uniformly mediocre and is not the same
  colony. The histogram's split is `hunger_fraction * start_energy` — the line
  `creature::act`'s own `hungry` tests, marked on the axis, not a display
  threshold invented for the picture.
- **Rates, not totals**, for anything cumulative. `moves` climbs for ever and
  says nothing after the first minute. Placed / born / died stay totals
  because that is what they are.
- **The rate window is 3,840 frames** — 128 samples 30 apart, just over
  `field::DAY_NIGHT_PERIOD_FRAMES` (3,600). Every rate a colony produces rides
  the day/night cycle, so a short window reports the hour. This is
  `CLAUDE.md`'s divide-out-the-oscillator rule applied to a readout rather
  than to a measurement.

Two rows appear only when they have something to say, both of them silent
failure modes otherwise invisible: **BIRTHS REFUSED NO ROOM / NO SLOT** (a
birth the terrain refused and one the engine's address space refused both read
as "nothing is breeding", which is also what a colony too poor to try reads
as, and they want opposite fixes), and **RATES ABOVE COUNT EVERY CREATURE**
when a second creature species is in the world — see *What is missing*.

Deliberately **not** shown: `nest_visits`, whose own doc records that it
counts loitering rather than trips (one ant, a nest and no food scores
`moves 648, nest_visits 389`). `forage_trips` is on the panel instead.

## What it costs

Measured on one binary in one process, 200 alternating pairs, on a **settled**
world — the sim held still after 3,000 frames, because a world still settling
redraws every pixel anyway and the closed arm would then not be measuring the
render skip at all. `report_the_colony_panel_frame_cost_when_asked`
(`COLONY_COST=1`), which is a measurement and not a gate.

| | ms/frame |
|---|---|
| panel shut | **0.086** |
| panel open | **3.880** |
| difference | **+3.794** |
| one census of the whole colony | 0.0128, taken twice a second while open |

**Shut, the panel adds one boolean test per frame** — the census and the trend
sample are both behind `show_colony`, and nothing else in the frame changed —
so a settled world keeps the dirty-rect skip, which is what the 0.086 is. Open
it costs a full redraw, which is the same bargain the help page, the options
panel and the palette already make and is why they are all in `force_full`.

The census itself is cheap because it walks `World::live_organism_ids` and
filters to species with a `creature` — not the 163,840-cell grid scan
`creature_probe` uses.

`cargo run --release --example ascii` is unaffected: nothing in the simulation
changed, and `ascii` never calls `App::draw`.

## What is missing, for whoever owns `creature.rs`

Named rather than added, because Lane D holds those files.

1. **`CreatureStats` has no species dimension.** Every counter on it is
   world-wide. The census is per-species (the panel names the species with the
   most live individuals and every censused figure is that species'), but every
   `PER 1K` rate is every creature in the world. With an ant colony and a
   beetle in the same world those rates are a mixture. The panel says so on
   screen rather than hiding it, but the fix is a per-species split of the
   counters.
2. **Deaths have no cause.** `deaths` cannot separate starved from burnt from
   crushed, and that is the single thing most worth knowing when a colony is
   shrinking. One `deaths_by_cause` array would put a real answer on the panel.
3. **No standing readout of what an ant is *doing*.** `carrying` and `flight`
   are the only behavioural states on `OrganismState`, so "how many are
   foraging vs digging vs heading home" cannot be drawn. The brain's chosen
   verb is not stored anywhere after it is acted on.
4. **`tumbles` is tracked and not shown.** Its ratio against `moves` is
   searching-vs-commuting and would be a good row; it was left off to keep the
   rate block to three lines. Free to add whenever someone wants it.

## Guards

- `a_closed_colony_panel_does_no_work_and_an_open_one_does` — both halves,
  because a closed-only check passes for a panel that never works.
- `the_colony_census_counts_creatures_and_not_the_forest` — every generated
  world's moss and trees own `OrganismState`s in the table the census walks;
  the negative half asserts a plant-filled world with no ants censuses as *no
  census at all*, not a colony of zero.
- `the_colony_census_reports_one_spread_per_trait_slot` — sized by
  `organism::CREATURE_TRAITS`, so a slot added by another lane appears without
  an edit here (as `TRAIT n` until someone names it in `colony_trait_label`).
- `the_colony_panel_stays_inside_its_own_border` — drawn onto a zeroed frame,
  every lit pixel must be inside the rect, the rect must equal the rows' own
  height, and **more than 2,000 pixels must be lit**: the containment half
  passes for a panel that draws nothing, which is the blind-guard shape
  `CLAUDE.md` says to put the fault back for.
- `colony_rates_need_a_window_before_they_report_anything`.
- Every string the panel draws goes through `App::colony_text`, which
  `debug_assert`s the font has a glyph for each character. The help page guards
  this with a test over its literals; this panel cannot, because most of its
  text is built at run time out of species names and formatted numbers.

## One thing that cost time and is worth carrying

**A 5x7 glyph read off a downscaled screenshot is not evidence about what the
panel said.** Reading a card image, I saw `PICKED 0` beside `FOOD HOME 13.9`
and spent a while establishing that deliveries cannot exceed pickups and that
digging destroys its spoil, i.e. hunting a bug in the simulation. The panel had
said `PICKED 30`; the `3` was one glyph wide at 1x. `dump_the_colony_panel_when_asked`
now prints every row as text beside the PNG, and the raw counters over the
window with it. Look at the picture for *what and where*; read the text for
*how much*.

## Gates

| | |
|---|---|
| `cargo test --lib` | 1099 passed, 0 failed, 54 ignored (re-run after the `main` merge) |
| `cargo +1.98.0 clippy --all-targets -- -D warnings` | clean |
| `cargo run --release --example ascii` | 31 scenes run, 0 skipped; worst frame 37.54 ms / mean 3.64 ms on the 166-organism scene |
| `bash scripts/acceptance.sh` | all cases met their expectations |
| `bash scripts/docscheck.sh` | clean |

## Review

Card **`20260830T060547213Z-a172fe`** on board `creatures`, posted
2026-08-30: the panel over a hungry 40-ant colony, asking whether the right
things are on it, whether the three rate rows earn their space, and whether
it should be narrower. Verdict not yet collected —
`python3 scripts/review.py inbox`.

It supersedes `20260830T052805753Z-7ae046`, posted half an hour earlier and
**not worth reading**: that shot predates the merge with `main`, so its ants
were on the old full-size food budget and every one of them sat at the same
energy — the histogram was a single bar, which is the worst possible
advertisement for the reason it is a histogram. Post-merge the same panel
reads `HUNGRY 37 OF 40` with the mass piled against the empty end and one ant
still nearly full, which is the case the shape exists for.

## Pulled in

`main` came in at `322ea66` — 33 commits, conflicting only in `README.md`'s
generated table of contents (resolved by taking main's side and re-running
`scripts/readmetoc.py`) and in `wiki/ants.md`'s freshness note, where both
sides had written a 2026-08-30 line and both were kept.

That merge is also the trait guard paying for itself: `CREATURE_TRAITS` went
1 -> 2 while this branch was open, and the panel grew a `DOWRY` row with no
edit to the layout — only a name added to `colony_trait_label`, which is one
line and optional (it would have drawn `TRAIT 1` and been correct). It also
made the panel a much better demonstration of itself: with main's smaller
food budget and its starvation path, the same colony reads `HUNGRY 37 OF 40`
with the energy piled against the empty end and one ant still nearly full,
where before the merge all forty sat on one bar.

## Head

`b07847b815f8a8ac11f1cb72ec9ec4ea4242a275` on
`claude/creature-lane-g-panel`, which is `322ea66`'s merge of `main`
(`3a86ff5`) plus `a25584f` (the panel) plus this note.

Files touched: `src/app.rs`, `src/main.rs` (the `SHIFT+Y` arm only),
`README.md`, `wiki/ants.md`, `Reports/lanes/creature-lane-g.md`. Nothing in
`src/sim/`, nothing in `assets/`, nothing in `examples/`.
