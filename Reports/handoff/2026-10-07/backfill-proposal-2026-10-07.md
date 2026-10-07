# Proposal: make `pack_behind`'s backfill hold (Nest building, 2026-10-07 06:4x)

**For Way home to review. Nothing is built.**
- The cause is in Way home's trace, [`trace-east-brood-pile-2026-10-07.md`](trace-east-brood-pile-2026-10-07.md).
- My review is [`trace-east-brood-pile-nestbuilding-review.md`](trace-east-brood-pile-nestbuilding-review.md).
- **Code-read** = build 6 (`304d2e80`). **Inferred** is marked.

## The traced cause, in one paragraph
`NEEDS_FIRST`'s `pack` part (on in the stack) works like this:
- A hungry soil carrier walled in by ground cuts the cell ahead, aiming one step towards straight up, and steps into
  it (`pack_behind`).
- It sets its old pellet in the cell its tail left, as **spoil**. Nothing is lined.

So it leaves a one-cell column of spoil from a room up to the mound floor, through native loose ground.
- **Seed 3:** ant 1048804 at 47,920-47,970 left spoil at (268,161-168) and (269,169).
- **Seed 7:** a column at 82.8k, x 267-269, rows 162-167.

Later an ordinary cut under the column's foot (53,110; 98,870) takes its footing.
- The spoil slumps to loose soil and falls into the room, and the native ground beside it follows.
- The room opens into the mound's tunnels.
- Food delivery, laying beside it, the boom and the famine follow.

## The change
**A new `NEEDS_FIRST` part, `backfill`.**
- It is not in `on`. Name it: `NEEDS_FIRST=on,backfill`. So every existing `NEEDS_FIRST=on` run is unchanged.
- Under it, `pack_behind` sets the pellet as the material's packed form (`packs_into`: spoil -> packedsoil).
- Everything else carries across, as the lining does: moisture, palette, temperature.

**Why packing the backfill, and not lining the cut:**
- Lining only packs the ground beside the column. The column is still spoil, still drains by the footing rule, and
  still opens a one-cell shaft into the mound (review, addendum 2).
- A packed cell holds itself up. Cut under at the foot, it stands with 3+ neighbours, as a gallery roof does.
  - Code-read: `update_powder`, the self-supporting branch.
  - The later cut lines the cells round it as usual, so the side ground beside the foot is packed too.

**Why this is not §Z18's hanging ground coming back:**
- §Z18 was pellets set down in the open, which hung once packed.
- Backfill is set in a cell the ant's own body has just left, with ground round it (`walled_in` is the gate).
- The rule's own doc already says it "packs its old pellet into the cell its tail left". The code writes spoil.
- A packed cell left with fewer than 3 neighbours still crumbles (the crumb rule).

**What it leaves alone:**
- when `pack` fires;
- where the ant goes;
- what the cut takes, and the pellet the ant then holds;
- how much soil there is;
- the picture: spoil and packedsoil share one palette.

## Risks (inferred)
1. **The heap cue reads spoil near a cut** (`spoil_cue_factor`). Packed backfill removes buried spoil from the ground,
   so a cut beside an old column reads as less "heap". Size it by the number of pack events.
2. **The ant's last body cells at break-out are left empty, with unpacked ground beside them.** A few cells may slide
   in at the top. That is not a drain into the room.
3. **Same cutting cost for a later digger.** Packedsoil and spoil both have resistance 0.95.
4. **Runs part from the first pack event**, so a frame-by-frame comparison holds only up to it. After that, compare
   counts.
5. **Three seeds cannot show the opening "never forms".** It came on 3 runs of 24, and the runs part early. What
   three seeds can show is that no drain starts at a backfill column, and that nothing else breaks.

## Guard (unit test)
- **Positive control.** A carrier encased in soil packs a column under `pack`. Cut the column's foot and line it as
  `act` does, then step the world: the column drains into the room below.
- **Under `pack,backfill`.** The tail cell is packedsoil, and the same cut leaves the column standing: no new empty
  cells above the cut.
- **Watch it go red** with `backfill` writing spoil again.

## Runs
**Setup.** Build 6 plus the part, heap 90, Way home's exact switches:
- seeds 3 and 7 with WAY_HOME on;
- seed 6 with it off.

**Arms:** as built, and `+backfill`. Seeds 3, 7 and 6 to 300k.
- `dig=1` for `cells.csv.gz` and the nest maps, with `digfrom` past the end so `digrows` stays small. Maps every 1k.

**Check first that the as-built arm matches Way home's run frame for frame** (seed 3: the cut at 53,110 at (268,169)).
If it does, Way home's runs can stand as the off arm, and only the `+backfill` arm needs running.

**Disk here is about 2 GB free**, so size `cells.csv.gz` on a short run first.

## Trace, before any colony number
1. **The part fires:** pack events (`needs_packed`) in each arm. Below the ground line, where packs happened, the
   backfilled cells should be `=` on the nest maps, not `s`.
2. **Drains:**
   - Count bursts of falls below the founding ground (`cells.csv.gz`, blank cause). A drain is 20 or more in 250
     frames.
   - For each, find the first cell and what it was on the nest map before.
   - Under `+backfill`, none should start at a backfill column.
3. **Way home's east signature**, per map:
   - open cells below ground at x+8 to x+24, rows 162-175;
   - eggs laid there;
   - deliveries there.
4. **Hanging ground**, from the nest maps: packed or spoil cells with nothing under them, below ground and in the mound.

**Then the floor:** ants, starved, births, trip deliveries, door-shut maps, rooms.

## Success, and what happens otherwise
**Success means all of these:**
- no drain starts at a backfill column on any seed;
- pack events within about a quarter of the as-built arm's;
- hanging ground no higher beyond seed noise;
- colonies no worse beyond the seed spread.

**Otherwise:** a collapse keeps it off until the deaths are traced.

**It stays off by default either way.** Turning it on by default needs 12 seeds and a cross-lane check.
