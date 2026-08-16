# Building should not be a physics puzzle

**Status:** proposal, not built. Written from a direct playtest steer,
after the load model shipped and building came out worse rather than
better.

**The steer, verbatim:** *"I want to be able to build and destroy
environments, but right now the building is very hard. I don't want my
constructions to just immediately fall down or to have to work at all to
make sure they are structurally stable, but I do want it to break
realistically. Maybe we need to rethink this. Also right now using a paint
brush type tool to build is not satisfying."*

---

## 0. What that sentence actually settles

It resolves the conflict `destruction-plan.md` records as open, and it
does it by rejecting a premise nobody had questioned: that the same rule
should decide both **whether a structure stands** and **how it comes
apart**.

Read the requirement again as two separate claims:

| | Requirement |
|---|---|
| **Standing** | Not a puzzle. Zero effort. A structure does not fall over because the player did not think about buttresses. |
| **Breaking** | Fully physical. Realistic, graded, load-aware. |

Those are not in tension *unless one mechanism serves both*. It has been
serving both, which is why every calibration attempt trades one for the
other — buff foreground so a span holds, and `scene=undercut` stops
spalling, because both are "foreground stone" to the model.

**And this is exactly what the prior art already said.**
`Reports/prior-art-destruction.md` §2.3: the games famous for destruction
— Noita, Teardown, Deep Rock Galactic — ship **no structural model at
all**. Every shipped stress system lives in a *building* game, and two of
the four are remembered as frustrating. Their failure mode is never "too
fragile", it is **unpredictable**. We built the frustrating kind and the
report predicted it before the playtest confirmed it.

---

## 1. The reframe

> **Material stands until something happens to it.**
> The load model decides what falls *once something has*.

Concretely: what the player places is **intact**, and intact material is
held. Not "strong" — *held*, unconditionally, like terrain. It does not
evaluate, it does not accumulate load, it does not fall down.

Damage is what makes material answerable to physics. A blow, a blast, an
excavation, a crack — all of which already exist and already strip
attachment (`detach_exposed_neighbours`, `detach_around_crack`) — convert
material from *intact* to *loose*. Loose material is where `load.rs` runs,
and it runs exactly as it does today: torque against capacity, section
failure, graded fragments.

So:

- Build a 400-cell span 20 thick, walk away. It stands. Forever. No
  buttresses, no thinking.
- Hit it in the middle. The struck region and its cracks go loose, the
  load model wakes up *there*, and the span comes down realistically —
  failing at the neck, calving into a size distribution, throwing debris.
- Undercut it. Digging strips attachment from what it exposes, which is
  already how mining works, so the roof spalls exactly as it does now.

**Nothing about the destruction model changes.** Everything built over
this session — the torque criterion, section failure, the flow over
parallel supports, cracks cutting capacity, the stress view — is kept and
is the point. What changes is *which cells it is asked about*.

---

## 2. Why this is not "player structures become indestructible"

That is the failure that killed four earlier support models, and it is
the first objection to raise. The difference is that in every one of
those, **nothing took the immunity away**. Confinement, thickness and
attachment-as-anchor were all permanent properties of shape or of a flag
that was never revoked, so a structure that qualified was immune to
everything forever.

Here the immunity is revoked by damage, and the machinery to revoke it
already exists and is already wired to every destructive verb:

- `structural::detach_exposed_neighbours` — erasing and blasting, radius
  `DETACH_DEPTH`
- `structural::detach_around_crack` — every crack a blow scores, out to
  `radius * CRACK_REACH` (added this session, and it is what makes a
  worked shelf give way)
- `rigid::strike` — loosens its whole chip zone directly

A wall you have not touched is immune. A wall you have hit is not. That is
the distinction the four failed models could not draw, and it is the same
insight that made `attached` work in the first place: **state the
difference as data rather than inferring it from shape.**

---

## 3. What this costs, and what it buys

**Buys:**
- Building becomes free-form. The stated requirement, directly.
- The calibration conflict narrows to a single number with one job. See
  §3a -- an earlier draft of this said it "disappears", which was an
  overclaim.
- **Performance improves substantially.** The load walks are the expensive
  part of the structural phase and they would run only on damaged
  material, which is a small fraction of a built world. The reported
  "performance gets bad" has a second, larger fix here.
- Terrain and player builds stop being different kinds of thing, which
  simplifies M10 streaming rather than complicating it.

**Costs:**
- A structure damaged into a state that *should* collapse but is only
  partly loosened may hold in a way that reads oddly. Mitigated by the
  detach radii, which are already tuned to be generous.
- "Floating in mid-air" is no longer automatically caught for built
  material. **Placement should still require contact** — you cannot paint
  a slab in open sky. That is a brush rule, not a physics one, and it is
  the same shape as `C4`'s "background must join background".

---

## 4. The other half: the brush is the wrong tool

*"Using a paint brush type tool to build is not satisfying."*

A freehand round brush is a **drawing** tool. It is the right tool for
terrain, sand and water, and the wrong one for building, for reasons that
have nothing to do with physics:

- No straight lines, no right angles, no repeatable dimensions. Every wall
  is wobbly, which is why the playtest structures all have organic edges.
- No sense of *placing* something. Building satisfaction comes from a
  piece going where you meant it, and a brush smears.
- It is why strike had to grow a minimum radius: the same control was
  sizing both the pencil and the hammer.

Prior art is unanimous that building games use **placement**, not
painting: Minecraft and Teardown place blocks, Besiege places parts,
Poly Bridge places members.

Proposed, cheapest first:

1. **Click-drag rectangle** — press, drag, release, fills the rect. One
   gesture, gives straight walls, floors and columns immediately. Almost
   certainly the single highest satisfaction-per-line change available.
2. **Click-drag line** with a thickness, for beams and diagonals.
3. **Shift to constrain** to horizontal/vertical/45°.
4. **A stamp**: arch, doorway, block. Later, and only if 1–3 leave a gap.

None of this touches the simulation. It is `World::paint_capsule` with a
different gesture in front of it, and the rectangle version is a small
change to `App`'s input handling plus a preview outline that
`draw_hud` already has the shape for.

---

## 3a. The cliff-edge objection, and why it changes the design

Raised by the owner before this shipped, and it is correct:

> *"we are deciding structures are solid when built, implying they are not
> true sound, then any little damage turns off the protection, the whole
> structure will just collapse, maybe?"*

Follow it through. A span stands because it is *flagged* intact, not
because it is stable. Chip one corner: that patch loses protection, was
never sound, and fails. Failing exposes the next ring, which
`detach_exposed_neighbours` promptly un-attaches, which is also not sound,
which fails. The wound eats the building. **One chip levels a castle.**

That is a cliff edge, and `CLAUDE.md`'s own ethos section names this exact
shape as something the project has already paid for twice: *"All-or-
nothing outcomes ... Real breakage is a distribution."* Binary immunity
produces binary collapse, and it would read as fake immediately.

**So protection must be a large capacity multiplier, not an exemption.**
The difference is what happens at the boundary of the damage:

- *Immunity:* an intact cell is never asked. The instant it is asked at
  all it is asked at bare capacity, and a structure that was only standing
  by exemption has no answer. Every cell the cascade reaches falls, and it
  reaches everything.
- *Multiplier:* an intact cell is always asked and nearly always passes.
  When the ring behind a wound is exposed, it is evaluated against a real
  capacity -- so a genuinely chunky wall **holds** and a genuinely
  over-reaching span does not. The cascade stops where the structure is
  actually sound, which is the graded outcome §0a demands, and it stops
  for a reason the player can see in the stress view.

This also keeps the model honest. Under immunity the stress view would
have to show intact material as *not evaluated*, which hides exactly the
information that makes the system legible -- and legibility is the thing
prior art says every disliked stability system got wrong. Under a
multiplier the overlay keeps working: an intact wall reads deep green, a
damaged patch flares, and the player can see the wound spreading and stop
it.

**What this means concretely.** `attached` already *is* a capacity
multiplier (`attached_span_bonus`, currently 12x for stone). So the change
is smaller than §1 implies:

1. The brush marks what it places intact, as §1 says.
2. `is_structurally_interesting` keeps evaluating intact material -- the
   §1 edit that skips it is **wrong** and should not be made.
3. The multiplier goes up enough that ordinary construction passes
   comfortably, and terrain's own figure is held constant by moving
   `attached_span_bonus` and `max_unsupported_span` in opposite directions
   (measured earlier: 16/12 -> 40/2 holds terrain at 1536 -> 1600).

The calibration conflict §3 claims disappears is therefore *not* fully
resolved by this -- it is narrowed to one number, the intact multiplier,
which now has only one job. That is worth stating plainly rather than
overclaiming: the previous version of this document said the conflict
"disappears", and it does not.

---

## 4a. Tried once, reverted, and what it showed

The reframe was implemented and backed out the same session. It is two
small edits -- `load::is_structurally_interesting` stops treating "next to
empty" as reason to evaluate an intact cell, and `paint_capsule_as` marks
everything it places intact -- so the size is not the obstacle.

**`scene=ligament` failed, and for the right reason.** That scene builds a
4,400-cell overhang on a thin neck and expects it to snap *from geometry
alone, with nothing touching it*. Under intact-until-damaged nothing ever
asks, so it stands. The code is behaving as designed; the acceptance case
encodes the premise this document rejects.

That is the real work item, and it is a judgement call rather than a fix:
the scene has to knock the neck first, and then assert it snaps **at the
neck** rather than at the tip -- which was always the claim worth testing,
since failing at the tip is what the reach model got wrong. Re-scoping an
acceptance case to match a deliberate design change is legitimate;
re-scoping one to make a failure go away is not, and the two look
identical in a diff. It deserves doing deliberately.

`scene=worked` also reported a 61.6 ms frame against a 60 ms bar, best of
two. Probably contention -- it measures 12-18 ms normally -- but it was not
re-checked, and should be before the bar is touched.

Everything else passed unchanged: `capped`, `undercut`, `terrain` and
`strike` were all unaffected, which is the evidence that the reframe does
not disturb the damage-driven half.

---

## 4b. The multiplier version, validated and not yet landed

Tried after §3a corrected the design, and it is **smaller than §1 says**.
Marking placed material intact is the *only* code change needed: `attached`
already carries `attached_span_bonus`, so a construction goes from 128
capacity to 1536 the moment the brush marks it -- a twelvefold buff with
no constant touched, and no exemption anywhere.

**All six acceptance cases passed**, including `ligament`, which the
exemption version broke. That is the evidence §3a was right: with a
multiplier, geometry can still fail a structure, so a scene that snaps a
neck from geometry alone still snaps it.

It was reverted only because three unit tests and two clippy lints stand
in the way and the session ran out of room to do them properly. **All
three tests assert the same thing, which is exactly the claim being
changed:**

- `structural::brushed_stone_is_foreground_and_unattached_terrain_is_not`
- `app::the_background_brush_authors_terrain_and_the_default_brush_does_not`
- `app::generated_terrain_is_structurally_real_and_still_stands`

They pin "brushed stone must be foreground, not attached". Under the
reframe that is no longer true, and their real claims -- that the brush
does not silently author *terrain*, and that generated terrain stands --
survive intact and should be restated rather than deleted.

The two lints are `paint_capsule_as`'s `attached: bool` parameter and
`NEIGHBOURS_4_PAINT` becoming unused. That is a genuine open question, not
tidying: if everything placed is intact, **what does the background brush
still mean?** The honest answer is probably "authors terrain for worldgen
purposes", which is a narrower claim than it makes today, and `B` may
deserve to become a worldgen-only tool or disappear from play entirely.
Decide that rather than silencing the warning.

---

## 5. Suggested order

1. **Click-drag rectangle build tool.** Independent of everything else,
   immediately felt, no simulation risk.
2. **Intact-until-damaged.** The reframe above. Biggest behavioural
   change; do it with the acceptance suite watching, and expect
   `scene=capped` and `scene=terrain` to be unaffected and `worked`,
   `undercut`, `ligament` and `strike` to be the ones that must keep
   working — they are all damage-driven and should be untouched by
   construction.
3. **Re-tune from a position where the two jobs are separate**, which is
   the first time `max_unsupported_span` will have meant one thing.

Step 2 should keep the stress view honest: intact material should read as
*not evaluated* rather than green, or the overlay will claim a stress
number for cells the model is no longer asking about.
