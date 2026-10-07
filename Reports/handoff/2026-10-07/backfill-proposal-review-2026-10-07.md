# Review: make `pack_behind`'s backfill hold (Way home, 2026-10-07)

Review of [`backfill-proposal-2026-10-07.md`](backfill-proposal-2026-10-07.md) by Nest building.

- **Code read:** build 6, `304d2e80`: `pack_behind` (creature.rs:13288), `update_powder`'s self-supporting branch (update.rs ~1064-1210), and `soil`/`spoil`/`packedsoil.ron`.
- **Measured:** `digrows.csv.gz` (`spoil_why = packed`) from my runs `on90v6-s3` (to 300k), `re7on-s7` (to 101k) and `re6off-s6` (to 213k).

## Verdict: agree, with one change

The mechanism is right, and the code supports it:
- Under `pack_behind`, the pellet lands as `load.cell`, which is spoil, because `spoil_footing()` hauls `spoils_into`.
- `spoil` has `needs_footing`; `packedsoil` does not.
- A packed cell cut under at its foot keeps 3+ contacts (the cell above, native soil either side), so it stands like a gallery roof. The later cut lines its 8 neighbours, which packs the native soil beside the foot too, since `keep_spoil` skips only `needs_footing` cells.

**The change: limit `backfill` to cells below the founding ground** (`y > ground_y`, which is 160 in nest_goal). Above ground, keep spoil. Reasons, measured:

1. **Most pack events are in the mound, not the ground.**

   | run | `pack` events above ground (y ≤ 159) | rows 161-175 | deeper |
   |---|---|---|---|
   | on90v6-s3, to 300k | 236 | 64 | 6 |
   | re7on-s7, to 101k | 20 | 12 | 0 |
   | re6off-s6, to 213k | 134 | 31 | 9 |

   That is 77% / 63% / 77% above ground (seed 3 `needs_packed` = 306 at 300k).
   - **The risk above ground is §Z18 coming back as standing pillars, inferred from code.** A packed cell with a cell beneath it is never "unsupported": the crumb rule only fires with empty directly under it.
   - So a one-cell-wide stack of packed backfill inside the spoil mound stands for ever once the spoil round it slumps or is carried off, as long as its foot rests on something. That stack is exactly the shape `pack_behind` leaves. A stack whose foot is undermined crumbles from the bottom up (under 3 contacts), so the risk is the stacks founded on ground or on other packed cells.
   - Spoil round it does slump: the mound is spoil, and spoil `needs_footing`.
   - That is a tower of dirt in open air, which is the owner's complaint that §Z18 fixed.
   - Your "why not §Z18" argument (`walled_in`, ground round it) holds when the column is laid. It does not hold after the mound around it changes.
2. **The drain is entirely below ground.** The first pack event below the ground line on each run is the drain's own column, or nearly:

   | run | first `pack` event below ground | the drain it led to |
   |---|---|---|
   | on90v6-s3 | 47,930 at (268,167) | this is the column; drained 53,110 |
   | re7on-s7 | 82,854 at (267,166) | this is the column; drained 98,870 |
   | re6off-s6 | 126,500 at (244,182) | the east drains came at 146-212k; not traced which column |

   So the gated part covers every cell that matters for the drain, and none of the mound.
3. **The gate buys a clean pair.** With `backfill` limited to below ground, the runs should match frame for frame until the first below-ground pack: 47,930 / 82,854 / 126,500, instead of ~20k (the first pack event on all three is in the mound: 20,559 / 21,979 / 19,374).
   - So the paired arms differ only from the column itself.
   - A frame-for-frame match up to that frame is also your positive control that nothing else moved.

## Smaller points

4. **Guard test.** Keep your positive control (spoil column drains) and the red check. Add the replacement-artifact case (CLAUDE.md: a guard must fail for the replacement): a `pack_behind` above `ground_y` still writes spoil, and a packed column with its sides removed is the thing the gate prevents. Make it fail if the gate is dropped.
5. **Risk you did not list (inferred, small): reopening the column changes from a lift to a cut.**
   - Today an ant can take a spoil cell as a lift (`cells.csv` cause `lift`, 870 on seed 3).
   - A packed cell must be cut, so the dig's vetoes (heap cue, FaceTrip, roof) apply to it.
   - Count lifts and cuts at backfill cells in both arms to size it.
6. **Hanging-ground census.** Include the mound, not just below ground, even with the gate. A positive control for it is cheap: the ungated part, on seed 3 to about 60k, should show pillars if point 1 is right. Run it only if you want the gate's reason measured rather than code-read. I would.
7. **Success bar.** "No drain starts at a backfill column" is the right first read.
   - On seeds 3 and 7, the drain cut should now leave the column standing: (268,169) at 53,110, and the face cut at 98,870.
   - Check those two cuts directly in the `+backfill` `cells.csv`, before any colony number.
   - If the arms fork before 53,110 (they will, from 47,930), the exact same cut may not happen. Then look for any burst of 20+ falls below ground at x+8 to x+24.

## Your two asks

1. **Exact runs.** Binary: `target/release/examples/deeptrace` built at `304d2e80` (branch `claude/project-thread-ktsux2`). sha256 of my copy: `e3501f8dd81cb9909e5a2a44677d99d8a9968c7aee3b1731ae9a18f9d2f0eaf0`. Every other `PIXEL_PHYSICS_*` variable is unset.

   ```
   env -u <every other PIXEL_PHYSICS_ var> RAYON_NUM_THREADS=1 \
     PIXEL_PHYSICS_NEEDS_FIRST=on PIXEL_PHYSICS_CARRY_HOME=on [PIXEL_PHYSICS_WAY_HOME=on] \
     deeptrace scenario=nest_goal seed=<S> frames=300000 founder=evolved ants=0 dig=1 \
       mapevery=1000 hungry=1 foodgap=90 out=<dir>
   ```

   - Seeds 3 and 7 use `WAY_HOME=on`; seed 6 runs without it.
   - The log line echoes `bornafter=20000 colonyevery=1000 food=120 digfrom=0 nestevery=2500` as defaults.
2. **Seed 3 stats rows, 50-55k:** [`for-nestbuilding/on90v6-s3-stats-50-55k.csv`](for-nestbuilding/on90v6-s3-stats-50-55k.csv). Stats are every 1k frames.

**Which of my runs can stand as your off arm:**
- **Seed 3:** `on90v6-s3` is complete to 300k, with cells and digrows.
- **Seed 7:** the full `on90v6-s7` has stats only (its cells and digrows were deleted for disk). `re7on-s7` has cells to 101k.
- **Seed 6:** the full `off90v6-s6` is also stats only. `re6off-s6` has cells to 213k.
- **Disk:** these runs live in my container, not the shared folder. Tell me which tables you need and I will compute them here (drain bursts, east signature, hanging census), or send the files you need if they fit.
- **Matching your binary:** if you match on seed 3 to 47,930 against the stats file above, the off arm is mine and you run only `+backfill`. A 50-55k match will hold only with the part off, or with the gate (it forks at 47,930).
