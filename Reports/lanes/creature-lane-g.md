# Lane G — the colony panel

*Written 2026-08-30. Landed; see the last section.*

## What landed

**The colony is now readable in the game.** `SHIFT+Y` opens a panel down the
left of the screen with the world still visible beside it: how many ants are
alive and whether that is climbing, what they are eating, carrying and
digging, how far short of breeding they are, and what is left of the family
lines. Play-facing account in `wiki/ants.md` §*Reading the colony*.

**Where it sits.** Every number this project tracks about creatures lived in
`examples/creature_probe.rs` and its siblings — logs the owner never runs — so
on screen a colony was fifty dots with no way to tell a thriving one from a
dying one. This is the readout half of the creature line: the same quantities,
in the running game, in the words `wiki/ants.md` uses. It does not change a
single thing an ant does.

Cost fork: **built**. Everything the panel needed was reachable from `World`,
so Lane D's files were not touched; what was not reachable is under *What is
missing*.

## The key

`SHIFT+Y`. Every plain letter was bound — `main.rs`'s own `KeyY` arm calls `Y`
"the last free letter" — and a modifier on the key that *founds* a colony is
the one binding nobody has to be told twice. Shift is `held.grab`, the single
source of truth the gnome's tree-grip already reads; plain `Y` still founds.

**The help page had no room for it**: both columns held exactly 30 rows and
both were full, so `HELP_MARGIN` came down 20 -> 14 to buy a 31st.
`the_help_page_fits_inside_its_own_panel` reads that constant, so the row
budget stays in one place — worth knowing before the next lane needs a key.

## What is on it, and why that

**Read `README.md`'s *The ant colony — status* for the full account** and
`wiki/ants.md` §*Reading the colony* for the play-facing one — those are the
documents a reader finds without being told this lane existed. In short: four
sections answering *is this colony doing well* / *can it afford to keep going*
/ *what is it doing right now* / *is there anything left to select on*, with
distributions rather than means, rates rather than totals, and a rate window
(3,840 frames) a little longer than a day.

Two things only here:

- **`nest_visits` is deliberately absent.** Its own doc records that it counts
  loitering, not trips — one ant, a nest and no food scores `moves 648,
  nest_visits 389`. `forage_trips` is on the panel instead.
- **Two rows appear only when they have something to say**, both otherwise
  silent: `BIRTHS REFUSED NO ROOM / NO SLOT` (a birth the terrain refused and
  one the address space refused both read as "nothing is breeding", which is
  also what a colony too poor to try reads as, and they want opposite fixes),
  and `RATES ABOVE COUNT EVERY CREATURE` — see *What is missing* below.

The one approach withdrawn — a fixed-size panel painted with a running cursor
— is in `Reports/dead-ends.md` under *rendering*.

## What it costs

| | ms/frame |
|---|---|
| panel shut | **0.086** |
| panel open | **3.880** |
| difference | **+3.794** |
| one census of the whole colony | 0.0128, twice a second while open |

**How it was measured matters more than the numbers**, and it is the part
`README.md` does not carry: one binary, one process, 200 alternating pairs,
and the sim **held still** after 3,000 frames. A world still settling redraws
every pixel anyway, so the closed arm would not have been measuring the render
skip at all and the delta would have flattered the panel. Held still, the
closed arm takes the skip and the open arm pays a full redraw, which is the
worst case and the one worth quoting.
`report_the_colony_panel_frame_cost_when_asked` (`COLONY_COST=1`) is a
measurement and **not a gate** — a wall-clock assertion is a flake generator.

Shut, the panel adds one boolean test per frame: the census and the trend
sample are both behind `show_colony`, so a settled world keeps the dirty-rect
skip, which is what the 0.086 is. Open it costs a full redraw, the same
bargain the help page, the options panel and the palette already make, which
is why they are all in `force_full`. `ascii` is unaffected — nothing in the
simulation changed, and it never calls `App::draw`.

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

Six, all named `*colony*` in `src/app.rs`'s test module. What is worth
carrying is *which direction* three of them assert, because the obvious half
of each is the blind one:

- `a_closed_colony_panel_does_no_work_and_an_open_one_does` — a
  closed-does-nothing check alone passes for a panel that never works.
- `hovering_a_colony_row_explains_it_and_hovering_a_gap_does_not` — a
  note-appears check alone passes for a panel that draws a box wherever the
  cursor is.
- `the_colony_panel_stays_inside_its_own_border` also asserts **over 2,000
  pixels lit**; containment alone passes for a panel that draws nothing.

And every string goes through `App::colony_text`, which `debug_assert`s the
font has a glyph for each character — the help page guards that over its
literals and this panel cannot, since most of its text is built at run time.

## One thing that cost time and is worth carrying

**A 5x7 glyph read off a downscaled screenshot is not evidence about what the
panel said.** I read `PICKED 0` beside `FOOD HOME 13.9` off a card image and
went hunting a contradiction in the simulation — deliveries cannot exceed
pickups, digging destroys its spoil. The panel had said `PICKED 30`; the `3`
was one glyph wide at 1x. `dump_the_colony_panel_when_asked` now prints every
row as text beside the PNG. The picture is for *what and where*; the text is
for *how much*.

## Gates

| | |
|---|---|
| `cargo test --lib` | 1114 passed, 0 failed, 54 ignored |
| `cargo +1.98.0 clippy --all-targets -- -D warnings` | clean |
| `cargo run --release --example ascii` | 31 scenes, 0 skipped |
| `bash scripts/acceptance.sh` | all cases met their expectations |
| `bash scripts/docscheck.sh` | clean |

Re-run in full after each `main` merge and after the hover, not once at the
start.

## What the owner said, and the one thing still open

**All three cards answered, all three approved.**

| card | asked | answered |
|---|---|---|
| `20260830T052805753Z-7ae046` | the panel, first version | *"This look good. more better graphs might be interesting… the user should be able to mouse hover over some of the words and get and explaination of what it means and this could also be a way to access more details data."* |
| `20260830T060547213Z-a172fe` | the same panel post-merge, on a hungry colony — right things on it? rate rows earning their space? too wide? | *"looks good"* |
| `20260830T070742689Z-1ae70e` | the hover, and what he would want graphed | *"looks good"* |

**The hover was the actionable half of the first answer, and it shipped in the
same PR.** Every row carries a `ColonyRow::note` drawn by
`App::draw_colony_note`, and a note is not a glossary entry: it says what the
row means *and carries what did not fit on it* — the raw counts behind each
rate, the exact hunger threshold behind the histogram's colours, what each
trait is and which way its axis runs. The box sits beside the panel, level
with the row, not under the pointer, because a box that follows the cursor
covers the line it is explaining.

**Still open: which graph.** *"More better graphs might be interesting"* was
put back to him rather than guessed at, with two concrete proposals — food
delivered over time beside the population strip, and the energy histogram as a
moving band rather than a snapshot. He answered **"looks good" and named
neither**, so read that as approval of the hover and *no direction on graphs*,
which is not the same as a request for them. Both proposals are still unpriced.
Anyone building one should post the picture before the code: "more better
graphs" is exactly the ambiguous complaint `CLAUDE.md` says to resolve by
rendering both readings rather than spending the detour on the wrong one.

Card `…a172fe` supersedes `…7ae046`, whose *picture* is not worth reading (it
predates the `main` merge, so every ant sat at the same energy and the
histogram was one bar). Its *text* is where the hover was asked for.

## Landing it, and the conflict every creature lane will hit

**Merged as `c4bb564` (PR #157), 2026-08-30.** The branch was deleted on
merge, so the SHAs are history rather than somewhere to push: `29501b8`
(second `main` merge) over `80ec04f` (the hover) over `b07847b` over
`322ea66` (first `main` merge) over `a25584f` (the panel).

`main` came in **twice** — 33 commits, then 17 more — because the creature
line landed several PRs while this one was open. Both merges conflicted in
exactly the same two places, and the shape is worth recording because it is
**structural rather than bad luck**: `README.md`'s generated table of contents
carries a line number for every section, so any insertion anywhere moves all
of them; and every creature PR writes a dated line at the top of
`wiki/ants.md`. Every creature lane will hit both.

Neither needs judgement. The TOC is resolved by taking main's side and
re-running `scripts/readmetoc.py` — but **check first that every conflict is
inside the generated block**, which is three lines of python and the
difference between a merge and a silent loss of real content. The freshness
note is resolved by keeping both dates in one sentence, newest first.

**A clean merge is not a safe one here.** At merge time `main` was 14 commits
further on and GitHub reported `clean`, which is the state `CLAUDE.md` records
twice as having broken the tree anyway. So the merge was simulated first —
`git worktree add --detach` at the then-current `origin/main`, merge the
branch into it, and run `docscheck`, the build and the colony guards there.
Clean, then merged, then `docscheck` re-run against the real landed `main`.
Clean both times. That whole check costs about a minute and is the only thing
that sees a stale generated file.

## The trait guard paying for itself

`organism::CREATURE_TRAITS` went 1 -> 2 in the first `main` merge, while this
branch was open. The panel grew a `DOWRY` row with **no layout edit** — only a
name and a one-line meaning added, both optional (it would have drawn
`TRAIT 1` and been correct). That is the whole reason the trait section is
sized by the constant.

It also made the panel a far better demonstration of itself: with `main`'s
smaller food budget and its new starvation path, the same colony reads
`HUNGRY 37 OF 40` with the energy piled against the empty end and one ant
still nearly full, where before the merge all forty sat on one bar.
