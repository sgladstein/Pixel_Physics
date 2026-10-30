# Fresh-cut mark (nest lane, 2026-10-30): the fun-hunt plan's nest track, stopped on day 2

The plan Scott approved on 2026-10-30 gave the nest lane four days for a short-lived "cut here recently" mark in the soil,
read by the hand-written digger (H3, `../handdig/`) beside its "open space, dig less" brake, judged by eye in the
played bed. Stop rule: still a bowl or a maze in the played bed on day 4. Stopped on day 2: every form of the mark was
a bowl or a crumb-filled blob in the played bed, with and without the lay brake, and none added tunnels even in the
garden worlds where H3 already digs them.

**The code is not on main.** It was built in `ffefc7a8` (all-round mark) and `a71e62cc` (tip-only mark) on
`claude/project-thread-yqi1sk` and reverted in the commit that added this file:
- `World::mark_cut` / `mark_cut_from` stamp a cut's neighbours (or, with `PP_FRESHCUT_TIP=1`, only the cell straight on
  from the cutter) with the frame, at every path that writes `dug_cells`: `cut_cell`, `pack_behind`, `escape_cut`, the
  founding shaft. A `PosMap` pruned to the fade window.
- `PP_FRESHCUT=W`: dig slot 4 reads the mark at the target, 1 for a cut this frame fading to 0 over W frames.
- `../handdig/make.py fresh=K`: unit 7 ranks a face beside a fresh cut up by about K.
Controls: H3 (no weight on slot 4) with the mark on gives the same nest map at 30k as with it off; `fresh=6` changes it.

`lab.sh` runs one pool in a lab box with nest maps every N frames (`labsurvive`); the garden runs used
`../poolrounds.sh ... 1 200000 POOL nest_r=0` with `SETX=PP_FRESHCUT=400[,PP_FRESHCUT_TIP=1]`. Tunnels are the chambers
judge's `tunnels` (`scratch/nest/selrun/tunnels.py`, branch `claude/chambers-labbed`), which crumb-packed shafts do not fool.

## Results (window 400 frames)

| | tunnels vs H3, 8 garden worlds at 200k | played bed, 4 seeds, 600k, by eye |
|---|---|---|
| all-round mark, weak (K=3) | more 4, fewer 4 | pit under the roots, then a crumb-filled bowl |
| all-round mark, strong (K=6) | fewer 8 of 8 (p 0.008): fewer, fatter arms | same |
| tip-only mark, weak | more 3, fewer 5 | (with the lay brake) a crumb- and water-filled blob |
| tip-only mark, strong | more 2, fewer 6 | same; 1 of 4 colonies dead |

Played bed with the garden's lay brake (`labsurvive laycap=48:8 nestrows=88`, test switch on `claude/chambers-labbed`):
colonies hold at 50-80 ants and starve ~8x less, nests are bigger, but every arm, H3 included, is one blob packed with
crumbs; tunnels 0-42 in all 16 runs. Positive feedback at the face thickens whatever is being dug: this is the fourth
fresh-cut try (FRESH_CUT=recruitfed 10-03, DIG_HOME=mark 10-10/10-11), and the brake did not change the outcome.
Pictures: https://claude.ai/artifact/Jnzf4Raro16z7wddNsQFq5#fresh
