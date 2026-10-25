# The guest seat demo: a beetle that lives off the colony by smelling like it

*2026-10-25, guest seat thread. One of the six seat demos the owner asked for
(`teaching-the-colony-2026-10-25.md`, its seat designs in
`teaching-the-colony-2026-10-25/seat-guest.md` and `seats-overview.md`). Run
through the owner's five steps: an adversarial review with a feasibility
verdict, a plan, an independent review of the plan, a revised plan, and the
build. The four planning documents are kept beside this report in
[`guest-seat-demo-2026-10-25/`](guest-seat-demo-2026-10-25/).*

```
cargo run --release --bin lab -- demo=guest            # land= gap= override the defaults
cargo run --release --example guestgif -- script=beggar out=beggar.gif
```

## What the player gets

You are a rove beetle: amber, three cells long, one more than a worker. The
box is steady income's (`assets/lab_scenarios/demo_guest.ron`) with the
trained colony on; the clock runs at 256x while the colony grows, and at
frame 30,000 you land 24 columns from the door and the clock drops to 1x.

You smell wrong. Every ant that touches you decides, on a graded ladder, and
the decision shows over its head for about a second:

| Mark | The ant | What it costs you |
|---|---|---|
| none | walks on | nothing |
| `?` yellow | stops and holds on you (1.5 s) | time |
| `!` orange | flares, backs off, a puff of alarm | the alarm makes the next ant stricter |
| `X` red | lunges and nips | three nips take a body cell; the head goes last |
| `+` green | (after a beg) gives you food | — |
| `-` grey | (after a beg) has nothing to spare | — |
| `*` blue | (after a rub) was groomed | its smell moved toward yours |

Your verbs: **R** rub the ant beside you (your smell moves a tenth of the
gap toward hers, hers a quarter as far toward yours, and she rolls at half
odds), **P** soak on or off (lying in the nest or beside brood pulls your
smell toward the nest's odour; refused while you are more than two radii
off), **Y** beg (an ant with spare bank gives up to 30 J), **B** kill a
larva beside you (a corpse worth its bank, and an alarm), **E** eat a
corpse, **J** jump, arrows to move. The disguise fades back toward your own
smell (time constant 60 s), so it has to be kept up. Death is graded (a cell
at a time), and a new rove lands three seconds after the last one dies.

A ringed outline in a fixed green-amber-red ramp says how far off you smell;
the ledger top left keeps your smell, energy, cells, gifts, refusals, the
ants' verdicts, rubs, larvae killed, the colony now against your arrival,
and a grade: **cuckoo** (lives on gifts), **thief** (on the dead),
**predator** (on the brood).

## How it works

- **The smell ladder** (`src/sim/creature/guest.rs`, `World::visitors`).
  `None` in every world but `demo=guest`, and then every hook is one
  `Option` read. An ant whose head touches a registered visitor rolls once
  per episode (6 s cooldown per ant) on the gap between the visitor's scent
  and its own, over its own tolerance radius, as its body expresses them.
  The effective gap adds alarm at its head (up to +0.25), a fixed per-ant
  pickiness (±0.2), hunger (up to +0.2) and a refused beg (+0.3). Antennate,
  display and nip are logistic in that gap, centred at 0.5, 0.9 and 1.3, the
  first two never 0 or 1, and no nip below 0.8.
- **The ants decide; the ladder only enforces the act.** `guest::meet` runs
  in `creature_tick` before the mind and never returns early; a held ant is
  offered only `Rest`, then one `Walk` away, inside `mind::tick` after the
  option list is built (`guest::force_options`), so its bills, memory and
  reschedule run as for any decision. Attack is never offered on a
  registered visitor, so the nip is the only harm and it is graded.
- **The nip** is the game's bite done-branch on the visitor's rear cell
  (`ENCOUNTER_BITE`, an alarm cried, three nips a cell, the kill tallied as a
  bite). **The beg** books `SharedOut` on the giver's colony and `SharedIn`
  on the guest's; **the larva kill** goes through `brood::larva_eaten`, which
  stamps the corpse with the larva's bank and books it once.
- **The body** is a new species, `rove` (`assets/species/rove.ron`): a
  carnivore gut, no old age, no breeding, held off the lab's stocking chip
  (`NOT_STOCKABLE_IN_LAB`), steered by slice 0's hooks. `SteerInput::jump`
  (J) is new and writes `Move` as well as `Impulse`.
- **The seat** (`src/lab/demo/guest.rs`) sits on the queen seat's launcher
  (PR #904): one arm per launcher match, plus the guest arm of
  `seat_takes`, which takes B and E from the body's own bite and eat.

## What was measured

**The ladder has a middle at every gap** (`examples/guestprobe.rs`, steady
income grown to 30,000 frames, trained ants, the rove standing in the brood
pile, 6,000 frames, seeds 1, 2, 4, 5, 6, 7; before the rear-cell nip and the
three-cell body): at gap 0.2, 0 nips and about 30% of contacts antennate or
display; at 1.0, mostly displays and nips, killed on 3 of 5 seeds; at 2.0,
all nips, killed in 679-1,794 frames. Grown adults hold 2-5 grants, so there
is food to beg.

**Disguise is what keeps you alive** (`examples/guestgif.rs`, 12 seeds, 60 s
after landing at gap 1.5, the real seat driven by a script through
`Lab::demo_key`):

| | killed on | deaths | nips (median) | fed on | gifts |
|---|---|---|---|---|---|
| raw: walk in, press nothing | 11 of 12 seeds | 29 | 30.5 | 0 of 12 | 0 |
| beggar: soak, rub every 20 frames for 20 s, then beg every 1.5 s | 5 of 12 | 6 | 5.5 | 11 of 12 | 0-23 (up to 690 J) |

The beggar's verdicts fill every rung (seed 1: 65 walk on, 63 antennate, 28
display, 0 nips); the raw guest's are almost all nips. On seed 1 the rubs
take the gap from 1.5 to 0.12 in 20 s, and when rubbing stops it fades back
to 0.79 by 60 s and the nips return, which is the loop the seat is about.
Seed 8 met the colony only twice in both arms (the guest never reached the
ants), so it carries no information either way.

**The guest does not cost the colony anything you can see in a minute**:
colony food runs (`creature_stats.trip_deliveries`) paired by seed, raw
against beggar, sit within each other's noise (2,846/2,877; 2,313/2,405;
2,335/2,257; 3,009/2,929 ...). The beggar's 600 J is two-thirds of one
provisions cell against an income of 38,400 J per 1,000 frames.

**Frame cost.** Off is off: the default lab, every training run and every
other world never sets `World::visitors`, and the off-is-off guard below
hashes a trained colony cell for cell. In the demo the hooks are a scan of
the visitor list (one entry) per ant decision and one pass over the visitor
per tick. The warm-up to 30,000 frames takes 25 s wall on the 4-core session
box, against the plan's 15 s target on a laptop; `land=` sets an earlier
landing.

**Guards** (`cargo test --release --lib sim::creature::guest lab::demo`):
off-is-off (no ladder = an empty ladder = a ladder naming a dead id, over a
trained colony with a rove at its door; the positive control registers the
rove and differs, with rolls > 0); Attack never offered on a visitor and
still offered on an unregistered stranger (watched red with the gate
removed); the beg books out = in and the energy moves; a rub moves the two
smells 1 : 0.25; soak refused at 3.0 and taken at 1.0 (watched red with the
refusal removed); the odds are graded and never certain, with no nip below
0.8; a killed larva leaves a corpse worth its bank, booked once
(`sim::brood`); and the seat end to end: the body is the rove, registered,
1x at landing, R Y B reach the ladder and B not the bite, presses clear each
tick, and a respawn joins the same guest colony.

## What playing it changed

The first scripted GIF overturned three things the plan had right on paper:

- **The head went first.** A two-cell rove landing on a crowded mound lost
  its head to three nips inside a second, because the nip landed on
  whichever cell was touched. Now it lands on the rear cell and the body is
  three cells: nine nips from landing to death, a graded loss.
- **The soak never fired.** Six cells round the door's surface cell missed
  a guest standing on a grown mound. Now the nest is 16 columns either side
  of the door, from 16 rows above to 60 below, or anywhere beside brood.
- **The view was covered** by the lab's welcome card and the battle panel;
  the demo closes both.

## Open, and what the owner should judge

- **How it feels**, by eye: hold lengths (1.5 s antennate, 1 s display),
  how fast a raw guest dies, whether begging is too generous (a disguised
  beggar went from 200 to 700 J in a minute on seed 1).
- **The predator path is tested but not filmed**: killing a larva needs the
  guest beside brood in the chamber, which the script's walk does not reach.
- **The keeper seat** may reuse the ladder (`shared/scent-ladder.md` on the
  project board); the interface is `guest::register`, `World::visitors` and
  `guest::odds`.
- **Trained ants only.** The hand-built ant's own mouth and fight are not
  gated by the ladder, so the demo forces the trained colony.
