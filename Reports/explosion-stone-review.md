# Explosions in stone — three-lens review

**Status:** review with measurements; recommendations at the end. Written to
be picked up cold by an implementation session (the owner has asked for the
prototype and follow-on work to be done by a cheaper model — §7 is written
for that model specifically, and assumes less context than the rest).

**The assignment, in the owner's framing:** what really happens when you
detonate a charge in solid rock, in caves; what of that is realistic inside
this engine (engine changes allowed); what is actually fun — for mining and
for blowing things up for its own sake; and the best intersection of the
three. Everything below is bound to one standing constraint, stated by the
owner at plan review: **it must work in a 2D side-scrolling cellular world.**

**Method:** per `CLAUDE.md` — look first. §1 is rendered scenes read against
printed counters, at shipped defaults, before any analysis. A new
`filmstrip` scene (`cavern`) was added for it, because nothing could
previously stage the one situation every mining blast is actually in: a
charge next to a void inside attached rock.

---

## 1. What a blast in stone actually does today, measured

All at shipped defaults (`radius 22` keyed, harness runs `r=20,
strength=180`), `cargo run --release --example filmstrip -- scene=...
explode=x,y,20,180,60`. Same-session `ascii` baseline: settled scenes
0.000 ms, blast scenes worst ~14-16 ms on this (contended) machine.

### 1a. Buried in solid stone: a bruise, not a cave

`scene=boom_stone explode=256,220` (40 cells of cover):

| quantity | value |
|---|---|
| cells dug at peak (frame 69) | ~1,350 |
| chunk bodies in flight, peak | 20 (1,192 cells) |
| net cells lost by frame 211 | 286 (rock −357, rubble +71) |
| surviving void | **57 cells** |
| cells fissured by the aftermath | 47 |
| debris particles at any sampled frame | **0** |

The staged cavity opens, `fracture_shell` cracks the wall into real tumbling
pieces — and every piece lands back in the hole it came from, because inside
a solid mass there is nowhere else. The end state, by eye: a perfectly
circular, orange-rimmed disc of packed rubble with a ~57-cell irregular void
at its centre. A bruise. The 47 fissured cells (confined crush failures) are
nearly invisible at play zoom. Nothing about the outcome depends on
direction: up, down and sideways are identical.

### 1b. Ten cells under the face: the one case that already reads well

`explode=256,190`: a real plume of dark ejecta into the sky, a crater that
keeps a 141-cell void, scorched rim. Still refills most of itself (net loss
213 of ~1,300 dug), and the rim is still a compass-perfect circle — but this
is what the mechanic was tuned on, and it shows.

### 1c. The same charge in sand, same 40-cell depth

`scene=sandbed explode=256,160`: **991 cells evacuated against stone's
286.** The blast itself has no material term at all — `clear_annulus` clears
stone and sand at the same radius, and the entire measured difference is
what happens to the material *afterwards* (sand avalanches and stays out;
rock pieces re-land inside). Stone is only "harder" today by accident of
settling dynamics.

### 1d. A cave wall: the overburden chimneys, and the world never settles

`scene=cavern explode=186,240` — charge sitting in the wall beside a 120×56
cave under an 82-cell roof:

| quantity | value |
|---|---|
| chunk bodies one frame after detonation | **189 (5,699 cells)** |
| cave volume | 5,269 → **3,598** (−32%) |
| net cells lost | 3,238 (rock −3,948, rubble +710) |
| worst frame | **264 ms at frame 201** — deterministic, reproduced exactly |
| pending structural sites | 6.3k (f61) → 14.9k (f400) → **17.9k (f1200), still climbing** |
| chunks awake at f1200 | 10-11 of 40, indefinitely; new bodies still spawning |

The blast undercuts the rock above it; the load model finds the whole
~40-wide, ~90-tall column to the surface unsupported; it detaches in one
frame, shatters into 189 bodies, and pours down — into the cave. **Blasting
a cave wall makes the cave smaller.** The aftermath then never goes quiet:
twenty seconds later the scheduler still holds three times the pending sites
it had one frame after the blast, and the 264 ms spike lands well after
everything is visually still.

### 1e. A cave roof: immortal

`explode=256,200` — charge 12 cells above a 120-cell-wide ceiling: cave
volume 5,269 → 5,305. **Nothing falls.** The crater's own rubble hangs as a
plug above the intact shell; no spall, no rockfall, no delayed collapse.

### 1f. The inversion, stated once

1d and 1e together are real blasting **mirrored**. In rock mechanics the
roof over a void is the classic failure — the reflected wave returns as
tension, rock is ~10× weaker in tension, gravity points into the void — while
laterally-confined overburden holds by shear. Here the roof cannot be
brought down by a blast that all but touches it, and the side-supported
column above a wall shot collapses to the surface wholesale. A player who
places a charge *well* (under a roof, to drop it) is punished with nothing;
a player who places it badly (into a wall) levels the neighbourhood and
fills their own excavation.

---

*Sections 2-7 — the three lenses, the intersection, ranked recommendations,
pathology diagnoses, and the implementation handoff — follow from the design
fan-out and are being assembled.*
