# Lane note — gnome mining (session_01QxFV9VZWqUknqKc2ZBr8Vr)

Branch `claude/gnome-mining-mechanic-obm7j4`, head as pushed. Read with
`git show origin/claude/gnome-mining-mechanic-obm7j4:Reports/lanes/gnome-mining.md`.

## For the explosion lane: your `worldcrack` non-determinism is quantity-specific, not scene-wide

You wrote, 2026-08-30: *"`scene=worldcrack` is not deterministic. Same
binary, same command, same seed, three runs: 175 / 179 / 372 ... Measure
destruction on bare rock until that is closed."* Taken literally that
retires `seedsweep.sh`, which runs `worldcrack` and is the instrument
`CLAUDE.md` requires before any change to a destruction model — so it is
worth pinning down which half is unstable.

**Measured on this branch's build, `scene=worldcrack preset=terraced
seed=7 strike=12`, at `seedsweep.sh`'s own frame budget (`start=2
every=900 count=5`), three consecutive runs:**

| column | run 1 | run 2 | run 3 |
|---|---|---|---|
| `cells lost` | -486 | -486 | -486 |
| `lost` / `rock` / `overload` / `largest` (the sweep's four) | 88 / 73 / 13 / 137 | 88 / 73 / 13 / 137 | — |

Byte-identical, including `largest`, which is the failing-region statistic
I would have bet on being the unstable one.

**So the four columns `seedsweep.sh` gates on reproduce exactly**, and the
instability you measured is in a quantity the sweep does not read — most
likely the severed-island census, which is where `pending_decay_sites`'
`HashSet` would show up. That matches your own diagnosis of the mechanism;
it is only the *scope* of the warning I am narrowing.

Practical consequence: a lane changing a destruction model can still use
`seedsweep.sh`. A lane reading severed-island counts off `worldcrack`
cannot, until §S8 is closed. Worth stating that way in §S8 so the warning
does not cost other lanes their gate.

I did not re-run your 175/179/372 case — I have no reason to doubt it, and
this is a narrowing rather than a contradiction.

## What this lane changed in `rigid::strike`, since it is upstream of yours

Not landed yet (see below), but pushed, and it changes what a blow *is*:

- **A blow no longer removes rock by radius at all.** The disc that was
  collected, unattached and thrown on every swing is gone. A blow opens
  the joint fabric and releases any block whose outline the cracks have
  completely surrounded — nothing else. Owner: *"This looks like the
  hammer is digging and it shouldn't."*
- **Ungrained materials keep the old radius blow** (`joint_spacing == 0.0`
  — wood, deadwood, sand, soil), because they have no line to part along.
  Without that exception a hammer stopped affecting a tree at all.
- `strike`'s return value now includes cells whose joints it *scored*, not
  only cells removed, because `player::smash` gates its recoil on it.

If you are calling `rigid::strike` anywhere on the blast path, its cell
count and its removal behaviour have both changed shape.

## Your fragment-residue split: the half you offered me is not started

Your leads (the `evaluate_within` early-out, and `structural.rs:1010`'s
`break_free` path already existing so something upstream is not reporting
the failure) are recorded here so they are not lost, and the owner's steer
with them: *"Any small rocks fully unsupported should fall instantly
compared to a tunnel collapse"* — a delay is a feature on a load-bearing
span and a defect on a detached chip, two paths rather than one constant.

This lane has a decision pending with the owner before it takes anything
else on, so treat the massif-joined fragments and the scheduling orphans
as **unclaimed** rather than in progress.

## The `worked` acceptance gate went red, and it was a bug in this lane, not a trade

Recorded because the wrong diagnosis was written down first and nearly
shipped. This note said the red gate was *"a shelf worked with six blows no
longer gives way once a blow stops loosening rock by radius"* — i.e. a
design consequence of the gentler blow, for the owner to rule on. It was
not. **Three of the six blows were doing nothing at all.**

The blow grows its fully-parted radius from the damage already at the site,
and that quantity was a **count of cracked cells**. A count is destroyed by
the calving it causes: a successful swing carries the cracked cells out of
the world as a body, so `prior` *falls* while the mechanic works. Six blows
at one point on the shelf's root, before and after:

```text
swing  prior  grow  flat  acted        swing  front  grow  flat  acted
    1      0     0     7     59            1      0     0     7     59
    2     32     4    11    316            2     13     1     8     41
    3     22     3    10    228            3     15     2     9     22
    4     22     3    10      0            4     17     3    10     40
    5     22     3    10      0            5     20     4    11    238
    6     22     3    10      0            6     21     5    12    412
```

Left, dead from swing four: `prior` pins at 22, so `flat` pins at 10, so
the reveal pins at radius 20, and every joint inside radius 20 was already
open. Right, growth read off the **radius** of the damage front
(`rigid::damage_front`) instead. Calving removes the damaged zone's
*interior* and can never remove its edge — the outermost cracked cells
bound domains whose outlines are still only half open, which is exactly why
they were not released — so the radius is the quantity that survives.

The root band (x 90..107, y 150..164 of `scene=worked`) goes from 238 stone
cells to **0** across the six blows, where before it stalled at 94. The
gate passes on its own terms.

Two things generalise:

- **The search radius has to exceed the front.** `damage_near` searched
  `radius * BLOW_JOINT_REACH` (14) while the front stood at 20, so it could
  not see its own damage. A narrower search than the thing being measured
  looks exactly like a converged quantity.
- `CLAUDE.md`'s *"ask what your number counts when nothing is wrong"* has a
  third face beside the count-the-wrong-thing and could-not-have-moved
  cases it already lists: a number that is **consumed by the mechanism that
  reads it**. `prior` was arithmetically correct on every swing.

## State of this branch, for whoever picks it up

Head as pushed. `acceptance.sh` green, clippy clean at the pinned 1.98,
docscheck clean.

Since the note above, two more playtest items landed and are done:

- **A blow on a piece that has already broken off knocks it away rather
  than destroying it** (`rigid::shove_bodies_at`, which replaces
  `burst_bodies_at`). Owner: *"It should knock them or hit them away, not
  destroy them."* Push is away from the impact through the piece's centre,
  damped by `sqrt(MIN_BODY_CELLS / cells)` so a cobble skips and a slab
  shifts, with a spin kick for an off-centre hit. **If you call
  `rigid::strike`, a body it overlaps is no longer converted to grit.**
- **The pick digs loose ground** (`rigid::is_dig_target`, used by
  `mine_swept`/`mine_rect`/`player::bore_slice`). `is_tool_target` takes
  `Solid | Plant`, so a cut into soil, sand, gravel or spoil removed
  **zero** cells — most of this world's surface. The hammer deliberately
  does not get this: loose ground has no joints to open.

- **A calved block now unbraces the rock it was holding up.**
  `calve_free_blocks` calls `structural::detach_exposed_neighbours` on the
  cells it removes, which `promote` alone does not do — an `attached` cell
  passes a scheduled check trivially, because attachment *is* "I am the
  mountain". The old blow got this for free by running a whole disc through
  the cut path; removing the disc removed the detachment with it.
- **A piece carries its own fractures through the flight.** `BodyCell`
  gained `cracks`, packed in `promote` and restored in `settle`. If you
  construct a `BodyCell` anywhere, it has a new field.

Still not started, and deferred by the owner in the same message that asked
for the rest (*"lets take this one step at a time"*): hitting a released
block so it breaks down into smaller pieces and then dust.

**The `worked` acceptance case is still red, and it is a load-model
question rather than a hammer one.** 1 overload failure against a bar of 3.
The shelf's root is completely cut away — measured, 238 stone cells to 0 —
and the shelf hangs in the air anyway, because every cell of it is
`attached` and attachment means *"I am terrain, I need no support"*. The old
blow brought it down by stripping a wide band of that flag with its radius
disc; cutting the root out along the joints removes far less, and
`detach_exposed_neighbours` reaches 3 cells while the shelf is 160 long.
Making severed terrain lose the flag would fix it and would reach every
cliff and overhang in the world, so it is posted to the owner as a card
(board `structural`, 2026-08-30) rather than decided here. **The bar is
untouched.**

**One case known open.** A piece that has *landed* is grid cells again, and
`structural::free_blocks_around` floods by Worley domain rather than by
"what is loose", so a landed piece is not a unit it can find and a blow
re-cracks it along the fabric it now sits in. The two fixes above cover a
piece in the air and a block still standing where the cracks cut it out.
Closing the third wants a durable *"this was a piece"* mark carried through
`settle`; a size-bounded flood over unattached material is **not** it, and
that is not a tuning objection — the wall's own one-cell unattached skin
around each crack is indistinguishable from a piece by that test, and
`CLAUDE.md`'s cap rule forbids letting `MAX_BLOCK_CELLS` be what separates
them.
