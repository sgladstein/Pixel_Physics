# The colony mind seat: no standalone demo on today's trained ants

*2026-10-25, the colony-mind seat thread of the six seat demos (board: `/mnt/project-files/seat-demos/BOARD.md`).
The owner asked each seat for an adversarial feasibility review, then a plan, an independent review of it, a
revision and a playable demo. This seat stopped after the plan's review, because the measurements that review asked
for came back negative. The working notes are in [`colony-mind/`](colony-mind/): the feasibility review, the step-0
measurements (read its correction), plan v1 and its review.*

## The answer

**The colony mind seat cannot be an honest demo on the ant the lab runs today.** Of the design's fifteen verbs, one
changes what trained ants do in a way a player could see and grade: **closing the shared stomach** over a chamber
stops nearly all mouth-to-mouth sharing there and doubles the spread of hunger. It changes neither food brought home
nor survival. Everything else either acts on signals the trained ant never reads, or moves a counter that turned out
to be counting the wrong thing. And the stake the plan leaned on, steering a colony through famines, could not be
moved by any player policy: the bed killed 14 of 16 colonies with or without a hand on it.

**What carries forward:** close-the-stomach is a real, cheap verb for another seat (it would gate the keeper seat's
"a place spreads by meals"); favour is the queen seat's who-lays choice; and two tools (a paired verb probe and a
way to lower a scent cell) are landed for whoever tries next.

## Why the design failed: it was checked against the wrong ant

The design and its first review were both checked against the hand-built ant. The lab binary has run the trained
mind by default since 2026-10-24, and a mind ant leaves `creature_tick` into `mind::tick` (`creature.rs:7998-8022`)
**before** sensing, `act()` and the hand-built walk. So the design's anchor verb, "swell the news a returning forager
brings", acts on the forage drive and the met-a-returner stamp (`colony_drive_level`, `creature.rs:26097-26135`),
which no mind reads and a mind's delivery does not even write. "Hush the door", "spend the reserve" and "make the
brood beg" fail the same way. `Reports/how-the-ant-works.md` §13 is corrected in this change; the finding was sent to
the other five seats, whose designs had the same blind spot.

## What was measured (16 paired seeds unless stated)

Instrument: [`examples/mindprobe.rs`](../../examples/mindprobe.rs), compared with
[`scripts/mindprobe_cmp.py`](../../scripts/mindprobe_cmp.py). Each arm runs the same seed as an unplayed twin and is
bit-identical to it until the press, so a difference after the press is the verb's, against a chaos floor of about
+-100 cells per 3,000 frames (the echo arm, which moved no ant, is that floor in practice). Bed: `steady_income`,
52 trained ants, 40 cells of food every 1,000 frames.

| Verb | What it does in the probe | Result | Verdict |
|---|---|---|---|
| **Swell the trail** (the design's swell, rebuilt on what a mind senses) | Laden ants in a disc at the door lay food trail x5 each tick | Food home (`trip_deliveries`): +30 [-113, +100] at 1,000 frames, +8.5 [-130, +152] at 3,000. Ants out of the nest -1.7 | Noise. A busier door, no more food |
| Hush the trail | Food trail under laden ants at the door scrubbed to 0 (new `World::scale_pheromone`) | Food home -35.5 [-137, +110] at 3,000 frames | Noise |
| **Close the stomach** | `Minds::closed_stomach` disc over the door: adults there are not offered Feed to adults | Adult feeds -1,054 of ~1,100; hunger spread in the zone +0.91 tank (doubled); ants under one tank +4.3 — **16/16 seeds** each | **Works**, graded by hold. No change to food home or headcount |
| Echo an alarm | Alarm held at 240/255 over a disc | Ants near, out, speed: no change | Not a verb on trained ants |
| Warm a spot | `add_heat` over a disc | Inconsistent sign, small | Not a verb |
| Favour a line | — | Lab minds do not mutate and the 30 pool members are near-copies (8-107 of 3,285 slots differ), so favour moves line share and no behaviour | The queen seat's verb |

**The stake** ([`scripts/mindstake.py`](../../scripts/mindstake.py); bed
[`colony-mind/demo_mind.ron`](colony-mind/demo_mind.ron): steady income switched off for 15,000 of every 35,000
frames; 16 seeds, 150,000 frames): extinct by the end **14/16** unplayed, 13/16 with the trail swelled throughout,
14/16 hushed, 11/16 with the stomach closed over the nest in every famine. Mean headcount paired against the twin:
swell -2.9 (up on 5/16), hush -9.3 (5/16), stomach in famine +1.1 (8/16). The plan review found the same on a
second famine bed: colonies often grow through the famine on stored food and crash 5,000-20,000 frames after income
returns, and the steady-income twin crashes with no famine at all. A player could not credit or blame their hand.

## The mistake worth remembering

**Step 0 first read the swell as "+19% to +43% food home", on `MindCounts::delivered`.** That counter books every
crop cell put down at the nest, including crumbs picked up inside the nest and set straight back; the unplayed twin
"delivered" 2.4x to 5.8x everything that ever arrived. `Reports/how-we-test.md` §4 already warned about the plain
`deliveries` counter; the mind's own counter is now named there too. The independent plan review caught it in one
pass, by asking whether the twin's number could exceed the income. That check (can the control's figure exceed what
physically entered the system?) is the positive control this measurement lacked.

## What would make the seat possible later

- **An ant that listens.** A trained sense for "met a forager coming home" (a context sense with a seeded weight,
  trained on a bed where it pays), so a swell has a reader. That is Teacher-lane work, not a demo patch.
- **A stake the player can see.** The boom and bust is the bed's own; a raider (the enemy seat's) produces deaths a
  player watches happen, where a famine's deaths arrive late and unattributed.
- **The arc.** The design's own best role was the late stage of queen -> scout -> mind. The working stomach verb fits
  there as a tool in the queen game, not as a seat.

## Engine changes in this change, all off by default

- `Minds::closed_stomach` and `options::stomach_closed`: discs where adults are not offered Feed to adults. Empty in
  every world, so the shipped lab pays one `Option` read per adult-feed candidate.
- `PheromonePlane::scale`, `Pheromones::scale`, `World::scale_pheromone`: lower one scent cell (the only write that
  lowers a plane outside decay). Called by nothing but the probe.
