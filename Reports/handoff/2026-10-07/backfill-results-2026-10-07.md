# Backfill that holds: results (Nest building, 2026-10-07 07:4x; seed 7 split added 18:2x)

**Verdict (18:2x).** The part does what it was built for, and costs nothing that can be pinned on it.
- On the three seeds where the east opening came, the drained column now stands. The east brood pile is gone, and
  starvation falls 3-10 fold (die-offs 20 -> 3).
- Seed 7's smaller colony is the walk home's, not the backfill's (§3, the split). With the walk home off, seed 7
  under the backfill is identical to Way home's off run, every stats row through 300k.
- It stays off by default. Nest race takes it into the WAY_FOOT rerun.

**The code** is on GitHub as branch `claude/needs-first-backfill` (`d1c4fa08`, pushed 19:3x). It is also in two patches:
- [`backfill-on-wayfoot.patch`](backfill-on-wayfoot.patch): commit `d1c4fa08` on Nest race's WAY_FOOT head
  `d5a2350d`. `git am` applies it cleanly; it gives the same tree. Clippy as CI runs it is clean, and the 6
  `needs_first` tests pass. The code merged without conflict; I hand-merged one line of `how-the-ant-works.md`.
- [`backfill.patch`](backfill.patch): the original commit `b9a54525` on Way home's build 6 (`304d2e80`). Every run
  below used it.

**What it is.** `NEEDS_FIRST`'s new part `backfill`, built off and not in `on` (name it: `NEEDS_FIRST=on,backfill`).
- A buried hungry carrier cutting its way upward (`pack`) sets its pellet behind it as packed wall instead of loose
  spoil.
- This applies only below the founding ground, the gate Way home's review asked for.

**Where it goes.** It closes the cause of the east opening:
- A carrier once left a column of loose spoil from a room up to the mound.
- A later cut under the column's foot drained it, and the room opened into the mound's tunnels.
- Then came the second store, the brood pile beside it, the boom and the famine.

**Where everything is:**
- Proposal: [`way-home/backfill-proposal-2026-10-07.md`](../../way-home/backfill-proposal-2026-10-07.md).
- Review: [`way-home/backfill-proposal-review-2026-10-07.md`](../../way-home/backfill-proposal-review-2026-10-07.md).
- Code: [`backfill.patch`](backfill.patch), one commit on Way home's build 6 (`304d2e80`).
- Tables: `reads/`.

## How it was run
- **Build:** Way home's build 6 plus the part (binary sha256 `e8bd9541…`).
- **Runs:** heap 90, seeds 3 and 7 with `WAY_HOME=on`, seed 6 without it, to 300k. Way home's exact env, except
  `hungry=0` and `digfrom` past the end.
- **Off arm:** Way home's own runs.
  - **Positive control that nothing else moved:** every stats row is identical to Way home's up to the first pack
    below ground, then the arms part.
    - Seed 3: identical through 48k; its first pack below ground is 47,930.
    - Seed 7: identical through 83k; its first is 82,854.
    - Seed 6: identical through 145k; its first is 126,500, which changed nothing visible until later.
  - This also shows Way home's `hungry=1`/`digfrom=0` are logging only.
- **Guard test:** `under_needs_first_backfill_a_packed_column_stands_over_a_cut_at_its_foot`.
  - Green.
  - **Watched red twice:**
    - with the backfill written as the pellet: the material half fails, and the spoil column drains;
    - with the gate dropped: the mound arm packs.
- **Checks:** clippy as CI runs it is clean. rustfmt hunks in `creature.rs` are 1,416 before and after.

## 1. The column holds (traced on the maps and the cut record)
- **Seed 3.** The same carrier (1048804) packed the same column at 47,920, now as wall at x 268, rows 161-168.
  - A cut under its foot came at 49,989: (268,169), ant 1048637.
  - The column still stands on the nest maps at 50k, 55k and 60k.
  - In the as-built run, the equivalent cut at 53,110 drained it (587 falls).
- **Seed 7.** The column at x 267, rows 162-165, is packed at 85k.
  - Cuts beside and under its foot came at 99,584 and 100,395.
  - It stands at 100k. As built, it drained at 98,873 (671 falls).

## 2. The east opening and its brood pile are gone on all three seeds
**Drain bursts east of the door** (20+ uncaused falls below ground in 250 frames, starting at x+8 to x+24):

| | seed 3 (to 300k) | seed 7 (to 101k) | seed 6 (to 212k) |
|---|---|---|---|
| as built: bursts / falls | 99 / 6,003 | 2 / 734 | 7 / 1,273 |
| as built: largest | 587 at 53,113 | 671 at 98,873 | 1,025 at 187,552 |
| backfill: bursts / falls | 2 / 47 | 0 / 0 | 2 / 63 |
| backfill: ground cells those bursts opened | 6 + 1 | - | 1 + 18 |

- **The four backfill bursts are not drains of a column.**
  - Two are a single loose grain falling through the open room: 20 falls, 1 cell opened.
  - Seed 3's 89k burst opened 6 cells.
  - Seed 6's 200k burst opened 18 cells 30 rows down, starting at a packed cell.
  - None is at a backfill column, and none opened the nest to the mound.
- **"Falls" are fall steps, so a lone grain falling 20 rows reads as a burst.** The reader now also prints
  `cells_opened`: ground that was there before the burst and is gone after it.
  - For the as-built drains, Way home's trace gives +47 open cells on seed 3.
  - Way home's off-side reads predate this column.

**Eggs laid at x+8 to x+24, 20-300k, counted as unique egg ids:**
- As built: 1,567 / 2,297 / 732 (Way home's recount). The trace's 5,191 / 7,301 / 2,481 were broodlog `laid` rows,
  which count re-appearances.
- Backfill: 0 / 3 / 0.

**The east band still fills with room as the nest grows,** but slowly. Its largest rise in any 4k is 9-12 cells,
against Way home's traced jumps of +25 to +36 at the drains.

## 3. The colony: the three die-off seeds become ordinary seeds
Way home's build-6 measures, backfill / as built:

| | seed 3 | seed 7 | seed 6 |
|---|---|---|---|
| starved 20-300k | **112 / 512** | **102 / 1,048** | **90 / 309** |
| die-offs (30+ starved in 4k) | 2 / 5 | 1 / 11 | 0 / 4 |
| mean ants 100-300k | 333 / 389 | 334 / 448 | 359 / 373 |
| trip food 100-300k | 5,889 / 6,234 | 4,974 / 10,845 | 6,235 / 6,698 |
| births 200-300k | 762 / 1,030 | 588 / 913 | 720 / 1,214 |
| deaths other than starved and old age | none / none | none / none | none / none |

- **Starvation falls 3-10 fold, and die-offs go 20 -> 3.**
- **Colonies are smaller on average,** by 14% / 25% / 4%.
  - The as-built size includes the boom: Way home traced seed 7's +71% trip food and seed 3's 353 -> 602 to the east
    store.
- **Against build 6 with the walk home off** (Way home's off arm; it never grew the pile on seeds 3 and 7):
  - **Seed 3 is level:** 333 against 344 ants, trip food 5,889 against 5,699, starved 112 against 199.
  - **Seed 7 falls short:** 334 against 400 ants (-17%), trip food 4,974 against 6,347 (-22%), births 200-300k 588
    against 960 (-39%), starved 102 against 73.
  - That pair differs in two things, the backfill and the walk home, so it cannot say which.
  - **The split (18:2x): it is the walk home.**
    - Seed 7 with the backfill and the walk home off matches Way home's off run on every stats row through 300k:
      400 ants, starved 73, trip food 6,347, births 960.
    - Its pack events ran as usual (226), and it shows 4 packed fills below ground. That is the as-built background
      (1 and 5 on Way home's as-built seed 7 and 6 reads). So, inferred: with the walk home off, no carrier packed
      its way up below ground on this seed, and the part had nothing to do.
    - So 334 against 400 is the walk home on (with the opening gone) against the walk home off. It is a Way home
      question, not a backfill cost. Tables: `reads/split-s7.txt`, `reads/backfill_read-bfoff-s7.txt`.
- **No new kind of death appears.** Only `starved` and `old_age` move, in both arms.

## 4. The gate's reason was not seen on one seed (positive control, seed 3, no gate)
- Without the gate, backfill packs in the mound too: 289 packed fills above ground against 19 gated. That run parts
  from Way home's at 21k.
- **No more pillars or floating ground by this census:**
  - pillar cells above ground: mean 42.4 against 43.0 gated, and 48.3 as built (max 99 / 84 / 87);
  - floating ground above ground: mean 64 against 60 and 115.
- It also stopped the drain: 1 east burst, 40 falls; starved 44.
- So Way home's inferred risk (packed stacks standing in the air once the spoil round them slumps) was not seen on one
  seed to 300k.
  - The census counts any one-cell-wide wall, so it is coarse.
  - **The gate stays**, as agreed. Re-check it by running the ungated arm on more seeds with a census of packed cells
    with nothing under them, if anyone wants the mound packed too.

## Not traced
- **Why the walk home's seed 7 is smaller than the walk-home-off seed 7 once the opening is gone** (334 against 400
  ants). The split puts it on the walk home, not the backfill. Not traced further; it is Way home's to take up if
  the walk home comes back.
- **Why the backfill colonies' births 200-300k are lower than as built.** Plausibly the missing east store. Not
  traced.
- **Seeds 3, 7 and 6 only.** It stays off by default. Default-on needs 12 seeds and a cross-lane check.

## Reproduce
- Reader and pairing: [`backfill_read.py`](backfill_read.py) (east signature fixed 07:3x to sample every 2k),
  [`backfill_pair.py`](backfill_pair.py) and [`colony_cmp.py`](colony_cmp.py).
- Way home's off-side reads are in `way-home/for-nestbuilding/`. Those predate the east fix, so their east column is
  not comparable. The drains, hanging census and stats are.
