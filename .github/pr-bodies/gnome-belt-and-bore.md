**The gnome carries three tools now, and the pick's default cut is a
passage rather than a hole.** M9 shipped one verb aimed with a free cursor;
this is the same milestone's *verbs* finished — one for getting through the
world, one for breaking it, one for cutting what is alive.

Play-facing: [`wiki/the-gnome.md`](../wiki/the-gnome.md#what-he-carries).
Build notes: README's *M9 status* gained a dated subsection.

## 1. The belt

`1`/`2`/`3` or the middle mouse button. A two-line HUD in the top-left names
all three with the held one lit, the cursor wears that tool's colour, and
the sprite carries the implement in the same colour — so "what will this
click do" is answerable from the middle of the screen, where the player is
looking, and not only from the corner where its name is printed.

| tool | verb | what it is for |
|---|---|---|
| `Pick` | `rigid::mine_rect` / `mine_swept` | getting through the world |
| `Hammer` | `rigid::strike` | breaking it — and cracking far more than it breaks |
| `Axe` | `mine_swept` at a third the radius, half the yield | cutting what is alive |

The split is real rather than three sizes of one swing. The hammer *removes*
less rock than the pick and *damages* far more of it — cracks reach
`radius * CRACK_REACH` and licence structural failure nearby — so it is what
brings a ceiling down and a poor way to dig a corridor. The cursor draws
both: a bright ring for what it takes, a dim one for how far the cracks
reach.

**The keys are shared, not stolen.** Every letter was already bound
(`main.rs`'s own note: "`Y` — the last free letter"), so `1`-`4` are the
gnome's only while `Tool::Dig` is up. That is mutually exclusive by
construction, the same argument `pan_camera` records for `WASD`: under
`Tool::Dig` the left button is his swing and the brush lays nothing down, so
the selected material cannot affect anything a click does. `Z` leaves the
tool and the palette has its keys back.

**One recovery timer for all three** (`Player::swing_cooldown`, charged to
whichever struck). Two would have made the belt an exploit — strike, switch,
strike again at the sum of both rates — and carrying all three would have
cost nothing. The HUD's swing bar draws it, because held digging is a
sequence of blows at a fixed rate and a player who cannot see the rate reads
a cooldown as the tool being unresponsive.

## 2. The bore

`bore_rect` sizes a box off `PLAYER_WIDTH`/`PLAYER_HEIGHT` plus a cell of
clearance (9x16 today, derived rather than written down — he has been grown
twice on playtest notes already), sites it in one of four cardinal
directions from the cursor, and draws it before it is cut. `bore_slice`
takes a `bore_bite` slice off the working face per stroke, so holding the
button drives the face through the box; the near slice is shaded so one
press is legible against the whole passage.

The cursor picks a **direction**, not a point. That is the whole difference
from the free bite, and it is what makes the preview a promise instead of an
estimate — the same gesture always produces the same box, and a corridor
comes out straight without the player steering it cell by cell.

Horizontally the box's floor is level with his feet, not centred on him: the
two cells of clearance go over his hat where headroom is, and the corridor
floor runs continuously out of the ground he is standing on. Centred instead
it cuts two rows out from under the floor, and every stroke forward becomes
a step down.

`DigStyle::Free` (`4`) is the old round bite, unchanged, and is the **only**
shape a buried gnome cuts: the bore is sited outside his rectangle, so under
a pile it would clear a room next door and leave him exactly as entombed.
`player::dig` sends a buried gnome down the free path whatever style is
selected.

## 3. Three obvious constructions, each wrong, each caught by a test

All three are in `Reports/dead-ends.md` with the condition their rejection
depends on.

**The slice anchored on the box's near edge.** The first stroke clears it
and every later one cuts air, so **a held button drove the passage exactly
one stroke and then stopped**. The passage advanced only if the player
walked forward between presses — a rule nothing on screen states, and which
reads precisely as the tool having broken.

**The box anchored flush against his body**, on the argument that a bore is
at arm's length by construction. A wall twelve cells away then got twelve
cells of air cut at it and a hole reported that was never made — which is
`Tool::Dig`'s own standing rule broken from the inside: a reach may bound
*where* a verb lands and must never decide *whether* it happens.

**Loose material siting the box** — `face_toward`'s recorded spoil-shielding
failure in a second costume, and the one worth reading. A cut leaves a
`dig_yield` fraction behind as spoil, and deep in a massif that spoil has
nowhere to be thrown, so the next stroke sites on the muck a cell in front
of the rock and **the bore grinds on its own spoil for ever**:

| | |
|---|---|
| a 144-cell box after 16 strokes that should clear it twice over | **80 cells still standing** |
| a single shaken-loose grain of sand | sited a whole passage **five cells short** of the wall it was aimed at |

The second was caught by an *existing* test —
`app::a_click_on_a_tree_shakes_it_rather_than_cutting_or_painting`, which had
nothing to do with the bore. `footing`'s `Hard | Scenery` sites the box now,
the same three-way split `face_toward` uses, with a fallback to anything at
all so a dune or a lone tree is still diggable.

## 4. Felling needed no new physics

The cut goes through `shatter_to_rubble`, which unregisters each cell from
its organism as it lands, and `plant::anchor_support` re-walks the plant on
its next tick and finds whatever the cut severed unreached. Chop a bole
through and the crown comes down as pieces by the machinery README's
*Felling status* already describes. What was missing was a verb aimed at the
trunk.

The pick keeps its two verbs — pointing at a plant still *shakes* it — and
that stops being a compromise now that the axe exists: shaking is what you
do to a tree you want to keep.

`creature::slay` puts the axe's kill on the engine's own death path (a
corpse, the energy ledger closed, the organism slot freed) rather than
beside it. An erase would have been two lines and wrong in three ways
`creature_dies` already gets right.

## 5. Counters, because the picture cannot tell these apart

`rigid::strike` returns the cells it acted on — pulverized plus loosened,
counted by what *changed* rather than what was attempted, so a decline on
unbreakable rock is not reported as a hit. A swing at air, a swing at
bedrock and a swing that calves a slab are otherwise the same picture.
`Chop` reports whether the stroke landed on living tissue.

`filmstrip scene=smash` and `scene=chop` print both beside the tile, and
**share their beds** with `scene=tunnel` and `scene=shake` — so each pair is
a controlled comparison with the belt as the only variable. `dig=bore|free`
does the same for the two cut shapes on one scene.

## 6. Guards

Four new ones, each **watched going red for the fault it names** before its
green was cited:

| guard | fault put back |
|---|---|
| `one_stroke_is_a_slice_and_not_the_whole_box` | slice anchored on the box edge |
| `a_buried_gnome_digs_out_in_bore_mode_too` | bore allowed while buried |
| `the_belt_shares_one_recovery_so_switching_is_not_a_second_swing` | a per-tool timer |
| `a_click_on_a_tree_shakes_it_rather_than_cutting_or_painting` (existing) | loose material siting the box |

Plus: the bore's direction, its floor alignment, that a cut passage is one
the *movement code* says he fits down, that a shaft sinks him, the hammer's
positive control and null (a blow into a face breaks something, a swing at
open sky breaks nothing) and its recoil on both, that the axe cuts living
wood where a shake leaves it alone and still chips rock, that `swing`
dispatches on the belt, and that the swing bar empties and refills.

The ten existing free-hand dig tests are pinned to `DigStyle::Free`, so they
keep measuring the mechanism they were written for rather than silently
starting to measure the bore — and passing, which is worse.

## Known limitations

- The axe **kills** a creature it lands on rather than damaging one. There is
  no partial creature in this engine to leave behind.
- The hammer's recoil is horizontal-dominant by construction: the vertical
  share is halved, or a downward blow launches him off his own floor.
- The belt rides shared keys, as above.
