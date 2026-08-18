# Next session: what the unzip actually was, and what is left of it

**Written to be picked up cold.** State, what was measured, what is still
wrong, and what has already been tried and must not be retried.

**Read first:** `CLAUDE.md`, then `Reports/building-rethink.md` §3a and §6,
then this. `Reports/destruction-plan.md` holds the wider backlog and the
"Pending owner verification" list.

---

## 0. Where things stand

`master`, **519 lib tests**, clippy clean, **13** acceptance cases gating in
CI via `scripts/acceptance.sh` (five of them on procedural terrain), plus
`scripts/seedsweep.sh` as the instrument the suite cannot be.

`field::a_sealed_room_holds_pressure_better_than_open_ground` fails on
master and belongs to another area.

Landed this session: one spoil model for every digger (`3084b7b`), and
confined rock cracking in place instead of falling in on itself
(`c709b4c`). **The live problem is §1a-NEW: caves cannot actually be dug,
because the gnome will not walk into his own tunnel.**

The destruction model is right and working: torque vs capacity, section
failure, load flow over parallel supports, crack-driven detachment, a
stress view (`N`), a rectangle/room/line build tool (`Z`), a precise dig
verb (`D`).

The owner's verdict after the intact reframe was **"big improvement"** —
building stands. Then one dig unzipped a room into grit, which was the
live bug, and this session found out why.

---

## 1. The unzip: the previous diagnosis was wrong

The version of this file that stood here blamed a **self-propagating
front**: `is_structurally_interesting` treating an intact cell as evaluable
when adjacent to empty, so each removal made its neighbours evaluable, one
cell at a time, handing `rigid::fracture` regions below
`MIN_FRACTURE_CELLS` that fell through to per-cell conversion — which *is*
powder.

**Disproved, by measurement, on `scene=room`:**

- Material below the cut reads `not evaluated` at every captured frame. The
  front never runs. It cannot, because those cells are intact and not
  adjacent to anything empty.
- Failing regions measured **80–1,573 cells**, not 1–2.
- 45 chunk bodies formed, mean 27 cells each. The fragment ladder was
  receiving perfectly reasonable regions all along.

Nothing about region sizes was wrong. Do not go back to the propagation
front; the reproduction is three commands away and says no.

### What it actually was

`scene=room wall=8 dig=0`, probing straight down the inner face of a wall:

```
load (148,170): mass 1627 torque 76028 capacity 443904 stress 0.17
load (148,200): mass 1657 torque 76028 capacity 443904 stress 0.17
load (148,250): mass 1707 torque 76028 capacity 443904 stress 0.17
load (148,300): mass 1757 torque 76028 capacity 443904 stress 0.17
```

**Identical torque all the way to the floor**, 140 cells below the roof
that produced it. `torque = |Sx − x·M|` is the moment of everything a cell
carries *about that cell*, which is right for a beam reaching sideways and
charges a column the eccentricity of its roof's centroid — fifty cells
away. So every cell of every wall in a building sat at exactly the stress
of the worst point of the roof it carried. Nothing was ever safely deep
inside a structure, and any one cell that lost its attachment bonus
(capacity ÷ 12, from `mine`'s detach band) failed wherever it happened to
stand. One failure, one subtree, the whole upper building.

The fix shipped in `0b5b175`: on a **vertical** support step the arm is
clamped to the column's own half-thickness. The load enters the column at
the joint; below the joint the column carries force, not the beam's
bending. Deliberately a clamp and not an exemption — a column still carries
`M × half-width`, so a thin wall under a heavy eccentric cap still fails.

---

## 1b. The live bug: one dig erodes far more than it should

**Start here.** Reported from play against the generated world: *"one crack
in the ground basically propagates throughout the whole world and slowly
breaks everything."* Reproduction is `scene=worldcrack` (commit `de8fedd`),
which digs once into a generated world.

Measured. It is **bounded** — every preset and seed settles and sleeps
within ~1,500 frames, `max_chain_reach` stays at 13–22 cells against
`ROOTWARD_CHECK_STEPS`' 128, and a 20,000-frame run is flat after 4,000. So
it is not unbounded erosion. But one radius-6 dig on the app's own default
world (`rolling`, seed `0x5EED`) takes **1,558 cells**: a scar down the
entire flank of the hill, mostly the soil mantle sliding off and exposing
bare rock. One click, one hillside.

```
rolling 0-201   terraced 0-201   wetland 0
arid   52-348   flat   338-637   canyon 439-2,102
```

### Look at it before doing anything else

`target/filmstrips/front.png` — `preset=flat seed=7`, zoom 8 on the rock
*beside* the dig, four frames from 400 to 1150. **The rock is being eaten
into a spongy filigree of voids.** Not a crater, not a collapse, not
chunks falling: a fretwork of holes opening all through the slab and
joining up, spreading sideways well past the dig. (Empty renders as sky
now, so the pale-blue and orange are holes, not material.)

Two things that picture settles, and neither is what the counters suggest:

- It is **not a front advancing from the dig**. Holes appear scattered
  through the whole region at once. A large area became marginal together
  and is failing cell-by-cell wherever the sweep reaches it.
- The pieces are **single cells**. This is `CLAUDE.md`'s "turns to dust"
  failure in a new place, and any fix judged only on the cell count will
  miss whether that changed.

### What has been ruled out, by measurement

- **Stale distances.** `relax=1` on `scene=worldcrack` runs a converged
  pass immediately after the dig. Identical to the cell on eight of nine
  preset/seed pairs. `canyon` seed 3 went 5,364 → 394, so staleness is real
  but is not the cause here.
- **Gating the granular divisor on `parent.is_none()`.** Not one cell moved
  anywhere. The cells taking the cut have no solid parent to begin with.
  **Superseded — this entry was wrong, and the reason is the bug.** The gate
  was not a dead lever, it was *vacuous*: `structural::tick` rooted a cell's
  distance at 0 the moment powder touched its underside, so every powder-backed
  cell was parentless by construction and the gate had nothing to discriminate.
  Demote ground to a **last-resort root** (relax from neighbours first; take 0
  only when that leaves no path at all) and the same gate becomes meaningful —
  measured, one radius-6 dig across six presets x three seeds went from a worst
  case of 27,409 cells to a worst case of 70. That is only half a fix: it
  overcorrects, `roomcut` drops to 2 overload failures against its bar of 5,
  and `a_sprinkle_of_sand_under_a_beam_does_not_hold_the_beam_up` fails. The
  other half is a bearing model (compression-only bed, kern rule, capacity
  proportional to pile depth) replacing the flat 64x divisor. Full measurements,
  patches and sequencing are in the session plan file.
- **Granular capacity as a cap rather than a divisor.** 899/215/0 →
  25,838/51,262/0. The cap sits far below what a deep intact section
  resting on soil used to get, and generated terrain is mostly that.

Two attempts on the granular term, both failed, which by this file's own
rule means **the term is not where the fix goes.** Do not try a third.

### The hypothesis that is left

`break_free` turns every failure into rubble. Rubble is not body material,
so it neither carries load nor counts toward a section — which means each
failure strictly weakens its surviving neighbours, in two multiplicative
ways at once. That is positive feedback with no damping anywhere in it, and
both failed attempts tried to damp it at the *capacity* end. The untried
lever is at the source, in `rigid.rs`: what a failed cell becomes, and
whether a single-cell failure should produce debris that undermines its
neighbours at all.

**Unverified.** Look at `front.png` first and form your own view.

### A caution about every number in this section

The worldgen session is tuning `assets/worldgen.ron` and its generation
passes live. Between two sweeps an hour apart, `flat` seed 7 went from 338
cells to 27,886 — same preset parameters, different strata/soil/water code.
**Re-baseline before trusting any figure here**, and quote the worldgen
commit you measured against.

**Work `flat` first, not the hillside.** It is the minimal case and it needs
nothing from the owner: a homogeneous slab of bare rock, dead level, no
soil, nothing standing on it, still loses ~600 cells to one hole. Whatever
does that is almost certainly what takes the hillside too, and on `flat`
there is no slope, no powder and no strata to argue about.

```
target/release/examples/filmstrip.exe scene=worldcrack preset=flat seed=1 \
    dig=6 start=2 every=60 count=6 crop=180,190,160,110 zoom=4 \
    loadmap=1 out=target/filmstrips/flatcrack.png
```

Prime suspect, and it is §2d wearing a different hat: `mine` detaches a band
(`DETACH_DEPTH` 3 per removed cell, plus the crack rays at
`CRACK_DETACH_DEPTH` 2), every cell in that band loses the 12x attachment
bonus at once, and the ones still carrying load fail. Narrowing the band was
tried on `scene=room` and did nothing there — but `room` was dominated by
the column-moment defect, which is now fixed, so **that experiment is worth
re-running here**, where there is no column at all.

Still open with the owner: whether the hillside scar *is* what they saw, or
whether there is a case `worldcrack` does not cover. Needed to tell — the
preset and seed from the title bar, where they clicked, and `D` or `C`.

---

## 1a-NEW. START HERE: the gnome cannot get into his own tunnel

**The owner's stated next priority, in their words: "we need to go back to
making sure we can dig a cave."** It does not work, the reproduction is one
command, and the cause is *movement*, not digging.

```
target/release/examples/filmstrip.exe scene=tunnel yield=0.0 \
    start=120 every=40 count=8 zoom=1 out=target/filmstrips/probe.png
```

```
gnome: at (178, 300), grounded, bites 15 (0 displaced), 518 dusted
gnome: at (178, 300), grounded, bites 20 (0 displaced), 518 dusted
...
gnome: at (178, 300), grounded, bites 50 (0 displaced), 518 dusted
```

He cuts a bore about 25 cells deep into the cliff face at x=180, walks up
to the mouth of it, and **stops there forever**. From bite ~15 onward the
dust total never moves again: he takes 110 further bites and removes **zero
cells** between them, because every bite lands inside the bore he already
opened. He never once crosses x=180 into it.

`target/filmstrips/tunnel-clean.png` (zoom 8) is the picture, and it is
unambiguous: a clean open tunnel, and a gnome standing outside it.

### The cause, found: **he does not fit**

`dump=` (new, `examples/filmstrip.rs`) prints the materials around him as
ASCII, which is what settled this after three wrong guesses read off a 20x
contact sheet. The column immediately in front of him, at `scene=tunnel
yield=0.0` frame 400:

```
    299 .................#....       x=185 is rock  <- ceiling
    300 ..........PPPPPPP.....       air
     ...                             air
    312 ..........PPPPPPP.....       air
    313 ########..PPPPPPP#####       x=185 is rock  <- floor
```

Thirteen free rows at x=185. **`PLAYER_HEIGHT` is 14.** He is one cell too
tall for the gap and simply cannot pass, so he stands at the threshold for
the rest of the run.

The step-up does not save him, and its logic is not at fault: the floor at
x=185 is one row higher than the floor beneath him, so a 1-cell lift is
exactly right -- but lifting puts his head into the rock at (185, 299). He
is wedged between a floor bump and a ceiling in a passage his own height.

**Why the passage is too short.** A bite is a *disc* of radius 7, so it is
15 cells tall only on its exact centre line; everywhere else the chord is
shorter. A bore cut as a row of overlapping discs therefore has a scalloped
floor and ceiling, and most of its length is 13-14 cells clear rather than
15. A 14-cell gnome needs essentially all of it, so he wedges on the first
scallop he meets.

### What has been ruled out, by measurement

- **The digging.** With digging switched off entirely after frame 300, he
  spends 800 further frames with `right` held and moves **zero cells**. It
  is not spoil, not the aim, not depenetration shoving him back.
- **Spoil silting the bore up.** The reproduction is at `yield=0.0`, where
  nothing is left behind and the bore is empty. The stall is *worst* there.
- **The aim, and `dig_reach`.** The face is always ~40 cells beyond
  wherever he stands (reach 30 + bite radius 7), confirmed at two yields:
  he stops at 178 and the bore ends ~218; he stops at 192 and it ends ~232.
  So "zero cells removed" is a *consequence* of him stopping, not a cause
  -- every later bite lands in the open bore. Fix the walking and the reach
  arithmetic takes care of itself.
- **The walk duty cycle.** Holding `right` continuously is *worse* (ends at
  173, against 178), so he is not merely failing to try.

### The fix is a design call, so ask

Four ways out, and they are not equivalent:

1. **Cut a capsule along the direction of travel, not a disc.** Gives a
   constant-height corridor with no scallops. `paint_capsule` already
   exists. Probably the right answer, and it makes a bore read as a bore.
2. **Make the bite bigger than he is** -- `dig_radius` 7 to 8. One number,
   but it widens every cut including the sandbox one, and a 17-cell hole
   from a single click is a lot.
3. **Let him crouch or squeeze** through a gap one cell short. The most
   character-ish answer and the most work.
4. **Make him shorter.** Cheapest and worst: it is a game-feel constant
   that was chosen, not derived.

The spoil sweep is a trap here and is what sent the first reading wrong:
ending x by yield is 0.0 -> 178, 0.2 -> 192, 0.35 -> 189, 0.55 -> 173, so
**more rubble gets him further** -- rubble fills the scallops and ramps him
over them. That is a knob moving the number in both directions, which this
file's own rule says means it is reading the wrong quantity. Do not tune
`dig_yield` to fix this.

---

## 1a-ii. Can a cave be dug and hold? Yes, and roof cover is what decides

The owner's scoping: *"you don't have to get him in the cave, just want to
make sure a cave can be dug and not collapse."*

**It can.** `scene=worldcrack dig=4 tunnel=N depth=D` drives a bore with
the same `rigid::mine` the sandbox cut and the gnome use. A ~145-cell
corridor at `depth=18` is still a tunnel at frame 1,800 — dumped rather
than eyeballed: solid rock roof, open passage, spoil underfoot.

```
    213 ########################################
    214 ##.###.###.###.###.###.###.###.###.###.#   scalloped ceiling
    215 ........................................   open
    218 ........................................
    219 .oo..oo..oo...oo.oo...oo..oo..oo..oo.oo.   spoil on the floor
    222 ##o###o###o###o###o###o###o###o###o###o#   floor
```

### It was a row of circles, and that invalidated the first envelope

The owner spotted it on sight. `tunnel=` spaced its bites `dig * 2 + 1`
apart, and **a disc of radius r is `2r+1` across only on its centre
line**, narrower everywhere else — so centres spaced `2r+1` apart leave
solid rock standing between every pair. Dumped, four bites came out as
four separate chambers joined only near the floor, with 2-4 cell pillars
between them:

```
    214 ####.########.########.########.####
    215 ##.....####.....####.....####.....##
    216 #.......##.......##.......##.......#
```

So every "does a tunnel hold" number measured before this was really
measuring **a row of small caverns separated by thin pillars**, which is
about the least representative geometry available and fails by crushing
the pillars rather than by dropping a roof. `step=` now defaults to `dig`
(half-overlap) and produces a continuous corridor.

**This retracts the non-monotonic bore result** recorded here earlier (24
/ 744 / 14 cells of rock for bores of 5 / 9 / 13). With a real corridor at
fixed depth, *every* bore size holds ~145 cells with **zero** rock
destroyed. There was nothing wrong with the criterion; there was something
wrong with the scene. Cross-check any other conclusion that rests on
`tunnel=` before this commit.

### What actually governs it: roof cover

`preset=flat seed=7`, bore 9, ~140-cell corridor:

| depth | rock destroyed | what the dump shows at frame 900 |
|---|---|---|
| 6 | 64 | roof gone, open to the sky, bore full of rubble |
| 12 | 402 | roof gone, partly open to the sky |
| 18 | **0** | intact roof, passage still open |

Read the third column, not the second. A 2-cell roof and an 8-cell roof
both come down *completely*; the shallow one merely has less rock in it to
lose, so `rock destroyed` reads lower for the worse outcome. That is a
metric conflating "how completely did it fail" with "how much material was
involved", and it is the same trap as counting failures instead of
damage — one level up.

**That metric now exists.** `roofed_void` counts empty cells with rock
somewhere above them in the same column — cave volume — and `min_cave=P`
gates the percentage of it still standing at the end. It catches both ways
a cave dies: fill it in and the cells stop being empty, drop its roof and
they stop having rock above them. On the three depths above it reads
**10% / 41% / 100%**, which is the ordering anyone looking at the dumps
would give, against `rock destroyed`'s perverse 64 / 402 / 0. It reads 0
on a world with no tunnel at all, which is the sanity check against a case
known to be fine.

So the behaviour is sensible: thin cover collapses, thick cover holds, and
a long gallery near the surface needs support — which is §1d requirement 1
already working.

### Gated, and the guards were seen to fail

Three acceptance cases (16 now): `cavedeep` and `cavedeep1` demand 90% of
the cave survive on two seeds (measured 100% on seeds 1, 7 and 24301), and
`caveshallow` demands the shallow one visibly fail (measured 214 overload
failures, bar 50). The pair is the point, exactly as with the room cases:
"nothing collapses" passes the first by making rock invincible, which is
how four earlier support models died. Both were run inverted and reported
`the cave did not survive -- 10% of its roofed void left (70 of 678
cells), wanted 90%` and `expected at least 50 overload failures, got 0`.

### Three measurement traps in this scene, all hit

- **The bore does not follow a slope.** `tunnel=` drives a *horizontal*
  bore at a fixed depth below the surface sampled at one x, so on hilly
  terrain it leaves the hillside: `rolling` seed 24301 starts with 124
  cells of roofed void where `flat` has 678, and `rolling` seed 7 with
  280. Cross-preset cave comparisons are therefore not like for like, and
  the acceptance cases use `flat` deliberately. Making the bore track the
  surface is the fix if hilly caves need gating.

- **Length is clipped by the world.** The bore starts at `WIDTH / 2` and
  runs right, so past ~64 bites at `step=4` it runs off the edge:
  `tunnel=70` and `tunnel=110` returned *bit-identical* numbers (22 / 388
  / 0 across three depths). Identical output across settings is the
  knob-not-connected tell. Keep total length under ~250 cells, or start
  the bore further left.
- **`depth` used to be coupled to `dig`** (`surface + dig * 3`), so a
  bore-size comparison silently varied cover as well. `depth=` exists to
  hold it.

### DONE: height is in the model, via the arch over the opening

§1d requirement 2 — *"a super short tunnel should be able to have a longer
span. ant tunnel vs digging a mine"* — is **met**, and requirement 1 fell
out of the same change. `load::arch_span`, switchable at
`World::arch_relief`.

Cave volume surviving a ~140-cell drive under 8 cells of cover, `flat`:

| bore | arch on | arch off |
|---|---|---|
| 5 cells (ant tunnel) | **100% / 100%** | 66% / 44% |
| 9 cells | 70% / 30% | 30% / 40% |
| 13 cells (gallery) | 25% / 28% | 25% / 28% |

The ant tunnel now holds a long drive outright and the gallery still needs
timber, which is both requirements at once.

**Why the model could not express this, and it is geometry rather than a
bug.** A real gallery's roof spans its *width* — a few metres — however
long the drive is, because rock either side carries it. A side view has no
"either side": a horizontal bore is a slot and its roof genuinely spans the
whole excavation, so the model was right about the span it saw. The owner's
intuition comes from three dimensions and does not transfer by itself.
Terzaghi's rock load is the standard way to put the third dimension back —
rock arches over an opening and only the arch's own span reaches the
roof — and **the owner's simplification is what made it affordable: assume
the opening is as wide as it is tall**, so the whole thing follows from
height, which a side view can see with a short downward scan. Measuring the
width would have meant walking the length of the tunnel per cell per frame.

### Three things this got wrong first, all caught by measurement

- **Relieving the mass, not the arm.** `min(mass, Hp)` never bit: under 8
  cells of cover a roof cell's mass is already below anything the arch
  would cap it at. The A/B read 30% and 39% with relief on against 30% and
  40% with it off — the knob-not-connected tell, and only visible because
  the switch made a same-binary control possible. The load was never the
  binding constraint at these depths; the *span* is, through
  `torque = M x arm`.
- **Clamping only the ceiling course.** The cells actually failing are in
  the roof *mass* above it, so the clamp reached one row and changed
  nothing on two of three bore sizes. A roof cell now looks down through
  its own cover to find the opening it roofs.
- **A 20-cell cover probe cost 2.7x frame time.** 62.56 ms against 23.06
  ms with arching off, over the 60 ms budget, on `caveshallow`. Cutting
  the probe to 8 cells of cover took it to **23.01 ms — level with
  arching off — with the discrimination table completely unchanged.**
  Relief only ever mattered at shallow-to-medium cover; the deep half of
  the scan was pure cost.

### The constant is calibrated, not derived

`ARCH_LOAD_K = 4`, swept: at k=2 everything holds (100/100/61), at k=8
everything collapses (26/30/25, i.e. the same as no arching at all), and
k=4 is the setting that separates the ant tunnel from the gallery.
Terzaghi's own table runs 0 to ~1.5 for `Hp = k(B + Ht)`; this is not that
number and should not be read as it — the arm here is `k x Ht` in a 2D
model with a fabricated width, so it is a game constant set from
measurement, per `CLAUDE.md`. Raising it lets a roof own a longer arm, so
tunnels collapse sooner.

### What it did not do

Nothing measurable to the rest of the world, which is the thing a load-model
change has to prove: 18-run seed sweeps, arch on vs off, `dig=6` identical
(max rock destroyed 3 either way) and `strike=12` slightly *better* (max
1,391 -> 1,152, cells lost max 617 -> 374). 16/16 acceptance including
`undercut` and `ligament`, which is the case that matters — the way this
change fails is by relieving every cell with a void beneath it and stopping
overhangs spalling, so `arch_span` requires rock *directly overhead* and
`an_overhang_with_sky_above_it_gets_no_arch` guards it.

---

## 1c. OPEN: a big strike unzips the surface sideways

The dig cascade and the small-strike cascade are fixed (see `df78bc7`,
`fcc9873`, both on master). **Large strikes are not.** This is what is left
of the owner's "they chain too far and too much" and it has its own
mechanism.

### Reproduction

```
target/release/examples/filmstrip.exe scene=worldcrack preset=flat seed=24301     strike=12 start=2 every=250 count=6 crop=180,190,200,120 zoom=4     out=target/filmstrips/bigstrike.png
```

`strike=` was added for this (`0aa354d`); nothing in the repo had ever struck
generated terrain before, which is why a dig-shaped fix measured clean while
the hammer was still eating the world.

| blow | rolling | flat | canyon |
|---|---|---|---|
| strike=6 (the minimum) | 0 | 0 | 48 |
| strike=12 | 40 | 12,283 | 12,662 |
| strike=20 | 12,752 | 18,502 | 8,282 |

### Look at it first: it is a *lateral* unzip, not a crater

`bigstrike.png`. Frame 2 looks right -- a clean star of fissures round a
small crater, which is the verb doing its job. Then over the next thousand
frames the damage **travels sideways along the surface layer**, well past
the blow, while the deep interior never moves at all. It is not the crater
growing and it is not a front from the impact: it is the top ten or fifteen
rows peeling away horizontally.

Read the region sizes next to it: mean 87 falling to 49, largest 835. Those
are not dust -- the pieces are reasonable. There are just hundreds of
events, 153 by frame 752 and climbing, which is why it "lasts a while".

### Ruled out by measurement

- **Crack rays.** `CRACK_RAYS` 5 -> 0, i.e. a strike that scores no cracks at
  all: `strike=12` on flat is still **10,852** cells against 12,283 with
  them. Cracks are not what scales a big blow. Do not spend time on
  `CRACK_REACH` or ray counts.
- **Detaching only near the impact** (`DETACH_REACH`, letting the crack run
  its full length but unbracing only within one bite of the blow): flat
  16,634 -> 12,283, but `strike=20` on flat went 10,541 -> 18,502. Moves the
  number in both directions, so by this file's own rule it is reading the
  wrong quantity.

### The hypothesis that fits the picture

`strike` loosens a **chip disc of radius 2r/3** -- for a radius-20 blow that
is ~400 cells that lose the 12x attachment bonus at once, and the area grows
as the *square* of the brush while the bite grows linearly. In a thin
surface layer that produces a wide, shallow, wholly-unbraced sheet.

Then the lateral unzip is `failing_region` taking a whole section: capacity
goes as section squared, so when a section fails the cell beside the new
hole has its own section cut short, drops in capacity, fails in turn, and
the process walks along the layer. Deep rock is immune because its sections
are long in every direction; a surface sheet has nowhere to go.

That is the same *shape* as the room unzip that `0b5b175` fixed with the
column-moment clamp, in a different place, and it should probably be
attacked the same way: find the quantity that is being charged to a cell
that does not own it.

**Do not start by tuning the chip radius.** Check first whether the sideways
walk is even legitimate -- a detached surface sheet genuinely has little
holding it -- and if it is, the question becomes pacing and granularity
rather than prevention: hundreds of small events over a thousand frames is
what looks bad, not the total. The owner has said explicitly that some
chaining would be *good* if it looked better.

### 1c-i. DONE: confined rock has nowhere to go — shipped

**Built and landed.** `structural::crush_in_place`, gated on
`World::crush_confined` (default on, and a runtime switch so an A/B is one
binary). A failing region with no air anywhere against it now cracks where
it stands instead of displacing. Keyed on the outcome, never the criterion,
with `a_confined_failure_still_fails_it_just_cannot_travel` guarding the
distinction from the retired anchor model.

Seed sweep, 24 runs, `scripts/seedsweep.sh <verb> confine=0|1`:

| | rock destroyed p90 | median | cells lost p90 |
|---|---|---|---|
| `strike=12` off → on | 1,754 → **1,236** | 23 → **0** | 792 → **617** |
| `strike=20` off → on | 1,907 → **1,532** | 819 → **0** | 905 → **398** |

Read the four dead ends in commit `c709b4c`'s message before touching the
crack pattern — three of them were wrong *on screen* while the arithmetic
was fine, including a lattice that drew visible graph paper across a
hillside, and one was a near no-op that produced bit-identical images
across two complete rewrites and was only ever visible to a counter
(`FailureCounts::crushed_cells`).

**Still open here:** the owner's read of the shipped version is that the
fissures are "criss cross irregular lines" rather than "a spreading crack
from a boulder". The sub-cell walker landed after that comment and is a
real improvement — cracks now curve and fork from one origin instead of
crossing as straight strokes from many — but it has not been re-judged by
the owner. **Show them `target/filmstrips/roll-crack.png` before doing more
work on the look.**

### 1c-i (original note). Why the unzip looks wrong: confined rock has nowhere to go

**The owner's framing, and it is the most useful thing said about this
defect:** *"one of the issues with unzipping is that it is stone in the
middle of a mountain falling in on itself. it doesn't look right. if it
happens in a cave and causes a cave in, or a cliff side falls over, that
makes sense — but in solid rock you should just have cracks that propagate
and maybe break rock into small pieces that for the most part stay where
they are."*

That is a physical statement and the model does not contain it. Rock deep in
a massif is confined on every side: when it fails it cannot displace, because
there is nowhere for it to move. It fractures **in place**. Material only
actually travels where there is a free face for it to travel into -- a cave,
a cliff, a crater, the hole you just dug.

Today `structural::tick` produces the same outcome either way: the region
converts to debris and the debris falls, whether it is at a cliff edge or
eighty cells inside a mountain. That is the mid-mountain collapse the owner
is seeing, and it is why it reads as fake even when the arithmetic that
produced it is right.

**The rule this suggests: key the failure *outcome* on the free space
available, not the failure *criterion*.**

- A failing region with a void adjacent to it (or beneath it) displaces:
  fragments promote to bodies, rubble falls, the collapse plays out. Cave-in,
  cliff calving, roof drop -- all still work.
- A failing region with no free face has nowhere to go: it **scores cracks**
  and at most breaks into pieces that stay put. Visible damage, no
  displacement, no debris manufactured inside solid rock.

Two reasons to expect this to help more than it costs:

1. It is the same lever that killed the dig cascade. Making failures leave
   `EMPTY` instead of rubble took every seed to zero (§1b), because a failure
   that manufactures no loose material cannot undermine its neighbours. A
   confined failure that only cracks manufactures nothing *by construction* --
   the damping is a side effect of getting the picture right.
2. It gives the crack-propagation behaviour the wiki already promises
   ("striking the same spot again drives existing fissures deeper... so a span
   you can't chew through can still be *worked* until it gives") a reason to
   fire in the one place it currently does not.

**This is NOT the retired confinement model.** `load-model-handoff.md` §6.1
retired "confinement as an anchor" -- inferring *support* from burial, which
made thick rock immune to failing at all and is on the do-not-retry list.
This is the opposite end of the pipeline: confinement decides what a failure
*produces*, never whether it happens. A buried cell can still be judged, can
still be over capacity, and can still fail; it just cannot fall into rock
that is already there. Keep that distinction explicit in any implementation,
because the two are one word apart and one of them is a dead end.

Likely also the answer to 1c's lateral surface unzip: a thin surface sheet
has a free face above it along its whole length, which is exactly where
displacement is permitted -- so the fix may narrow that case rather than
remove it, and the remaining question there becomes pacing.

---

## 1d. OWNERSHIP CHANGE: the gnome and creatures are now structural work

**Stated by the owner, and it reframes the milestone:** *"you can take over
the gnome mechanism. that and creatures are what really dig in the game they
need to work with your structural mechanics."*

So `src/sim/player.rs` joins `load.rs` / `structural.rs` / `rigid.rs` as this
area's files. The player's `D` key is a *sandbox* verb; the gnome and the
creatures are what actually excavate in play, and they are the diggers the
structural model has to be correct for.

### Where the two systems already agree

`scene=tunnel` (the gnome held-digging into a cliff, written by the M9
session) runs **zero structural failures over 1,000+ frames** with the
bearing model in place. His bore opens and stays open. Nothing here is on
fire.

### What is wrong, measured

**Pending sites climb 233 -> 11,190 and keep climbing**, on a scene with no
failures at all. That is the structural scheduler re-examining a bore that
has already settled, forever, and held-digging is the case that generates it.
`load-model-fit-review.md` predicted this shape ("a settled, standing,
motionless foreground structure costs a 12,000-cell walk on a repeating
schedule"). It is a real cost and it belongs to whoever owns this area now.

**Two diggers, two spoil models.** `player::dig` calls `rigid::mine` and then
applies `thin_to_dust(.., tuning.dig_yield)`; `App::mine` calls `rigid::mine`
and takes whatever falls out. So the gnome honours `dig_yield` and the `D`
key ignores it. Same one-number-two-verbs shape as `brush_radius` (2c).

### The owner's requirements for tunnels, stated this session

1. **Small tunnels self-sustaining; big ones need built support, like a mine.**
2. **Collapse must depend on height, not only span.** *"a super short tunnel
   should be able to have a longer span. ant tunnel vs digging a mine."*
   Today height does not enter the criterion at all -- a roof is judged purely
   as a span, which is why the engine and the intuition disagree. The textbook
   form is Terzaghi's rock load, where the load a roof carries scales with
   *(width + height)* rather than the full overburden: an ant tunnel carries
   almost nothing and can run a long way, a gallery carries much more and
   cannot. It drops into the existing capacity arithmetic.
3. **Collapse must be obvious and delayed**, so the player can get supports in
   first. Same warning-band machinery the big-strike nibbling (1c) needs.
4. **Rock and compacted soil must dig differently.** Mostly a data question,
   except soil is a `Powder` and not load-bearing at all, so "compacted soil
   you can tunnel through" wants to be its own material or state.

### Spoil: decided, and the options that were weighed

The owner asked for **vanish now, with the alternatives recorded as future
plans**. Then the M9 session turned out to have already built the better
version, so *do not hardcode vanish*:

- `Tuning::dig_yield`, 0.0-1.0, live in the tunables panel, with named
  `SpoilMode`s cycled on **F2** -- `DUST` 0.35 ("a third stays as rubble, the
  rest blows away"), `SPOIL` 0.55 ("half stays - tunnels silt up behind you").
- **Vanish is `dig_yield = 0.0`.** It wants a named `VOID` mode, not a code
  change.
- A blanket vanish inside `rigid::mine` was written, measured and **reverted**:
  it breaks `player::at_full_yield_nothing_leaves_the_world`, whose contract is
  "at yield 1.0 a dig may move material but never delete it", and it drops
  `roomcut` to 2 overload failures against its bar of 5.

Future options, recorded rather than discarded:

- **Bulking inverse** -- yield less spoil than was cut. Real rock bulks ~1.5x,
  so inverting it is one tunable. *This is what `dig_yield` already is.*
- **Eject toward the mouth** -- spoil displaced back along the tunnel axis,
  heaping outside the hole. `displace()` and `nearest_free` already exist.
- **Low repose spoil** -- debris that runs flat along the floor instead of
  heaping to the roof. Stacks with the above.
- **Spoil as a carried resource** -- mining yields the material that pays for
  the supports a long tunnel needs. Closes the loop requirement 1 opens, and
  waits for supports to exist.

### Measured tunnel envelope (with spoil vanishing, `dig=4`)

| length | rolling | flat |
|---|---|---|
| ~36 | holds | holds |
| ~72 | holds | holds |
| ~108 | holds | 10,010 cells |
| ~180 | holds | 6,785 cells |

`rolling` holds throughout because the bore sits under far more cover.
Reproduce with `scene=worldcrack preset=flat dig=4 tunnel=N`.

### Suggested order

1. ~~**One spoil model for every digger.**~~ **Done** (`3084b7b`). The
   thinning lives in `rigid::mine` now, so the sandbox cut, the gnome and
   the creatures share it; `App::mine` passes `player_tuning.dig_yield`.
   Note the sandbox mining key is **`H`**, not `D` — `D` runs the gnome
   right, and the wiki records the rebind. No `VOID` mode was added because
   `SPOIL_MODES` already has **CLEAN at 0.0**, which is the same thing under
   a name that was already shipped.
2. **The gnome cannot get into his own tunnel** — see §1a-NEW, which is now
   the owner's stated priority and blocks everything else about caves.
3. **Height in the criterion** (requirement 2). The single biggest gap
   between the model and what the owner expects.
4. **Warning band** (requirement 3) -- also fixes 1c's nibbling.
5. **The site backlog**, folded into whichever of those touches the
   scheduler. Still real and still climbing: `scene=tunnel` sits at ~11,000
   pending sites, and the big-strike scenes reach 15,000-16,000.

## 2. What is still wrong

### 2a. `wall=3 span=200` collapses untouched while 2 and 5 stand

```
./target/release/examples/filmstrip.exe scene=room span=200 wall=3 dig=0 \
    start=150 every=1 count=1 out=target/filmstrips/w3.png
```

1,064 cells, against 0 for `wall=2` and `wall=5`. **Non-monotonic, so
probably a real defect rather than a threshold** — a thicker wall carrying
a proportionally thicker roof should be monotonically safer, and
`CLAUDE.md`'s own advice is that when the same knob moves a number in both
directions the rule is reading the wrong quantity. Worth one `loadmap=1`
run to see which cell tops out and what its section is; the suspicion is
the section walk, which is where the last non-monotonicity lived.

### 2b. Rooms wider than about 200 fail at every thickness

`span=260` loses 890–3,978 cells at wall 2 through 8. That may simply be
correct — a flat stone roof spanning 260 cells at 17 thick is a 15:1
span-to-depth ratio and real masonry does not do it either — but nobody has
decided whether it is the *envelope we want*. It is a design question for
the owner, not a bug to fix silently. The honest current envelope:

| span | wall 2 | 3 | 5 | 8 |
|---|---|---|---|---|
| 100 | ✓ | ✓ | ✓ | ✓ |
| 140 | ✓ | ✓ | ✓ | ✓ |
| 200 | ✓ | ✗ | ✓ | ✓ |
| 260 | ✗ | ✗ | ✗ | ✗ |

### 2c. The dig always cuts clean through a room wall

`Tool::Room` sets wall thickness from `brush_radius` and `App::mine` passes
the **same** `brush_radius` as the cut radius. A capsule of radius r is
`2r+1` thick and a dig of radius r is `2r+1` across, so a cut severs the
wall completely, at any height, at any brush size, and no ligament can
remain. Two verbs sharing one number where the whole point is that one must
be smaller than the other.

Not fixed here because the right answer is a design call: a smaller dig, a
thicker room wall, or a doorway that is dug from the ground up over several
clicks (which is the *satisfying* answer — it makes cutting a doorway a
verb rather than a click). Ask.

### 2d. Load still concentrates on one path

The one-pixel stress line the owner reported three times is **not fixed**,
only made less lethal. Measured on an intact wall: the inner face carries
mass 1707 and the outer face 307, because the whole roof's shortest path to
the ground runs down the single innermost column. Capacity happens to
compensate (it is computed from the full 17-cell section), which is why the
room stands — but it means damage *on that path* is catastrophic while
damage anywhere else in the same wall is free. The clamp removed the worst
consequence; the concentration is still there and is still the largest
open defect in `load.rs`. Its comment at `evaluate_within` already says so.

---

## 3. What has been tried and must not be retried

Newly added this session, both recorded in `capacity`'s comment:

- **Grading the attachment bonus over the section** — attachment as the
  mean over the section's cells, so three loosened cells of seventeen cost
  three seventeenths rather than the whole 12×. The obvious
  graded-beats-binary fix. It **took `scene=undercut` to zero failures**:
  undercut spalls precisely because the rows a dig loosened are weak while
  the rows above them are not, and at a section of 6 with 3 rows loosened
  the mean reads 6.5× where the old rule read 1×. Weakness being *per cell*
  is the spalling mechanism, not an artifact of it. It also did not help
  the case it was written for — at a cut, the entire cross-section is
  loosened, so the mean equals the minimum and nothing moves.
- **Narrowing the detach footprint** (`DETACH_DEPTH` 3→1,
  `CRACK_DETACH_DEPTH` 2→1). Acceptance stayed green and the room was
  unchanged (2,595 → 2,540 cells lost), because one loosened cell in a load
  path carrying the roof's whole moment is already fatal. The footprint was
  never the driver. Both constants are back at 3 and 2.

Carried over, still true:

- **Dividing torque by the section.** Fixed a beefy block, broke
  `scene=undercut`. Peak bending stress in a section of depth D is `M/D²`,
  which the model already has right — capacity carries the `D²`, torque the
  `M`. Dividing again gives `M/D³`, and it double-counts, because a shelf's
  rows already chain independently.
- **Intact as an *exemption*.** Broke `scene=ligament`, which fails from
  geometry alone. A structure standing only by exemption has no answer the
  moment anything asks, so one chip levels a castle. It must be a
  multiplier.
- **Raising `max_unsupported_span` to hold player spans.** 16→40 with
  `attached_span_bonus` 12→2 holds terrain capacity constant and does make
  built spans stand — and stops `undercut` spalling entirely.
- **Scheduling the parent on settle.** 26 pending sites climbing to 4,064,
  frame cost 2.5 ms to 3,160 ms. The bounded in-tick chain walk replaced it.
- **Four support models** (confinement, thickness, attachment-as-anchor,
  reach) — `Reports/load-model-handoff.md` §6.

---

## 4. The measurement loop

**Two instruments, and the second one is not optional.** The acceptance
suite is blind by construction to procedural terrain — two load-model
changes have already shipped green through it — so `seedsweep.sh` runs
*before* a model change, not after.

```
cargo build --release --example filmstrip
bash scripts/acceptance.sh                     # 13 cases, mechanism-asserting
bash scripts/seedsweep.sh strike=12            # 6 presets x 4 seeds, order statistics
bash scripts/seedsweep.sh dig=6 confine=0      # any filmstrip arg rides along
```

**A failure count is not a damage count.** `filmstrip` now censuses Solid
and Powder before and after and prints
`cells lost since the cut: N (rock -X, rubble +Y)` per tile, with a
`max_lost=` gate. Read the split: rock turning to rubble is *damage* and
moves nothing out of the world; rubble leaving is *removal*. A run can chew
a whole surface layer to gravel with `lost` near zero.

Counting every non-empty cell instead was tried and lies — it reported
`canyon` *gaining* 167 cells on a run where nothing failed, because a
`Liquid` cell spreading into two half-full ones is +1 occupancy at
unchanged volume.

**Frame timings on this machine are currently very noisy.** One unchanged
scene measured 20.53-50.90 ms across three runs in one command, and the
acceptance timing bars flaked on a *different* scene each run, including
`crackflat0`, which has no structural failures at all. The mechanism
assertions were stable throughout. Re-measure the baseline in the same
session before believing any timing regression, and prefer `repeat=3`.

```
target/release/examples/filmstrip.exe scene=room wall=8 dig=1 \
    start=2 every=8 count=6 crop=100,120,280,200 zoom=2 loadmap=1 \
    out=target/filmstrips/room.png
```

`scene=room` is the reproduction: a hollow room built through
`paint_capsule_as` and cut with `rigid::mine`, exactly as `Tool::Room` and
`App::mine` do it. `wall=`, `dig=` and `span=` are separate knobs *because
the app gives two of them the same number* (§2c). **`dig=0` is the
control** — it makes no cut at all, and it is what established that the
room was collapsing untouched, which nobody had checked before assuming the
dig caused it.

Read `failures: overloaded N (M cells)` and `failing region size: mean X,
largest Y` next to the image every time. The mean alone lies: one 200-cell
break averaged with fifty 1-cell ones reads as a respectable 5, and 1-cell
failures are the shape that produces dust. `loadmap=1` prints the single
most-stressed cell with its mass, torque and capacity, and is the fastest
way to find *where* something is giving way.

**Timings:** always `repeat=2` or more, always read the minimum. This
machine has produced 60.65 ms and 52.72 ms as the slow half of pairs whose
fast half was 14.86 and 22.57 on the same scene, in the same run.

**Images:** write to `target/filmstrips/` (gitignored) and link them with
relative markdown paths — the owner's client does not render file-send
cards.

---

## 5. After this

In order, and **re-judge each rather than inheriting its justification**:

1. **§2a**, the non-monotonic `wall=3` case. Cheapest, and the one most
   likely to be a real bug rather than a design question.
2. **Ask the owner about §2b and §2c** — the build envelope, and whether a
   doorway should take several clicks. Both are design calls and neither
   should be decided quietly in a commit.
3. **Tumbling.** The owner wants "things tilted and fell over more as large
   pieces". Regions are now large and 45–62 bodies form per collapse, so
   there is finally something to tumble; check whether it already reads
   right before touching `SPIN_PER_SPEED`.
4. **§2d**, load concentration. The largest open defect, and the one the
   owner has reported most often.
5. **F3** (replay a playtest report from a world dump) — still the biggest
   gap in the loop. Every report has had to be reconstructed into a scene by
   hand, and at least two reconstructions have been wrong.
6. **C2** (mortar as a material) and doorway/window cuts on the room tool.

### Known defects not yet confirmed

- **`GRANULAR_CAPACITY_DIVISOR` may be dead code.** Flagged by a concurrent
  review: `evaluate_within` early-returns on `is_anchor`, which includes
  `rests_on_ground`. **Not verified.**
- **`filmstrip` never renders inside its timed loop**, so every worst-frame
  number in this repo's history excludes drawing. The owner found a render
  regression the harness structurally could not see.

---

## 6. Repo gotchas these sessions paid for

- **The app locks its exe.** `cargo build` fails with "Access is denied"
  while it runs; `cargo test` and building `--example filmstrip` still work.
- **The tree is worked concurrently.** Stage explicit paths, never
  `git add -A`, and check `git status` first — `Reports/worldgen-design.md`
  and `Reports/prior-art-worldgen-slicing.md` are someone else's work in
  progress.
- **Frame 0 is not a measurement.** Every scene spikes there; `filmstrip`
  excludes it deliberately.
- **A guard test must be seen to fail.** Both new room cases were verified
  in the inverted direction (demanding the standing room collapse reports
  "expected at least 1 overload failures, got 0"; demanding the cut room
  stand reports "expected at most 0 structural failures, got 30"), and the
  standing room was verified non-vacuous with `loadmap=1` at frame 300
  (stress 0.45) — because `scene=capped` once passed for two commits while
  its entire structure was frozen and had never been evaluated.
