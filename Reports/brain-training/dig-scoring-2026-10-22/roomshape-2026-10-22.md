# roomshape.py: how far a nest is on the way to multiple rooms (dig-scoring thread, 2026-10-22)

> **Landed 2026-10-24** by the report sweep (thread *Land unlanded thread reports*): copied verbatim from the project shared folder `teacher-lane/nestmaps-1022/ROOMSHAPE.md`. [`scripts/roomshape.py`](scripts/roomshape.py) and the Teacher lane's [`scripts/nestcount.py`](scripts/nestcount.py) are beside it; the 192 end-of-round nest maps it was built on stay in the shared folder (`teacher-lane/nestmaps-1022/`). (`Reports/handoff/needs-ant/tools/roomshape.py` is an older, different tool of the same name.)

For the Teacher lane's lexicase, beside the crater case. `import roomshape; roomshape.score(rundir)` returns one
float, higher = better; `parts(rundir)` also gives room sizes and tunnel scores. Reads `nestmap.txt` + `surface.txt`.
About 0.06 s per map. `python3 roomshape_test.py` prints the hand-drawn shapes.

## What it pays

- **A second room** (anywhere a 5x5 block fits, as a separate piece from the main room): min(cells, 50) / 2.
  A first 5x5 pocket pays 12.5, a 7x7 chamber 24.5, capped at 25 per room.
- **A tunnel or the start of one** (open space outside every room, reaching away from the main room): its reach - 2,
  capped at 15. Only the best 4 count. So a side passage pays from its 3rd cell on: 1, 2, 3, ... per cell dug.
- **Nothing for the main room**: bigger, taller, longer, ragged-walled, all 0.
- **Nothing for the crater**: open-to-sky space is never a room and never a tunnel. That is the crater case's job.

## Hand-drawn shapes (roomshape_test.py)

| shape | score |
|---|---|
| one room 13x7, 25x7, 13x11, 41x9, and 10 ragged-wall variants | 0 |
| funnel, open pit, open bowl split by pillars | 0 |
| comb of 2-deep stubs on the floor | 0 |
| side tunnel 1 / 2 / 3 / 4 / 6 / 10 / 20 long | 0 / 0 / 1 / 2 / 4 / 8 / 15 |
| tunnel 8 + room at its end: 3x3 / 5x5 / 7x7 / 13x7 | 10 / 18.5 / 30.5 / 31 |
| two rooms stacked on a shaft | 27 |
| open pit + a roofed undercut 2 deep / 6 deep | 0 / 4 |

## Real maps: old ant vs the selected colonies

The old ant: 12 common-garden runs (hA1 genome with `nomind=1`, 200k frames, sel off, worlds s*1000+99, s = 1..12).
Maps are in `old-ant-cg/`.

| | median | p90 | max |
|---|---|---|---|
| old ant, 12 worlds | **20** | 65 | 70 (3 worlds with a real second chamber: 49.5, 65, 70) |
| cs3lex, 64 colonies | 6 | 16 | 27 |
| cs3lexles | 4 | 13 | 17 |
| cs3lexlesinh | 3 | 13 | 20 |

The old ant is above the selected median (3-6) in all 12 worlds and above 16 in 8 of 12, but not above every colony
in every world: its worst world (8) is matched or beaten by 58 of the 192 colonies. The 2 colonies at or above its median that I looked at: cs3lex c9 (27) has two
real roofed chambers either side of the shaft, and cs3lex c13 (27) is an open pit with four short burrows off its walls.

## Two things it got wrong on the way, kept so nobody re-finds them

- **Leaving the crater out of the space made fake rooms.** Selected colonies dig 2-3-wide open slots 25 deep. The
  slot's overhung walls then counted as separate rooms (cs3lexlesinh c39 scored 76). The crater is now in the space
  and counts as body.
- **A pocked crater bowl split by pillars counted as three rooms** (cs3lex c61 73). Now any room mostly open to the
  sky is dropped.

Known soft spot: a chamber pinched to a 4-tall neck counts as two lobes (old ant world 12099, rooms 70 + 65). That is
a step toward two rooms, so I left it in.

## Per-ant credit for the same thing (`w_shape`), 19:50Z: built, small test negative

- **A local rule can't do it.** On the old ant's runs, the open cells in the 5x5 around a cut had the same spread for
  cuts that ended up as tunnels as for cuts that ended up in the main room. Rooms start as tunnels.
- **What can: pay each cut of original ground by how much it raised this score at that moment** (capped at 4 points).
  It is paid at the cut, so it avoids `w_room`'s problem (cutters dead before payment). On the old ant it pays 17-29%
  of cuts that end up as tunnels, against 9-17% of main-room cuts.
- Code: `w_shape=W` in inrun.rs (`shape_score`, a Rust copy of this file; its end-of-run `shape` line matches
  roomshape.py exactly on old-ant worlds 2, 7, 10, 12). Patch vs 529ff42a:
  `/mnt/project-files/nest-score/digloss/wshape-vs-529ff42a.patch`. Off by default.
- **Small test** (hA1, `rounds.sh`, 8 seeds x 3 rounds, sel=score, the colsel5 base args without inheritance,
  w_shape=3 vs 0):

  | round | room score, median (shape vs base) | paired | paid shape cuts per colony | home food | starved |
  |---|---|---|---|---|---|
  | r0 | 2 vs 3 | 3 up, 5 down | 26.5 | 394 vs 323 | 71 vs 30 |
  | r1 | 4 vs 3 | 3 up, 3 down | 22.5 | 701 vs 548 | 6.5 vs 28 |
  | r2 | 1 vs 8 | 0 up, 5 down | 30 | 890 vs 580 | 4 vs 76 |

  Neither arm builds a room. Both arms' scores are short burrows off a crater wall. The base arm lost one colony.
  Paid shape cuts did not rise over rounds, so selection inside a round is not picking it up at this weight.
  On its own it is not a lever. Its use, if any, is inside colony selection with the +shape case, as a within-colony
  gradient. That is untested.
