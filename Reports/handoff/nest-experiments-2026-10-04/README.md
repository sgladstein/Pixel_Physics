# Parked nest experiments, 2026-10-03/04: kept to retest

**What this branch is.** It holds the goal-bed experiments that lanes 3 (Nest building) and 20 (Nest race) built on
2026-10-03/04, looking for separate chambers, a food store and a door that stays open. Every switch here is
**off by default**. Each was measured, and none was shipped. The owner asked on 2026-10-07 that they be kept to
retest "once we solve some of these issues", and not deleted. So this branch is their home. Each one also has an
entry in `Reports/dead-ends.md` on `main`, which points back here.

**Base.** This is `main` as of 2026-10-03 (`7adddd24` / `192b7103`, merged in). It is about 160 commits behind
`main` as of 2026-10-07, and its `assets/lab_scenarios/nest_goal.ron` differs from main's own copy. **To retest
one, cherry-pick its commit onto today's base. Do not merge this branch.** The code is in `src/sim/creature.rs`
(switch parsing near the other `PIXEL_PHYSICS_*` reads) and `examples/nestgoal.rs`.

**Where the numbers are.** The shared project folder: `nest-race/scoreboard.md` (sections are dated and named for
the lane), `nest-race/lane3/`, and the cross-sections page linked from the scoreboard. A git bundle of this exact
head is at `nest-race/lane3/nest-building-scratch-2026-10-04.bundle`.

## The experiments

**Reading the "retest when" column:** it is the condition the rejection depended on, as judged when parking. It
is not a measured threshold.

| Switch | Commit | What it does | What was found | Retest when |
|---|---|---|---|---|
| `FRESH_CUT=aim\|draw\|recruit\|recruitall\|recruitfed` (lane 20; was draft PR 599, closed) | `12a428a9`, `a5074d14` | Diggers go back to where nestmates just dug (positive feedback at the face; Buhl et al. 2004, Pielström & Roces 2013) | `recruitfed`, 12 seeds: dug home bigger on 12/12, per ant on 11/12 (3.99 vs 2.06). **The first rule that moved digging per ant.** But fewer live ants at 40k (26 vs 37). `aim`/`draw` moved nothing. | Ants live deep in the nest and the colony is stable to 300k. Build it beside a local crowding signal. |
| `FOOD_SORT=on` (lane 20) | `e1b1f898` | A forager at home doesn't put crop food down within 2 cells of brood | 2 seeds: one colony bigger, one dead. Food underground 2-14 cells, the same as control: **no surplus to sort.** | Food lies underground for long (a working storeroom). |
| `CROP_UNLOAD=<p>` (lane 20) | `6f440db1` | A fed ant at home drops crop food | **Bit-identical** to the baseline: fed ants at home already drop at that rate. | Not as is. The crop store is held by ants away from home. |
| `SATIATE=<f>` (lane 20) | `6f440db1` | The urge to feed tapers above the grant | f=1.2 slowed births (57 vs 120 by 75k). f=1.5: seed 3 died by 150k. | A storeroom exists, and births no longer depend on banking. |
| `STORE_CHAMBER=on` (lane 20) | `7521123e` | Fed ants at home eat floor food only as readily as it is piled | 4 seeds to 300k: **2 colonies lost** (0 vs 426, 0 vs 507). Most food ever underground was 17-45 cells, the same as main. No store. | Same as `FOOD_SORT`. |
| `FOOD_HANDLE=<frames>` (lane 20) | `6edeba8d` | Food put down at home can't be taken for N frames (husking) | 300k: no store. Seed 2 died at both handling times. Food cells move constantly, so a record kept by position loses them. | Built as a material ("unhusked food" that decays to provisions), not a timer. |
| Two crowds: `FLAT_ROOM=4` + `BROOD_CARRY=on` + `BROOD_DEEP=on` (lane 3) | `dcafc8cc` | Rooms capped in height; brood carried to the lowest floor in reach | Old main: **3 chambers on 4 of 12 runs** (today: 1 chamber in 11 of 12). On the resting base (`NEST_REST=on`): **colony halved on 4 of 4 seeds**, and 1 chamber. | Colonies are stable with ants living in the nest. Retest for the separate-chambers goal. |
| `BROOD_DIG=on\|<rows>` (lane 3) | `dcafc8cc` | A digger beside brood cuts the wall next to it | 1 chamber on both seeds. The spaced version fired 3-19 times a run: a non-test. | Brood lies in a layer, not a column. |
| `PILLAR=<n>` (lane 3) | `dcafc8cc` | Leave pillars in wide rooms | One domed room round the brood, as today. | With other chamber work. |
| `DIG_MODES=on` + `DIG_TIP=on` (lane 3) | `dcafc8cc` | Two digging modes: widen beside contents, else cut only a tunnel tip | A maze, not rooms. Already in dead-ends. | See its dead-ends entry. |
| `DIG_AHEAD=<r>`, `DIG_STRAIGHT=on`, `DIG_NARROW=<m>`, `DIG_STAY=on\|tip` (lane 3) | `69295d61` | Long tunnels: no cut toward open space ahead, no side branches, advance only from inside a tunnel, stay at the face | No tunnel ran 10+ cells out; still a fringe of 3-6-cell stubs. Traced cause: no ant carries on its own tunnel (92 of 432 advancing cuts within 2 cells of the same ant's last). `DIG_STAY` restores that, but the work goes into widening round the room's contents. | A digger keeps cutting one tip for many cuts (the `DIG_MODES` entry's condition). |
| `DOOR_HEAP=<rows>` (laying lane's patch) | `b8b7a5d5` | A cut in the founding shaft's columns over the mouth counts as reopening the door | Alone: 0 / 0 ants at 200k. With `DOOR_DIG`: 337 / 527 at 137.5k. | See the `DOOR_DIG` entry in dead-ends. |
| `DOOR_DIG=on\|heap` (lane 3) | `b8b7a5d5`, `35ecff7d` | An ant shut out over its door digs back down into it | Already in dead-ends. | See its entry. |
| `LEAN_DOOR=keep` (lane 3) | `35ecff7d` | A hungry ant keeps its pellet rather than drop it in the door | **Built, never tested.** | The door seals again (today `DOOR_LOOSE` and `MOUND_OUT=dig` are on). |
| `DOOR_CLEAR=drop\|tamp\|both` (lane 3) | `34bcb046` | No pellet set down over the door / the doorway not tamped | Already in dead-ends. | See its entry. |
| `JAW_FOOD=off` (lane 3) | `860f385d` | The jaw never cuts food as dirt | Already in dead-ends (it kills colonies). | See its entry. |
| `SPOIL_ON_FOOD=refuse\|swap` (lane 3; the owner's idea) | `665a5b14` | A pellet isn't left lying on food | Already in dead-ends. | See its entry. |

## Instruments on this branch (superseded, kept)
- **`examples/nestgoal.rs`, the goal-bed readout** (`4a3339ac` onward). Its lines:
  - population, chambers, food/brood separation;
  - `FLOW`, `SITES`, `ENERGY`, `GONE`, `TUNNELS`, `REFILL`, `DOOR`;
  - deaths by cause;
  - `rain=<0-3>` (`20759c93`).
- Today `examples/deeptrace.rs` (`scenario=nest_goal`) and the chronicle's nest census do this job on `main`.
- `e7f7571a` holds scratch traces for the patchwork over the nest: food beside each cut and pellet, buried crumbs,
  heap materials.
