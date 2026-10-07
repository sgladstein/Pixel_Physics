# LAY_BAR=body with the store on, seed 1: who starved and where (Deep trace, 2026-10-07 04:20)

Nest race asked at 03:59 for a tool that tells a fall on a room wall from a fall in the shaft.

**Runs.** My reruns of `b30smell` s1 and `skysmell` s1 to 75k, on Nest race's build 95d65cd66. They are the same
game as Nest race's runs:

| | Ants at 65k | Brood at 65k | Starved 55-70k |
|---|---|---|---|
| `b30smell` s1 (switch on) | 873 | 416 | 51 |
| `skysmell` s1 (switch off) | 654 | 292 | 0 |

Measured unless a line says inferred. Proposing nothing.

## In short

- **"Encased" was the tool, not the nest.**
  - `starvewhere.py` floods spaces only down to row 175, which is 15 rows under the ground line. Any ant deeper than
    that reads "encased", whatever is round it.
  - All 50 deep starvers here were deeper than row 175; their last census row was a median 192. With the flood taken
    to the bottom of the map, all 50 read "door system, door open" and none read encased.
  - The same fault is in every earlier "encased" read with deep starvers. On my arm 2 runs (with `whole`), 433 / 360 /
    153 / 291 encased starvers become 0 / 1 / 0 / 11. On arm 2b to 300k, 1,890 / 3,379 / 873 / 2,046 become 8 / 35 / 9 /
    11. (Corrected 04:40: this line first called the arm 2 runs arm 2b.)
- **A crowd that kept changing, not a sealed pocket.**
  - From 59k to 68k the bottom of the room held 80-140 ants. That is west of the brood column, 20-45 rows under the
    ground line. Ants came and went.
  - Their energy fell together: a median 189 J at 59k, 37 J at 68k.
  - 50 of the 51 starvers died there, within 736 frames (68,494-69,230).
- **They were climbing and falling back.**
  - Per starver, over its last 15,000 frames (medians): 583 steps up, 89 steps down, 496 falls.
  - Their pull: the hungry walk out 58% of the time, carrying soil out 31%.
- **The falls are in the room's open middle.** That means more than 2 cells from soil or brood.
  - 81% of the falls were there, 9% at a wall, 5% beside brood, 5% in side tunnels.
  - None were in the door shaft, out of 571 hungry decisions made there.
  - Without the switch the falls fall in the same places, at about the same rate per decision. What changed is how
    many hungry decisions were made in the open middle: 142,850 with the switch, 25,869 without.
- **With the switch the room is deeper, and more adults are born deep.**
  - The room reaches about row 212 at the foot of the brood column, against about 198 without it.
  - A brood column about 9 wide stands under the door from row 168 down.
  - Adults born at 50-65k: 365, against 163. Of those, 176 were born more than 25 rows deep, against 56.
  - Most of those born deep still got out: 91% reached the top 10 rows by 75k. 12% starved.

## 1. The fault in `starvewhere.py`

- **The bug.** `spaces()` floods the walkable cells only for rows under 176 (`min(y0 + h, 176)`, twice).
  - For a head at row 177 or deeper, no neighbouring cell is in any flooded space.
  - `bucket()` then finds no door, no sky and no free cell, and returns "encased".
  - The row was likely copied from `doorseal.py`'s door walk ("down to row 175"), where it does no harm: the door
    question only needs the top of the nest. (Inferred.)
- **The fix.** Flood to the bottom of the map. `tools/starvewhere_deep.py` is the shipped file with only those two
  bounds changed. I will open a pull request with the same change for `scripts/deeptrace_tools/`.
- **Before and after:**

| Run, starvers | Encased as shipped | Encased with the fix | Door open with the fix |
|---|---|---|---|
| b30smell s1, 55-70k | 50 of 51 | 0 | 50 |
| arm 2 s1, 20k-end | 433 | 0 | 454 |
| arm 2 s2 | 360 | 1 | 521 |
| arm 2 s3 | 153 | 0 | 1,170 |
| arm 2 s4 | 291 | 11 | 865 |
| arm 2b s1, 20-300k | 1,890 | 8 | 1,881 |
| arm 2b s2 | 3,379 | 35 | 4,195 |
| arm 2b s3 | 873 | 9 | 929 |
| arm 2b s4 | 2,046 | 11 | 2,238 |

  The "all ants" base rate falls from 6-14% encased to 0%. The rest are "off the map": far west on the surface,
  unchanged.
- **What "door open" means.** It means an unbroken 8-way path of air, ants, brood or crumbs joins the ant to the
  door. It does not mean an ant can climb that path. That is section 3's question.
- **Earlier reads that lean on "encased" need re-reading with the fix.** These are the files that use it:
  - `arm1b/README.md` (61-89%);
  - `arm2b/read-150k.md` and `read-300k.md`;
  - `arm2b/why-they-starve-at-the-store.md` §1;
  - `arm2c/read-150k.md`;
  - `sky-meal/starvewhere.txt` (skymeal s3: 1,701, 74%), and the README lines that quote it;
  - my `review-deep-trace-2026-10-07.md` §2;
  - `laybar-body-results.md`.
  - Re-read at 04:40: my arm 2, 2b and 2c notes are corrected in place, from `encased-recheck-deep-trace.txt` (all three
    arms, 20-150k and 20k-end, before and after). Nest race had already corrected `README.md` and
    `laybar-body-results.md`.

## 2. The crowd at the bottom of the room (census, every 1,000 frames)

The box is x 230-252, rows 180-205: the room's floor west of the brood column.

| Frame | Ants in the box | Median energy in the box | 51 starvers: p10 / median / p90 energy |
|---|---|---|---|
| 57k | 6 | 144 J | 159 / 547 / 1,760 J |
| 59k | 130 | 189 J | 185 / 195 / 537 J |
| 62k | 110 | 132 J | 126 / 135 / 192 J |
| 65k | 125 | 96 J | 79 / 95 / 182 J |
| 67k | 104 | 66 J | 55 / 67 / 84 J |
| 68k | 81 (66 under 50 J) | 37 J | 21 / 33 / 43 J |
| 69k | 14 | 26 J | 1 alive |

- **How it formed.** It formed between 57k and 59k. Of the 130 ants there at 59k, 68 had become adults at 57-59k,
  and 79 held a soil pellet.
- **Who left.** Of those 130, by 75k:
  - 90 were alive. All 90 reached the top 10 rows after 59k, and 84 went outside the nest.
  - 23 starved there.
  - 17 died of other causes.
- **Who died.** The 50 who died at 68.5-69.2k were whoever was there then. 23 of them came from the 59k crowd. Others
  arrived later, some by falling in.
  - Ant 1049753 was near the door at 66.5k. Within about 250 frames it fell about 22 rows, west of the shaft.
  - It then stepped and fell at 25-35 rows deep for 2,000 frames, and died at 68,766.
- **They starve together, which is inferred to be sharing.** The tight spread (p10-p90 of 55-84 J at 67k) points to
  energy passed downhill between nestmates, evening out the crowd's energy. That is inferred: the trace has no share
  column.
- **Where the starvers were born.** 34 of the 51 became adults at 55-65k.

## 3. Falls (`fallwhere.py`, hungry ants deeper than 10 rows, 55-70k)

| Where | Decisions with / without switch | Share of falls | Falls per decision | Starvers' decisions there |
|---|---|---|---|---|
| Chamber, open middle | 142,850 / 25,869 | 81% / 69% | 38% / 35% | 62% |
| Chamber, at a wall | 38,305 / 14,311 | 9% / 16% | 15% / 14% | 15% |
| Chamber, beside brood | 53,369 / 29,845 | 5% / 12% | 6% / 5% | 11% |
| Side tunnel | 25,878 / 5,330 | 5% / 3% | 13% / 8% | 11% |
| Door shaft | 571 / 168 | 0 / 0 | 0 / 0 | 0.1% |

- **How the places are drawn.** They come from `chambers.py`, the owner's chamber rule, on the map written at or
  before each decision. A fall is one cell of drop: the move roll won with nothing solid, powdery or plant beside the
  body, and no grip on a nestmate standing on ground.
- **Drops are short.** A median of 1 cell, p90 of 3-5.
- **Footing below a fall.** On the map, the first soil, brood or crumb straight below is a median 7 cells down in the
  open middle. Ants are not counted as footing there, and the map is up to 999 frames old for ants. So I infer the
  fallers were standing on other ants, but that is not measured.
- **What the fallers were trying to do.** Their last scored pull before a fall was the hungry walk out (52-60% in the
  room) or carrying soil out (27-40%).

## 4. Births (`birthfunnel.py`, adults born 50-65k, followed to 75k)

| Born | With switch: born / reached top 10 rows / alive at 75k / starved | Without: born / reached top / alive / starved |
|---|---|---|
| top 10 rows | 58 / 100% / 86% / 5% | 5 / 100% / 100% / 0 |
| 10-25 rows deep | 131 / 95% / 79% / 11% | 102 / 99% / 85% / 0 |
| deeper than 25 rows | 176 / 91% / 77% / 12% | 56 / 100% / 91% / 0 |

## Not traced

- Why the colony bred more and dug deeper with the switch on. On s1, adults born at 50-65k were 365 against 163.
- What the climbers stand on when they fall.
- What draws ants to the bottom-west. Some fall in from the top.
- Seeds 2 and 4, and `b90smell` s4. Those are Nest race's runs; the fixed tool needs running on them.

## Files

- Tools in `tools/`:
  - `fallwhere.py`, which needs `chambers.py` (copied here from `scripts/deeptrace_tools`);
  - `starvetrace.py`;
  - `birthfunnel.py`;
  - `starvewhere_deep.py`, which needs `doorseal.py` (copied here).
  - Each takes a run folder made with `hungry=1 mapevery=1000`.
- Tables: `body-s1-tables-deep-trace.txt`. Its §5 is the before and after of the fault, and §6-7 are the room, map by
  map, in both arms.
- The reruns stay in Deep trace's scratch.
