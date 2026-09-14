# Lane D — the trail, 2026-09-14

Coordinator: `session_01TngZpRY8LoqpFWHuXjUTTD`. Branch `claude/druid-trail`,
cut from `main` at `9fd637a7`. Files owned and touched: the trail functions in
`src/druid/mod.rs`, the scent rendering in `src/druid/hud.rs`, and one new
instrument, `examples/druid_trail.rs`. Nothing else is touched — in
particular **`pheromone::DECAY_RHO` is untouched at 0.03**, and so is
`pheromone::DEPOSIT`, `DIFFUSE` and `PHEROMONE_INTERVAL`. The lab's foraging
work stands on exactly the constants it stood on this morning.

## The item

Owner, 2026-09-14 playtest: *"my pheramone trail should last way longer. it
dissapears so much faster than an ant could even move. I wonder if this is a
problem for ant laid trails too. The animation can be improved too. it should
be more diffuse looking, not like a bunch of dots."*

Scope ruling carried in the brief: *"For pheromone, you make the gnome laid
trail last longer. The evolution lab will explore the ant laid ones."*

## The headline

Measured in the real app (`Druid::update`/`Druid::draw`, the calls
`src/bin/druid.rs` makes), one walked route of 143 cells:

| | before | after |
|---|---|---|
| trail still on the ground | **3.5 s** | **~14 s** |
| ...in ant-cells walked | 35 | 135 |
| peak strength at t+3.5s | 0 | 88 |
| cells of the route still holding scent at t+3.5s | 0 of 143 | **143 of 143** |
| where the scent sits | 3 cells above the floor | on the floor |

**Both of the owner's complaints turned out to be one defect**, which is why
one constant answers both.

## What was actually wrong

**Diffusion, not decay, is what kills a trail here, and nothing in the
codebase said so.** `pheromone::DIFFUSE` blends every cell a quarter of the
way toward its own 3x3 mean each pass. For a cell on a **one-cell-wide line**
six of its nine neighbours are empty, so that mean is about `v/3` and the line
sheds roughly **17% a pass** — against `DECAY_RHO`'s **3%**. Diffusion is five
to six times the term everyone reaches for, and it is invisible in the
module's own write-up, which frames trail lifetime entirely around the decay
LUT's strict-decrease floor.

Three consequences, all measured:

- **Depositing harder barely helps.** On the plane, a one-cell line at the
  shipped deposit stays legible 1.0 s; pushing that deposit all the way to the
  255 ceiling reaches **4.8 s**. The lifetime is not in the deposit.
- **Width is the lever.** A cell in the middle of a *band* has a 3x3 mean of
  roughly its own value and sheds almost nothing. At an **unchanged** deposit,
  an `r = 3` band measures **10.8 s** against the line's 1.0 s.
- **The same fact makes the trail look like dots.** A mark laid one cell wide
  rounds to nothing a cell or two out, so there is no cloud to draw — the
  readout was drawing the spine of a cloud that did not exist.

Two more defects fell out of looking at the picture rather than the numbers:

- **The scent was laid at his chest.** `lay_trail` marked `Player::center`,
  which this build measures at a median **3 cells above the floor**. The
  trail hung in the air over its own route — visible as a band of green mist
  at waist height once the swath made it wide enough to see — and it is the
  half of the plane an ant can never read, since an ant senses around its own
  head while walking the floor. Now `Player::feet`.
- **The readout had one colour and seven unreachable ones.** The band was
  `v * SCENT_BANDS / 256`, so everything under 32 landed in band 0 — and the
  plane never held more than about 31 along a walked route. **Every mark of
  every trail drew in the same single dimmest colour.** A readout with one
  value has no gradient, which is the other half of "a bunch of dots".

## What shipped

1. **`druid::TRAIL_RADIUS = 3`** — `lay_trail` lays a graded swath about him
   rather than a single cell, strongest under his feet and fading to the rim.
   Graded rather than a flat disc for the ethos reason: a uniform disc is a
   hard-edged slab whose rim is a step down to nothing, which is the dots with
   bigger dots.
2. **`lay_trail` anchors at `Player::feet`**, not `center`.
3. **`hud::band_of`** — the readout's bands come off `sqrt(v/255)` rather than
   a straight scale, so the small values a cloud's *edge* is made of resolve
   into their own bands instead of all collapsing into band 0.
4. **The readout blends rather than stamps** — alpha as well as colour carries
   strength, so the core lands opaque and the rim fades into the world.

**`TRAIL_DEPOSIT` is deliberately not raised.** Raising it alongside the swath
pins the plane at 255 for about 2.8 s more life, and a pinned trail is one no
ant walking it can reinforce — `pheromone::DEPOSIT`'s own P-14 note says halve
it rather than let that happen.

### What this costs, stated plainly

- **29 plane writes a tick while `G` is held**, against 1. Deposits are a `u8`
  write plus a `write_watch` mark; nothing in the sweep changed.
- **The plane does now run close to saturation along his route** — peak 241 of
  255, against about 80 before — because he re-marks each cell some ten times
  as he walks over it at 0.6 cells/tick. That is the trade the extra lifetime
  is bought with, and it means an ant walking his road adds 14 rather than 40.
  Recorded here rather than tuned away: reducing `TRAIL_DEPOSIT` to restore
  the headroom is a second decision and the owner asked for lifetime.

## The owner's aside, measured — *"is this a problem for ant laid trails too"*

**Yes, identically, and the answer is free because the shipped gnome trail
*was* an ant trail.** `TRAIL_DEPOSIT` was defined as `pheromone::DEPOSIT` and
laid one cell at a time, so every number in the "before" column above is also
the number for one ant laying one unreinforced route.

- An ant-laid mark, not re-walked, is legible for about **1 second** on the
  plane and its trail is off the ground in **3.5 s** in a real world.
- A colony round trip is roughly 2,200 frames — about **37 s**, or **367
  ant-cells**. So a route a scout lays and does not re-walk is gone before the
  scout can get home, by a factor of ten.
- **What keeps ant trails alive is reinforcement, not the mark's own life.**
  That is a working system, not a bug — but it means a colony cannot recruit
  to anything a single ant found and left.
- **The lever is not `DECAY_RHO`.** At 3% a pass it is not what is removing
  the trail; the 17% going sideways into empty ground is. Anything the lab
  does about ant trail persistence should be aimed at reinforcement rate or
  mark width, and a decay sweep will measure a small term.

**Measuring this was mine; fixing it is not.** Nothing ant-laid was changed.

## Instrument

`examples/druid_trail.rs` — new, and `Reports/instruments.md` was grepped
first (26 existing; `trailfollow` answers *does a laid trail move a colony*,
which is the other half and is deliberately not duplicated). Two modes:

- the default sweep, on the plane alone, printing the peak-on-route decay
  curve per arm in seconds **and in ant-cells**, against a colony round trip;
- `shot=` , which drives the real `Druid::update`/`Druid::draw` loop headlessly
  (no window, no GPU) and writes a column sheet with the plane's peak, live
  cell count, route geometry and **clearance to the ground** printed per frame.

`selftest` runs five positive controls. **They earned their keep twice in one
session**, and both failures are recorded in the source:

- the decay loop advanced `frame` by 1 while `Pheromones::step` gates on
  `frame % 12`, so its "passes" were ticks and every lifetime printed **12x
  too long** — a 1.0 s trail reported as 12.0 s, with a perfectly plausible
  decay curve;
- the repair then advanced by 12 from an *unaligned* frame and never hit a
  multiple again, so no pass ran at all and every arm reported a flat curve at
  its laid value. Control B caught that one; control E was added for it.

A third instrument bug was caught by asking what the number counted: the
`slope` column sampled the last cell of the *route*, which at a stride above
one the gnome never steps on, and so reported the newest end of the trail
*weaker* than the oldest — the exact reverse of the property it exists to
check.

**And the harness disagreed with the app by 3x even after both repairs**,
which is why every headline number above is the app's. The plane model's clean
horizontal line is a best case; a real walk is not.

## Gates

