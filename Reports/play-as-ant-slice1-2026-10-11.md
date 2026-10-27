# Play as one ant: slice 1, the first build you can play (2026-10-11)

*Copied onto main 2026-10-27 from PR #686, unchanged apart from this note; it predates the trained ant and describes the code of its date. The queen-mode code it describes (`lab::queen`, `examples/queenplay.rs`, `scenario=queen_start`) is not on main; it stays on the PR's branch, `claude/project-thread-14bobk`.*

**What this is.** The second slice of the owner's play-as-an-ant game: you
are a founding queen in the lab, you drive her with the keys, and every
other ant follows the shipped rules. Slice 0
([`play-as-ant-slice0-2026-10-11.md`](play-as-ant-slice0-2026-10-11.md))
built her body behind a switch and measured three numbers; this slice puts
her on the keyboard. The design is two Claude Docs linked from the slice 0
report.

**Where it sits.** Slice 1 is the first playable. The owner chose (card,
2026-10-11 17:39) to build it now, with the trail still not pulling workers,
and to trace the trail alongside (section 4). Slice 2 is time and touch:
danger dropping the dial to 1x, touching a worker to read her, hints, and
the player passing to a worker when the queen dies.

## 1. How to play it

```
cargo run --release --bin lab -- scenario=queen_start
```

Or pick **PLAY AS THE QUEEN** from the lab's scenario list. The box opens
paused; `Space` runs it.

| Key | Does |
|---|---|
| `W` `A` `S` `D` | steer her (screen directions: W is up a face or out of a hole) |
| `E` | eat what is in reach |
| `Q` | drop what the crop or the jaws hold |
| `F` | offer the crop to a nestmate |
| `G` | dig the cell she faces (steer into the ground and hold G) |
| `B` | bite |
| `L` | lay an egg, if her bank is over the egg bar |
| `T` | hold to lay the food trail |
| arrows, `1`-`7` | the speed dial, as in the lab; the keys drive her at 4x and below |

While the mode is on, those keys are hers: `WASD` no longer pans (the camera
follows her) and `E Q F G B L T` no longer reach the lab's tools and pages.
Every other key and the mouse are the lab's as before.

## 2. What was built

- **`queen_start.ron`**: the planted box (two herbs, the mister at its
  shipped default), one queen at column 256 with 900 J granted, and a food
  spot 60 columns east. No beetles.
- **Two scenario placements** (`lab::scenario`):
  - `Queen { species, x, reserve }` founds a colony of one, marks that ant as
    the steered body, and grants the reserve. A scenario with one puts the
    lab in queen mode.
  - `Larder { material, x, cells, cap }` piles `cells` like a `Heap`, but
    only while fewer than `cap` lie within 12 columns. Slice 0's
    steady-income heap had no cap and buried a lone queen on 6 of 24
    founding runs. The queen's spot tops up 20 cells every 1,000 frames to a
    cap of 60.
- **`lab::queen`**: the keys, the steering cap, the camera, the panel and
  three chronicle lines.
  - Each key is a held level, and each press is also latched, so a tap
    between two of her six-frame decisions still lands once.
  - At 4x and below the keys drive her; above 4x her genome does, and
    presses made at speed are dropped rather than saved.
  - The camera starts two zoom rungs in (x3, the food spot still in view)
    and follows her with the lab's dead-zone follow.
  - The biosphere page, open on every fresh lab, is closed on her first
    frame because it covers the panel.
  - The panel, top right, shows:
    - her bank against her egg bar (`creature::egg_bar_of`, the laying
      rule's own number);
    - her crop and what her jaws hold;
    - her colony's eggs, larvae, pupae and workers against the goal of 20;
    - who is driving;
    - which keys are down.
  - The chronicle gets `QUEEN LANDED`, `GOAL MET: 20 WORKERS` and
    `YOUR QUEEN DIED`.
- **Her colour**: the renderer draws the steered body magenta in every
  creature-colour mode (amber, the obvious choice, is colony 0 and the
  forager). The carry cue, which brightens a laden ant toward white, is
  skipped for her: a full crop washed her to pale pink.
- **`examples/queenplay.rs`**: slice 0's scripted player, driving her only
  through `Lab::queen_key` (the window's own call) and filming `Lab::draw`
  into a GIF.

**Not built, by the design's build order**: time dropping to 1x on danger,
touch-to-read, hints, the hand-off on her death (slice 2); the rival and
the result card (slice 3); known ground and the sniff view (moved to slice 2
by the design review). When she dies the mode stays on with nobody to steer,
and her colony keeps the queen rule, as in slice 0.

## 3. Checks

- **Off is off.** Main (c3bd4a1e) and this branch give byte-identical
  output for `deeptrace scenario=steady_income food=0 frames=40000 ants=2
  seed=3`: all seven files, `ticks.csv.gz` included. Every hook is one
  `Option` read when nobody is steered.
- **Queen mode comes from the scenario, not from any steered ant.** A
  harness that marks an ant itself and sets her input (`queenprobe`, the
  needs walk's steered test) keeps control of her. The first push let the
  mode's empty keys overwrite that input every tick, and CI's
  `the_steered_ant_is_not_walked` caught it.
- **Eight tests in `lab::queen`.** Four were watched red with the code
  broken (the fourth: the harness test above, with the scenario gate taken
  out):
  - the keys never reaching her (`feed` always `None`) fails the 4x test and
    the tap test;
  - the larder's cap taken out fails the cap test.
- **Scripted play through the keys** (`queenplay`, seed 1). Five trips to
  the spot, and the goal of 20 workers met at frame 11,761. That matches
  slice 0's scripted median of 11,400 at 900 J, driven there by setting her
  input directly. One seed: a check that the keys reach her the way slice 0's
  harness did, not a new founding number.
- `cargo clippy --all-targets --release --locked -D warnings` is clean, and
  the lab's 306 tests and the renderer's 133 pass.

## 4. Why the trail pulls nobody (traced alongside)

A second worker traced every naive worker in slice 0's trail experiment,
worker by worker. The full record is in the project's shared folder,
`game-design/trail-trace-2026-10-11/README.md`.

**The answer: her trail lands on a route the colony has already marked.**
By the time she turns for home, the colony's foragers have found the heap
and laid their own trail B along it. The step chooser's trail reading
saturates at a tenth of one deposit (`x/(1+x)` over `TRAIL_HALF`), so her
line adds almost nothing. Mean presence on the span just after her walk was
0.67 without her line and 0.79 with it.

- **The sense, the brain and the step all work.** B is sensed on 10,455 of
  10,658 naive on-trail decisions.
- **With the colony's own B silenced, her line doubles workers at food**
  (22 to 44 over 12 seeds). It beats both baselines on 7 of 12, against a
  bar of 9; the ramp does on 10 of 12.
- **Her line fades in 250-500 frames** after she gets home. A colony's
  trail lasts because every laden forager re-lays it.
- **Two losses cap any trail at about a fifth of naive workers:**
  - about 40% of them scout west, away from the food;
  - over half of those who do reach the trail are under a pull home (the
    nest worker's leash, or a digger walking back to its face), and a pull
    switches off the away-from-home term.

**What this means for the game.** In a real founding the player finds the
food first, which is closer to the silenced bed than to slice 0's bed.
Slice 0's gate could not have shown any trail working: even the ramp beat
the baselines on only 2 of 12 there. The trace suggests:

1. re-running the gate on `queen_start` itself;
2. a lasting deposit for the player's trail only (a slower fade on all of B
   is a recorded harm, in `Reports/dead-ends.md`);
3. a stronger or directional cue for her trail.

The trace's fourth suggestion, a rally verb, would cross the owner's
"trails only, no lead rule" decision.

**The first suggestion, run: the gate on `queen_start` itself** (`queenplay
trail=1` against `trail=0` and two nudged baselines, `delay=7` and
`delay=13`, 12 seeds, 24,000 frames). Distinct workers ever at the food
spot: 398 with her line, against 338, 351 and 370 for the baselines. That
is ahead of all three on only 4 of 12 seeds, against a bar of 9. She reaches
20 workers sooner than all three on 5 of 12 (median 10,561 frames against
11,251-11,371). So there is no effect clear of the spread here either. Her
first workers find the spot themselves within a few hundred frames of
emerging and lay their own trail. A one-frame nudge (`delay=1`) was
byte-identical to no nudge on every seed: she decides every six frames, so
the nudge has to cross a decision. Record:
`game-design/queen-trail-gate-2026-10-11/README.md`
in the shared folder.

## Reproducing

```
cargo run --release --bin lab -- scenario=queen_start
cargo build --release --example queenplay
RAYON_NUM_THREADS=1 ./target/release/examples/queenplay frames=12000 every=40 out=queen.gif still=11800
cargo test --release --lib lab::queen
```
