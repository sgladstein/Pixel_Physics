# Coordinator — the druid program (owner playtest, 2026-09-14)

**Status: live. Coordinator is session `session_01TngZpRY8LoqpFWHuXjUTTD`.**
Protocol is [`Reports/session-programs.md`](../session-programs.md); this note
carries only what is specific to this round. Kept small on purpose.

## The seven items, in the owner's words

1. bring in the lab's main stat menu (plants, creatures, births)
2. the big arrows over the ants go; no energy indication; read the yield from
   how many particles come
3. an actual GUI — minimal, buttons for the main actions, subtle hotkey always
   visible. *"the menu isn't even a menu, it is a shortcut list"*
4. the world starts with no seeds; the druid carries her own and planting
   populates it
5. an easy way to fully turn the sphere off so no power is used
6. one hue becomes default
7. easier to found a colony while standing in a thicket

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

## Standing facts for this round

- **Run `git ls-remote origin 'refs/heads/claude/*'` at every check-in, for
  every live lane.** A lane three hours and $7.85 into a working build once
  had never pushed a branch; every other signal comes from inside the
  container and none of them crosses it.
- Lanes cannot reach back. The return path is files: each writes
  `PR_BODY_LANE_*.md` and a lane note, ending with its head SHA. To reach a
  lane: `create_trigger(persistent_session_id=…)` poke-only — omit both
  `cron_expression` and `run_once_at` — with the **whole message in the
  trigger's `prompt`**, then `fire_trigger` bare. A preamble in the prompt and
  the message in `fire_trigger`'s `text` spawns a fresh sessionless session
  instead, twice recorded.
- **A CI verdict quoted to a lane has a shelf life of one push.** Date it and
  name the head SHA, or do not send it.

## Item 6, landed by the coordinator

`91567399` — one hue set on the druid's own renderer (`mod.rs:659`) rather than
by moving `HeldLook`'s `#[default]`, which is shared with the other two games.
The env hook had to become two-way in the same change: `PIXEL_PHYSICS_DRUID_LOOK`
could only ever *select* `onehue`, so the moment that became the default the
selector had no way to render its own baseline.
