# Held world — the coordinator's round archive

What each round of the **held world** (`cargo run --release --bin druid`)
program overturned, moved out of
[`lanes/druid-program-coordinator.md`](lanes/druid-program-coordinator.md) so
that note can stay small and current. The lab keeps its finished rounds the
same way, in
[`evolution-lab-rounds-archive.md`](evolution-lab-rounds-archive.md), and for
the same reason: **what a round overturned is the part a later session cannot
reconstruct**, and it is also the part nobody needs loaded to do today's work.

Read the one round, not the file.

---

## Round 1 — the seven playtest items, 2026-09-14

Five lanes, five PRs: #411 (the screen and the GUI), #413 (a bare world and
the seed pouch), #414 (founding in a thicket), #415 (the absorb diagnosis,
§Z22), #418 (the bubble aura). Item 6 (one hue) landed direct from the
coordinator, `91567399`.

## What the sizing probe overturned, before any lane was dispatched

All seven items have landed, so the per-item sizing is now in the PR bodies.
**Two findings outlived the round**, and both are the same shape — *a doc
comment claiming a behaviour the code does not have*:

- **Item 2 was not an arrow deletion.** `Draw::amount` carried the energy and
  its own doc at `mod.rs:368` already claimed it *"sets how heavy the flow
  looks"*; the renderer never read it, and the count was the constant
  `PER_DRAW = 26`. So the order was forced — make the count proportional
  first, then delete the chevrons, or the player is left with no readout at
  all. Same shape as the `ant_wide` blurb PR #408 unshipped.
- **Item 5's premise was partly false.** The carried sphere at base radius
  **already cost exactly zero** — `carried_cost` prices area *added* — so
  there was no power to save, and what was actually missing was an **off**
  state. Reading the words rather than the code would have built the wrong
  thing.

The general one: **before sizing an item from a doc comment, check the code
does what the comment says.** Four of the seven were sized off docs and two of
those four were wrong.

## Why three lanes and not seven

**Six of the seven items live in three files totalling ~3,800 lines**
(`src/druid/hud.rs` 1,249, `mod.rs` 1,730, `bin/druid.rs` 831).
`session-programs.md`'s finding that a same-file gate is *"nearly always
wrong"* was measured on `src/lab/ui.rs` (11,412 lines) and
`src/sim/creature.rs` (20,000+) — **it does not transfer to a file this
small**, where four items all want to add panels to one `Interface::draw`.
So the split is by file, and the parallelism available is genuinely three.

Checked before dispatch, not assumed: **no other open PR touches
`src/druid/` or the druid's render paths.** #409 and #402 are both docs-only
and say so in their own bodies.

## Ownership — the whole point of this note

| lane | session | model | owns | items |
|---|---|---|---|---|
| A — the screen | `session_013etYZf9Wg3HZMgiz9aj62J` | Sonnet | `druid/hud.rs`, `druid/menu.rs`, `bin/druid.rs`, one line of `lab/stats.rs` | 3, 2, 1 |
| B — world and sphere | `session_01CbyvwaFsrCF6jnA87mYnky` | Opus | `druid/mod.rs`, `assets/worldgen.ron`, `worldgen/`, carried parts of `sim/world.rs`+`frame.rs` | 4, 5 |
| C — thicket | `session_01KMtNERinzqM9NjyMdwZwqM` | Opus | `sim/creature.rs`, `examples/` | 7 |
| coordinator | this session | Opus | item 6 (landed), merges, review cards | 6 |

**Two reciprocal narrow licences, authored deliberately rather than stumbled
into** — both told to both sides, each to be kept in its own commit:

- Lane A may delete `Druid::charged_animals` (`mod.rs:710-720`) and nothing
  else in Lane B's file; it goes dead when the arrows go and clippy would fail.
- Lane B may add one field to `Readout` (`hud.rs:185-210`) and one line to
  `Readout::lines()` (`:218-258`), for the seed count and the sphere's OFF
  state. That region is ~140 lines clear of Lane A's `Interface::draw`
  (`:395-571`), so the hunks are disjoint — testable with
  `git diff origin/main...<branch> -- src/druid/hud.rs | grep '^@@'`.

Model choice follows the round-30 invoice in `session-programs.md`: Sonnet for
a bounded UI build with clear acceptance tests (the master-menu lane is the
precedent, and it caught two runtime defects no gate would have), Opus where
being wrong is silent and reaches another game — Lane B changes a worldgen
preset and an engine contract, Lane C changes machinery the evolution lab
founds colonies through.

## What a lane overturned, 03:47 — read this before trusting the table above

**Lane C corrected the brief I gave it, and the correction is the better
finding.** Both halves are measured; both are real.

- **The traced line was right and is fixed.** `colony_ant_site`'s
  `is_empty(cx, sy - 1)` was refusing a forest floor. Lane C's own probe
  bucketed all 135 refusals by the material standing there — **53 wood, 51
  leaf, 12 grassblade, 10 rootwood, 9 grassroot, and zero** spoil, litter,
  powder or water — so the scene did contain the situation, which is the
  check `CLAUDE.md` demands and which I had not run.
- **But the headline symptom I handed it is a different bug, and no ground
  rule can move it.** `Druid::new` on a *grown* start grows **4,093
  organisms against a ceiling of 4,095** — `Cell::organism_id` spends 12
  bits on the slot index. Over nine separated stands: **4,095 of 4,095
  live, 26 births refused by `push_organism`, 2 animals placed, 8 of 9
  stands placing nobody.** `Start::Dead` is identical, because a senescent
  plant still holds its slot and in a held world nothing rots. Filed as
  **§Z21**, not fixed, because it is not that lane's to fix.
- **Two errors of mine, both worth recording.** I wrote *"places nobody"*;
  it places **two**, and that gap was the whole thread. And I attributed the
  symptom to a line that explains only the other half.
- **The tidiness tell fired, on the lane's own work.** Its paired sweep read
  **stations 31 → 63, animals placed 2 → 2** — a lever demonstrably
  connected, doubling the ground offered, moving nothing a player sees.
  `CLAUDE.md` says a clean first result is evidence of an artifact rather
  than of a strong effect; here it was evidence of a ceiling downstream.

**The cross-lane consequence, which neither lane could see and is the whole
reason a coordinator exists.** Lane B is zeroing the same preset's three
life densities for item 4 (`life_scatter` **993 cells → 0**, paired, every
other pass byte-identical). On `Start::Bare` — the default, and what the
owner plays — that takes the starting organism count toward nothing, so it
**relieves** §Z21 where the owner meets it while leaving grown and dead
untouched. Both lanes were told; §Z21 is being tightened to name the starts
rather than "the held world", because an entry that overstates its scope
gets discounted later.

## Where the round closed, 13:0x — five lanes, five PRs

| PR | lane | what landed |
|---|---|---|
| #411 | A — the screen | items 3, 2, 1: a two-row button bar over thirteen verbs with the hotkey drawn dimmer beneath, the biosphere page on `TAB` reusing `lab::stats` almost unchanged, the energy arrows gone and the mote count finally a function of the amount drawn |
| #413 | B — world and sphere | items 4, 5: the world generates with nothing growing in it and she carries a pouch per seed kind; the sphere switches fully off and the world holds still where she stands |
| #414 | C — thicket | item 7: a floor of plants is a floor — the station search steps up through plant tissue |
| #415 | E — absorb | the owner's absorb bug, diagnosed rather than patched: absorb never touches the world, and the plants are eaten by grazing that the speed dial multiplies. Filed §Z22 |
| #418 | D — bubble aura | the hazy shimmering aura, and speed carried in the haze rather than in a number |

Item 6 (one hue) landed direct from the coordinator, `91567399`.

**Three review verdicts from the 2026-09-13 cards were answered by this round
without anyone routing them there**, which is worth noticing: the charge-flow
asks (*"goes to the top of the gnome instead of the middle"*, *"make it
slower"*, *"pixels + a diffuse aura around each"*) are all three on `main` —
the bow now decays before it arrives, `DRAW_FRAMES` went 42 → 90, and each
mote gets an aura pass. The scent verdict (*"more diffuse, not a single sharp
line"*) is `SCENT_HALO = 4` cells in eight bands. **A card's verdict and a
playtest item can be the same defect wearing two descriptions**, so read the
inbox before dispatching a round, not after.

**A claim in `session-programs.md` did not hold here, and the correction is
now in that report rather than this note** (search *"A measured
counter-example, 2026-09-14"*): a poked lane is **not** stripped of
`mcp__github__*` — Lane A was poked twice and opened its own PR #411 — so the
coordinator-opens-every-PR rule drops to a belt-and-braces default. The half
that did hold is the half the protocol rests on: no lane reached back, and the
return path is still files.

**What the round cost the coordinator, recorded because it is the recurring
one.** Three CI restarts on #408 were my own successive pushes cancelling
in-progress runs — **a push while CI is running is a decision to start CI
again**, and on this repo that is a twenty-minute round trip. And my brief to
Lane C named the wrong scene (`Start::Grown` grows to saturation; it does not
make a thicket), which cost Lane E a threefold-overstated headline and a
reposted card.

**Two lanes caught their own guards being blind, which is the part worth
keeping.** Lane B put the faults back and found its first pouch guard passed
with the decrement deleted, passed with the crediting rule wrong, passed with
the carried-circle gate dropped and passed with the maturity bar dropped —
because the guard's world had no ground, so `plant_seed` returned on its first
line and every assertion was about the refusal path. A false *pass*, from a
scene that did not contain the situation. Lane C shipped one of four new guards
asserting against a material named `rock`, which does not exist in this engine
(`stone` does) — caught not by review and not by the assertion but by the scene
builder's own `panic!` naming the material it could not resolve. Its remedy
generalises and is cheaper than the discipline: **make a scene builder panic on
a name it cannot resolve**, so a test that has lost its subject fails as a
missing material rather than as a confusing assertion.

## Round 37 — the twelve playtest items (2026-09-14 to 2026-09-15)

Four lanes, twelve items, all landed. PRs #430 (the vanishing circle, the
simplified readout), #435 (the stress setting, restart, one size dial), #437
(bubbles merge, speed as colour, creature colours, the lab's overlays made
readable), #438 (the scent trail), #449 (founding: the patch, the placement,
the menu), plus #439 (the shrunk gnome's speed) and #402 (a scope report that
had been stuck un-mergeable for two days).

**What it overturned, which is the part a later session cannot reconstruct:**

- **A body scaled against a world that did not scale must not scale its
  speeds by the same factor.** The first shrunk gnome covered **0.043 of her
  own body-lengths a tick — the identical figure as the 14-tall one**, which
  is what a uniformly scaled picture does and not what a small animal does.
  In plain cells per tick the same arm read 0.129 against 0.600 and looked
  like an ordinary consequence of being small, which is why nobody caught it
  from the numbers. With gravity fixed, a velocity goes as `sqrt(k)` and an
  acceleration goes as nothing at all.
- **A nest patch that is regular has no middle**, and that is the ethos's
  first law failing on a patch of ground: 36 columns in eighteen runs of
  exactly two, equally dense at the centre and the rim, then a hard edge.
  The repair is a solid core thinning to scattered cells — and then the
  owner rated the *shape* fix 1 of 5 anyway, because what he objected to was
  that it could be seen at all.
- **The evolution lab had already solved the visible-nest complaint** in
  August, from the same owner's words, and the third game simply never
  inherited it. A palette swap on the game's own `Materials` costs nothing at
  draw time; the three tempting alternatives (edit `creature.rs`, edit
  `nest.ron`, test per pixel in `render.rs`) are each ruled out in
  `lab::earth_toned_nest`'s doc. **Check the sibling game before designing.**
- **A debug overlay can be routed, drawn, and still invisible.**
  `apply_held_look` is a full replace that ran *after* the overlays, so on a
  held world every ramp arrived with only its luminance left — heat wrong on
  **43,456 cells**. A guard asking "is the overlay on the path" was green
  throughout.
- **Committing a founding had never once cost the other two offers.**
  `commit_founding` called `reroll()` and *then* dropped the offer, so the
  fresh draw went out with it. The design was correct from day one and the
  code had never executed it.
- **Diffusion, not decay, is what removes a pheromone trail here**, and
  nothing in the codebase said so: `DIFFUSE` sheds ~17% a pass off a
  one-cell line against `DECAY_RHO`'s 3%. So a decay sweep measures a small
  term, and width is the lever.
- **`Player::feet()` is not ground level on soil.** `wade_rows` sinks a
  standing gnome knee-deep by design, so "at her feet" is about four cells
  underground on any powder surface. It cost the trail lane a render and the
  shrink verb a refusal on every soil surface in the world.

**On running the round itself:** a lane that finishes writes its PR body to a
file on its own branch and has no way to tell you — the tell is a last commit
reading *"lane note, PR body"*, and a finished branch with no PR is invisible
to everyone. A merged lane's branch then disappears from the remote, which
looks identical to work deleted out from under you; pin the sha before
`--prune`. And a finished lane that keeps polishing holds its own PR open:
five CI restarts on one branch, the last two for a PR body and a twelve-line
note edit, each costing a full thirty-minute cycle.
