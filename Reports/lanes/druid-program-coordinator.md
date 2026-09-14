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

Four parallel reads, and **three of the seven items are not the size they
look**:

- **Item 1 is wiring, not porting.** Two different things here are called
  "census". `lab::census` (`src/lab/census.rs:209`) takes `&LabBox` and cannot
  serve the druid. The panel the owner means is `lab::stats::Stats`, which is
  **already `&World`-only** (`observe` `stats.rs:565`, `draw_at` `:624`) and
  already proven to run outside a `Lab` (`examples/labstats.rs:1291`). **One
  lab-bound line in the whole panel**: `stats.rs:710` clamps its bottom to
  `super::ui::bar_top()`. Births/deaths need nothing new — cumulative on
  `World` at `world.rs:1740` / `:1684`.
- **Item 2 is not an arrow deletion.** The particle count is a hard constant,
  `PER_DRAW = 26` (`hud.rs:732`), and `Draw::amount` — which already carries
  the energy and whose doc at `mod.rs:368` already *claims* it "sets how heavy
  the flow looks" — **is never read by the renderer**. So the order is forced:
  make the count proportional first, then delete the chevrons, or the player
  is left with no readout at all. A false doc claim, the same shape as the
  `ant_wide` blurb PR #408 unshipped.
- **Item 5's premise is partly false, and that changes what gets built.** The
  carried sphere at base radius **already costs exactly zero** —
  `carried_cost` is `(r/28)^2 - 1` floored at 0 (`mod.rs:1488`), area *added*
  rather than area held, with a guard whose message is *"the circle you already
  are must stay free"* (`mod.rs:1603`). What is actually missing is an **off**
  state: `carried_radius = 0` is silently read as the default 28
  (`frame.rs:101`), which `world.rs:3951-3954` documents as deliberate —
  *"a dial that can be turned to off by accident is a different mechanic."* So
  off means `world.carried = None`, and the engine's own comment says that is
  authoring rather than tuning.
- **Item 3 has a complete precedent** in `src/lab/ui.rs` and no lane should
  invent one: `Rect`+`contains` (`:267`), `Widget` whose `line2` **is** the
  subtle hotkey (`:1042`), `Bar::hit` (`:1074`), a press/release protocol where
  sliding off takes the press back (`:2982`), widths **measured not written
  down** (`:1164`, `:1183`), and one dispatch point — `Lab::act`
  (`lab/mod.rs:2680`) — documented as "no second copy of what SPACE does".
  The druid reads **no mouse at all** today, and its mapping is 1:1 (`zoom`
  untouched, `Hud::new(w,h,1)` at `hud.rs:376`), so `window_pos_to_pixel` is
  the whole conversion.

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

## Standing facts for this round

- **Lanes A and B are based on `claude/determined-ramanujan-c9szc5`, not
  `main`**, because PR #408 carries druid changes in their files. Lane C is on
  `main` because its change is shared engine code and wants the trunk.
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
