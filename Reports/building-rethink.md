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
- The calibration conflict disappears. `max_unsupported_span` no longer
  has to be small enough to spall an undercut roof *and* large enough to
  hold a player's lintel, because only damaged rock is ever asked.
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
