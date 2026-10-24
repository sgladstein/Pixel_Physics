# Which of the ant mind's senses are capped, flattened or on/off? (audit, 2026-10-24)

**Asked by the owner (2026-10-24 14:05Z)**, after the far-from-home sense (`PP_HOMEVEC=40:30`) turned out to read 0
inside 40 columns and stop at 1 past 70, which made trained ants turn home past 70 and starve when lab food is
farther: *"check all the senses to see if any others are limited by such caps or binarizing."* The owner prefers a
graded sense unless it is significantly worse and cannot be trained better. The far sense itself (context slot 9,
option feature 40) is the Teacher lane's: it is building the graded log-distance version, so it is left out here.

## The answer in four lines

1. **Scent A has a hard ceiling, and the nest sits on it.** Trail scent is stored as a 16-bit number that stops at
   its maximum. On the training bed, 30-33% of all decisions are made standing on cells pinned at that maximum,
   all within about 4-9 cells of home, and there 41-43% of the walk choices show a difference of exactly 0. Near the
   nest, trail A cannot say which way is in or out. In a lab colony that lives, it is 19% and 35%. A sense change
   cannot fix it, because the clamp has already thrown the gradient away. The deposit has to change.
   `PP_SCENT_SOFTCAP=1` (new, off) does that, and takes the ceiling share to 0.0%.
2. **The dig head's "when to dig" part only switches on within 40 columns of column 128.** That is the training
   bed's door, written in as a number. A lab colony lands at column 256 (or anywhere in the owner's 1024-wide box),
   so it never gets that read-out. `PP_DIGGATE_DOOR` (new, **on by default** at the Teacher lane's call; `=0` restores the old gate) measures from the nearest nest site instead. It is
   **bit-identical on the training bed** (events.csv hash equal, 10k frames). In a lab box with near food it gave
   alive at frame 46,000 of 144-159 → 199-200 (2 seeds, both up; too few to call).
3. **A nestmate's hunger is on/off.** Feature 33 is 1 below half a start grant and 0 otherwise. On the training bed
   it fired on **no option at all**, because every nestmate banked 7-60 tanks. In the lab it fires on 2-12% of
   options. `PP_HUNGRY_GRADED=1` (new, off) reads it as the ant reads its own bank, `1/(1+e)`, which takes 751
   distinct values where the switch had 1.
4. **Everything else is graded, or binary because the thing is binary.** The two soft cases worth knowing about
   are not caps. Home distance (slot 12, feature 39) is `squash(d/32)`, which flattens far out: 0.76 at 100, 0.82
   at 150. The energy sense is graded, but training ants live at 7-60 tanks, where it reads 0.88-0.98, while lab
   ants live at 0-3 tanks. Training never shows the ant the range the lab needs.

## How it was measured

`PP_MIND_LOG` (one row per decision: the 22 context slots, and every Walk, Dig and Drop option's non-zero
features), on `main` at 828d5ce6 with the shipped pool (`assets/minds/std_pool.txt`, 30 members) and every
standard switch at its default:

| Run | What | Decisions logged |
|---|---|---|
| bed | `inrun` training bed, standard line, seed 1, 60k frames, food 25 columns out (every 20th decision) | 34,974 |
| lab far | `labsurvive lab_far_mix grant=3`, seed 1: food 150 columns out, all 52 starve by frame 21,000 | 25,255 |
| lab bed | `labsurvive played_bed`, seed 1: the owner's planted box, 1 tank, all 42 starve | 7,972 |
| lab near | `lab_far_mix` with both heaps moved to 25 columns (`lab_near_mix`), `grant=3`, 40k frames after landing; the colony lives (every 20th) | 41,295 |

The histogram script and the scenario copy are in the project folder (`sense-caps/`), not the repository.

## Every input, one row each

`squash(x) = x / (1 + |x|)`. "Bed" and "lab" columns are p10 / p50 / p90 of the reading itself.

### Context (what the ant feels about itself, 22 slots)

| Slot | Formula | Bed | Lab (far / near) | Verdict |
|---|---|---|---|---|
| 0 energy | `e/(1+e)`, `e` = bank / sensed tank | .88 / .97 / .98 | .23 / .60 / .73; .48 / .78 / .86 | Graded, no cap. **Exposure gap**: training lives in the flat tail (p1 = 2 tanks), the lab in 0-3 tanks. Fix the bed (the far-food ramp does), not the sense |
| 1 crop | fill / capacity, 0..1 | 0 / .32 / .82 | — | Bounded by nature |
| 2 spoil, 3 at_nest, 19 roofed, 20 roof_cut, 21 hole_drop | 0/1 | | | Binary because the fact is (roofed's graded companion is depth) |
| 4-6 scent A, B, alarm | stored / 65,535 | A .47 / **.98** / **1.0** | A .04 / .22 / .54; .44 / .88 / **1.0** | **A is pinned at the u16 ceiling** for 30-33% of bed decisions (19% lab near). See *Scent A* |
| 7-8 Δ scent | `squash(4·Δ)` | ±.13 A, ±.41 B | | Graded |
| 9 (d_alarm) | far-from-home ramp | | | **Teacher lane's** |
| 10 Δ energy | `squash(10·Δ)` | | | Graded |
| 11 home_seen | 0/1 | | | Binary by design |
| 12 home_dist | `squash(d/32)` | max 51 cells | max 105-113 cells | No cap, but 0.76 at 100, 0.82 at 150: **flat far out**. The Teacher lane's log distance on slot 9 covers the far case; slot 12 can stay |
| 13 Δ home | `squash(Δd/2)` | ±.32 | ±.34 | Graded |
| 14 since_nest | `squash(n/200)` | .005 / .06 / .64 | .13 / .57 / .73 | Graded; lab inside training's range |
| 15 age | `squash(frames/20k)` | | | Graded |
| 16 self | fixed per ant | | | By design |
| 17 crowd | other animals' cells touching / 8 | | | Graded count |
| 18 depth | `squash(rows below datum / 16)` | −.27 / −.11 / .39 | | Graded |

### Option features (what each choice looks like, 45)

| Feature | Formula | Verdict |
|---|---|---|
| 0-8, 43 kind of cell | one-hot | Categorical |
| 9-17 scent there, Δ, Δ two further | level; `squash(32·Δ)` | Level: **A's ceiling, as above.** Δ is graded but steep: on the bed B's Δ reads beyond ±0.9 on a fifth of options (a gain of 32 squashes any step over 0.3). Low priority |
| 19-21 stand / fall / climb, 22 cut, 42 back, 44 roof_drop | 0/1 | Binary by nature |
| 23 hard | `squash(resistance)` | A material constant (3 values) |
| 24 wet | soil water / saturated, ≤1 | Graded |
| 25 up | sign of the row step | The target is one cell away, so the sign is everything |
| 26 depth | as slot 18 | Graded |
| 27 drop below, 28 headroom | open cells, up to 6, / 6 | **Capped at 6 cells** (two body heights). The engine has no fall damage, so a 6-cell and a 60-cell drop do not differ for the ant. Harmless today; revisit if falls ever hurt |
| 30-32 nestmate, stranger, laden | 0/1 | Laden could be the load's size; low value |
| 33 hungry | `1 − energy/start_energy > 0.5` | **On/off, and clamped at one grant.** Never fired on the bed; 2-12% of lab options. `PP_HUNGRY_GRADED` |
| 34 larva, 35 larva hunger | larva hunger graded 0..1 | Graded (read against the adult grant, so it sits at 0.40 for most larvae: graded but barely moving) |
| 37 crowd, 38 room | counts / 8; open / 24 / (1+near) | Graded |
| 39 home | `cos · squash(d/32)` | As slot 12: flat far out; the log far sense covers it |
| 40 | far ramp | **Teacher lane's** |
| 41 warmth | `squash(T there − T here)` | Graded. **Exposure gap**: ±0.003 on the bed (18 distinct values), ±0.06-0.67 in the lab, whose lamps make heat. A weight on it has never been tested against the lab's values |

### The dig head (12 features per Dig option, on by default with dh7)

| Feature | Formula | Verdict |
|---|---|---|
| 0 out, 1 down, 10 level | signs, 0/1 | By nature |
| 2 target depth, 7 own depth | rows below datum / 8, clamped to −1..3 | **Cap at 24 rows.** The bed reached 27 rows by 60k frames (under 1% of decisions). Watch it as nests deepen |
| 3, 8 light; 4 darkness | `0.5^cover cells`, cover ≤ 12 | Physically graded, but below about 6 cells of cover it is ~0: a dead zone underground. The depth features carry depth there. Low |
| 5 open neighbours / 8, 9 open share of 7×7 | | Graded |
| 6 door distance | columns / 16, ≤ 3 | Capped at 48 columns, but the head only matters near the door |
| 11 bottom sense | rows / 4, ±2 | Capped at ±8 rows. Low |
| **the "when" gate** | only within 40 columns **of column 128**, below the datum, jaws and crop empty | **Hard-coded door.** `PP_DIGGATE_DOOR=1` |

## Scent A

The deposit is `saturating_add` on a `u16` (`pheromone.rs`, `Plane::deposit`). Sixty-odd ants laying A round a
small nest pin it at 65,535 across the shaft and chambers. Measured on the bed, two seeds:

| | seed 1 | seed 2 |
|---|---|---|
| Decisions standing on A = max | 32.5% | 29.8% |
| ...of those, home distance p50 / p90 | 4 / 8 cells | 4 / 9 cells |
| Walk options with an A difference of exactly 0, when standing at the max | 43.1% | 41.3% |

The lab colonies that starved never got there (0.0%: too few ants laying, for too short a time). The lab colony
that lived (`lab_near_mix`) did: 19.1%, and 35.2% of its walk options there read flat. So it is a training fact
today, and a lab fact as soon as a lab colony thrives.

**Why a sense change cannot fix it**: a log or square-root reading of a cell at the ceiling still reads the
ceiling. The gradient is lost when the deposit is written. **`PP_SCENT_SOFTCAP=1`** scales every deposit by the room
left, `amount · (MAX − here) / MAX`, so a busy cell climbs toward the ceiling instead of sitting on it, and two
neighbours stay ordered by their traffic. It is one rule for every species that lays scent (owner: one physics for
every world), so it is a world switch, not a mind's.

| Bed, std pool (not retrained) | off s1 | on s1 | off s2 | on s2 |
|---|---|---|---|---|
| Decisions at the ceiling | 32.5% | **0.0%** | 29.8% | **0.0%** |
| Walk options with A difference 0 | 14.2% | 0.1% | 12.3% | 0.1% |
| Scent A p50 / p90 | .98 / 1.0 | .71 / .93 | | |
| Scent B p50 / p90 | .41 / .77 | .27 / .53 | | |
| Food cells delivered (`deliver`) | 4,262 | 3,226 | 3,828 | 4,501 |
| Dig cuts below the datum | 367 | 355 | 446 | 333 |
| Deaths | 5 | 6 | 7 | 1 |

It also lowers every trail's mid-range (B's median 0.41 → 0.27), and this pool's weights were selected on the old
levels, so food moved −24% on one seed and +18% on the other. **It needs training on top before it is judged**:
a Teacher lane arm, not a verdict here.

## The dig gate's column 128

`scratch_log::dig_prepare` adds the dig head's "when" read-out to the Dig job only for an ant that is empty, below
the datum and `(x − 128).abs() ≤ 40`. On the training bed the door is column 128. In `lab_far_mix`,
`lab_mid_fruit` and `played_bed` the colony lands at 256, and in the owner's 1024-wide box at four other columns,
so no lab ant has ever had it. `PP_DIGGATE_DOOR=1` uses the nearest nest site's column, which `dig_ctx` already
finds for the head's own door-distance feature.

| | Bed, 10k frames | lab near s1 | lab near s2 |
|---|---|---|---|
| off | events.csv `2163230…` | alive 159, lived 123.9 | alive 144, lived 124.4 |
| `PP_DIGGATE_DOOR=1` | **identical hash** | alive 199, lived 131.0 | alive 200, lived 129.3 |

Dig picks moved 776 → 930 and 1,944 → 1,855: no clear change. Two seeds is not a result. The useful facts are
that it is bit-identical in training (so turning it on cannot cost the training line anything), and that it is the
only reading under which "within 40 columns of the door" means what its comment says in a lab box. In the far-food
lab boxes it changes nothing (dig picks 36/32 → 34/38), because those colonies starve before any ant gets below the
datum.

## Nestmate hunger

`describe` sets feature 33 to `hunger > 0.5`, with `hunger = 1 − energy/start_energy` clamped to 0..1. That is one
bit, and it says nothing about a nestmate above one grant. `PP_HUNGRY_GRADED=1` reads `1/(1+e)`, with `e` the
nestmate's bank in its species' sensed tanks: 1 when empty, 0.5 at one tank, 0.25 at three, the mirror of the
ant's own energy slot.

| | off | `PP_HUNGRY_GRADED=1` |
|---|---|---|
| lab far s1: options with feature 33 non-zero | 6.5% (all exactly 1) | 31.5% (0.25 / 0.41 / 0.78 at p10/50/90; 751 values) |
| lab far, lived s1 / s2 | 10.6 / 10.6 | 10.5 / 10.7 |
| lab near, alive and lived s1 / s2 | 159, 123.9 / 144, 124.4 | 197, 125.5 / 201, 134.0 |

No harm seen, and nothing proven. On the training bed it can teach nothing until nestmates actually go hungry
there. The far-food ramp is what makes them hungry.

## What goes to the Teacher lane

`PP_SCENT_SOFTCAP` and `PP_HUNGRY_GRADED` are off by default; `PP_DIGGATE_DOOR` is on (bit-identical on the bed). Ranked by what they could buy:

1. **`PP_DIGGATE_DOOR`.** A bug fix more than a sense. It is bit-identical on the bed, so it went default-on
   (Teacher lane, 2026-10-24; owner rule: a gain or neutral switch ships on). `=0` is the old gate.
2. **`PP_SCENT_SOFTCAP=1`.** The one real cap left after the far sense. It moves every trail level, so it needs a
   trained arm (paired with the current line) before anyone judges it.
3. **`PP_HUNGRY_GRADED=1`.** Worth stacking once nestmates go hungry in training (the far-food ramp). Cheap to carry.

**Not proposed**: home distance (slot 12, feature 39), because the log far sense covers it. Drop and headroom,
because nothing hurts past 6 cells. The dig head's depth cap, until nests pass 24 rows. Also not proposed, but
worth knowing: **energy and warmth are exposure gaps, not caps.** Training never shows the ant a hungry bank or a
lamp's heat. The fix there is the bed, not the sense.
