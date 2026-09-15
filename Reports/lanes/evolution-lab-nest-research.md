# Nest research lane — what should a nest *be*

*Round 36 research lane, 2026-09-14. Branch
`claude/evolution-lab-nest-research`. Deliverable:
[`../nest-design-2026-09-14.md`](../nest-design-2026-09-14.md). No `src/`
change; one harness, `examples/nesthome.rs`.*

## 2026-09-14 → coordinator

**Verified against the brief before resting anything on it** (every claim
below re-read in the tree, not taken from
`evolution-lab-nest-question-2026-09-14.md`):

- `AtNest` **is** `adjacent_nest` — `src/sim/creature.rs:6629`, an 8-neighbour
  material-id test. Confirmed. Its own doc says *"since 2026-09-02 this
  decides nothing"*, which is true of the *verbs* and false of the circuit:
  it is the charge wire of hidden unit 4 (`ant.ron` `(AtNest, 4, 0.05)`,
  recurrence `0.99995`, output `(4, EmitA, 32.0)`), and the gate on units
  5/6 (chamber digging), the cohesion blend, the room census and the
  delivery counter all hang off it.
- **`NestSite` IS read for location** — `World::nearest_nest_site`
  (`world.rs:6873`) is called by the room census, the crowding input, the
  odour blend and the drift walk. The brief's *"nothing reads it for
  location"* is wrong as written; the true statement is **nothing
  *navigates* by it**. That is a smaller gap than the brief implies.
- **Homing is not a nest-position mechanism at all.** The laden ant walks
  up channel A through units 0/1 (`(Carrying, 0, 45.5)`, `(PheroAAlong, 0,
  6.0)` → `(0, Move, 2.5)`); channel A is laid by every ant, scaled by its
  own odometer. So "where is home" is answered by the *field*, and the
  material's only job is to recharge the odometer on contact.
- **Path integration is already half in the tree, as a measurement.**
  `OrganismState::forage_anchor` (`organism.rs:5862`) is the world position
  of the last nest contact, re-anchored on every touch, and its doc says
  *"the moment one [creature] reads this, the homing model has changed"*.
  An ant here knows its absolute position, so a home vector is
  `forage_anchor - head`, exact, with no integration and no error.
- **Crust 6.0 against jaw 1.0, and the ceiling is 1.0 + `trait_reach`
  (default 1.0) × `DIG_FORCE_SPAN` (1.0) = 2.0** — `creature.rs:2569`,
  `allele_bound` returns `reach` for arms-race slots. So no shipped lineage
  can ever reach 6.0 at the default reach; `TRAIT_REACH_MAX` is 8.0, so a
  box at maximum reach could. Confirmed.
- **The 414** is `ascii`'s foraging scene, 12,000 frames, worldgen seed 1,
  a **74-column** hand-painted patch (`for x in 16..90`) under a 55-ant band
  spanning `24..240` (`Reports/creature-direction.md` §13g); it read 348
  after the tree fix and `instruments.md` prices the column at 154–980
  across six seeds of an unchanged ant.
- `dead-ends.md` has **no entry** for a site-based `AtNest`, a home vector,
  or a blob; the two nest entries are both about `nest.ron`'s water.

Measurements follow in the report.

## 2026-09-14, later → coordinator, druid coordinator, Lane C

**Report is up: [`../nest-design-2026-09-14.md`](../nest-design-2026-09-14.md).
Harness `examples/nesthome.rs`; nothing in `src/`.** The findings a later
session would otherwise pay for again:

- **The 414-delivery scene is not a homing scene.** It places **15 of 55**
  ants; channel A is 0 by frame 6,000 while deliveries climb to 670. A
  delivery there is an ant eating beside its door. Do not quote it against
  a footprint change.
- **On the played bed, cutting the homing circuit out of the genome
  changes nothing detectable** — six seeds, `noemit` / `nosteer` / shipped
  medians 2,207 / 1,524 / 1,761 deliveries, and the two mechanically
  identical no-homing arms differ by 3.6x on one seed. Nothing steers a
  laden ant home today. Owner's reading 1 ("they go where food is") is
  not what the census shows (laden ants 0–10% at the food); reading 2
  (the plane does not last) is what it shows: A stands at 3,421 at frame
  10,000 and **0–9** from 20,000 on.
- **→ Lane C:** `DIFFUSE` on channel A alone at **0.02** lifts laden-at-door
  on 3 of 3 seeds (1.6% → 14.5% on seed 1); 0.08 lifts it 3 of 3 too. The
  pheromone work is the homing fix. The cost to the B trail is yours.
- **A footprint wider than the band kills the colony, monotonically, 3 of
  3 seeds** (alive at 40k: 59 → 38 → 19 → 2 across widths 36/72/144/288).
  Deliveries cannot rank footprints (every drop at 288 is one).
- **The patch is never dug** (`lost` 0/0/4 over 120k frames, `nestdoor`)
  because the crust is 6.0 against a jaw ceiling of **2.0** (`1.0 +
  trait_reach × DIG_FORCE_SPAN`). The blob defends a case the crust
  suppresses; the site is what makes the crust safe to drop.
- **→ druid coordinator:** the recommendation keeps `nest` `kind: Solid` as
  paint, so `a_nest_still_stops_him` and `is_tool_target` are untouched;
  `Druid::found_colony` is unchanged and gains a home on unpaintable
  ground. Nothing lands from this lane; a druid lane follows whatever does.
- **Recommendation**: site-based `AtNest` at `COLONY_HALF_WIDTH` × ±2 rows
  (a value, not a dial — it reproduces the one footprint whose numbers
  exist), crust to soil's 0.8, then a `HomeBearing` brain input on
  `forage_anchor` as its own seed-swept change. Not a blob. The trail
  ordering in the brief is wrong for the nest's identity; both can start.
- **Two harness lessons** (in `instruments.md`): a genome patched into
  `Lab`'s world before `load_scenario` is thrown away by `reset()`; a
  surface scan from row 0 on the lidded bed paints on the ceiling. Both
  produced byte-identical arms that read as findings.

## 2026-09-15 → coordinator, Lane C

**Re-measured on PR #450 (`u16` planes), report §12.** The plane now
stands all run (near band at 30k: 42–313 old units vs 0–9), and the homing
circuit is detectable on 2 of 3 seeds (laden-at-door 8.7% vs 1.9/1.2 cut;
30.7 vs 13.7/16.8). Owner's reading 2 confirmed by the fix itself.
**Withdrawn**: the `DIFFUSE`-on-A suggestion. **Now conditional**: the
home bearing — run the §5.1 sweep on `main` at six seeds after #450 lands
before building it. Site, blob, crust, footprint findings unchanged.
