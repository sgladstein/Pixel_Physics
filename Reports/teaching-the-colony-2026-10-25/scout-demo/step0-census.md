# Scout step 0a: a trained colony that stays alive (2026-10-25)

Probe: `census.rs` (this folder; a scratch crate linking pixel_physics at main 9e32979e). Box = steady_income's bed
(512x320), trained ant (`set_new_ant(true)`), lab defaults (mutation as the lab ships it), 52 ants land at column 128
at frame 200; near trickle of provisions at column 153 every 1,000 frames; far heap 120 cells at column 330 every
6,000 frames; a soil bank 14x6 at columns 236-249 between. Columns: alive / hungry (energy < 0.5 tank) / median
energy in tanks / ants within 20 columns of the far heap, every 6k frames.

| trickle | seed | 6k | 12k | 18k | 24k | 36k | 48k |
|---|---|---|---|---|---|---|---|
| 0 | 1 | 3 | 2 | 3 | 2 | 1 | 0 |
| 4 | 1 | 15 | 21 | 32 | 58 | 70 | 34 |
| 6 | 1 | 6 | 15 | 21 | 39 | 65 | 109 (h13, far10) |
| 6 | 2 | 12 | 17 | 24 | 38 | 38 | 91 (h2, far17) |
| 6 | 3 | 12 | 16 | 31 | 57 | 100 | 72 (h3; 134 at 42k, h27) |
| 6 | 4 | 14 | 15 | 37 | 49 | 59 | 79 (h10, far8) |
| 12 | 1 | 10 | 15 | 20 | 34 | 43 | 107 |
| 24 | 1 | 16 | 28 | 29 | 33 | 75 | 110 |

Findings:
- **The landing cohort starves**: 44 of 52 die of Starved between frames 500 and 2,000 after landing, with the
  trickle on (same when landing at 6,000). The known "most landers starve". So the demo must warm up past it.
- **Trickle 6 keeps the colony alive on 4 of 4 seeds** (min 72 at 48k) and it grows from ~20-37 at 18k. Choice:
  trickle 6, warm-up to 18,000 frames.
- **Ants are not short of energy on average**: median 1.5-4.9 tanks; "hungry" (< 0.5 tank) is 0-27 ants. An accept
  rule keyed on energy < 0.75 tank would refuse almost every tap. The accept chance must be graded over 0.5-5 tanks.
- Some colony ants already reach the far heap (0-17 within 20 columns). The far heap is not unknown, only rarely
  worked: the player's job is to make it a route, not to discover it from nothing.

## 0b: far distance and trickle (2026-10-25 ~09:15Z; no bank)

Heap at 90 / 120 / 150 columns (x 218 / 248 / 278), trickle 6, seeds 1-2: colonies boom and bust hard (seed 1 at 90:
13 at 18k, 4 at 24k; seed 2 at 120: 1 ant at 42k). Removing the soil bank alone changed the 6k count from 6-16 to
31-43: outcomes are chaotic in placement, so judge by seeds, not one run. Then heap at 120 columns, trickle 10 / 14,
seeds 1-4, to 54k (the demo window is 18k-54k):

| trickle | seed | 18k | 24k | 30k | 36k | 42k | 48k | 54k | far max |
|---|---|---|---|---|---|---|---|---|---|
| 10 | 1 | 90 | 85 | 69 | 36 | 70 | 97 | 67 | 9 |
| 10 | 2 | 38 | 52 | 67 | 67 | 88 | 88 | 93 | 10 |
| 10 | 3 | 76 | 92 | 76 | 61 | 85 | 72 | 84 | 12 |
| 10 | 4 | 83 | 52 | 28 | 29 | 12 | 30 | 23 | 8 |
| 14 | 1 | 47 | 55 | 94 | 120 | 143 | 57 | 92 | 11 |
| 14 | 2 | 16 | 21 | 14 | 9 | 19 | 21 | 31 | 0 |
| 14 | 3 | 79 | 95 | 111 | 77 | 49 | 71 | 51 | 4 |
| 14 | 4 | 63 | 83 | 141 | 108 | 64 | 82 | 100 | 6 |

Choice: **trickle 10, heap at 120 columns, hand-over at 18k**, `grant_ants` top-up below 25 at hand-over. No seed went
extinct in the window; one (t10 s4) fell to 12, which is a pressure the player can relieve, not a countdown.
Unled ants within 20 columns of the heap: 0-12 of 12-143, so the heap exists but is not a route.
