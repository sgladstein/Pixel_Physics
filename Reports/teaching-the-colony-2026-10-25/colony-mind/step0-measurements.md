# Colony mind seat, step 0: does a trained colony answer the mind's verbs?

*2026-10-25, colony-mind seat thread. Measured on branch `claude/project-thread-6xokl9` (main 9e32979e plus one
off-by-default hook, `Minds::closed_stomach`). Instrument: `examples/mindprobe.rs`, compared by
`scripts/mindprobe_cmp.py`. Bed: `steady_income` (52 trained ants at column 256, 40 cells of food 30 columns east
every 1,000 frames), trained pool (the lab's default brain). Each arm runs the same seed as its `arm=none` twin and
is bit-identical to it until the press, which starts 20,000 frames after landing; the window is 3,000 frames.*

## CORRECTION (2026-10-25 10:05Z, after the independent plan review): swell does NOT bring more food home

The deliveries below are `MindCounts::delivered`, which books every crop cell put down at the nest, **including food
picked up inside the nest and put straight back** (`Reports/how-we-test.md` §4 already lists this trap for plain
`deliveries`). Re-measured on the honest count, `CreatureStats::trip_deliveries` (16 paired seeds, twin 105-328 per
3,000 frames): swell at the door **+30 [-113, +100] at 1,000 frames, +8.5 [-130, +152] at 3,000**; hush **-35.5
[-137, +110]**; the echo chaos floor **+8.5 [-100, +87]**. Swell makes the door busier (more handling, ants out -1.7)
and brings no more food home. The swell/hush rows below measure re-handling, not food. The stomach and echo rows
stand as written.

## The answer in one paragraph (as first written; read with the correction above)

Two of the three verbs the feasibility review left standing move a trained colony, visibly and by degree, so the
review's stop rule passes. **Pressing on the foragers arriving at the door, so their food trail is laid louder,
lifts deliveries by about a fifth to two fifths depending on how long you hold.** **Closing the shared stomach over
the door stops nearly all mouth-to-mouth sharing and doubles the spread of hunger there.** Alarm and warmth, the
other two things a trained ant senses, move nothing we could count.

## Results (16 paired seeds each unless stated; twin means: 50 ants, 334 cells delivered per 3,000 frames)

| Verb (arm) | Hold | What moved (median paired difference, [range over seeds]) | Seeds in the direction |
|---|---|---|---|
| Swell the trail at the door (`trailnear`, r=15, x5 deposit) | 240 frames | deliveries **+62** [-104, +155] | 12/16 up |
| | 1,000 frames | deliveries **+143** [-50, +361] (+43%) | 15/16 up |
| Swell every laden ant's trail (`trail`, upper bound) | 60 | deliveries +37 [-77, +120] | 11/16 |
| | 240 | deliveries **+76** [-27, +192] | 13/16 |
| Close the stomach over the door (`stomach`, r=12) | 1,000 | adult feeds -190; hunger spread in the zone +0.19 tank; lean ants +0.5 | spread 15/16 up |
| | 3,000 | adult feeds **-1,054** (twin ~1,100: sharing nearly stops); hunger spread **+0.91 tank** [+0.52, +1.22] (twin 1.09, so doubled); lean ants (<1 tank) **+4.3** [+0.3, +9.0] | **16/16** both |
| Hush the trail at the door (`hush`, r=15: food trail under each laden ant and its 8 neighbours scrubbed to 0 each tick; needs the new `World::scale_pheromone`) | 1,000 | deliveries -23 [-157, +87]: the chaos floor | 9/16 down |
| | 3,000 | deliveries **-107** [-213, +12] (-32%); ants within 15 cells of the door -5.4 | **14/16 down** |
| Echo an alarm (`echo`, r=6, held at 240/255) | 60-1,000 | ants within 15 cells, out of the nest, speed: no change (medians within +-1 ant, 0.000 speed); deliveries +11.5 [-87, +133] = the chaos floor | none |
| Warm a spot (`heat`, r=6) | 60-1,000 (6 seeds) | ants near: -2 to -3 early, inconsistent sign; nothing else | none |

**The noise floor.** Any perturbation makes a run diverge from its twin, so differences in deliveries have a spread
of roughly +-100 cells per 3,000 frames even when the verb does nothing. Echo is that floor in practice: alarm laid
over a disc (the plane rose by 73 full deposits within 15 cells, the positive control on the press) moved no ant
count. A one-unit trail deposit (`arm=null`) did not even diverge, so it cannot serve as the floor. Swell's paired
difference over echo on the same seed is positive on 14 of 16 seeds (mean +54).

**What each result means for a player.** Swell is a verb with a payoff and a grade (more hold, more food home, up
to a plateau between 240 and 1,000 frames), and hush is its inverse, slower to bite (it needs a long hold): one two-way dial on the food trail. Close the stomach has a grade (longer, wider spread) and a visible
shape (a few fat, more lean), but in a fed colony it changed neither deliveries nor survival in the window; its
payoff has to come from famine, which step 6 of the plan must measure. Alarm and warmth are not verbs on today's
ants.

## Two facts about the bed a demo must live with

- **The trained colony booms and busts on steady income.** Headcount every 15,000 frames over 150,000 (8 seeds):
  seeds swing 9 -> 230 -> 80 and back; 2 of 8 went extinct (seed 4 at ~120k, seed 6 at ~30k). The colony is
  small (9-42 ants) at 15,000 frames.
- **On a far-food bed (`far_larder`) the trained colony is dead within 18,000 frames on all 4 seeds, swelled or
  not**: swell acts only on laden ants, and nobody gets laden. The voice cannot fix far food.

## Reproduce

```text
cargo build --release --example mindprobe
mindprobe arm=none seed=S dx=0                                  # twin (dx=0: target on the door)
mindprobe arm=trailnear hold=1000 r=15 dx=0 seed=S
mindprobe arm=stomach hold=3000 r=12 dx=0 seed=S
mindprobe arm=echo hold=240 seed=S                              # target 15 columns east of the door, on the surface
python3 scripts/mindprobe_cmp.py DIR near:all,lean:all,zone_sd:all,delivered:end,fed:end
```

Raw outputs were in `/tmp/claude-0/mp*` of the session (not kept); every number above re-runs in about 90 seconds
per 80 runs on 4 cores.
