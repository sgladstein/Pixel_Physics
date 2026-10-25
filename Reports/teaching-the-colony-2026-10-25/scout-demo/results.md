# Lone scout demo: what was built, and what four seeds say (2026-10-25)

*Built by the scout seat thread. Design: [`../seat-scout.md`](../seat-scout.md)
and its critique. Feasibility review: [`review-step1-feasibility.md`](review-step1-feasibility.md).
Plan, its independent review, and the revision: [`plan-v1.md`](plan-v1.md),
[`plan-v1-review.md`](plan-v1-review.md), [`plan-v2.md`](plan-v2.md). The box
and its food: [`step0-census.md`](step0-census.md).*

## What it is

`cargo run --release --bin lab -- demo=scout arm=A`. You are one worker in a
hungry trained colony. A rich heap lies 120 columns east of the nest and
nobody goes there. You tap the sister beside you (`Y`); if she is hungry
enough she follows with her head on your body, stops now and then to look
around, and you have to stop with her or she loses you. At the heap she is
taught it, and from then on her walk leans toward it (the keeper seat's
recall head, a scripted pull named on the panel). A taught sister home from
the heap may tap a sister of her own (the cascade, arm A).

Three arms on one box: **A** the scout; **B** the design the owner turned
down, on equal terms (`Y` calls every willing sister within six cells into a
column that never stops or strays, same hunger bands, same cap of six);
**C** A without the cascade. `arm=1|2|3` seals the letters by the box seed
and hides the knobs.

## The pieces

| Piece | Where | Note |
|---|---|---|
| Tandem runs | `src/sim/tandem.rs` | Tap, follow by touch, rest beside the body, give way, pauses (a tell, then 60-90 frames) at landmarks and on a 1-in-50-steps clock, break after 45 frames over 4 cells, ground sweep [8, 16, 32] for 360 frames, arrival, graded lesson, cascade per trip home, B's column. Off (`World::tandem` `None`) is byte-identical, tested against a live pair |
| The seat | `src/lab/demo/scout.rs` | On the shared launcher (#904). Warm-up to frame 18,000 at the top of the dial, then the body (a modal-pace adult by the trickle, worker role). The leader is held while her pupil lags 3+ cells. Overlays drawn as full replaces on fixed colours |
| Headless player | `examples/scoutdemo.rs` | The seat's own start and keys; a script that walks, taps, waits, eats when hungry, digs out when boxed. `gif=1`, `gain=`, `off=1`, `map=FRAME`, `rude=1` |
| Memory | `creature::recall` (keeper, #907) | `plant_with` on arrival, renewed on each trip home from the heap |

## Four paired seeds

18,000 frames after the hand-over (5 minutes at 1x), scripted player, seeds
1-4. "Trips home" counts deliveries by an ant whose memory lies in the heap
(`trip_deliveries`, never `delivered`).

| Arm | Led | Arrived | Lost | Trips home by taught sisters | Trips home from the heap, all ants | Cascade runs |
|---|---|---|---|---|---|---|
| A, pull 6 (lesson renewed) | 12/11/14/14 | 11/10/11/12 | 0/0/0/0 | **6/18/4/15** | 39/64/40/66 | 1/0/1/2 |
| A, pull 0 (honest) | 11/13/11/13 | 10/12/11/11 | 0/0/0/0 | **2/14/1/10** | 8/54/37/59 | 0/0/0/1 |
| B, column, pull 6 | 33/36/23/26 | 23/34/20/20 | 6/0/2/5 | 29/49/18/14 | 56/80/39/53 | - |
| C, escort, pull 6 (before renewal) | 11/12/12/12 | 10/12/11/12 | 0/0/0/0 | 2/26/5/3 | 10/82/48/51 | - |

Colony alive at the hand-over and the end ranged 47-147 with no consistent
order between arms: the colony's own boom and bust swamps the player.

**What this says.** The run works on the trained ant: nearly every sister
led arrives and none is lost under a patient leader; the pupil sits one or
two cells from the leader's body on 94% of frames (seed 1). The memory pull
makes taught sisters bring food home from the heap more often, on all four
seeds, but the effect is small (four or five more trips in five minutes).
B leads two to three times as many sisters and wins every raw count; the
question the demo exists for, whether leading one sister by touch *feels*
better, is the owner's and is asked on the first-look page.

## Found on the way (each cost a run)

1. **Contact is to the leader's body, not her head.** Head to head, a pupil
   touching the tail of a leader facing away reads three cells off; the
   leader was held waiting for a gap that could not close, 2,000 frames.
2. **A steered worker must think with her mind.** On the launcher's first
   hand-off a keyboard body ran the old hand-built ant and drifted east
   while held west; the narrowed hand-off (queen's body only) fixed it, and
   leads went from 2 to 7 in ten minutes.
3. **Arrival cannot need food in reach.** The heap is eaten down between
   drops; a pupil stood in it with her leader for 1,800 frames, never
   arriving. The leader stopping in the patch for a second now counts.
4. **A heap drop entombs whoever stands in it**: her options shrink to
   walking backwards, turning round and resting. The scripted player stops
   at the near edge.
5. **She was taught the heap, not a cell.** The recall head keeps a lesson
   within six cells of where it was planted; her own pick-up anywhere in a
   20-column heap erased it, and the head tilted 17 decisions in a whole
   seed-1 run. A trip home from the heap now renews it.
6. **The welcome card covers every headless shot** unless
   `Lab::show_help = false`.
7. **"Did not step this tick" is not "blocked".** The search first turned
   back on any tick the pupil had not stepped on, which is five ticks in six
   for a walking ant: she paced four cells for the whole search and never
   reached a leader eight cells off. She now climbs after 20 still frames
   and turns back after 60. No paired run above lost a pupil under a patient
   leader, so the search never ran in them and their numbers stand.

## Frame cost

Mean tick 0.79 / 0.86 ms with the module on against 0.81 / 0.84 off (seeds
1, 2, 6,000 frames, the same player; `scoutdemo off=1`). Within noise.

## Open

- The cascade fires 0-2 times a run at c = 0.25 a trip; the plan's half-c
  second run is moot until it fires more.
- Pause frequency is waiting on the owner's answer (does a stop read as her
  checking the way, or as the game stalling?).
- The pull is scripted. A heritable "how much to listen" rider, so selection
  can tune it, is the keeper seat's next step and the honest route to
  shipping it.
