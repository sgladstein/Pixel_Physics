# Open bugs handoff

Rewritten at the end of the session that landed `15b2e51` … `ad1e227`.
Everything here was measured, not reasoned — where something is a guess it
says so, and where a plausible idea was measured and found wrong it is
recorded with its numbers so it is not tried twice.

Read `CLAUDE.md` first; it holds the method these bugs keep re-teaching.

---

## Open

### 0. Roofed water: `ponds` fills both sides of an overhang (worldgen)

`ponds` fills any hollow that reaches the open surface, and an overhang
(`brows` lip, or now an erosion-shaped shelf) over a flooded hollow can
leave water standing both above and below a rock shelf — water buried
under stone that the guards `generated_water_is_full_and_never_inside_
the_ground` / `every_solid_is_anchored_and_no_liquid_carries_a_stale_
fill` only catch at their 1 hardcoded seed each, so they pass by luck.
Present at `world_age 0` on a majority of seeds for several presets —
pre-existing, surfaced (not caused) by round 4's age flip; the full
measurement is finding **R4-3** in
`Reports/worldgen-implementation-tasks-2026-08.md`. Two narrow `brows`
guards shipped in round 4 close the paths that broke the structural
suite; the pattern itself needs a `ponds`-focused session with a real
seed sweep, not another guard clause. Do not widen the two named tests'
seed lists as a "fix" — they would go red on the standing defect.


### 0b. The deep massif reads as television static, and it is a per-cell palette dither (worldgen) — **FIXED**

> **Fixed 2026-08-21**, along the direction this entry names.
> `palette_family` now compares against fBm on the same `Purpose::Palette`
> stream instead of a per-cell `noise::unit` draw, so the boundary wanders
> because the field does. Measured, canyon seed 1, deep-rock crop, paired
> in one tree: **luma MAD 5.612 -> 2.216 (-61%), chroma MAD 1.775 -> 0.318
> (-82%)**.
>
> `FAMILY_DITHER_WAVELENGTH` = 40, chosen by eye from a three-point sweep:
> 14 reads as camouflage blotches, 96 as a bare curve, 40 as a coastline.
>
> **`FAMILY_DITHER_CONTRAST` is the part that would have been missed.**
> `noise::unit` is uniform on 0..1; a normalised three-octave fBm spans
> roughly 0.30..0.60, so thresholds tuned for the tails of a uniform draw
> stopped firing and `wetland` seed 1 came out with every rock cell in one
> family. Caught by `a_varied_world_uses_more_than_one_rock_family`, not by
> the author. Re-deriving the constants that read a changed quantity is
> part of the fix.
>
> **Still open, deliberately:** `strata_shade`'s separate "12% of cells jump
> a tone" rule, which this entry asks to be re-judged in the same pass. It
> is the same shape at much smaller amplitude -- brightness only, no hue --
> and now reads as rock texture rather than noise. Left rather than change
> two things at once; it is a one-line follow-up if the owner disagrees.
>
> **Measurement note, because it cost three invalid readings:** piping a
> render into `grep -q` closes the pipe on the first match and can kill the
> producer before it writes its PNG, leaving the previous run's file on
> disk. That produced a byte-identical image across three wavelengths --
> which reads exactly like "the knob was never connected" -- and a
> cross-worktree baseline no clean build could reproduce. Redirect, never
> pipe, and prefer a paired `git stash` comparison inside one tree.

The original entry, kept for the diagnosis and the mis-attribution:


Every cave render at 4x zoom shows the surrounding rock as full-contrast
salt-and-pepper speckle — louder than any cave feature in the frame, and
directly against the two things a cave picture is composed on (darkness
preserved; rock with grain and *flow* rather than noise). Images:
`Reports/img/cave-anatomy/`.

**Attributed wrong the first time, and the wrong attribution is the
useful part.** The obvious suspect was `render.rs`'s `JITTER_STRENGTH
0.12` — a per-pixel proportional brightness jitter applied at full
strength to deep rock. It was measured by setting the new
`DEEP_GRAIN_FLOOR` to **zero** (grain entirely off below the depth
ramp) and re-rendering the same crop. The picture barely moved.
`examples/pixel_stat` apportions it (canyon s1, deep-rock crop):

| | luma MAD | chroma MAD |
|---|---|---|
| shipped | 3.017 | 1.374 |
| grain floor 1/3 (now shipped) | 2.325 | — |
| grain **off** at depth | 2.090 | 1.301 |

So the render grain is **31% of the luma speckle and 5% of the chroma
speckle**. Sixty-nine per cent of it survives with the grain switched
off. On `rolling` seed 7 the chroma MAD (3.43) is *larger* than the luma
MAD (2.34) — the speckle there is predominantly a **hue** dither, which
the grain cannot produce at all (it scales all three channels by one
factor).

**The mechanism is `passes::palette_family`** (`src/worldgen/passes.rs`):
it draws `u = noise::unit(seed, Purpose::Palette, x, y)` **per cell** and
compares it against a family probability. Wherever that probability is
mid-range — which is most of the world, by design — the result is a
per-cell Bernoulli dither between two palette families that differ by
~40 brightness points *and* a large hue shift (neutral grey `128,128,132`
against warm sandstone `168,146,112`). At play scale that is confetti,
not a boundary.

**It is doing exactly what it was built to do**, which is why no test
sees it: the round-1 comment calls this "the dither band" and records
that the aridity ramps were deliberately *widened* to make it broader,
because a narrow ramp gave "solid blocks of one family" with the
families interleaving over only a few columns. The intent — a meandering
boundary between differently-coloured countries — is right. The
implementation puts the meander in white noise per cell instead of in
the field, so what should be a wandering coastline is dithered surf
everywhere.

**Direction, not yet built**: decide the family from a *continuous*
field — threshold the existing `PaletteField` fBm (plus the smoothstep
on `Character`) against a smooth spatial value rather than against a
fresh per-cell white-noise draw — so the boundary meanders because the
field does. If an interleave at the boundary is still wanted, an ordered
or blue-noise dither confined to a narrow band around the threshold
gives it without spraying the interior. Note `strata_shade`'s separate
"12% of cells jump a tone" rule is the same shape at smaller amplitude
(brightness only, inside one family) and should be re-judged in the same
pass.

**Owner's verdict, 2026-08-21, on a blind A/B of the grain grade**: *"I
see no difference. The problem is the big sharp squares that look like
giant white gray pixels."* Two things follow. The grain grade was
**reverted** — it measured a real 23% cut in luma speckle and bought
nothing anyone can see, which is the outcome the card was posted to test
(`DEEP_GRAIN_FLOOR`, reverted in the same session it landed; do not
re-attempt it as a standalone change). And the deep-rock texture
complaint is now **two** defects, not one: this per-cell palette dither,
*and* the light field's 8-cell quantisation (see 0c below), which is what
"giant white gray pixels" names. Fix 0c first — it was picked out by
name, unprompted, on a card that was not about it.

**Owned by the worldgen data track** (`passes.rs`), which is why this is
recorded rather than fixed: round 5 is mid-flight in that file. Do not
race it. **Scheduled: round 6, immediately after round 5 merges**
(owner's ruling, 2026-08-20), so the cave strips get judged twice — once
with the static and once without — and the round-5 bars are not measured
against a moving palette. `DEEP_GRAIN_FLOOR` shipped anyway on the render side — a
measured 23% cut for nothing, skip-safe — but it is **not** the fix and
must not be reported as one.

**Sanity note for whoever picks this up**: `pixel_stat` reports mean
absolute deviation from the 3x3 neighbourhood mean, not variance, so a
smooth large-scale gradient (a strata band, the depth ramp) scores near
zero and only per-pixel departure counts. Check it against a region you
know is clean before trusting it about one you don't.


### 0c. Cave light is quantised to 8-cell squares (render) — **FIXED**

> **Fixed 2026-08-21** by the near-field glow term this entry asks for, in
> `render.rs`'s `rebuild_near_glow`/`near_glow_at`. Every cell with
> `Material::glow > 0` splats a squared-linear falloff over
> `NEAR_GLOW_RADIUS` (14 cells) into a per-chunk, per-cell buffer;
> `glow_at` returns `coarse.max(near)`, gated on the coarse field being
> non-zero so the term inherits the field's blocking rather than shining
> through rock. Shipped behind `GlowShape` (`'` in the app,
> `glow=field|near` in `viewshot`) with the **new** behaviour as default,
> since this was a reported bug rather than an open question of taste.
>
> The cost trigger took a correction worth keeping: keying the rebuild on
> `glow_unsettled` rebuilt on every draw (9 in 9, measured), because the
> day/night cycle means a tile with any sky in it never settles. The splat
> depends on world cells only, so the trigger is `touched`.
> `a_settled_glow_does_not_rebuild_its_halo_every_frame` guards it.
>
> **Residual, deliberately not chased:** beyond the radius the coarse
> field's own blocks are still faintly legible on a large halo. They are
> much dimmer there; growing the radius until they leave the screen would
> cost the whole point of a *short*-range term.
>
> Note for 0b, which is still open: with the light blocks gone, what is
> left to look at in a cave is the palette static. The two were named
> together and only one of them is done.

The original entry, kept because the diagnosis is the load-bearing part:


`FIELD_SCALE = 8`: the light channel holds one value per 8x8 cells and
`field_at_bilinear` smooths between those. So a glow's smallest possible
feature is **8 cells**, its gradient is smeared over ~16, and it aligns
to the field lattice rather than to the emitter. A 1-2 cell crystal
therefore lights a **rectangle** of rock, offset to one side, with hard
vertical edges.

Named independently by the owner on two different cards: *"The
rectangular lighting looks bad"* and — on a card about something else
entirely — *"the problem is the big sharp squares that look like giant
white gray pixels"*. That is the strongest signal in the session: it was
volunteered against the question being asked.

**An earlier note in this repo called this "glow halo block-edge
softening, low priority" and was aimed at the wrong thing.** The halo is
not too hard-edged for want of smoothing; it is too *coarse* to have a
shape at all. Smoothing a 16-cell-wide blob harder makes it a vaguer
16-cell-wide blob.

**Do not fix by raising `FIELD_SCALE`** — the field is deliberately
coarse because pressure and light are low-frequency, and a finer grid is
64x the work for detail nothing else reads. The fix is a **short-range
term computed from the emitting cells themselves**, evaluated only in
chunks that contain one (`Renderer::glow_tiles` already gates exactly
this), with the coarse field left to carry the far falloff. That reads
neighbour cells, so it inherits landmine §7.22 — touched-chunk screen
rects must widen by one cell or it ships a stale-pixel class — and it is
not free; price it before building it.

**Also reverted here, so it is not retried**: an emissive-core term
(`EMISSIVE_RESTORE`) that drew a cell with `Material::glow > 0` at its
own unlit palette brightness. It gave the crystal a bright core and the
owner chose *against* it in a blind A/B, correctly: crystal's four tones
are luma ~205/224/240/250, all in the top fifth of the range, so pulling
them toward full brightness **collapses them into one white** and removes
the only facet variation the object had. Their words on the pane they
preferred: *"mostly the texture on the crystal."* A brightness lift for
an emitter has to preserve the tone spread, not compress it — and
crystal's spread is too narrow and too pale to survive one. See the
cave beauty review's round-5 verdict for the general rule (a shape needs
coherent shading; this codebase assigns per-cell random tone almost
everywhere).


### 0d. The organism support search asks the wrong question — see `Reports/felling-blockers.md`

Not new, but newly written up. `structural::organism_is_supported` anchors
on `MaterialKind::Solid` (soil is a `Powder`, so it anchors nothing) and
searches outward from the cell under test bounded by
`max_unsupported_span`, so it answers "am I within 8 hops of stone" rather
than "can I reach a root". Any structural check fired mid-crown therefore
amputates the tree — measured at 772 cells against 20,213 (`plant.rs`'s
`shed_stranded_leaves`).

**Superseded by the plant-line merge, 2026-08-22 — read this before
acting on the paragraph below.** This entry and `felling-blockers.md` were
written on a trunk that did not yet have `plant-substrate-v2`. That line
**replaced the mechanism they are about**: `structural::organism_is_supported`
no longer exists as a function anywhere in the tree (only as references in
comments), and what decides whether a plant cell stays up is now
`plant::anchor_support` plus `OrganismCell::support` — a Dijkstra run
*from the anchors outward*, once per organism tick, with **no span budget
to run out of** and an eight-connected walk matching `Grow`.

That is the same shape of fix `felling-blockers.md` §1 asks for. Both
defects it names are addressed by construction rather than by tuning: the
search no longer starts at the cell under test, so a check fired mid-crown
does not amputate, and a diagonal branch is not read as disconnected.

Two cautions against over-reading that. **It does not follow that felling
is unblocked** — `felling-blockers.md` lists other items, and it says two
of them are redesigns wearing the costume of constants; the whole report
wants re-reading against the merged tree rather than assuming one change
cleared it. And the claim below that "every organism path deliberately
schedules no check" is **no longer true**: `anchor_support` schedules one
whenever a cell's distance rises. That is safe for the reason §B records
(creature cells are discarded at `is_body_material`, and the plant path is
the one this mechanism was built for), but it means the latency argument
below has expired along with the mechanism it protected.

It is latent rather than live only because every organism path
deliberately schedules no check: growth, germination, abscission and
`player::shake` all say so in place. It goes live the moment anything
does, and it is the blocker under felling. The fix, the cost, and the six
paths that would trigger it are in `Reports/felling-blockers.md`.

### 0e. ~~A decay site does not follow its cell~~ — **FIXED 2026-08-21**

*(Was §0. Renumbered when this file merged with the trunk, which had
independently taken §0 for the roofed-water bug above. Four source comments
cite it and were updated in the same change.)*

Kept because the *reasoning* is reusable, not because the bug is open.

**Was:** a scheduled `ActiveKind::Decay` site is a bare coordinate;
`CellSurface::move_cell` touches no scheduler state; `decay::tick`
unschedules on a material mismatch, which is also what "the cell fell out of
this coordinate" looks like. So anything that moved before its first check
(200 frames) was immortal. Live for ash (fire makes it where the fuel just
burned away, so it usually falls) and total for litter (shed in a canopy,
falls every time).

**Fixed by changing *when* a site is scheduled, not by making sites follow
cells.** Decay sites are now created at the **awake→settled transition** in
`World::end_step`, riding the chunk scan `recompute_reach` was already doing
there. That is not a workaround for the strand — it is what the rule always
meant. Weathering happens to matter that has come to rest, so settling *is*
the event, and a cell that moves afterwards simply gets a fresh site when it
stops. Bounded (one chunk), rare (chunks settle once and stay settled), and
no hot-path cost.

Two riders it needed:

- `Material::decays_into` / `decay_reseeds`, so the scan gates on a `Vec`
  index at a site that already holds the `Cell` (ash → soil and litter →
  soil are both data now; ash keeps the reseed roll, litter does not).
- The dedup index extended from `StructuralCheck` to `Decay`. Without it a
  drift that is disturbed and re-settles stacks a site per settle, and since
  each rolls `DECAY_CHANCE_*` independently the decay rate would become a
  function of how often the ground was walked on — a correctness problem,
  not a performance one.

**The four candidates it was chosen over**, kept so they are not re-derived:

| candidate | why it lost |
|---|---|
| Re-schedule from `move_cell` | Its own comments call it the hottest path in the engine, and a falling cell moves every frame — it would push a site per frame of fall, each 200 frames out. |
| Have `tick` search for the cell | Bounded scan, fragile, wrong the moment two cells swap which one it finds. |
| Per-cell age in `aux` | **Cannot work.** Something must tick the age and the CA sweep skips settled chunks — a settled litter layer is exactly when decay must run and exactly when the sweep is not visiting. This is *why* the scheduler exists. Also `aux` already carries two opposite conventions. |
| Slow global sweep for decayable material | Trades a per-cell schedule for scanning the world; wrong direction with M10 streaming coming. |

**Guard:** `decay::tests::ash_that_falls_before_its_first_check_still_decays`
(was `#[ignore]`d as the reproduction, now passes and stays as the guard),
plus `litter_rots_away_instead_of_accumulating_forever` and
`a_world_where_nothing_sheds_holds_exactly_no_litter`.

**Measured after:** paired against the pre-change commit, same machine,
minutes apart — worst frame **240.60 ms vs 257.74 ms** on a settled tree
grove, i.e. no regression. Pending decay sites went **105 → 12,056**, which
is the mechanism working rather than leaking: every settled litter cell holds
one deduped site, and the count converges (8,424 → 11,671 → 12,056), so that
is a standing forest floor at equilibrium between leaf fall and decay.

**Still open, and it is cosmetic:** litter's palette was authored close to
soil's on purpose ("reads as texture, not a second canopy lying down"), and
on the close-up that looks like a mistake — twelve thousand cells of it and
it barely separates from the ground. Posted to the review queue; if it does
not read, the fix is the palette, not the mechanism.

### NEW. Plants grow nothing on generated terrain — **OPEN, BLOCKING, 2026-08-22**

**This is the one finding here that should stop a merge to `main`.** It was
found by the second integration (bringing `origin/main` at `98ac541`, 144
commits, onto the plant lines), and it fails `examples/ascii`, which CI
runs.

`ascii`'s *"ants: the foraging loop"* scene plants **six trees** on
worldgen terrain, warms up 2,400 frames, and counts `leaf` cells as the
ants' food (`ascii.rs:1393`, `food_left`). Measured:

| | `origin/main` | merged |
|---|---|---|
| leaf cells at frame 2,000 | **2,140** | **0** |
| leaf cells at frame 12,000 | **3,087** | **0** |
| ant pickups / deliveries | 712 / 683 | **2 / 0** |
| live organisms | 75 → 75 | 75 → 63 |

The assertion that fires is the scene's own: *"no ant completed the loop."*
The ants are not broken — **there is nothing to forage.** Looking at the
frame confirms it: no trees are visible at all, only nest, stone and ants.

**What has been ruled out, each by a controlled run:**

- *The creature guard in `step_organisms`* — disabled it, output was
  **bit-identical** (moves 8371, pickups 1, digs 195). Not the cause, and
  this doubles as confirmation that the guard really is inert for creatures.
- *The plant-line slot reclamation* — disabled it; organism count returns
  to 75, and the ant numbers are again **bit-identical**. It explains the
  63 and nothing else.
- *`leaf.ron`'s expanded palette* — swapped main's file in; no change.
- *Worldgen divergence* — the spawn frame is **byte-identical** between the
  two branches, so the same world, the same six trees, the same food site.
- *The merge resolutions* — `pheromone.rs` and `field.rs` are byte-identical
  to main's; `creature.rs` differs only in a `#[cfg(test)]` scene.

**The mechanism, stated with its evidence level.** `main` has **no
`absorb_water` at all** — plants there run on one currency and do not need
soil moisture. The plant line makes water a real second currency. And
`worldgen::passes` wets soil **only within two cells of water**
(`passes.rs:3146-3170`: distance 0/1/2 get capacity/fringe/fringe, and
everything else hits `continue`). So a tree planted on generated ground
away from water sits in soil at `aux == 0`, which is *dry*, and a root in
dry soil has no income.

That chain is read from the code and matches every measurement above, but
**the last link is not directly instrumented**: nobody has printed soil
moisture at those six tree positions, and germination itself also reads
moisture, so "they germinated and then starved" and "they never germinated"
are not yet told apart. That is one probe.

**This is the third instance of one class**, and the class is now the
important thing rather than any single case. §E was a hand-written test bed
at `Cell::new(soil, 0)`; the same session then fixed that bed's missing
floor; this is the same defect in **worldgen**, which is not a scene anyone
can dampen by hand. **When a merge introduces a new currency, every place
that creates the substrate it is drawn from becomes a scene that may no
longer supply it** — including the procedural ones.

**THE OWNER HAS DECIDED THE DIRECTION, 2026-08-22, and it inverts the fix.**
Stated directly:

> plants should only grow where it's wet. rain and weather should allow that
> to happen everywhere. maybe some plants slow down where and when it's
> drier. if it's not wet at the time the seeds should sit there and wait
> until it is rain and then the soil gets wet and then they germinate. we
> could always build a scene that is ideal for plant growth and stable for
> comparisons

So **dry ground refusing plants is correct and must be kept.** Do not
baseline all soil wet — that was considered and rejected. The defect is not
that the ground is dry; it is that **seeds germinate anyway and then
starve** instead of waiting.

**The good news, verified: the dormancy machinery already exists and
works.** `Behavior::Germinate`'s not-ready path sets `found_candidate =
true` and reschedules with `stale_ticks` reset, and `is_frontier` includes
`Seed`, so the retirement branches are unreachable and
`ORGANISM_STALE_LIMIT` never applies. **A seed already waits forever.** The
only thing wrong is what the predicate reads: `world.field_at(x,y).moisture`
— the field channel at the seed's own cell, which the in-code comment
correctly measured as useless. The repair is to read the **soil the seed is
resting on**, via `update::plant_available_fraction` on the already-fetched
`below` cell (`pub(crate)`, already called from `plant.rs` in three places).
Suggested threshold 0.25 — strictly above 0.0, well under field capacity,
and under every existing test bed. The RON field wants renaming: its unit
changes from the field's 0..4 scale to a 0..1 fraction.

**Three traps, each verified in code, each of which would let the mechanic
be built exactly right and still not work:**

1. **`update::soil_moisture` has no material check**, and on a `Liquid`
   `aux` is *fill*, where 0 means FULL on the same 1000 scale as
   `SOIL_SATURATED`. A seed floats (density 0.6 against water's 1.0) and
   `resting` accepts any non-empty cell, so a seed on full water would read
   **bone dry** and one on half-drained water would read well-watered. Gate
   on `water_capacity > 0` first, as `plant.rs` and `update.rs` already do
   elsewhere.
2. **Rain cannot wet the soil under a resting seed.** `weather::step`'s soak
   loop starts at the topmost non-empty cell of the column and `break`s at
   the first cell with `water_capacity == 0` — **and the resting seed is
   that cell**, since `seed.ron` declares no capacity. Zero soak reaches its
   own column; only lateral capillary flow can, and that does nothing until
   the gradient exceeds 380. **This is the same defect as F1** (litter and
   grassblade block soak for the identical reason), which makes it a class
   rather than two bugs: *anything that rests on soil and declares no
   `water_capacity` shadows the ground beneath it from rain.*
3. **The failing scene cannot rain at all.** `weather::step` runs only from
   the CA drivers, and `ascii`'s forage scene grows its six trees in a
   2,400-frame warmup that calls only `step_active_sites` + `step_fields`.
   No CA in that window means no rain, no infiltration and no capillary
   flow, so those seeds cannot germinate there under **any** wait-for-rain
   predicate. The scene needs fixing alongside the mechanic.

**What "rain wets everywhere" needs to become true.** Measured: soil aux has
three sources and exactly one sink (root uptake) — there is **no
soil-to-air drying at all**. So wet is an absorbing state, and without a
drying sink "slow down where and when it's drier" is a transient that never
returns, and the grassfire steer (§G, *"moisture vs dryness should play a
role"*) would have nothing to vary. A drying sink is the missing half of
the owner's model.

**And a knob that already exists for "only where it's wet".** `aridity` is
per-column, varies smoothly within a world, ships per preset (wetland 0.08
→ arid 0.92), and is **already read three lines below the soil-moisture
pass** to decide where trees get planted. An aridity-scaled soil baseline
would make the same number decide both *where a tree is planted* and
*whether it can drink* — which closes this bug class structurally rather
than by picking a constant. Note the collision it must resolve in the same
change: the "damp" gates for moss, decay and fire trigger at soil `aux >
75`, while plants get nothing below 180, so any baseline that feeds a root
already reads damp to everything else (decay 25x, moss up to 175x).

**Not fixed here, deliberately.** The candidate fixes are a worldgen change
(wet soil more widely, or by climate rather than by distance-to-water), a
plant-economy change (let a root draw from something other than adjacent
soil moisture), or accepting dry ground as real and giving worldgen a
reason to place trees where water is. Those are three different games, and
picking between them is a design decision, not a merge resolution.

### U. Water stress makes a tree BIGGER — **OPEN, 2026-08-22, backwards from real plants**

Measured while trying to write a replacement guard for §V, on one bed over
12,000 frames with only the soil moisture differing:

| | nearly dry (aux 310) | field capacity (620) |
|---|---|---|
| total cells | **982** | 734 |
| wood cells | **428** | 299 |

**Both are the wrong way round.** Real drought reduces total biomass, and
reduces secondary growth in particular — narrow rings in dry years is the
entire basis of dendrochronology. What genuinely rises under water stress
is the root:shoot **ratio**, and it rises because shoots suffer *more*, not
because roots gain in absolute terms. Here every quantity goes up when
water is short.

An earlier note in this file waved this through as "exactly as a real plant
raises its root:shoot ratio under drought". That was too charitable and is
corrected here: the ratio shift is real, the absolute increase is not.

**Likely mechanism, unproven.** `break_root_tips` is gated on
`water_status < 0.95`, so water stress *triggers* root re-initiation — but
the stress does not appear to throttle the carbon that pays for it. Scarcity
buys extra tissue at no cost: a compensation response with the penalty
missing. If that is right, the fix is that `water_status` should scale what
the plant can *afford* as well as what it decides to build, and the two are
currently decoupled.

**Why it is logged rather than chased.** It surfaced inside another
investigation and is not a merge regression — it is a property of the plant
economy the plant lines brought, and it needs its own pass with a probe on
carbon income under stress. Deliberately not tuned: §A is already a live
warning about re-deriving plant constants without a seed sweep, and this
touches the same `water_status` path.

### V. ~~A tree with no seedlings under it never stops growing~~ — **ACCEPTED AND RETIRED, 2026-08-22, by owner decision.**

`a_tree_eventually_stops_growing` was passing after the worldgen work and
fails once seeds wait for water: the subject reaches **1,718 cells and is
still growing at 120,000 frames**, where the recorded plateau is ~565.
Isolated by control — neutralising the germination gate alone makes it pass,
and the run takes half as long (72 s against 146 s), which is the extra
growth showing up as work.

**The mechanism, stated as the hypothesis it is.** The bed is at field
capacity, so the *subject* germinates either way. What changes is its
offspring: `Behavior::Reproduce` recruits a stand indefinitely, and a
mature tree draws the soil around it down toward the wilting point. Once it
does, its own seedlings cannot clear the 0.25 threshold, so they sit as
dormant seeds instead of becoming competitors — and the parent, now
uncontested, keeps growing.

**Which means the test's premise may never have been true.** Its name says
the tree *"exhausts its resource economy"*, but if what actually bounded it
was competition from its own offspring, it was measuring crowding and
calling it economy. That is this file's recurring shape: a guard that passes
for a reason other than the one it is named for.

**The owner accepted it:** a solitary, well-watered tree growing without
bound is correct, and a mature tree drying the ground and suppressing its
own seedlings is what a real stand does. The claim is retired rather than
tuned, and `a_tree_eventually_stops_growing` is gone.

**No replacement guard shipped, and that is the interesting part.** Two
were written and both had their premise falsified by the first run:

- *"A tree grows less on less water"* is **false as stated**. Over 12,000
  frames on the same bed, the thirsty arm grew **982 cells against the
  watered arm's 734.** Not noise: `break_root_tips` is gated on
  `water_status < 0.95`, so a water-stressed plant re-initiates root tips
  and invests in roots — exactly as a real plant raises its root:shoot
  ratio under drought. The same mechanism §A is about.
- Counting **wood alone inverts it a second way** (299 watered against 428
  thirsty), because a well-watered plant spends a larger share on foliage.

So **growth here is not monotone in water**, and any future guard must say
which quantity it means — shoot mass, total mass, or time-to-plateau — and
be measured before it is asserted. `plant_tree_on_ground_with_moisture`
exists so that comparison is one argument away when someone has a premise
worth testing. The full account sits where the test was, in
`plant.rs`'s test module.

**Unverified:** the competition mechanism is inferred from the control plus
the population count (22 live organisms at failure, which includes dormant
seeds and so cannot separate "fewer competitors" from "more waiting seeds").
The measurement that would settle it is a count of *established* offspring —
organisms past the seed stage — in both configurations.

### Z. The stand still reads as one mass — **JUDGED 2026-08-22. Two verdicts, and a metric that lied.**

Two cards, both answered by the owner, and together they settle a question
this session got wrong twice in opposite directions.

**Card 1 — the merged stand judged against the absolute standard** (one
stand, frame 28,800 at noon, "how many separate trees can you count? eight
were planted"):

> **"No. Everything has merged together into a big mass. I cannot identify
> individual trees."**

**Card 2 — a blind A/B, merged against `plant-substrate-v2` alone:**

> *"In A everything has merged together, In B two of the trees have merged
> and 2 are more seperate. Big improvement but not a full solve"* — with
> the merged stand confirmed as the better side.

**Both are true and they are not in conflict: the merge improved a bad
situation that is still bad.** The delta is positive; the absolute is a
fail. That is precisely why `tree-architecture-research.md` §7d says to
judge against a clear bole and a foliage crown rather than against the
previous frame — and this session demonstrated the trap in both directions,
first raising a false alarm from an A/B and then retracting too far from
the owner's "not wildly different".

**The metric lesson, which is the reusable part.** The absolute-standard
card reported *"crown shyness is working"* on the strength of one number:
the **widest unbroken run of plant cells above ground was 39 cells against
a 56-cell founder spacing**, i.e. no row is continuous across two crowns.
That number was correct and the conclusion drawn from it was wrong. Crowns
interleave with one- and two-cell gaps: every row breaks, and the eye still
reads one mass. **A contiguous-run metric measures whether crowns *touch*;
it cannot measure whether they are *distinguishable*.** Anyone reaching for
it again should know it has been believed once and overturned by looking.

What would actually measure the claim is unsolved here. Candidates worth
trying before trusting any of them: count connected components of foliage
at the field's resolution rather than the cell's; or measure the width of
the *sky gaps* between founders rather than the runs of plant; or simply
accept that this one is judged by eye and post the card.

**Still open**, and now with an owner verdict behind it rather than a
suspicion: the stand does not read as separate trees. The bole findings in
§Y (bottom crown band 60 where a clear-boled tree reads 0, foliage centre
58, foliage share 27% and falling with age) are the measured shape behind
it.

### Y. The gnome cannot get through the wood, and it is the missing bole — **OPEN, BLOCKING, 2026-08-22**

`scripts/acceptance.sh`'s `wood` case fails on the merged branch:

```
gnome: at (43, 189), wading, travelled 34 cells, 36/98 cells behind foliage
FAIL: expected the gnome to cover at least 200 cells, he covered 34
```

**Attribution, because it took a controlled look.** `wood` is a case
**`main` added** — the suite had 16 cases at the first integration and 17
now, and `scene=wood` is absent from `9d3176c`'s `acceptance.sh` and present
in `origin/main`'s. It came in with the second integration and fails with
the plant lines merged. It is **not** caused by the Phase 1 worldgen work:
`scene=wood` builds from `common::PlantScene`, which is hand-built at
`SOIL_FIELD_CAPACITY` and never calls worldgen. Verified deterministic —
two runs, identical to the cell.

**A reporting error of mine, corrected here.** I reported acceptance as
green after the second integration. It was not: I read `tail -2` of a file
the suite was still writing and never saw the verdict line. The case has
been failing since `f424f98` with these exact numbers.

**Not "trees are walls" — that was checked and is false.** `Footing::Climb`
and `Material::climbable`/`fall_drag` are present and identical in
`origin/main` and here, so living plants are already walk-through and
climbable. The gnome-tree work is in.

**It is litter, and the split is measured.** Disabling `shed_to_litter`
(shed leaves vanish, as they did before the ecology line) and re-running:

| | travelled | reached |
|---|---|---|
| as shipped | **34** | x = 43 |
| litter disabled | **152** | x = 161 |
| bar | 200 | |

So **litter accounts for 118 of the 166-cell shortfall.** He reports
`wading` in both runs — that state means *overlapping loose powder*, not
water, and `wade_slowdown` cuts his horizontal speed for every cell of it.
The forest floor is now deep enough in shed leaves to bog him down. Note
`wade_rows = 4` is the point where wading stops and *stuck* begins, so this
has a cliff in it, not just a slope.

**A residual 48 cells is not litter**, and is the likelier home of the
shape argument below: even with litter off he reaches 152 against a bar of
200. `PlantScene` plants its first founder at **x = 56**, so the as-shipped
gnome stops **thirteen cells short of the first trunk** — blocked by
ground-level spread rather than by the trunk itself.

That is the same defect the absolute-standard review measured from the
other end. Judged against a clear bole and a foliage crown, the merged
stand scored **bottom crown band = 60** where a clear-boled tree reads
**0** — foliage running all the way to the ground — and a foliage centre of
58, which is a mound rather than a crown. **A tree with a clear bole leaves
a gap at ground level to walk under. A tree whose foliage reaches the
ground is a hedge.** The shape measurement and the gameplay failure are one
fact, and this case is what it costs.

Worth noting the case's own comment records the same class from the other
direction: it exists because a gnome *"travelled 0 cells and spent the run
BURIED, having been entombed by a crown that grew over the spot he was
standing on."*

**OWNER'S CALL, 2026-08-22: not the plant merge's to fix.** *"If the gnome
is just sinking a little into powders we can either remove that effect or
the player can jump out. Either way it doesn't seem like your
responsibility to fix."* So this is handed to whoever owns the player: the
options named are dropping the wading slowdown for shallow powder, or
giving the gnome a way out of it. Note the plant side is not blameless —
it is the ecology line's litter he is wading in — but the *response* lives
in `player.rs`, and `wade_rows`/`wade_slowdown` are its knobs.

**Two things that are true at once, and worth keeping for whoever takes
it.** The bar
(`min_travelled=200`) was calibrated on `main`'s trees, which are a
different shape — so it is partly the water-branch problem in miniature, a
constant measured against a world that no longer exists. But *"the gnome
gets through a wood"* is a gameplay property, not a calibration detail, and
if he cannot, that is a regression whoever set the number. **Not fixed
here**: the fix is the missing bole, which is the tree-architecture
programme, not a merge repair.

### X. A desert with no desert plants — **DESIGN DIRECTION, 2026-08-22. Do not "fix" this by watering deserts.**

The worldgen soil baseline now scales `0 -> SOIL_FIELD_CAPACITY` by
`1 - aridity`, so an arid column lands near **50** against a wilting point
of **180** — plant-available water of **exactly zero**. Arid country is
genuinely dead, and for the tree that is correct: a tree should not grow in
a desert.

**But the owner's stated direction is that there should be different plants
for different biomes, including plants that can live in a desert** — and
as the engine stands **that is impossible by construction**, which is the
part worth writing down:

- `material::SOIL_WILTING_POINT` is a **single global constant**
  (`material.rs:67`), and
- `update::plant_available_fraction(cell)` takes only a `Cell`
  (`update.rs:991`) — **there is no species in scope**, so every plant in
  the world has the identical drought floor.

**State this precisely, because the loose version is wrong.** Species *can*
already differ in how they **cope** with shortage: `stomatal_reserve` sets
how early an individual closes its stomata and hoards, `drought_death` sets
how readily it sheds, and storage scales with root mass
(`water_capacity_of`). What no species can differ in is how much water it
can **get** from a given soil — that is the wilting point, and it is one
number for the whole world.

So a would-be desert plant can be given a huge reserve and no shedding, and
it will extract **not one extra drop** from dry ground; it will simply die
more slowly on the same zero income. And extraction is exactly the trait
that makes a xerophyte one: a cactus's trick is reaching water others
cannot, not enduring having none. In real biology the permanent wilting
point is species-specific — that is most of what being a desert plant
*means* — so this constant is modelling as universal a thing that should be
a trait.

**The change is small and well-shaped, which is why it is worth recording
rather than doing hastily.** One function signature gains a floor
parameter, three call sites in `plant.rs` pass it (all three already have
organism/species context: `absorb_water` at `:384`, and the two
root-scoring sites at `:3068` and `:3605`), and species files gain a
drought-tolerance field. `life_scatter` already thins placement by
`aridity`, so worldgen already knows which biome it is in — it just has
only `tree` and moss to place.

**The aridity baseline is the prerequisite for this, not an obstacle to
it.** Before it, soil was either bone dry or wet: a binary with no gradient
to adapt *along*. There is now a continuum from ~50 in arid country to ~570
in wetland, which is exactly what makes a lower wilting point worth having
— it buys a species somewhere to live that a tree cannot. **The dead desert
is the niche, not the bug.**

### W. The water-cycle branch and this one are two halves of one mechanic — **SEQUENCING DECIDED, 2026-08-22**

`origin/claude/water-phase-changes-ki6g8c` (tip `dcbdf7f`) is not adjacent
work. It builds **the half of the owner's model this branch records as
missing**: soil-to-air drying, with `SOIL_DRY_FLOOR =
material::SOIL_WILTING_POINT` and the rationale *"drying to zero would be
claiming that sunshine can do what a plant cannot"* — plus a **conserved
atmosphere** (`World::atmospheric_bank`, `spend_atmosphere`,
`storm_supply`). Their floor and this branch's zero-point are the same
number, reached independently.

**Land THIS branch first. The reason is not convenience.** Every constant
the water branch shipped — its `SOIL_SOAK_PER_DROP`, `STORM_RESERVE`,
`SOIL_DRY_PER_CHECK`, and its measured 0.76 supply equilibrium — was
measured in a world whose plants **had no `absorb_water`**, because that is
what trunk looked like when they forked. If water lands first, this branch
silently invalidates all of it. If this lands first, those constants get
re-derived against the consumer that actually exists. Note also that their
conservation tests run on **plantless** worlds, so they will pass while the
invariant is false in play.

Two further reasons: this branch is 0 behind trunk and can land now, while
theirs owes a 160-commit rebase regardless — and 11 of the 15 files a
dry-run merge conflicts on are *main-vs-water*, not plant-vs-water. And
sequential merges onto trunk beat a direct cross-merge, which would force
one person to resolve main-vs-water and plant-vs-water at once with no CI
midpoint.

**The good news, and it is substantial.** Because the drying floor is
exactly the plant zero-point, the pair is a **two-sided attractor**: bare
soil parks at exactly 180, and one rain strike (+10 at full intensity) puts
it at 190, which is immediately 2.3% plant-available. Once-wet soil then
oscillates just above the wilting point — a trickle after every shower,
nothing between. That is "some plants slow down where and when it is drier"
delivered as a **dimmer rather than a kill switch**, almost for free.

**Four things the merge must handle, in rough priority:**

1. **A resting seed shadows its own soil from BOTH rain and drying.** Their
   `is_damp_soil_surface` requires the cell above to be empty; their soak
   loop breaks at the first cell with no `water_capacity` — and a seed is
   that cell in its own column. So the planned "germinate when the soil
   below is wet" predicate would make a seed on dry ground **wait forever
   by construction**: rain physically cannot reach the one cell it polls.
   Fix before that predicate lands — give `seed.ron` a small
   `water_capacity`, or let the soak pass through a one-cell zero-capacity
   occupant. Same class as F1.
2. **`FLAG_MANAGED` goes live in production.** `Cell::is_empty` is
   byte-identical on both branches; what changes is that `rigid.rs` now
   reserves a promoted body's footprint with `Cell::EMPTY.with_managed(true)`.
   This branch's plant code was written under the explicit assumption that
   nothing promotes in production, so two deliberate raw-`EMPTY` checks
   become real: `growable()` lets roots grow **into** a floating body's
   reservation (and rigid has no demotion path), and Germinate's `resting`
   test reads a managed cell as "nothing holding this up", so a seed landing
   on one never germinates and never falls.
3. **Plant consumption is a one-way exit from the conserved cycle.** Soil
   and bank balance 1:1, but root uptake moves water into a pool the ledger
   cannot see, `transpire` vents to a non-conserved channel, and
   `absorb_water`'s Liquid arm destroys whole cells (F3). So the sky thins
   as the forest grows — *the forest that rain built drinks the sky dry*,
   over tens of thousands of frames. The cheap fix is to credit
   transpiration back to the bank.
4. **`transpire` has no wilting floor.** Bare soil can never go below 180;
   *rooted* soil can, all the way to 0, because transpiration subtracts
   without the check `absorb_water` makes. Expect dead halos around
   water-stressed stands that only bank-charged rain can heal.

**One thing the merge does NOT change:** the blocking ascii failure above.
That scene structurally cannot rain (its warmup runs no CA), so it is zero
leaves before and zero leaves after. The 10x soak cut does not touch it —
but it does make the intended fix roughly ten times more expensive: crossing
the wilting point from bone-dry goes from ~2 strikes to ~18.

### A. The slot-1 root spread has collapsed, and the first two explanations were both wrong — **OPEN, 2026-08-22**

**Found by the merge that brought the plant lines onto `main`, not by a
playtest.** Two of the plant line's own tests fail after the merge and pass
on `plant-substrate-v2` alone. Both were controlled, so this is measured
rather than suspected.

| test | `plant-substrate-v2` alone | + main (step 2) | + ecology (step 3) |
|---|---|---|---|
| `a_tree_eventually_stops_growing` | plateaus at 565 cells (~frame 50,000) | **1,929 and still climbing at 120,000** | **passes again** |
| `root_and_shoot_branching_read_different_slots` | 336 vs 448 root cells, a 33% slot-1 spread | 411 vs 437, a 6% spread | **440 vs 448, a 1.8% spread** (bar is 10%) |

Controls: both pass on `plant-substrate-v2` alone in 35 s; every figure
above reproduced bit-identically across runs, so this is deterministic and
not load or seed noise.

**The termination failure fixed itself when the ecology line landed on
top**, which is worth more than the fix: it says the missing quantity was a
*sink*. `plant-ecology-design` sends abscised foliage to `litter` instead
of deleting it, and with that the tree plateaus again. So growth was not
running away because income rose without bound — it was running away
because nothing was taking mass back out. Whatever is done about the row
below should be judged against that, not against a carbon number in
isolation.

**What is left is the slot-1 spread, and it is getting narrower, not
wider** — 33% → 6% → 1.8%. The trait still orders root mass in the right
direction at every step; what has collapsed is the *size* of the effect,
which is what the bar was set to detect. That is the shape of a signal
being swamped rather than a mechanism being broken.

**Two explanations have now been offered and BOTH are falsified. Read this
before proposing a third.**

*First explanation — "main rewrote `field.rs` by +553/-44".* True, but it
was a diff statistic dressed up as a mechanism. It named no code path.

*Second explanation — "main added weather, so it rains into scenes
calibrated dry".* This one had a real mechanism behind it and survived a
code review: `weather::step` is the first call in both drivers
(`update.rs:76`, `parallel.rs:104`), the plant harness `run_with_fields`
drives `update::step`, and `root_slot_run`'s bed is open sky. Every step of
that is true.

**It is still wrong, because it never rains during this test.**
`weather::step` reads `at(world.seed, world.frame)`; `root_slot_run` pins
`w.seed = 1` and starts at frame 0. `weather::at` is a pure function of
those two, so the question is answerable without stepping a world at all —
which is what `print_dry_window_for_the_slot_seed` (this file's own control,
in `plant.rs`'s test module) now does:

```
seed 1: frames 0..12000 — 0 of them precipitating (0%)
  epoch 0 (frame 0):     None  intensity 0.00
  epoch 1 (frame 7200):  None  intensity 0.00
  epoch 2 (frame 14400): Rain  intensity 0.83   <- the first rain, after the test ends
```

**The first precipitation at seed 1 arrives at frame 14,400, and the test
stops at 12,000.** Rain cannot be the cause. Neither can evaporation (the
bed holds no `Liquid` at all, and `Material::evaporates` is Liquid-only), nor
the soil-moisture ratchet, which needs rain to ratchet.

**Third explanation, and this one is measured rather than argued.** A
paired `plant_probe species=tree trees=8 frames=12000` on
`plant-substrate-v2` against the merged tree — same scene, same harness,
one run each:

| | substrate-v2 | merged | |
|---|---|---|---|
| plant cells, mean | 3440.9 | 3435.0 | **unchanged — it is not income** |
| **root** cells, mean | 288.2 | 219.8 | −24% |
| **root** cells, max | **745** | **287** | −61% |
| root cells, range | 114–745 (6.5x) | 129–287 (**2.2x**) | **the spread collapses** |
| uptake / tick, mean | 16.46 | **27.43** | +67% |
| water stock, mean | 657.9 | 440.4 | −33% |
| **stomatal term, mean** | **0.90** | **0.96** | **crosses a threshold** |

Read the last row against `ROOT_REINITIATION_STATUS = 0.95` (`plant.rs`),
which `break_root_tips` tests as `if status >= 0.95 { return }`.

**`break_root_tips` is the amplifier, and main's world switches it off.** It
re-initiates a `RootTip` from mature root tissue, once per organism per
upkeep tick, and it is **genotype-blind** — slot 1 does not reach it. On
`plant-substrate-v2` the mean stomatal term is 0.90, under the gate, so it
fires routinely and multiplies the stepping lineages that *consume* primed
sites; that is what turned a difference in priming density into a 33%
difference in root mass, and what produced the 745-root outlier. In the
merged world plants take up 67% more water per tick and meet demand at 0.96
— over the gate — so the amplifier stays shut, root systems shrink and
converge, and slot 1 is left moving only the supply of sites in a plant that
no longer converts many of them.

Note what this is *not*: not more carbon (plant size is identical to within
0.2%), not rain, not evaporation. It is water reaching roots **more
efficiently**, which is a change in main's field/soil path — the same
`field.rs` rewrite the first explanation gestured at, but now with the
specific quantity (uptake per tick) and the specific consequence (a gate
crossing) attached.

**Judged by eye, 2026-08-22, and the alarming reading was wrong.** A
before/after of the stand was rendered at a dry noon frame and put to the
owner. The session's own reading was that the merged canopy had closed into
a continuous slab and the roots into a surface mat -- i.e. the "canopies
merge into a slab" failure `Reports/tree-architecture-research.md` exists
for. **The owner's verdict, given directly:** the new trees look *"a little
different, fatter merging a bit more, not wildly different"*, and the roots
*"a little different but not obvious given the plant to plant variability
that already exists."*

That is a much smaller claim than the one it replaced, and it changes what
this entry is. The −24% root mean and the collapsed max are real numbers,
but they do **not** cash out into a player-visible regression: they sit
inside the spread twelve identical genomes already produce (31 to 153 cells
in the recorded census). So §A is a **test-calibration problem**, not a
symptom of the world looking wrong -- which is the right prior for anyone
deciding how much to spend on it.

Recorded here because the session that rendered it argued itself into the
larger claim from two real measurements plus one picture, and only the
owner's eye cut it back. The card is
`20260822T081525474Z-8c4bc2` on the `plants` board and is still open in the
queue; this verdict arrived in conversation rather than through the tool.

**Still not confirmed, and this is the measurement that would do it:** a
direct count of `break_root_tips` firings per run on each branch. The
mechanism above is inferred from a threshold crossing in an aggregate mean,
and a mean can cross while the distribution that matters does not. Counting
the firings is a `#[cfg(test)]` counter at `plant.rs:3017` and one paired
run — the `S8E` atomic-array pattern in the same file is the template.

**The lesson worth keeping, because it cost two wrong answers.** Both
explanations were reached by reading diffs and reasoning about mechanism,
and the second one passed an independent code review. The thing that
settled it was a **pure function evaluated over the exact seed and frame
range the test uses** — a control that took one probe and no world stepping.
Ask "does this mechanism fire in *this* run" before "could this mechanism
cause this".

**Deliberately not fixed by the merge session.** The two available fixes
are re-deriving `tree.ron`'s constants against main's field model — a
retune over procedural content, which `CLAUDE.md` says wants a seed sweep
first and is a design decision — or moving a bar that was set from
measurement. Both are the owner's call. Recorded here so the next session
does not re-derive the diagnosis.

**The cheap next step, and it is now a different one.** Since weather is
excluded, the question is which of main's *remaining* changes moves a
plant's carbon income in a rain-free world. `examples/plant_probe.rs` runs
on both branches unchanged, so a paired
`plant_probe species=tree trees=8 frames=12000` on `plant-substrate-v2`
against the merged tree measures exactly that, in two runs and no code
change. If the merged trees are simply bigger, the income hypothesis holds
and the field solver is the place to look; if they are the same size, the
cause is inside the root pass and the priming sweep becomes the next move.

**What is *not* wrong:** the merge resolutions themselves. The slot
allocator, the species registries and the scheduler dedup sets were each
audited against both parents; the only real defect found was a scene error,
below.

### B. ~~`anchor_support` runs over creature organisms, unguarded~~ — **FIXED 2026-08-22. Churn, not damage; the guard is in.**

**A collision only the merge could produce.** `plant::anchor_support`
arrived on `plant-substrate-v2`; ants, beetles and worms arrived on `main`;
neither line ever had both. `plant::step_organisms` iterates
`world.live_organism_ids()`, which is **every** organism in the shared
generational storage — creatures included — and `anchor_support` guards
only on `state.cells.is_empty()`.

Creature cells really are in that map: `World::reindex_organism_cell`
inserts into `OrganismState::cells` for any organism whose id a cell
carries, not only plants.

So for a creature: `is_structural_anchor` wants a `Solid` 4-neighbour (the
`root_tissue` arm cannot fire — creature materials do not
`reinforces_powder` and a creature cell is not a `RootTip`), an airborne or
soil-surrounded creature reaches none, every cell settles at `u16::MAX`,
and since `was` defaults to 0 the `dist[i] > was` arm fires
`schedule_structural_check` on **every creature cell, every organism
tick**.

**A correction, because the first version of this entry got the reason
wrong.** It said the sibling pass `accumulate_support` is safe because it
returns early on `state.collar_y == None`, "which a creature never has."
**That is false after one organism tick.** `organism_upkeep` also runs over
creatures unguarded, and its census walk sorts every cell that is not a
`RootTip` and does not `reinforces_powder` into the shoot branch — which is
every creature `Head`/`Segment` cell. It then writes `shoot_cells`,
`collar_y` and `shoot_top_y` onto the creature's own `OrganismState`
(`plant.rs`, the `state.collar_y = collar_y` write at the end of that walk).
So from a creature's **second** organism tick, `collar_y` is `Some(...)` and
`accumulate_support` runs its full BFS on it too.

The consequence is still churn rather than damage, and the reasons are
elsewhere: creature species declare no behaviours, so nothing dispatches;
`settle_water` at demand 0 returns status 1.0; `break_root_tips`,
`break_buds` and `allocate_to_frontier` all bail on the missing `Grow`
entries; and `transport` builds `Plant`-kind topology only, so a creature
has no faces. But **six** plant passes run over every creature, not one,
and each writes something.

**It is wasted work, not damage — and that correction matters.** The
first reading of this was that it risked the amputation `CLAUDE.md` warns
about, where a structural check fired mid-organism converts everything past
the span limit to deadwood. **It does not**, and the reason is one line:
`structural::tick` bails at `is_body_material`, which is
`Solid | Plant` only, and creature materials are `kind: Creature`. Every
site scheduled this way is discarded on arrival. Nothing is converted,
nothing is broken free, no creature is taken apart.

Two further limits, both checked: `nest.ron` is `kind: Solid`, so ants
standing on the nest patch *are* anchored and never schedule at all; and
`schedule_active_site` dedups against `pending_decay_sites`' sibling index
for structural checks, so a cell cannot stack duplicates. The cost is
bounded, not unbounded.

So what is left is **scheduler churn on exactly the colony scenes the
creature line added its cost instrumentation for** — a structural check
enqueued, popped and thrown away, per creature cell, per organism tick.
That is worth a guard, and it is worth *not* being alarmed about.

**Evidence level: the mechanism is read and verified** — the iteration is
unguarded, `reindex_organism_cell` really does put creature cells in
`OrganismState::cells`, `OrganismCell::default()` really does reset
`support` to 0 on every move, and the discard really is unconditional.
**Not measured:** how many sites this actually costs per frame in a live
colony. `World::live_organism_count` and the existing creature counters
would say in one run, and nothing has asked them.

**Fixed as described**, in `plant::step_organisms` after the cadence check:
one species lookup, keyed on the **`creature` field** — the declaration of
intent — skipping all seven plant passes for creature organisms.

Two things it deliberately does not do. It does **not** key on `collar_y`:
per the correction above the plant side sets that on creatures itself, so
such a guard would switch itself off on the second tick and still look like
it was working. And it does not guard `anchor_support` alone, which would
have fixed the one pass this entry was named for and left five others
running.

**The slot-reclaim arm must stay outside the guard.** It is the one part of
`step_organisms` that is genuinely for every organism, and a creature death
path that empties a cell list between ticks relies on it. A bare `continue`
at the top of the loop would leak the slot and resurrect
`pixel-physics-issues.md` #8, which is the bug this whole allocator exists
to close.

### C. ~~`grass` and `creeper` root branching is running a retired model~~ — **MEASURED AND CLOSED, 2026-08-22. Both knobs fire. Do not zero them.**

**A legitimate question with the opposite answer, kept in full because the
reasoning that produced the wrong prediction was good reasoning.**

The concern: `plant-substrate-v2` measured that a root tip's *in-tick*
`branch_chance` roll cannot be funded — the tip must hold two steps' carbon
at once, and over 351 tree root steps the gate opened **twice** and the roll
fired **zero** times. It replaced the mechanism with `branch_priming` and
set root `branch_chance: [0.0]` in tree, conifer and shrub.
`plant-ecology-design`, developing in parallel, authored `grass` (0.4) and
`creeper` (0.05) with no `branch_priming` at all. The two lines edited
different species files, so this auto-merged in silence.

The prediction, from the shared gate: grass *might* differ, creeper is
"near-certainly inert" since its `cost: 0.25` gives it the tree's exact
≥0.50 gate and its 0.05 sits beside the 0.04 that never fired.

**Measured instead of argued, by running each species paired against its own
`branch_chance: [0.0]` and comparing output — deterministic, so identical
output would have proved the knob dead:**

| | as-shipped | knob zeroed | verdict |
|---|---|---|---|
| grass (sod bank test) | **137** grassroot cells, crest +27% | **55** grassroot, crest +23% | **fires hard — zeroing costs 60% of the mat** |
| creeper (`plant_probe`, 8 plants, 12k frames) | mean 204.1 cells | mean 202.5 cells | **fires weakly — 3 of 8 individuals moved, ~0.8% mass** |

**So both knobs work, and both proposed fixes were regressions.** Zeroing
grass's roll would have cut the fibrous mat its `reinforces_powder` bank
depends on by more than half. Zeroing creeper's would have been a smaller
regression, but a regression.

**Why the prediction failed, which is the part worth keeping.** The
inference transferred the *gate* (identical `cost` ⇒ identical ≥2× bar) and
silently assumed the *economy* transfers with it. It does not. A tree's
0.053 mean-held carbon was a 2,400-cell canopy's income diluted across a
large, distant frontier; grass's whole shoot photosynthesises, its frontier
is ≤22 cells, and its cost is 0.15 — a ≥0.30 bar it clears routinely. Even
creeper, which really does carry the tree's 0.25 cost, clears it sometimes,
because a ground-hugging plant's source-to-root path is short.

**A measurement on one species does not transfer to another through a shared
constant.** The constant was shared; the economy that has to pay it was not.

**Left as it is.** Nothing to fix. What remains open is a *documentation*
gap rather than a defect: `grass.ron`'s comment justifies 0.4 by comparison
to "a tree root's 0.04", a value that no longer exists anywhere — worth
rewording to cite this measurement instead, next time that file is touched.

### D. ~~Two smaller things the merge exposed~~ — **BOTH RESOLVED, 2026-08-22**

**E1. The repaired creature bed is damp but still has no floor and no
walls.** `eating_one_leaf_does_not_kill_the_tree_that_grew_it` fills soil
into `y=150..159` of a `0..199` world and plants on top. Soil is a
`Powder`, nothing floors or walls the bed, so it avalanches ~40 rows to the
world floor and the seed rides down with it. The test passes — dampening
the bed was enough to make the tree leaf — but it passes *despite* the
scene, not because of it. `plant::tests::plant_tree_on_ground` walls **and**
floors its bed, with a comment saying this exact error has cost time twice.
**Fixed 2026-08-22**, once the review pointed out that a test passing
*despite* its scene hands the next change a bed that does not stay where it
is put. Floor **and** walls, matching `plant_tree_on_ground` — a floor alone
still lets an open-sided bed spill off its own edges, which that helper's
comment records as having cost time twice. Still passes, in 2.96 s.

**E2. A bar in the ecology line's sod test predates the substrate line's
root economy.** `sod_crest > bare_crest * 1.10` is justified in-file by a
paired same-session measurement (bare 185 → sod 235, +27%, 135 `grassroot`
cells in the bank). Those runs happened on `plant-ecology-design` before
the stomatal reserve, the primed-site conversion and the root
`branch_chance` supersession existed — all three of which move how much
`grassroot` the sod arm grows, which is the quantity the margin is made of.
**Re-measured 2026-08-22 on the merged tree, and the provenance is
restored**: shed bare 327 / sod 305 (-7%), crest bare 185 / sod 235 (+27%),
**137** grassroot cells against the recorded 135. The recorded pair
reproduces almost exactly, so the bar is still a measurement of the system
it guards. Worth knowing *why* it barely moved: the sod scene is short and
its outcome turned out to be insensitive to everything the merge changed —
which is itself the reason the §C probe below had to vary the knob directly
rather than trust this test to reveal it.

### E. A test scene can outlive the economy it was written for — **FIXED 2026-08-21, kept for the reasoning**

`creature::tests::eating_one_leaf_does_not_kill_the_tree_that_grew_it` built
its bed as `Cell::new(soil, 0)` — and `aux == 0` is *dry* on a `Powder`.
That was fine while a plant ran on one currency: `main` has no
`absorb_water` at all. The plant line makes water a real second currency
with a real source, so a root in dry soil has **no income**: the tree grew
wood, never a leaf, and the test failed on its scene rather than on the
organism-freeing behaviour it is named for. Dampened to
`SOIL_FIELD_CAPACITY`, matching `plant::tests::plant_tree_on_ground`, which
has always done this — passes in 3.26 s.

Same class as the moss scene `main` repaired when evaporation landed, and
the third time `CLAUDE.md`'s "a scene that contradicts the code will look
like a bug in the code" has been paid for. **When a merge brings a new
currency, every scene that grows something is a scene that may no longer
supply it.**

### F. Cross-line seams neither branch's tests exercise — **OPEN, 2026-08-22**

Two plant branches developed for 111 commits while main added creatures,
weather, evaporation and a rewritten field solver. **The merge conflicts
were the easy part** — an independent three-way review found no runtime
defect in any of them. The risk is where a plant-line mechanism meets a
main-line one, which no test on either side covers. What follows was read
off the merged source; where something is measured it says so, and where it
is inference it says that too.

**F1. A litter blanket blocks rain from reaching the soil — LIVE, verified.**
`weather::step`'s soak loop walks down from the surface and `break`s at the
first cell whose `water_capacity == 0` (`weather.rs:482`, whose own comment
explains it as "a puddle on bare rock does not wet the rock beneath it").
`litter.ron` and `grassblade.ron` declare **no** `water_capacity`, so it
defaults to 0 — and cell `aux` is the only channel roots drink from. A
column topped by shed litter therefore takes **zero** soak. Real mulch
conserves soil moisture; this does the opposite, and the blanket deepens
fastest exactly over rooted ground, where root `deplete_moisture` also holds
the field dry enough to slow litter's own decay. *Measure:* paired storm
over littered vs bare soil, summing soil `aux` after one epoch.

**F2. Snow defoliates canopies through the shade-death rule — LIVE, inferred.**
Snow is placed one cell above the topmost non-empty cell, which for a treed
column is the crown; snow is non-empty, so it attenuates light; `tree.ron`
leaves carry `shade_death: 0.003` rolled as `0.003·darkness³` per organism
tick. A snow epoch is ~100 organism ticks. Nobody designed deciduous
winters; they may be lovely. *Not measured, and the field's 8x8 block
resolution may blunt a 1–2 cell snow cap.* *Measure:* paired leaf census
across a snow epoch vs a clear one, same seed.

**F3. Root drinking destroys water unconserved — LIVE, verified by reading.**
`absorb_water`'s Liquid arm sets the adjacent cell to `Cell::EMPTY` and
credits at most `rate` — the cell's remaining fill is destroyed, not
transferred. That was tuned on branches where ponds never evaporated; main
added evaporation drawing down the same ponds. Nothing tallies held water,
so the loss is silent. *Measure:* pond volume vs time, 2x2 over
tree/no-tree and weather/no-weather, plus a conservation tally on that arm.

**F4. Grass cannot die, and the guard that would have caught it was removed
— LATENT, and it goes LIVE the day grass is plantable.** Both abscission
rules gate on `CellType::Leaf`; grass has `plastochron: 0` and never makes
one, so it has no shade death, no drought death, no age death. Separately,
the "do not germinate on another plant" guard was deleted on the explicit
argument that a mis-sited seedling "is shed leaf by leaf by
`drought_death`" — a cleanup that does not exist for grass. A grass seed
landing on a branch, a stone, a litter drift or a nest roof would germinate,
never root, and stand forever, holding an organism slot (reclamation
requires an *empty* cell list). At the 4,095 ceiling `push_organism`'s range
check is a `debug_assert` and `encode_organism_id` does not mask, so the
index would bleed into the generation bits — **silent organism identity
corruption in release**. Today worldgen plants only `tree` and moss and the
brush only plants trees, so nothing reaches it. *Measure:* count organisms
with `root_cells == 0 && shoot_cells > 0` on a grass stand under canopy at
30k frames, and the slot high-water mark.

**F5–F8, in brief.** Grass seeds are ant food and a nest-dropped seed loses
its organism id, so a colony beside a sward is an unbounded larder and a
sink on grass recruitment (LATENT with grass). Decay's settle-scan schedules
a whole chunk's cohort at the same `next_frame` where evaporation
deliberately staggers by a position-derived phase — a 200-frame comb, cost
not correctness, unmeasured at forest+pond+storm scale. Soil `aux` has three
sources and exactly one sink (root uptake): there is no soil-to-air drying,
so unplanted soil ratchets toward field capacity across rain epochs
(*measure:* sum soil `aux` over 10 epochs on a plantless world — monotone
non-decreasing confirms it in one number). And `reinforces_powder` does not
stop digging, only avalanching, so ants can hollow a sod bank into a lattice
that never collapses.

### G. Grassfire arrives with a standing negative verdict — **OPEN, inherited, 2026-08-22**

Not a merge regression: it was built and judged on `plant-ecology-design`
before the merge, and the merge carries it forward unchanged. Recorded here
because a rejected mechanic that nobody tracks gets rediscovered.

The owner's verdict, in full, on the review card *"Grassfire: does a fire
front across a meadow read as its own regime?"*:

> **"The fire looks bad. Just looks like you are cycling colors. It also
> doesn't spread at all (if we are going to do this, moisture vs dryness
> should play a role."**

Three separate claims in that, and they want separating before anyone
works on it: the *look* is wrong (colour cycling rather than a fire front),
the *behaviour* is wrong (it does not spread), and there is a design steer
(**moisture vs dryness should gate spread**) which is a mechanic that does
not exist yet. The last one is the interesting one, and F1/F8 above are
about exactly the moisture channel it would have to read.


### 1. Whiskers on a spreading front (the remaining half of "banding")

One-cell-tall sheets of water with open air above *and* below, drawing as a
comb of detached horizontal ledges along a spreading front. Reported from
live play. Distinct from the row banding that was fixed — that was a fill
deficit *inside* the body; this is the shape of its *edge*.

Barred, not fixed: `update.rs`'s
`a_spreading_front_does_not_shed_a_comb_of_detached_ledges`, at 400 against
a measured 290. The bar holds the line; it does not claim a fix.

**Three candidates measured and rejected** (numbers in the test's own doc):

| tried | result |
|---|---|
| Disable `find_lateral_descent` | −75% whiskers, and water reads as sand again — the original bug |
| Land the mover at `(tx, y)`, fall next frame | whiskers 2540 → 1635, but enclosed holes 289 → **1040** |
| Shrink `LIQUID_LATERAL_REACH` | pure trade against levelling, no path to zero: 24/12/6/3 → whiskers 290/175/151/119, levelling 343/557/1017/1661 frames |

**What the measurements say about the cause**, which is *not* what it looks
like: `find_lateral_descent` is not teleporting water. **75% of its moves
are a single-cell diagonal step and only 3% land with air two cells below**,
and whiskers survive at reach 3. So they are not primarily long jumps. They
look instead like the surface monolayer advancing one diagonal step per
frame with nothing above it to refill the row it vacates — which raises the
real possibility that the honest fix is not in the movement rule at all, but
in how a one-cell-thick sheet is *drawn*. See the grain prototype below.

Note this is **not** the VOF flotsam-and-jetsam the liquid research reports
diagnose. Their fix is a three-cell height function for partial-fill
droplets orphaned by interface reconstruction; measured here, the drained
basin strands 54 cells while producing **zero** films, and the films
elsewhere are mostly *full* cells. Do not adopt that fix without first
measuring which mechanism is producing the cells.

### 2. Sand-into-water displacement

Unchanged from the previous handoff and still the design gap it was.
`abffff2` is **kept** — the decision was made explicitly with numbers:

| metric | before `abffff2` | now |
|---|---|---|
| water rise | **29 rows/frame** | **1 row/frame** |
| sand/water/sand stripes | 41 | **1379** |
| sand cells with air beneath | 86 | 115 |

Water crossing 29 rows in one frame is a gross physics violation; the
striping it traded for is ugly. **Option 1 from the old list (sideways-
preferring displacement) was implemented as a mass-conserving 3-cycle and
measured: it does nothing** — stripes 1379 → 1370, stall unchanged, and it
*regressed* water rise to 2 rows/frame. Reverted, not committed. It cannot
work as specified: inside a pool there is no free-or-lighter cell beside the
mover, so the sideways path only opens where the blob is already at a free
surface, which is where striping was never the problem.

The striping follows from two individually-correct premises — displaced
material moves at most one row per frame, and displacement is a straight
vertical swap — so no local `try_move` tweak can remove it. Remaining
options: let an unsupported refused mover fall (fixes the 115 floating cells
only), move a coherent body *as a body* (`rigid.rs` — the only thing that
removes the premise), or accept it.

### 3. Scheduler under-enforces `max_active_tips` — **FIXED, and the tripwire earned its keep**

**Resolution (2026-08-17):** the tripwire fired exactly as this section
predicted it would — the session multiplicative crowding stopped crowded
tips from dying, simultaneous tips finally approached the cap, and the
under-enforcement measured 19 against 14. Fixed by the route this section
also predicted: `organism_active_tip_count` counts the organism's own
cell list (Decision 2's sidecar, maintained at the `World::set` seam under
both drivers) instead of scanning the schedule heap, so in-flight
dispatch is no longer invisible. That took the overshoot to 16, and the
remainder was a second gate nobody had needed before: `break_buds`
creates frontier too and never checked the cap — `supportable` is now
throttled by `max_active_tips`, one gate for both creators. The tripwire
test asserts the cap holds through 8,000 frames and passes.

The original finding, kept because its reasoning about *why it could not
bite yet* was correct and is the reason the tripwire existed at all:

### (was) Scheduler under-enforces `max_active_tips` (a tree bug) — measured, and it cannot bite yet

Review finding. `scheduler::step` pops the entire due batch into `due_sites`
*before* dispatching any of it, so `world.active_sites` does not hold the
batch while `plant::tick` runs. `organism_active_tip_count` counts only the
heap, so it cannot see any tip in the current batch — and when a tree's tips
all come due on the same frame, which is the normal case, the count it
returns is far too low and `Behavior::Grow`'s cap (`max_active_tips`, 14 and
10 in `tree.ron`) is under-enforced. **The reading is correct.**

**Now reproduced properly, and the answer is that the cap is unreachable.**
The previous attempt "grew no tips at all (`plant_tree` on a soil floor with
no field step)" — germination is light-gated, so a run that never steps the
field never germinates and can only ever report zero. With fields stepped
(`plant.rs`'s `a_trees_simultaneous_tip_count_stays_within_its_species_cap`,
8,000 frames), the **peak simultaneous `GrowingTip` count for one tree is
1**.

Not "under the cap" — one. Tip retirement converts a `GrowingTip` to
`MatureBody` in the same tick it grows, with the child carrying the frontier
forward, so a lineage holds exactly one live tip and branching only briefly
makes it two. `max_active_tips: 14` was sized for the pre-retirement system
where tips persisted; against the current one it has nothing to do.

So the bug is **real as read and unreachable as built**: a cap that is never
approached cannot be exceeded, however badly it is checked. Deliberately
*not* fixed on that basis — the fix (dispatch-one-at-a-time, which changes
the cap's meaning and risks a tip producing a due-now tip in the same frame;
or making the in-flight batch visible to the count) costs more than the
defect currently does.

**What changes that:** `Reports/plant-substrate-v2-design.md`'s bud break
(retrofit step 9) exists specifically to let a mature tree open new
frontiers, and is the first thing that would push simultaneous tips toward
the cap. The reproduction above is kept as a tripwire and should start doing
real work exactly then. Decision 2's sidecar also fixes it structurally for
free — `organism_active_tip_count` becomes a count over the organism's own
cell list rather than a scan of the schedule (design doc §3e), which has no
in-flight-batch blind spot at all.

### 4. Levelling is O(width²)

Not a bug so much as a known cost, quantified here because the previous
handoff's numbers were read before convergence and were wrong:

| frame | 1024-wide pool tilt | wall clock |
|---|---|---|
| 8,000 | 29 cells | 2¼ min |
| 40,000 | 3 cells | 11 min |
| 70,000 | 1 cell, asleep | 19 min |

It **does** converge flat and **does** sleep — there is no limit cycle, and
the earlier "residual tilt" figures were mid-convergence readings. A 512
world (the sandbox's own width) is ~4x faster: near-flat around 2 minutes.
The real cost is CPU, not appearance: the visible defect is gone early and
what persists is chunks awake doing invisible fill shuffling.

This is what the heightfield bodies exist to fix (O(width) instead), and
they are blocked on the promotion gap below.

### 4b. ~~A cell alone in the air drops its column's skyline~~ — **CLOSED, by removing the inference entirely**

Logged and closed in the same session. It was the tail of "shade under a
tree is way too intense": the skyline was the topmost non-empty cell, so
anything in the air above a column made everything below it draw as the
inside of a cave.

Fixed by not inferring it. `World::sky_surface` records the top of the
ground once, on the world's first frame, and nothing revises it —
`Reports/underground-definition.md` has the reasoning and the numbers.

**What is worth carrying forward is why every inferred version failed**, and
it is a case of `CLAUDE.md`'s "when a rule must tell apart two things that
can look identical, state the difference as data". Four shapes have to be
distinguished — a hill, a shaft someone dug, a roof someone built, and a
grain in mid-air — and from the world as it stands they are the same
arrangement of cells. Measured on the last inferred version, which took the
topmost cell and then repaired any column with higher ground within six
either side:

| shape | verdict |
|---|---|
| one floating cell | 20 rows of cave under it |
| plank 1 to 51 wide | identical to the floating cell |
| shaft ≤ 12 wide | tunnel (correct) |
| shaft ≥ 13 wide | open daylight 35 rows into the mountain |

No reach setting fixes that: the repair rule had a width threshold in one
direction and no rule at all in the other, and mining is the activity that
walks a shape across exactly that threshold. The difference between "I dug
this" and "this is a hill" is *history*, not geometry, and history has to be
stored.

### 5. Automatic promotion — blocker removed, still not ready

`promote_liquid_body` is called **only from tests**, so `liquid.rs` — the
pipe solver, the seam, ~1000 lines — never runs in play and every bug in it
is latent.

**The documented blocker is now fixed.** `127e177` reverted automatic
promotion because "the persistent-flux solver has no mechanism to drive an
internally-level body to expand into open floor space beside it", and
`edge_with_room` is that mechanism (`95c917f`, `68371d7`). A promoted body
that can still spill no longer sleeps through it and sheds its edge column
back to the CA, which is what §6c always said outflow should be.

**But promotion is still not worth turning on**, measured on the exact
scene the revert names — the 100-column block from
`a_wide_deep_water_column_levels_out_instead_of_only_eroding_at_the_edges`,
promoted deliberately at frame 0:

| | spread at 6000 |
|---|---|
| before the fix | **106, frozen from frame 10** |
| after the fix | 57–68, still moving |
| no promotion at all (plain CA) | **128** |

So the freeze is genuinely gone — the body sheds steadily, 100 columns and
4.9M fill down to 50 and 2.45M — and it still ends up *worse* than leaving
the water to the CA. Shedding one column per `DEMOTE_COOLDOWN_FRAMES` is
simply slower than the CA spreading it directly.

### 6. The heightfield does not deliver the speed it was built for

**Measured, and it inverts the premise the whole subsystem rests on.**
Report r2 §5's argument for the heightfield is a *speed* one — "levels a
pool in **O(width)** rather than the current O(width²)". Levelling time to
the 2% flatness bar, on a walled basin with water spanning every column
(the shape most favourable to the body — it never has to spread, only
redistribute):

| columns | CA | promoted body | ratio |
|---|---|---|---|
| 50 | 77 | 204 | 2.6x slower |
| 100 | 307 | 742 | 2.4x |
| 200 | 1,323 | 2,421 | 1.8x |
| 400 | 5,659 | 6,864 | 1.2x |

The CA quadruples per doubling — O(width²), as documented. **So does the
body** (3.6x, 3.3x, 2.8x per doubling). It is not O(width). The ratio is
closing, so a crossover presumably exists somewhere past 400 columns, but
the sandbox's world is 512 wide and the heightfield never wins on speed
inside it.

The persistent-flux solver was supposed to avoid exactly this — §7a's
"flux must be persistent state, **or you have rebuilt diffusion**". The
measurement says diffusion is what it behaves like. Whether the flux is
not persisting, or a clamp is throttling the wave, is unknown and is the
thing to look at first.

**What the body does measurably win at is accuracy, not speed**: it
finishes at a flatness of **1** where the CA leaves **11**, because
`terminal_snap` solves the exact analytic equilibrium. That is a real
property and worth something — it is just not the property the subsystem
was justified by.

Before spending anything more here, settle what the heightfield is *for*.
If the answer is exactness, it is much cheaper to reach that another way.
If it is speed, the flux solver needs diagnosing against §7a first, and
nothing downstream (promotion criteria, the trigger, the deferred B-8/B-2/
B-6/B-7 bars) is worth building until it delivers.

Two bugs found while measuring this are fixed: a body shed down to one
column stranded its fill instead of handing it back (`94a0c12`), and
`edge_with_room` always picked the left edge, so a body spread in one
direction only (`68371d7`).

The promotion *criteria* question — promote only once contained, since §4a
already argues quiescence is the wrong gate — is now moot until the above
is settled.

---

## Closed this session

- **Chunk-seam cliffs** (powders) and **terracing** (liquids), both from the
  chunk-by-chunk sweep order. `FLAG_UNDERCUT`. The previous handoff's
  leading hypothesis (seam cells never getting `flowing()`) was **measured
  false**.
- **Dark lines on horizontal chunk seams.** Fixed by sweeping chunk rows
  bottom-first (`pass_key`) rather than by penalising the crossing cell —
  two attempts at the latter were reverted, because they replace the tear
  with a *throttle* at the same seam (2236 and 1948 summed row-fill deficit
  against 988 for correct ordering).
- **Chunks awake but never swept.** `is_settled` now answers from
  `sweep_region`.
- **Four of five review findings**: liquids scanning through a promoted
  body's cells; explosions spawning debris made of `material::EMPTY`;
  `try_extend` freezing CA water it did not claim; `absorb_liquid`
  destroying fill at a body's edge. The fifth is §3 above.

`particle::step`'s landing check was flagged by the same review and
**deliberately left alone** — the reasoning is recorded in place.

---

## Awaiting a decision

### ~~The plant model bounds height and does not bound width~~ **FIXED**

**Resolved by path-length turgor** (`OrganismCell::path_len`): the gate now
reads hydraulic distance from the collar, stamped at creation, instead of
`collar - y`. `a_tree_eventually_stops_growing` passes in 61s where it
previously ran its whole 120,000-frame budget and failed. `plant-branch-angle`
is merged. Kept below because the measurements are the reproduction, and
because the *reason* it went unnoticed for so long is reusable.

---


Found by measurement while building branch angle and the internode
straightness budget, which sit **unmerged** on branch `plant-branch-angle`
with `Reports/branch-angle-and-the-width-bound.md` beside them.

`plant.rs`'s turgor gate is `let height = (collar - y).max(0)`. That is
purely vertical, so a cell two hundred columns sideways at collar height has
`height = 0` and full margin. **Nothing in the model bounds lateral
extent** — width is limited only by self-shading and crowding, which is
enough in a tall scene and nothing in a shallow one:

| single tree | outcome |
|---|---|
| planted with 20 rows of sky (what `a_tree_eventually_stops_growing` uses) | **never plateaus** — +180–400 wood per window at frame 295,000, 24,946 cells |
| planted with 190 rows | plateaus at frame 180,000, flat for six windows |
| `PlantScene`, 200 rows | `MatureBody` identical at 120k / 200k / 300k |

Wide branch angles did not create this; they made lateral spread efficient
enough to reach it. It matters more once M10 streaming makes worlds wide.

**The fix it argues for** is bounding turgor by *path length from the
collar* rather than by height: water potential falls with the hydraulic path,
not with altitude, so a 200-cell horizontal limb is under the same
constraint as a 200-cell trunk, and one quantity change bounds both axes
with the mechanism already in place. The cost is that path length is not
tracked per cell today, and the property that made height attractive — it
never equalises when growth stops — has to be shown to hold for path length
too (it plausibly does; that is an argument, not a measurement).

Blocks: merging `plant-branch-angle`, which otherwise measures well and
appears to fix the conifer lean (handoff §4).

---

Five `GrainMode` variants are prototyped behind a runtime switch, default
unchanged, with GIFs generated for comparison (`examples/filmstrip.rs`,
`grain=`). They address the report that a pool reads as *static* in the
middle while its edges move — the grain is keyed on world position, so water
flows through a pattern nailed to the screen.

Worth knowing before choosing: a settled pool changes 431 cells per step
with **zero occupancy changes**. Its interior genuinely does not move. So
`Cell` grain makes moving water *read* as moving, which it currently does
not, but nothing can animate an interior that is standing still — `Muted`
and `Animated` are the variants aimed at that half, and `Animated` is the
only one that costs the dirty-rect render skip.

---

## ~~Open~~ **CLOSED** — the three the polarity review raised (M18 plant v2)

All three are now fixed, each with a guard verified to fail against the old
code. Kept here rather than deleted, because what they have in common is
worth more than any one of them: **all three were invisible to the suite
for the same reason — nothing tallies held water, and nothing walked the
frontier cell types.** A new test that covers either of those covers a
whole class.

| finding | fixed in | guard |
|---|---|---|
| allometry gate permanently retiring roots | `ab39721` | `a_root_tip_that_ages_out_retires_instead_of_becoming_a_phantom` |
| `Grow` into soil destroying stored water | (next commit) | `a_root_growing_into_soil_displaces_its_water_rather_than_destroying_it` |
| capillary exchange over-filling a neighbour | `13bce0a` | `capillary_flow_never_pushes_a_neighbour_past_its_own_capacity` |

Two of them turned out differently from the review's framing, and the
difference is recorded at each site:

- The root bug was **not** fixable by marking the "not now" gates as
  `found_candidate`, which is what the framing suggests. That breaks
  `a_tree_eventually_stops_growing` immediately — the staleness counter is
  the only thing that makes growth terminate. The real defect was that
  ageing out had no landing site for `RootTip`.
- The capillary bug needed a **second water-holding material to be
  testable at all**. With equal capacities the drier cell is by definition
  below its own limit, so the clamp can never bind. The guard writes a
  `tightsoil` into a temp dir and loads it additively.

The original descriptions follow, since the reproductions are still the
cheapest way back into each area.

### 1. `MAX_ROOT_FRACTION` feeds the staleness counter, permanently retiring roots

`plant.rs`'s allometry gate `continue`s without setting `found_candidate`,
so a *transient* root:shoot ratio counts as a failed tick. After
`STALE_LIMIT` blocked ticks the `RootTip` stops rescheduling — and
`organism_upkeep` skips frontier cell types, so nothing ever visits it
again. It loses `Absorb`/`Transpire` permanently while still counting
toward `root_cells`, which ratchets the very ratio that blocked it.

The gate is meant to say "not now", which is the "temporary shortfall"
framing `Divide`'s own resource gate uses — that path sets
`found_candidate` and this one does not. Suspect this first if roots look
like they stop drinking on a mature tree.

### 2. `Grow` into soil destroys the soil's stored water

Growing a root into a penetrable soil cell overwrites the cell wholesale,
replacing its `aux` — which for a `Powder` is moisture — with cell-type
bits. In the `forest` scene each root cell silently deletes
`SOIL_FIELD_CAPACITY` (620) units; a 100-cell root system loses roughly 62
water cells' worth. No conservation tally covers held water, which is why
nothing noticed.

Note this interacts with the still-open `water_capacity` item below: any
liquid-conservation test taught about held water will start failing here.

### 3. Capillary exchange can push a neighbour above its own capacity

`update.rs`'s capillary step bounds the transfer by *this* cell's
`water_capacity` and writes `there + moved` without checking the
neighbour's. Latent today because `water_capacity` is opt-in and only
`soil` has it, so every exchange is soil-to-soil with equal capacity. It
goes live the moment a second water-holding powder exists with a different
capacity — which is exactly what widening `water_capacity` to sand would
do.
