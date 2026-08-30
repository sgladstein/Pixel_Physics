# The EXPLOSION menu, knob by knob

*Written 2026-08-29, from the owner's report: "in the menu there are so many
different options for changing explosions and I don't really know what each
does and it is too complicated." This is the "give me a guide" half of that
request. The "simplify" half is the menu's new grouping and the `type` row,
both described below; the third option he offered — remove the less important
ones — was deliberately **not** taken, and the reason is at the end.*

Every number here is measured on one scene so they can be read against each
other: `filmstrip scene=boom_stone blast=256,26,22,180,20`, a radius-22
charge 26 cells under a flat stone surface, at rest 240 frames after the
bang. Re-run any row with the flag named beside it.

---

## The one row that matters most: `type`

Five whole tunings, walked with the arrow keys. This is the row to reach for
first, because it moves all twenty-six numbers at once and it is the only
control here whose effect you can predict before you press it.

| type | cells cleared | joints woken | seams opened | what it is |
|---|---|---|---|---|
| `CAP` | 27 | 182 | 76 | a detonator — a pockmark and a tight web |
| `POWDER` | 671 | 231 | 80 | black powder: a slow **heave**, mostly thrown rubble and smoke |
| `DYNAMITE` | 370 | 833 | 418 | the general-purpose stick — `Tuning::default()` exactly |
| `MINING` | 23 | 786 | 494 | a confined shot-hole: it **shatters** rather than digs |
| `DEMOLITION` | 1,026 | 1,400 | 923 | big crater, big throw, big fireball |

Read the first two number columns against each other and the design is
visible: `POWDER` clears 671 cells with 231 joints and `MINING` clears 23
with 786. Those are opposite corners, not two points on a size axis — a low
explosive pushes material, a high explosive confined in rock cracks it.

The row reads **`HELD`** the moment you move any other number, which is not
an error: it means the live tuning is no longer any named charge. Walking
back onto a type restores that whole charge.

`filmstrip charge=cap|powder|dynamite|mining|demolition` renders any of them
headlessly, and `blast=x,depth,0,0,frame` takes the charge's own radius and
strength instead of overriding them.

---

## `the bang` — three numbers that scale everything else

| knob | flag | what moving it does |
|---|---|---|
| `radius` | — | the crater, in cells. Most other numbers here are fractions *of* this, so this is the one that rescales the whole event. |
| `strength` | — | how hard debris is thrown, how hot the spike is, how far a fragment punches through cover. **Not** how big the hole is. |
| `duration` | — | frames the cavity takes to open. Low is a deletion, high is a detonation — and it is what lets debris *leave*, because material cleared on a later frame launches into a cavity earlier frames already opened. |

---

## `cracks` — the half of an explosion that is still there afterwards

These four are the pattern. The counters below separate them cleanly, which
is the point of listing them together: **each moves a different quantity**,
and a knob that looked like a duplicate of its neighbour is not.

| setting | joints woken | opened | scored |
|---|---|---|---|
| `joint_reach` 1.2 / **2.4** / 4.0 | 250 / **833** / 1,686 | 147 / 418 / 874 | 85 / 387 / 788 |
| `joint_density` 0.4 / **0.9** / 1.0 | 323 / **833** / 923 | 190 / 418 / 482 | 127 / 387 / 427 |
| `joint_open_fraction` 0.10 / **0.30** / 0.60 | 833 / **833** / 833 | **186 / 418 / 657** | **608 / 387 / 149** |
| `joint_seam_width` 1 / **3** / 5 | 833 / **833** / 833 | 417 / **425** / 443 | 390 / 387 / 381 |

- **`joint_reach`** (`jreach=`) — how far out the web goes, as a multiple of
  `radius`. It is the only one of the four that changes how much rock is
  involved at all, so it is also the one with the steepest cost. There is no
  hard edge at this distance: it is where the activation ramp reaches zero,
  and individual cracks stop raggedly on either side of it.
- **`joint_density`** (`jdensity=`) — what fraction of the available joints
  are there at all, in the thick of it. Thins the whole web evenly. If the
  pattern reads as too *fine*, this is the wrong knob — that is
  `stone.ron`'s `joint_spacing`, which sets how big the polygons are.
- **`joint_open_fraction`** (`jopen=`) — the split between cracks that
  **part** (a black seam of void and grit) and cracks that are only
  **scored** (a hairline, nothing removed). Note the third row: the joint
  count does not move at all, only how many of them are bold. This is the
  knob that pays for the look, because opening removes material. `0.0` is a
  hard off: everything is scored, nothing is removed.
- **`joint_seam_width`** (`jwidth=`) — the cap on how wide the boldest seams
  get. Note the fourth row: it moves *neither* count, because it changes only
  the **weight** of seams that were already opening. `1` is the uniform
  one-cell seam that shipped before 2026-08-29 and is the A/B control; the
  default is `3`.

  **This is the knob to reach for if the web reads as too heavy** — before
  `joint_open_fraction`, and long before `joint_reach`. It is the only one of
  the four that changes nothing about *which* rock is affected.

  A note on why it did not seem to do much at first: the width ladder was
  linear, and under a linear ladder the top rung only ever fires at a point
  on a crack, so caps of 2, 3 and 4 rendered as nearly the same picture. It
  is concave now — a bold crack holds its width for about half its own run —
  which is what makes the setting visible at all.

  Cost, `seedsweep.sh` over 3 presets x 4 seeds run to rest, `rock
  destroyed`, both arms from the same build: total **5,062 -> 6,831** and max
  **1,022 -> 1,397** going from `1` to the default `3`. So a third more rock
  comes down per blast, which is the price of the weight and is not obviously
  the wrong trade -- more of what a blast cuts free actually leaves.

Three more in this section, all about timing rather than shape:

| knob | what moving it does |
|---|---|
| `crack_growth` | how fast the fracture front races outward, in cells per frame. Large is instant, which reads as a graphic stamped on the stone. |
| `crack_stagger` | how ragged the arrival is, in frames. `0` sends every crack out on the same frame — a synchronised starburst. |
| `crack_glow_temperature` | how hot the crack tips run as they travel. `0` is off. |
| `calve_depth` | how deep a collar the finished web breaks off the crater rim, in cells. **This is the beat where the cracks stop being a picture and pieces come away** — `0` turns it off entirely and the star becomes decoration. |
| `chip_sweep_cells` | the largest fragment a finished blast sweeps up, in cells, when it is left touching nothing but air. `0` is off and restores the pre-2026-08-30 behaviour. Reported from play as *"lots of single pixel or small clumps that stay floating"* — measured, they drain for ~3,000 frames and then stop dead, so they were permanent. Five times fewer floating specks at the default `6` (507 → 102 cells at frame 600), for +13% rock. Raising it past 6 puts the sweep in competition with the fragmenter over the same rock and makes grit where pieces belong. |

---

## `rubble, smoke and fire` — what flies and what is left burning

| knob | what moving it does |
|---|---|
| `debris_fraction` | chance a cleared cell becomes a flying particle rather than simply vanishing. Higher is more spectacle and, past a point, more cells landing again on the first frame. |
| `vaporize_fraction` | the fraction of `radius` that is gone outright, no debris. Small on purpose: a blast's signature is material flying, not a clean hole. |
| `smoke_fraction` | how much of the crater is backfilled with smoke. The one item here with an ongoing frame cost — gas keeps its chunk awake while it rises. |
| `fireball_fraction` | how far past the crater the scorch ring reaches, as a fraction of `radius`. |
| `flash_temperature` | how hot that ring is. Past ~420 the glow ramp is saturated and the extra buys ignition headroom, not brightness — and nothing shipped has a finite ignition temperature, so past ~420 it buys nothing at all. |
| `afterglow_retention` | how much heat survives each frame. A *per-frame* retention, so the interesting band is the top tenth: 0.94 fades in ~90 frames, 0.99 in ~550. |

---

## `advanced` — real, and not the first thing to reach for

Ballistics of a single grain (`speed_per_strength`, `debris_jitter`,
`pierce_divisor`, `shockwave_multiplier`, `heat_fraction`), the confinement
probe (`containment_floor`, `confined_cavity_fraction` — how much a *buried*
charge excavates versus how much it puts into the grain), and two knobs
belonging to a superseded mechanism.

`crack_rays` and `crack_reach` are the **radial walker star**, which the
Worley joint fabric replaced. `crack_rays` defaults to `0`, so the walker is
off and the fabric is the whole pattern; setting it to 4-6 puts the old star
back *on top* of the fabric for an A/B. It is kept reachable rather than
deleted because the question "was the walker better" should be re-askable
from the panel without a rebuild.

---

## Why nothing was removed

The report offered three options and the third was to delete the less
important knobs. Every row here was measured above and every one of them
moves something, which makes this a findability problem rather than a
surplus. A knob that is hard to find and a knob that should not exist call
for different fixes, and deleting the second-tier ones would have thrown away
the A/B controls (`joint_seam_width: 1`, `crack_rays: 4`) that any future
argument about this pattern has to be settled with.

So the menu keeps all of them and orders them by what you would reach for
first: the charge type, then how big the bang is, then the two things you can
watch it do, then the numbers that only matter once you are tuning one of
those. The panel draws a subheader wherever the section changes, so the split
costs no new UI.
