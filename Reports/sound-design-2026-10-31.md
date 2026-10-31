# Sound for Pixel Physics — what it is for, and how to build it

**Status: stage 1 built** (2026-10-31, the owner's go-ahead on the
decision card): the lab box you can hear, as §11 stage 1 describes, with
what it measured in README `Sound status`. One change from this document:
the mix needed 12 dB of make-up gain after the master (`synth::MAKEUP`) --
the first render was -41 dB RMS, inaudible on a phone. Stages 2 and 3 are
still design. The text below is the design as approved, kept as written.

**Original status: design, nothing built** (2026-10-31). Asked for by the owner:
*"This is important, so let's think really deeply on how we do it before we
implement it."* No engine code changes until the owner approves stage 1
(§11). Listening sketches and diagrams for this document are on the
Artifact page linked from the thread; the sketches are synthesised from
made-up event streams, not recorded from the game, and are labelled so.

Claims are marked **[traced]** (read in the source at the cited line, on
`main` at b5769ea) or **[inferred]** (reasoned, not measured). No number in
this document comes from a run of the game; every cost is a budget to be
measured against, not a measurement.

**Revised after an independent code-checking review (2026-10-31).** It
found two outright errors (cpal's `realtime` feature pulls an MPL crate;
the lab's FLING throws one animal, not soil), and six holes: the
touched-chunk set is too coarse to tell sand falling from soil drying; the
dial modes must key on delivered speed, not requested; `notable` includes
the player's own actions; tool strokes do not go through `Lab::act`; the
determinism guard as first drafted did not test the seam; and a rebuild
resets every counter. Each is corrected in place below and listed in §15.

## 0. The answer in one paragraph

Sound in this game has four jobs: **answer the player's verbs**, **let you
hear the colony's state**, **make the box feel alive**, and **raise the
alarm when something is at stake**. Thousands of cells and ants are made to
sound good by never giving a cell or an ant its own voice: the listener
counts what is moving per region and per kind, and drives one *texture*
per kind whose density grows with the square root of the count, plus a
handful of close-up *grains* for what the player is touching, plus slow
*beds* for colony health. The time dial is a constraint unique to this game:
when many ticks run per displayed frame, per-event sound is meaningless, so
individual grains switch off at the engine's own motion crossover
(`reads_as_motion`) and the box switches to a "time-lapse" sound. The engine is untouched: a **read-only listener** takes
`&World` after each displayed frame and sends commands to a synthesiser on
its own audio thread. Training, CI and the VM never construct it, and on
Linux they never even compile the audio-device crate. Everything is
**procedural first** (synthesised from the material and the event, no
sample licensing), with a small CC0 or self-recorded sample set reserved for
a few hero sounds if synthesis cannot carry them. Library: **`cpal` plus our
own small voice bank** (~1.5k lines), with `kira` as the named fallback.
Stage 1 is one lab box you can hear: three tool verbs, the colony murmur,
and the line-ended alarm, plus an offline `soundtrack` example that renders
any run to a video with sound for an Artifact page.

## 1. What sound is for in this game

### 1a. Four jobs, in priority order

| Job | What the player gets | The law it serves | Example |
|---|---|---|---|
| **Verb feedback** | Every tool and every body verb answers instantly, and *what it touched* is audible | Law 2: a verb must deliver something | water on dry soil hisses; water on rock splashes; a dig bites grit |
| **Colony state you can hear** | Health, mood and work of the colony without opening a page | Law 1: graded, not binary | a thriving nest murmurs densely; a starving one thins out and slows before the census shows it |
| **World ambience** | The box is alive where it moves and quiet where it has settled | "Looks good in motion" | sand trickles while it falls, water runs while it flows, fire crackles |
| **Stakes and alerts** | Something important happened, possibly off-screen | Stakes the owner's review found missing in all three games | a line ending, an enemy in the nest, a collapse coming |

**Sound carries information that pixels cannot**, which is why it matters
more here than in most sims: it reaches off-screen, it needs no screen space,
and it is processed while the eyes are busy (`design-load-telegraph.md` §2
made this argument for structure; it generalises). The owner's repo review
(10-29) named "no stakes in any of the three games"; the cheapest stake a
player can feel is hearing their colony go quiet.

### 1b. Principles, each from this project's own history

1. **Sound comes from the simulation, not from the gesture.** When the
   player pours soil, the click makes a small interface tick; the trickle
   lasts exactly as long as grains are actually falling, and when the
   lab's FLING throws an ant, its landing thud comes when it lands. This is the sound
   version of the hinge dead end (`dead-ends.md`, `rigid.rs struct Hinge`,
   rejected because it *imposed* a motion instead of deriving it): an
   authored "explosion sound" on the button press would read the same way.
   Every world sound has a cause in the world.
2. **Graded, never binary** (law 1). A collapse is a few deep thuds, more
   cobble knocks and a lot of grit hiss, mixed from the actual fragment
   sizes. The colony murmur is a continuous density, not "busy/idle".
3. **Silence is information, so it must be earned.** A settled box goes
   quiet because nothing moves, not because a timer faded it. A faint room
   tone (lamp hum, about -40 dB) keeps "quiet" from sounding like "broken".
   A colony that has died is the silence the player should notice.
4. **Legible over exact.** No physical acoustics. A sound is right if the
   player can tell what happened and where, and it feels satisfying.
5. **It must never change the game.** Not one cell, not one RNG draw, not
   one millisecond of training time (§8).
6. **Judged by ear, on a page, paired.** Sound on vs off, same run, with
   event counts beside the video (§12).

## 2. What the engine already gives us

The listener needs no new simulation data for stage 1. Everything below
exists today.

| Signal | Where | What it answers | |
|---|---|---|---|
| The frame loop: one `frame()` per window pass, `advance` then draw | `src/bin/lab.rs:408`, `:469` | where the listener runs | [traced] |
| The time dial: presets `1,2,4,16,64,256,1024`; display rate falls to 10 Hz at the top | `src/lab/time.rs:740`, `:777` | the requested speed | [traced] |
| **Delivered** ticks per displayed frame, and the engine's own motion crossover (12 ticks) | `TimeControl::ticks_per_frame`, `reads_as_motion`, `achieved` (`time.rs:661`, `:667`, `:194`, `MOTION_TICKS_PER_FRAME` `:341`) | which sound mode. The request can be 6,144 ticks a frame (`requested_ticks_per_frame`, 60x1024/10 Hz) but the wall-clock budget caps what is delivered: the shipped bed reached 2.6-4.4x in September | [traced; the 2.6-4.4x figure is the reviewer's, from `evolution-lab-frame-cost-2026-09-01.md`] |
| Chunks touched since the last draw, consumed by the renderer | `World::take_touched_chunks`, `src/sim/world.rs:13200`; used at `src/lab/mod.rs:2448` | **which 64x64 chunks were awake at all** (see below) | [traced] |
| The run log (`Born`, `Died`, `FirstFeed`, `FirstSeed`, `LineEnded`, `GroupSplit`, `LineMilestone`, `LineRecord`, `PlayerAction`) with `recent()` and a total | `src/sim/world.rs:283`, `:914`, `:976` | alerts and milestones | [traced] |
| What the clock reacts to by default, and its 4 s cooldown | `default_react_on`, `REACTION_COOLDOWN` (private), `src/lab/time.rs:408`, `:357` | the alert set. **Not `notable`**: it includes `PlayerAction` (`is_line_event`, `world.rs:397-401`), which would sound an alert on the player's own clicks | [traced] |
| The graveyard: recent deaths with cause and position, capped at 2,048 | `Graveyard`, `src/sim/world.rs:~810-840` | where deaths happen, clusters of deaths | [traced] |
| Cumulative colony counters: `moves`, `falls`, `eats`, `pickups`, `digs`, `gnaws`, `store_delivered`, `recruited`, `water_steps` and ~100 more | `CreatureStats`, `src/sim/world.rs:1651` | colony-wide rates by diffing per frame | [traced] |
| Live organisms with state | `World::live_organisms`, `src/sim/world.rs:8135` | where ants are and what they carry | [traced] |
| The camera: `screen_to_world`, `world_rect_to_screen` | `src/render.rs:9419`, `:9473` | panning and distance | [traced] |
| The steered body in a seat demo | `World::steered`, `src/sim/world.rs:4349` (type in `steer.rs`) | the "close mic" anchor | [traced] |
| Lab tools: look, plant, colony, cull, soil, dig, water, release, wall, food, scent, fire, alarm, fling, lamp, prune | `Tool`, `src/lab/ui.rs:772`; strokes run `press`/`drag`/`release` → `use_tool` (`src/lab/mod.rs:2646`, `:2750`, `:3051`); `Lab::act` only arms a tool | the verb catalogue and where verb sounds hook | [traced] |
| FLING launches **one animal** | `fling_at`, `src/lab/mod.rs:3145` → `creature::launch` | the lab has no debris-throwing verb; debris is the sandbox's | [traced] |

Four shapes matter for the design:

- **The touched set is taken, not peeked.** The renderer consumes it in
  `Lab::paint` (shared by `draw` and `draw_layered`), which runs only on
  passes that draw; the set accumulates across undrawn passes, and while
  paused it is empty because `end_step` never runs. The listener reads it in
  `paint`, by reference, never takes it. [traced by the reviewer]
- **The touched set is coarse.** A chunk enters it if it was unsettled in
  any tick since the last take, and moisture writes insert too
  (`world.rs:12294`, `:13506`); every ant step wakes its chunk. So it says
  "something changed in this 64x64 square": soil drying, a root growing and
  sand falling all look the same. The shipped bed keeps about half its 40
  chunks awake, and `labdial` found under 0.16% of cells change material
  between displayed frames even at 256x (`Reports/instruments.md`). **What
  that means for sound:** in the lab, the colony is most of what moves, so
  the colony is most of what should be heard; world textures (sand, water,
  fire) fire mainly where the player pours or lights something. Telling
  them apart needs a strided snapshot diff inside touched chunks (§3b).
  [traced by the reviewer; the 0.16% figure is quoted, not re-measured]
- **A rebuild or chamber switch replaces `Lab::world`**, and every
  cumulative counter, the run log and the graveyard start again from zero
  (`world.rs:7530`). The listener must detect a new world and re-baseline,
  or its diffs go negative or burst. It diffs `RunLog::pushed(kind)`
  (cumulative per kind, `world.rs:952`), not `recent()` over the merged log.
  [traced by the reviewer]
- **Several logs are `Option<Vec<…>>` that grow** (`cut_log`, `bite_log`,
  `death_log`, `src/sim/world.rs:4371-4397`). The listener does not switch
  them on: a growing log in a long session is a memory leak, and draining it
  needs `&mut World`. Positional digs and bites, if stage 2 wants them, get a
  small fixed ring (§6c). [traced for the shape, inferred for the leak]

## 3. The hard problem: thousands of sources and a fast clock

### 3a. Why one sound per event fails

An ant colony of 1,000 makes thousands of steps a second; a sand pour moves
thousands of cells a frame. One voice per event is (a) a white-noise wall,
(b) a CPU cost that grows with the box, and (c) meaningless: the ear cannot
count past a few events at once. The run log's own doc already measured the
flood for *notable* events (3,099 seeds against 15 births in 90,000 frames,
`world.rs:275-281`); footsteps are orders of magnitude denser. The sketch
"naive vs designed" on the page plays both. [traced for the seed figure]

### 3b. Three tiers

| Tier | What | How many at once | Driven by | Example |
|---|---|---|---|---|
| **Grains** | single short sounds with a place | ≤ 16 voices total, ≤ 4 per kind | discrete events near the camera or the player's body | a tool stroke landing, the steered ant's bite, a food find |
| **Textures** | one granular cloud per kind of activity | one voice per kind, ~8 kinds | counts per kind per region, every displayed frame | colony walking, digging, sand falling, water running, fire |
| **Beds** | slow continuous tones | 2-3 | colony-wide state, smoothed over seconds | colony health, room tone |

**Textures are the trick.** A texture voice is a grain scheduler on the
audio thread: it fires short grains at a rate the listener sets, each grain
jittered in pitch, length, filter and pan from the audio thread's own RNG.
One voice can sound like three ants or three thousand. Its parameters are
just `rate`, `level`, `pan centre`, `pan spread`, `brightness`, `muffle`.

**Loudness follows the square root of the count, rate follows the count up
to a ceiling.** Ten ants and a thousand ants should differ by about 10x in
perceived busyness and ~20 dB at most, not 100x and 40 dB. Grain rate rises
with the count until grains overlap (~200/s), after which only level and
spectral thickness rise. [inferred; the exponent is a first guess to tune
by ear]

**Space comes from the regions.** The listener bins the visible area into the
chunk grid (64-cell chunks, `src/sim/chunk.rs:11`; 8x5 on the shipped
512x320 bed). Each texture's pan centre is the count-weighted mean x of its
bins, its spread is their spread, so a nest on the left sounds on the left,
and a pour across the whole box sounds wide. [traced for chunk size]

**Where the counts come from.**

- *Ants:* the colony counters (`CreatureStats`) are world-wide, so where the
  activity is comes from the ants themselves: the listener keeps each ant's
  previous head cell (`chain.first()`) and counts, per bin, ants that moved,
  ants at a soil face, and ants carrying. That is one pass over live
  creatures per displayed frame. [inferred cost]
- *Cells:* the touched set only says which chunks woke. To tell falling sand
  from drying soil, the listener keeps a strided snapshot of material ids
  in each touched visible chunk (every 4th cell each way, 256 reads per
  chunk) and counts samples whose material changed, by class. On the
  shipped bed that is ~20 awake chunks x 256 = ~5k reads per displayed
  frame; a hard cap on reads per frame bounds it on a bigger box. This is
  stage 2; stage 1 needs only the ants. [inferred]

### 3c. Level of detail by zoom and depth

- **Zoom is the listener's distance.** Zoomed in: grains on, textures
  detailed. Zoomed out: grains off, textures only, slightly duller. Same
  idea as a visual LOD.
- **Underground is muffled.** A source below the surface line gets a
  low-pass proportional to depth of soil above it; the nest is a warm
  rumble from the surface and crisp when you are in it with a seat body.
- **Off-screen is quieter and duller, not gone.** Within one screen of the
  edge, attenuated and low-passed; beyond that, only alerts. That is what
  lets sound warn about what you are not looking at.

### 3d. The time dial

Modes key on what the loop **delivered**, `ticks_per_frame()`, never the
requested preset: a box asked for 1024x that manages 4x should sound like
4x.

| Delivered ticks per displayed frame | Mode | What plays |
|---|---|---|
| 0 (paused) | **Hush** | world textures fade over ~0.3 s; room tone stays; verb sounds still answer, fired from `use_tool` (the touched set is empty while paused) |
| below the motion crossover (`reads_as_motion`, 12 ticks) | **Real time** | all three tiers; grains for single events |
| 12 to ~100 | **Time-lapse** | grains off; textures driven by *per-tick* activity so a busy box is not denser for running faster; slightly brighter and faster grain envelopes, the "fast-forward" signature |
| above ~100 | **Overview** | textures blur into the beds; only alerts punctuate, rate-limited by wall clock |

Reusing `reads_as_motion` means the sound switches at the same point the
engine already decides a displayed frame stops reading as motion. The ~100
boundary is a first guess. [traced for the crossover]

The key rule: **texture density is computed per simulated tick, alert rate
per wall-clock second.** Diffing a cumulative counter between displayed
frames gives events per frame; dividing by `ticks_per_frame()` (or the
`world.frame` difference) gives the colony's activity, which does not depend
on the dial. Alerts get their own wall-clock spacing like the clock's 4 s
`REACTION_COOLDOWN` (`time.rs:357`, private, so the sound keeps its own
constant). [traced]

**Alerts are graded by what they cost the colony, not by kind.** At 1,000+
ants a line can end every few frames (`time.rs:351`), so "a line ended" every
4 s would stop meaning anything. The alert's weight is the share of the
living colony the event removed: one of 52 lines ending is a soft tick, the
last three lines is the full two-note fall, and a colony's end is its own
sound. Below a threshold, line endings fold into the colony-health bed
instead of playing at all. [inferred]

### 3e. Keeping it clean

- **Smoothing with hysteresis.** Control values rise fast (~50 ms) and fall
  slow (~400 ms), so a count that jitters frame to frame does not flutter.
  At 10 Hz display the audio thread interpolates per sample, so no zipper
  noise.
- **Priority and voice stealing.** Each grain request carries a priority:
  loudness x proximity x novelty (a kind not heard for a while wins).
  Over the cap, the quietest playing grain is stolen.
- **Variation, never repetition.** Every grain is jittered; sampled sounds,
  if any, rotate through variants. The machine-gun effect (the same sound
  twice in a row) is the most common reason game audio sounds cheap.
- **Ducking.** Alerts duck textures by ~6 dB for a second; verb feedback
  ducks them by ~3 dB for a stroke.
- **One limiter at the end.** Buses (verbs, colony, world, alerts, room)
  each get a level; a master soft limiter keeps a big collapse from clipping.
- **Habituation, so hour-long sessions do not tire the ear.** A texture
  whose input has been steady for ~30 s eases down a few dB and drifts
  slowly in timbre; a change brings it straight back. That makes the sound
  report *change*, which is what the player needs, and keeps a constant
  murmur from becoming wallpaper. [inferred; tune by ear]
- **A sensible first-run level.** Master starts at about -12 dB below full
  with the colony bus lowest, so the first launch is never loud; the level
  is remembered between sessions.

## 4. The sound catalogue, first pass

Stage numbers refer to §11. "Signal" is what the listener reads.

| Source | Signal | Tier | Sound (procedural recipe) | Stage |
|---|---|---|---|---|
| Interface tick on any tool press | `use_tool` (strokes), `Lab::act` (arming) | grain | soft wooden tick, 5 ms | 1 |
| Water tool | stroke position + material under it | grain + world texture | hiss on dry soil, splash on rock, drips as cells land | 1 |
| Soil / sand tool | stroke + falling powder census | texture | granular trickle, settles to silence | 1 |
| Food tool | stroke | grain | soft thunk | 1 |
| Ants walking | live creatures in view, moving | texture | tiny bright clicks (bandpassed noise, 3-8 ms) | 1 |
| Ants digging | `digs` diff + ants at soil faces | texture | gritty scrapes (rough noise, 20-40 ms) | 1 |
| Ants carrying / delivering | `pickups`, `store_delivered` diffs | texture tint | softer, lower patter; a faint tock on delivery at the store | 2 |
| Line ended | `RunLog::pushed(LineEnded)` diff, weighted by share of the colony lost | alert (graded) | soft tick for a small line; low two-note fall when much of the colony goes | 1 |
| Group split, milestones | run log line events | alert | rising two notes (milestone), a split chime | 2 |
| Death clusters | graveyard positions over a window | grain / alert | single deaths silent; a cluster gives a dull tap per death and a low swell | 3 |
| Colony health | living count, eats vs digested trend, store level | bed | warm drone whose fullness tracks food; thins and drops a step when starving | 3 |
| Alarm pheromone | alarm field near ants, `stranger_alarm` | texture | sharp chirps that thicken with alarm level | 3 |
| Enemy (centipede) | steered body / creature kind | grain + texture | skittering legs; crunch on bite; the colony's alarm rises around it | 3 |
| Falling powder | touched chunks, powder cells moving | texture | sand hiss | 2 |
| Flowing liquid | touched chunks, liquid cells moving | texture | running water: filtered noise + drip bubbles | 2 |
| Rain | weather state | texture | rain on soil vs rain on water | 2 |
| Fire | burning cells in touched chunks | texture | crackle (random impulses) + roar bed by size | 2 |
| Fling (lab: throws one animal) | `use_tool` + the animal's landing | grain | whoosh at the throw, a small thud when it lands | 2 |
| Thrown or blasted debris (sandbox) | explosions, debris landing | grain cloud | whoosh, then thuds where and when pieces land | 4 |
| Cull / prune | tool stroke on tissue | grain | snip / soft crunch | 2 |
| Plants | growth is silent; leaves rustle only where something moves them | (texture) | — | 4 |
| Lamp / day | lamp state, `noon_equivalent_light` | bed | a lamp hum that is the room tone | 1 |
| Seat bodies | `World::steered` | grains, close-miked | your body's steps louder and centred; each seat's verbs get a grain (the seat-demo key layer must pass the SOUND key through) | 4 |
| Collapse (outdoor/sandbox) | structural failures, fragment sizes | grains + texture | graded: thuds by large pieces, knocks by cobbles, hiss by grit | 4 |
| Creak (load telegraph) | `torque / capacity` (`design-load-telegraph.md`) | texture | creak density and pitch by load ratio | 4 |

**What stays silent on purpose:** ordinary births and deaths one at a time
(they happen every few frames at 1,000 ants, `time.rs:360-366`), plant
growth, gas diffusion, anything settled. [traced for the birth/death rate
comment]

## 5. Procedural or sampled

**Recommendation: procedural first, a small sample set allowed later for a
few hero sounds.**

| | Procedural (synthesised from parameters) | Sampled (recorded audio files) |
|---|---|---|
| Fits density-driven textures | yes: every grain differs, any rate | needs many variants and still repeats |
| Fits material-aware feedback | yes: material picks filter and pitch | one file per material x action |
| Realism ceiling | good for clicks, grit, hiss, drips, crackle, thuds; weaker for voices and complex textures | as good as the recording |
| Licensing | none | every file needs provenance (§10) |
| Size | code only | MBs |
| Offline render and tests | deterministic, so a run's soundtrack can be hashed | deterministic too, but files must ship |
| Style control | one knob changes everything | re-record |

Most of this game's sources are small, granular and numerous — exactly
where synthesis is strongest. The weak spots are big hero moments (a large
rock crack, a tree falling) and anything with a voice. Those are stage 4
and may want a few samples.

**Style is the owner's call** and the page plays a blind sketch of three:
natural foley, musical (events tuned to a scale, the colony as a soft
generative tune), and a hybrid (natural world, musical alerts and
milestones). The design does not depend on the choice: all three are the
same listener and voice bank with different grain recipes. The key to the
blind letters is in §13.

**Music** (a score) is out of scope until stage 5 and is the owner's call.
If it comes, it should be generative and driven by the same beds, so the
colony's state is the score.

## 6. Architecture

### 6a. The shape

```
 sim (unchanged)                 game thread                       audio thread (real time)
 ───────────────                 ───────────                       ────────────────────────
 World ──advance()──▶ Lab::paint ┬─▶ renderer.draw(&world, &touched)
                                 └─▶ Listener::observe(&world, &touched, view, dial)
                                          │  read-only, own RNG
                                          ▼
                                   SoundCmd ring (lock-free, fixed size) ──▶ VoiceBank: grains, textures, beds
                                                                              buses ─▶ limiter ─▶ device (cpal)

 offline: examples/soundtrack.rs runs the same Listener + VoiceBank, writes WAV, muxes with a filmstrip GIF
```

- **`Listener`** (pure Rust, no dependencies, in `src/audio/listen.rs`):
  takes `&World`, the touched set by reference, the camera rectangle and the
  dial; keeps its own previous counter values and its own `Rng`; returns a
  small `Vec<SoundCmd>` (capacity reused). Because it only ever holds
  `&World`, the borrow checker proves it cannot write the world. The two
  ways it could still change a result are a shared RNG (it must have its
  own) and interior mutability inside `World` (none is reachable through
  the accessors it uses — to be confirmed by the guards in §6d).
- **`VoiceBank`** (pure Rust, `src/audio/synth.rs`): fixed arrays of grain,
  texture and bed voices; no allocation after start; parameter ramps per
  sample; a small feedback-delay reverb for "inside a box"; a soft limiter.
  Pure function of its command stream and its seed, so the offline render
  is the same code as the live one.
- **`Device`** (`src/audio/device.rs`, the only file that touches `cpal`):
  opens the default output, runs `VoiceBank::render` in the callback,
  drains the command ring. Compiled only where an audio feature is on
  (§8).
- **The game thread never waits on the audio thread** and the audio thread
  never takes a lock. If the ring is full, the oldest-priority commands are
  dropped on the game side. If the game stalls, textures hold their last
  values and decay gently, so a hitch does not glitch the sound.

### 6b. Where it hooks in

- **Lab:** one call in `Lab::paint` next to `renderer.draw` (`src/lab/mod.rs:2448`),
  so it sees the same touched set; verb sounds from `use_tool`, where every
  stroke from keys, mouse and bar ends up (`Lab::act` only arms a tool). `bin/lab.rs` owns the
  `Device` and passes it in, keeping the seam the file documents ("this file
  knows winit and pixels exist and nothing else does").
- **Sandbox and held world** later, through the same `Listener`; one set
  of physics, one set of sounds.
- **Seats:** a demo marks the steered body as the close-mic anchor; seat
  verbs are grain requests from each seat's own file.

### 6c. The only simulation edit, and only if stage 2 needs it

Positional digs, bites and structural breaks are not readable from state
without growing logs. If stage 2 needs them, `World` gets `pub sound_tap:
Option<SoundTap>`: a fixed ring of `(kind, x, y, frame)` with a monotonic
sequence number, pushed only at discrete-event sites (a cut, a bite, a
break — never per-cell movement), and read by the listener by sequence
number through `&World`. `None` in every training, test and headless run, so
the cost there is one branch per event site. It must pass the guard in
§6d with the tap on. `world.rs` is a contested file (`CLAUDE.md`); this edit
lands alone and quickly.

### 6d. Guards that make "never changes the game" a test, not a promise

The review confirmed no interior mutability is reachable through `&World`
(no `Cell`, `RefCell`, `Mutex` or atomic in `World` or `Chunk`;
`Chunk::nutrient_deficit` is a pure closed form; the process-global atomics
are written only from `&mut` sim passes). So the realistic way sound could
change the game is a *later* edit: the listener's signature growing to
`&mut World`, a cache added to `World` for its benefit, or the stage-2 tap.
The guards target those.

1. **The seam is read-only by type:** `Listener::observe` takes `&World`,
   and a compile-time test pins that signature, so widening it is a
   deliberate, visible change rather than a drift.
2. **Same-world test through the real seam:** the lab driven the way the
   game drives it (`Lab::tick_for_harness` plus `Lab::draw`, so the touched
   set is taken exactly as in play), run twice with and without a listener
   observing every drawn frame, and with the tap on once it exists; the
   checkpoint hashes must match. **Positive control:** the same harness
   with one extra `world.rng` draw per frame, run long enough that the hash
   diverges, shows the hash can see a single-draw leak at that run length.
   This proves the instrument's sensitivity; the type test proves the seam.
   (`tests/determinism.rs:367`'s bed calls `frame::step` and never takes the
   touched set, so it cannot stand in for this.)
3. **Training cannot reach it:** `cargo tree -e normal --target
   x86_64-unknown-linux-gnu` shows no `cpal` in the default build (§8), and
   no `examples/train*`, `inrun`, `labsurvive` or batch path names
   `audio::`. A grep check in `scripts/` makes this permanent.
4. **Offline render is deterministic:** the same run renders the same WAV
   hash twice; a change in hash says the listener or synth changed.
5. **Level sanity:** the offline render's peak stays under 0 dBFS and its
   loudness over a busy scene stays within a band, so a change that makes
   the colony a wall of noise fails a check rather than a playtest.

## 7. Rust audio libraries

Checked on crates.io 2026-10-31. [traced from the registry API]

| Crate | Version, licence | What it is | Fit |
|---|---|---|---|
| **cpal** | 0.18.2, Apache-2.0 | cross-platform audio device I/O; everything else sits on it | **Use.** Thin, standard, CoreAudio on Mac with no system deps |
| **kira** | 0.12.6, MIT/Apache | game audio: mixer tracks, effects, tweens, clocks, spatial, custom sounds, real-time-safe command queue | **Fallback.** Strong, but most of its value (sample playback, streaming, clocks) is not what this game needs; its decoders pull symphonia (MPL-2.0) by default, so it must be `default-features = false` |
| rodio | 0.23.0, MIT/Apache | simple playback and source chaining | weaker parameter control and mixing for textures; default decoders are symphonia (MPL) |
| fundsp | 0.23.0, MIT/Apache | DSP graph library for synthesis | good building blocks, heavy generic types and compile time; we need ~6 recipes, not a graph language |
| firewheel | 0.14.0, MIT/Apache | Bevy's new audio graph engine | young, Bevy-centric |
| oddio | 0.7.4, MIT/Apache | small real-time spatial mixer | last release 2023 |
| symphonia | 0.6.1, **MPL-2.0** | decoders (mp3, ogg, flac, wav) | **blocked** by `scripts/licensecheck.sh` (not in `about.toml`'s accepted list) |
| hound | 3.5.1, Apache-2.0 | WAV read/write | **Use** for the offline render and any WAV samples |
| lewton | 0.10.2, MIT/Apache | Ogg Vorbis decoder | if samples ever ship compressed |
| rtrb | 0.4.1, MIT/Apache | lock-free single-producer ring | **Use** for the command ring (kira uses it too) |
| FMOD / Wwise | proprietary SDKs | industry middleware | no: closed SDK, licence terms, C bindings and CI setup out of proportion to a granular sim |

**Why our own voice bank over kira:** the sound of this game is ~8
procedural texture voices and a few grains, all parameter-driven; that is
about 1.5k lines we would write anyway as custom kira `Sound`s. Owning it
keeps the dependency count at the project's habit (seven runtime crates
today), keeps compile time down, and keeps the offline render trivially the
same code. The switch condition is stated: **if stage 2 finds us writing
our own mixer tracks, effect sends or tweens beyond simple ramps, move to
kira** — it does all three well.

## 8. Build, CI, headless runs and training

**The constraint, found while reading this tree: cpal on Linux needs the
ALSA development package to build, and neither this cloud container nor
CI's `ubuntu-latest` jobs install it** (`pkg-config alsa` fails here; no
`apt-get` step in `.github/workflows/ci.yml`). [traced] An unconditional
`cpal` dependency would break every agent session's build, every CI job,
and the VM's training build.

**The design:**

- `cpal` is a **target-specific dependency for macOS and Windows** (where
  it needs no system packages), so the owner's `cargo run --release --bin
  lab` on a Mac has sound with no flags — the defaults-on rule.
- On Linux it is behind a feature, `audio-alsa`, off by default. Linux
  players opt in; CI, cloud sessions and the VM never compile it.
- `Listener` and `VoiceBank` have no dependencies and are compiled and
  unit-tested everywhere, including CI. Only `device.rs` (~150 lines) is
  platform-gated.
- **One new CI job** installs `libasound2-dev` and runs `cargo clippy
  --features audio-alsa`, so the device file is checked on every push, and
  runs `cargo check --target aarch64-apple-darwin` and `--target
  x86_64-pc-windows-msvc` for the lab binary. There is no macOS runner, so
  without the cross-check the owner's own default sound path would be
  compiled nowhere in CI; cpal's Mac and Windows dependency sets are pure
  Rust bindings, so `check` needs no SDK. [inferred; to be proven in stage 1]
  It runs in parallel with the existing jobs (`ci-wall-time` memory: ~15-18
  min, no single long pole).
- **CI already builds `--bins` on Linux**, so the lab binary must compile
  with the device code gated out; a missing `cfg` breaks the main suite, not
  just the new job. [traced by the reviewer]
- **Adding cpal changes `Cargo.lock`**, so `THIRD-PARTY-NOTICES.txt` must be
  regenerated (`scripts/notices.sh`) in the same change or `notices.yml`
  fails. [traced by the reviewer]
- **Runtime fallbacks:** no output device (a VM, a container, a headless
  screenshot run) means the lab runs silently and logs one line; a `SOUND`
  button on the bar (key `\`: unbound in `bin/lab.rs`, and not F11, which is
  macOS "Show Desktop") mutes and cycles volume. The bar has no spare
  pixels today (`bin/lab.rs:834`), so the button needs a cell found for it; `sound=off` on the command line and
  `PIXEL_PHYSICS_SOUND=off` disable it for screenshot hooks and CI-style
  runs.
- **Training cost: zero.** No training or batch binary constructs a
  `Listener`; on Linux they do not compile `cpal`. The audio modules add a
  few seconds of compile time to the library and nothing at run time.
  Guard 2 (§6d) keeps it true.

## 9. Performance budget

Frame cost of the shipped bed for reference: tick 7.3 ms, draw 4.7 ms
(`src/bin/lab.rs:483-485`, citing `Reports/evolution-lab-frame-cost-
2026-09-01.md`). These date from September; the same report later shows
tick 3.8-6.4 ms. To be re-measured in stage 1. [traced]

| Cost | Budget | Why it should fit |
|---|---|---|
| Listener on the game thread | ≤ 0.25 ms per displayed frame (~2%) | stage 1: one pass over live creatures, ~20 counter diffs; stage 2 adds the strided chunk snapshot (~5k reads on the shipped bed, hard-capped) [inferred] |
| Voice bank on the audio thread | ≤ 5% of one core | ~32 voices x tens of operations per sample at 48 kHz is ~100 M simple ops/s [inferred] |
| Latency, verb to ear | ≤ 30 ms at 1x | 512-frame buffer (10.7 ms) + up to one display frame |
| Memory | < 2 MB procedural; < 10 MB with a sample set | |
| Training, CI, VM | **zero** | §8 |

Three real risks. At high speed the sim's rayon pool saturates every core,
so the audio callback must keep real-time priority. CoreAudio gives its
callback thread that by default. **On Linux, cpal's `realtime` feature is
off-limits**: it pulls `audio_thread_priority`, which is MPL-2.0 and fails
`licensecheck.sh`, and because `Cargo.lock` resolves every feature, even an
opt-in feature would put it in the lockfile. Linux players get normal
priority, or our own small priority call later. The dirty-rect render skip
must survive: the listener reads the touched set by reference and never
marks anything dirty. And the listener's time feeds the lab's `overhead`,
which sets the tick budget (`time.rs:536`), so a slow listener lowers the
delivered speed of the live box (never its results). `ascii` runs the
sandbox, not the lab, so stage 1 times the listener with `labdial rate` or
`lab_cost` and prints it beside their frame figures.

## 10. Assets and licensing

The engine is proprietary and the repository is public
(`Reports/dependency-license-audit.md`). Both matter for sound.

- **Code dependencies** go through `scripts/licensecheck.sh` as now. All
  recommended crates are MIT, Apache-2.0 or both; symphonia (MPL-2.0) is
  kept out.
- **Sound files are allowed only with a provenance row** in
  `assets/sounds/SOURCES.md`: file, source URL or "recorded by", author,
  licence, date, and any modification. Accepted: **CC0**, **recorded by
  the owner or by us**, **commissioned with a written transfer**. CC-BY
  only with an attribution line added to `THIRD-PARTY-NOTICES.txt`. Not
  accepted: CC-BY-NC (the game is paid), CC-BY-SA (share-alike), "free for
  personal use", and anything without a licence.
- **Royalty-free packs that forbid redistribution of the raw files** (for
  example the GDC bundles) can ship *inside a built game* but **cannot be
  committed to a public repository**, which is redistribution. Until the
  repository is private, they are out.
- A small script, like `licensecheck.sh`, fails if any file under
  `assets/sounds/` lacks a row or carries an unaccepted licence. CI runs it
  when that folder changes.
- Procedural recipes are our own code and need none of this, which is a
  real argument for procedural first.

## 11. Staged plan

Each stage ends with a page the owner listens to, sound on vs off, same run,
with the counts beside it.

**Stage 1 — one box you can hear (the first prototype).** Lab only.
- `src/audio/{listen,synth,device}.rs`, the hook in `Lab::paint`, the bar
  button, the platform-gated `cpal`, the CI job.
- Sounds: interface tick; water, soil and food tool feedback; the colony
  murmur (walking and digging textures, panned, underground muffled); the
  line-ended alert; the lamp hum room tone.
- Time-dial modes (hush, real time, time-lapse, overview).
- `examples/soundtrack.rs`: render any lab run offline to WAV and an MP4
  for Artifact pages. `filmstrip` has no lab path, so it drives
  `Lab::tick_for_harness` plus `Lab::draw` the way `foodroad` does, with the
  listener and the frame capture sharing one take of the touched set.
- Habituation, first-run level, and the world-reset re-baseline (§2, §3e).
- Guards 1-5 (§6d).
- **Deciding measures, named now:** the owner's ear on a paired page (the
  only pass); listener ≤ 0.25 ms per displayed frame from `labdial rate` /
  `lab_cost`; the determinism guard green with its positive control
  red; `cargo tree` for the Linux default build shows no `cpal`.
- Size: ~1.5-2.5k lines, about two sessions. [inferred]

**Stage 2 — the world.** Powder, liquid, rain and fire textures from the
strided snapshot diff; fling (one animal), cull and prune; carry and delivery tint;
milestone and split alerts; the sound tap only if positional digs or bites
are needed.

**Stage 3 — stakes.** Colony-health bed, alarm texture, death clusters, the
enemy seat's centipede and the colony answering it.

**Stage 4 — bodies, structure, polish.** Seat bodies close-miked and each
seat's verbs; collapse, blasts, debris and creak in the sandbox (the load
telegraph); a
volume page per bus; hero samples if synthesis has not carried them.

**Stage 5 — music, if wanted.** Generative, driven by the beds.

**Fun hunt (Nov 2-15).** Sound is one of the cheapest ways to make a verb
satisfying, and the hunt is testing verbs. If the owner wants it, stage 1's
verb feedback can target the hunt mode first; under the owner's 10-30
ruling that work is judged by play and exempt from the report rules until
Nov 15.

## 12. How it will be judged

- **By ear, on an Artifact page**: a short video per scene with sound, the
  same run with sound off beside it, and the event counts (ants in view,
  digs per second, alerts fired) printed beside each. "Did it fire" needs a
  counter, not a sound.
- **Paired**: the same seed and the same frames for both arms; the
  soundtrack example makes this free.
- **Blind A/B for style questions** (§5), letters shuffled, key kept here.
- **No "pass" until the owner has listened.**

## 13. Open questions, risks, and the key

**For the owner (decision card in the thread):** approve stage 1 as scoped,
and which sketch style sounds like the game.

**Risks.** Procedural textures can sound thin or "synthy"; mitigated by the
offline render, which lets recipes be tuned against pictures in minutes, and
by the sample escape hatch. Silence-is-information can read as broken;
mitigated by the room tone. Game-thread budget at 1,000+ ants in view;
mitigated by the stride sampling and a hard cap on reads per frame. Linux
players get no sound by default; acceptable while the owner plays on a Mac.

**Blind key for the style sketches on the page:** A = hybrid, B = natural
foley, C = musical.

## 14. Records

- Earlier reports that asked for sound and are answered here:
  `design-load-telegraph.md` §2 (creak), `load-model-fit-review.md` §6,
  `destruction-plan.md`, `evolution-lab-what-is-missing-2026-09-09.md` row 5,
  `evolution-lab-round-26-2026-09-10.md` ("needs a ruling on the
  dependency"), `weather-handoff.md` (thunder). This document is the
  ruling's proposal; the owner's answer is the ruling.
- `dead-ends.md` has no entry about audio (grepped `audio`, `sound effects`,
  `rodio`, `cpal`, `kira`, `sfx`, 2026-10-31).

## 15. What the independent review changed

Reviewer: a separate session given the draft and told to check every
[traced] claim against the source (2026-10-31). Findings and where each
landed:

| # | Finding | Kind | Fixed in |
|---|---|---|---|
| 1 | cpal `realtime` pulls MPL-2.0 `audio_thread_priority`; adding cpal also needs the notices file regenerated | wrong | §8, §9 |
| 2 | the touched set is chunk-level and includes moisture and every ant step, so it cannot say "sand is falling" | misleading | §2, §3b |
| 3 | dial modes must key on delivered ticks (`ticks_per_frame`, `reads_as_motion`), not the preset; "up to 1,024 ticks" is stale both ways | hole | §0, §2, §3d |
| 4 | `notable` includes `PlayerAction`; `REACTION_COOLDOWN` is private; LineEnded at scale is not rare | misleading | §2, §3d, §4 |
| 5 | the lab's FLING throws one animal | wrong | §1b, §2, §4 |
| 6 | strokes go through `use_tool`, not `Lab::act` | misleading | §2, §4, §6b |
| 7 | the determinism guard as drafted did not exercise the seam, and its control proved the hash, not the listener | hole | §6d |
| 8 | a rebuild resets every counter; diff `RunLog::pushed(kind)` | hole | §2, §11 |
| 9 | no interior mutability reachable through `&World` | confirmed | §6d |
| 10 | line numbers for logs and `steered`; frame-cost figures are September's | corrections | §2, §9 |
| 11 | no macOS CI, so cross-check the Mac and Windows targets; F11 is a macOS system key; bar has no spare cell | suggestion | §8 |
| 12 | `ascii` cannot time a lab listener; `soundtrack` must drive the lab harness; per-ant head diff for regions; paused verbs; first-run level; listener fatigue | suggestion | §3b, §3d, §3e, §9, §11 |

Not re-measured by this session: the reviewer's 2.6-4.4x delivered speed,
20.4 of 40 chunks awake, and the 0.16% changed-cells figure, all quoted from
earlier reports.
