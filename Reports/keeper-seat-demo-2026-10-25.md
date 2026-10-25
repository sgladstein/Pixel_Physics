# The keeper seat demo (`demo=keeper`), 2026-10-25

*Owner's ask (2026-10-25 07:19Z): for each seat in
[`teaching-the-colony-2026-10-25.md`](teaching-the-colony-2026-10-25.md), an
adversarial feasibility review, an implementation plan, an independent review
of the plan, a revision, and a playable demo. This is the keeper (gardener)
seat's: the player is a hand over the formicarium, not an ant. The pictures,
the film and the chart are on an Artifact page:
<https://claude.ai/artifact/617nmHWDrw4ZKVrgPqLEsn>.*

## What it does

`cargo run --release --bin lab -- demo=keeper` opens a 512 x 320 box paused
at frame 6,000. The colony landed at the west end (column 107) on a founding
heap and has had time to dig a first chamber. Food arrives three ways:

- a ration inside the nest, 3 cells per 1,000 frames, below what the colony
  eats;
- a station ~280 columns east, kept topped up to 80 cells so it never runs dry;
- whatever the player adds with the lab's FOOD brush.

The player's verbs are the launcher's seat keys, so the lab's own bar and keys
are untouched:

| Key | Verb | What it does |
|---|---|---|
| `Y` | CARRY | Press on an ant, release where it goes (`creature::set_down`). Set down within 12 cells of food away from home, it remembers that food (`recall::plant`) |
| `U` | TREAT | A mouthful of provisions into the crop (`creature::treat`, booked as granted food); away from home it also remembers the place |
| `P` | LISTENING | The scripted gain on and off; the panel names the live arm |
| `J` | OVERLAY | Off, then who knows a taught place (white by hand, amber fed once, red fed twice or more, grey asleep), then traffic (where heads have been) |

A panel under the frame counter shows station visits, cells carried home from
the station, the first delivery's frame, how many ants know the station, the
player's carries and treats, and how many times a place was passed on.

## What the measurement says

Instrument: `examples/keeperdemo.rs`, which builds the box exactly as the
window does (`Lab::start_demo_with`, then the warm-up) and scripts the hand.
12 seeds, compared on the same seed, over the 24,000 frames after the demo
opens. The ants were the pool of PR #906.

**Carrying is a real verb on today's trained ant, graded by how many you
carry.** With listening off, compared against nobody carried:

| Ants carried | Station visits | Seeds up | Cells home | Seeds up | Ants alive at 30k (median) |
|---|---|---|---|---|---|
| 0 | | | | | 120 |
| 3 | −17 | 4/12 | −7 | 3/12 | 122 |
| 10 | +52 | 10/12 | +1.5 | 6/12 | 146 |
| ~17 (asked for 20) | +105.5 | 10/12 | +15.5 | 9/12 | 168 |

A carried ant feeds at the station, walks the food home, and walks back. Three
carried ants are inside the noise.

**The scripted listening adds to that, unevenly.** With the same carry,
listening on compared against listening off:

| Ants carried | Station visits | Seeds up | Cells home | Seeds up |
|---|---|---|---|---|
| 3 | +36.5 | 9/12 | +8.5 | 9/12 |
| 10 | +12 | 6/12 | +13.5 | 9/12 |
| ~17 | +89 | 9/12 | +3.5 | 8/12 |

No arm lost digging or survival. Digs rose with colony size, and starved
deaths after the opening stayed between 80 and 93 (median) in every arm.

**The meal copy moves nothing measurable.** With 10 carried and gain 6, copy
strength 0, 0.25, 0.5 and 0.75 gave the same station traffic. The number of
ants that "know" the station (strength × freshness ≥ 0.25) never rose past the
ants taught by hand. A fed place starts at half the giver's strength times its
freshness, and the next stray bite puts it to sleep. It is left on (0.5) as
neutral, and the HUD counts it.

**Untaught colonies find the station themselves.** Nobody taught, 12 seeds ×
30,000 frames: every colony lived (67 to 190 ants), and the first station
delivery came between frames 7,116 and 14,112 (median 11,517). The demo's
contrast is therefore graded, never 0 against something.

**Frame cost.** On the same colony the demo's own work costs nothing
measurable. Three seeds, 3 × 1,200 frames alternated per arm: the demo on ran
at 0.92 to 1.27 ms a frame, and with the hooks off at 0.88 to 1.15. A demo box
is slower than an idle one only because its colony is bigger. The traffic
overlay walks the whole grid each drawn frame, and only while it is shown. The
shipped lab with no `demo=` runs one `Option` check per tick.

## What is scripted and what is the ant

- **Carry and treat are physical.** A carry moves the body and reschedules its
  active site; a treat is a real meal. Neither writes behaviour.
- **Listening is the one labelled fake.** It runs in `recall::tilt` in
  `decide_planned`, after the job head has drawn Walk. Each Walk target's
  weight is multiplied by `exp(gain × strength × FOOD_WAY)`. It applies only
  to ants that were taught or fed the place, and only while they carry
  nothing. The trained pool cannot use `FOOD_WAY` itself yet, because its
  weight on the sense is zeroed on load (`mind::enter_last_food`). Off, the
  head is byte-identical to main (PR #907).
- **The warm-up hides the landing.** About 30 of 52 founders starve by frame
  3,000 on every landing tried, `nest_goal`'s own included. The queen seat's
  thread traced the same die-off to landing slots on `steady_income`.

## What tracing individual ants showed

These come from `keeperdemo trace=300`, which prints each taught ant's head,
crop, memory and record:

- **Taught ants spend 67 to 70% of their frames laden** (4 seeds), walking food
  home slowly, and the tilt never acts while they carry. The time that remains
  is split between empty with the lesson awake (4 to 10%) and empty with it
  asleep (19 to 26%).
- **When taught and empty, an ant chooses Walk in 97 to 98% of its
  decisions** (`RecallCounts::offered` against `tilted`). Walk share, the
  plan's first-named risk, was never the bottleneck. Between an eighth and a
  third of tilted picks (13 to 34%) changed the target chosen.
- **One stray bite used to erase a lesson for good.** A carried ant took a cell
  that a nestmate had dropped on the way home, and its record was pruned. Its
  next real bite at the station a thousand frames later taught it nothing.
  `recall::prune` now drops only the dead, and a lesson whose memory has moved
  elsewhere sleeps until the next bite at the taught place wakes it.
  `recall::SAME_PLACE` was also widened from 6 to 12, because the station heap
  spans about 9 columns either side.

## Corrections made along the way

- **"Carrying alone changes nothing" was wrong.** The first dose read used a
  scripted hand that asked for 10 ants and lifted about 6. The ants nearest
  the nest were stacked on one another, and `set_down` refuses a stack. The
  hand now picks the nearest ant it can lift (`creature::can_carry`). With
  true counts, carrying matters (table above).
- **The station memory first pointed at the box's ceiling.** It was placed at
  the first non-empty row, which is the lid. It now uses
  `creature::colony_surface` from below the lid.
- **The step-0 claim that "a hand trail evaporates in ~100 ticks" was a probe
  artefact.** The probe sat on a slumping heap. The lab's B decay is ρ 0.03 per
  12-frame pass, which leaves about a third at ~400 frames.

## Instruments

- `examples/keeperdemo.rs` (new):
  - the demo headless; `carry=N@F` scripts the hand;
  - `listening=0` gives the honest arm;
  - `trace=F` traces taught ants;
  - `png=`/`shots=` and `gif=` render through `Lab::draw`;
  - `costab=1` gives the hooks' frame cost on one colony;
  - `nodemo=1` runs the box with the hooks off.
- `examples/keeperlook.rs` (PR #907): step 0's look at a sketch, an alarm puff
  and a hand trail on the trained colony.

## Files

- `src/lab/demo/keeper.rs`, with one arm per hook in `src/lab/demo/mod.rs`.
- `assets/lab_scenarios/demo_keeper.ron`.
- `src/sim/creature/recall.rs`: the dormant-lesson prune, `SAME_PLACE` 12,
  and the `offered` counter.
- `src/sim/creature.rs`: `can_carry`; `set_down` now checks it;
  `is_home_for` borrows.
- `src/sim/creature/mind.rs`: counts `offered`.

The plan, its reviews, and the board the six seat threads shared are in the
project's `seat-demos/keeper/` folder: `review-feasibility.md`, `plan-v1.md`,
`plan-review.md` and `plan-v2.md`.
