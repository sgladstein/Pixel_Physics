"""SCRATCH (chambers judge, 2026-10-29): `tunnels`, the judge's passage measure -- open passage cells that do not depend
on the door's neck. For judge.py only, never for selection.

Why: nestshape's `gallery` floods from the door through open cells, so one crumb plug in the neck zeroes a nest with real
open side tunnels (H3 worlds 62, 64, 65 score 1-2), and its `galleryc` (every open piece of 20+ cells, door or not)
gives a copy-3 crumb-packed pit up to 160 -- above the trained ant's best -- for the open strips left between the crumb
fill and the soil wall (world 68 F r0: two 30-long strips, no room anywhere). Measured on the shape controls
(/mnt/project-files/nest/shape-controls-2026-10-29.tgz, worlds 61-68, end of round 1 at 200k).

Definition:
  1. space   roomshape's nest space (every non-ground cell joined to the door below the datum; crumbs and food pass, so a
             crumb plug does not cut it -- only soil, packed soil, spoil and stone do) and its crater.
  2. open    space cells that are not crumbs or stored food, minus the crater; 4-connected pieces; keep pieces of at least
             MINC (40) cells. A crumb-packed shaft's open cells are the crumbs' gaps, pieces of 1-20 cells.
  3. rooms   the 5x5 opening of (kept open + the crater's open cells): the crater counts as room for this, so the open rim
             of a crater pit is not a passage (copy-3's second-round pits otherwise score 80-130 on their rims).
     passage kept open cells outside the rooms (the gallery-share rule).
  4. A kept piece HOLDS A ROOM if the 5x5 opening of the kept open cells alone has a roofed piece of ROOM (75)+ cells in it.
     There every passage cell counts. In a piece with no room a passage cell counts only if it is also outside the 5x5
     opening of the whole dug cavity (space with crumbs, food and crater): dug into soil, not a gap between a crumb fill
     and the wall.
  tunnels = the cells counted. Also: rooms (roofed rooms of 25+ cells), largest_share (largest open piece / open cells).

On the controls (H3 vs trained ant on H3's build, 8 worlds; copy-3 r0 and r1, 16 runs): H3 > ctl 8/8, no H3 below the
highest copy-3 (min H3 92, max F 84), but 3 of 16 copy-3 runs (r1 of worlds 61-63, 77-84) sit above the median ctl (76):
their side arms off a crater pit are real dug passages by eye. H3 62 and 65 beat ctl on rim cells only (92 v 87, 109 v
61): at 200k those two H3 nests are rooms split by a crumb pile, with crumb-filled or no tunnels, so no honest passage
measure ranks them far above the trained ant. Every variant that removed room rims (distance 2-4 from the room) lost
H3 > ctl in 62 or 65; using a cavity-thin rule everywhere lost 65. Plateau: MINC 30-40 and ROOM 50-100 give the same pass
pattern; MINC 20 lets copy-3 strips back (F max 129), MINC 50 drops an H3 tunnel piece (H3 > ctl 7/8).

    python3 scratch/nest/selrun/tunnels.py RUNDIR [RUNDIR...]
import: measure(rundir, mapfile=None) -> dict or None
"""
import os, shutil, sys, tempfile
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', 'inrun'))
import nestshape
from nestshape import roomshape  # nestshape's private copy, OPEN_ONLY False (colsel's threads flip the shared one)

MINC = 40   # smallest open piece that counts
ROOM = 75   # a piece holds a room if its roofed 5x5 opening has a piece this big
K = 2
BLK = [(a, b) for a in range(-K, K + 1) for b in range(-K, K + 1)]


def _opening(c):
    ctr = {p for p in c if all((p[0] + a, p[1] + b) in c for a, b in BLK)}
    return {(q[0] + a, q[1] + b) for q in ctr for a, b in BLK}


def measure(d, mp=None):
    if mp is not None:  # a snapshot: roomshape reads RUNDIR/nestmap.txt, so lend it a folder holding this map
        tmp = tempfile.mkdtemp()
        try:
            shutil.copy(mp, f'{tmp}/nestmap.txt')
            shutil.copy(f'{d}/surface.txt', f'{tmp}/surface.txt')
            return measure(tmp)
        finally:
            shutil.rmtree(tmp, ignore_errors=True)
    if not os.path.exists(f'{d}/nestmap.txt'):
        return None
    space, _, _, crater = roomshape.nest_cells(d)
    legend, grid = nestshape._grid(d)
    food = {chr(65 + k) for k, m in enumerate(legend) if m in nestshape.FOOD}
    opn = {p for p in space if grid[p[1]][p[0]] not in food}
    work = opn - crater
    pcs = roomshape.pieces(work)
    kept_pcs = [c for c in pcs if len(c) >= MINC]
    kept = set().union(*kept_pcs) if kept_pcs else set()
    passage = kept - _opening(kept | (crater & opn))
    roofed = [r for r in roomshape.pieces(_opening(kept))]
    big = [r for r in roofed if len(r) >= ROOM]
    held = set().union(*[c for c in kept_pcs if any(r & c for r in big)] or [set()])
    dug = passage - held - _opening(space)
    tunnels = (passage & held) | dug
    return dict(tunnels=len(tunnels), passage_in_room_pieces=len(passage & held), dug_no_room=len(dug),
                rooms=sum(1 for r in roofed if len(r) >= 25), largest_share=round(len(pcs[0]) / len(work), 3) if work else 0.0,
                open=len(work), kept=len(kept))


if __name__ == '__main__':
    for d in sys.argv[1:]:
        print(d.rstrip('/'), measure(d))
