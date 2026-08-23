# The Gnome

*Current as of: 2026-08-23. New since the last pass: loose grains at
chest height no longer wall him, living plants are
walk-through and climbable (hold `Shift` to take hold — climbing has its
own key now), living creatures aren't walls either, he weaves in front of
and behind trees and a tree in front hides him, a crown breaks a fall, a
lip at the top of a jump is mantled, the sprite faces where it's going,
holding `W` out of water puts him on the bank, left-clicking **a plant
you are pointing at** shakes it while the pick sees straight through
living wood to the rock behind, and with nobody summoned WASD scrolls the
map instead. Cutting a tree down is deliberately not in yet.*

The world can be inhabited. Press `U` with the cursor over an open spot
and a small gnome — pointed red hat, green tunic — appears there; press
`U` again and he's gone. Until he's summoned, nothing about the sandbox
changes: every tool works exactly as it does without him.

While he exists, `A` and `D` run him left and right and `W` jumps. A
tapped jump is a short hop; a held one rises a good deal higher — about
two of his own heights and change — enough to clear the wall of a
reference room from a standing start. He can also still jump for a few
moments after running off an edge, and a jump pressed slightly before
landing still fires on touchdown, so movement should feel forgiving
rather than exacting.

He treats the world the way you'd expect from standing in it. Solid rock
holds him up, shallow bumps and rubble (up to a couple of cells) are
stepped over without jumping, and anything taller is a wall. Living things
are not walls — he walks through a tree and he walks through an ant — but
anything a colony has *built* is as solid as rock.
The world's edge is a wall too. A lip a little higher than a jump quite
reaches is caught at the top of the arc and pulled up over, provided
you're pressing into it and there's actually something up there to stand
on — a sheer face with no top is not something he can climb.

## Trees are scenery

A growing plant is not a wall. He walks through trunks, branches and
foliage the way a tree reads in a three-dimensional game — something you
pass, not something you bump into. Before this he wedged against the first
trunk he met with no way over, round or through, and a crown that grew
over the spot he was standing on would bury him where he stood.

**Timber someone put there is still solid.** A wall built of wood is a
wall; only *living* tissue is scenery. So building is unchanged, and the
distinction is one you can see: growing things are scenery, cut and placed
things are matter.

He also passes in front of some trees and behind others, which is purely a
matter of drawing — a tree is walk-through either way. Which side a given
tree is on is fixed for that tree's life, so a wood has a front and a back
rather than flickering. A tree in front of him **hides** him: you see
whatever shows through the gaps in the foliage and nothing else. `,`
cycles between weaving through the stand, weaving with the far trees
dimmed so the two layers read apart, always drawing over everything (what
he used to do), and always drawing behind everything.

## Climbing

The trade for a tree not stopping him is that he can go up it — but only
when you ask. **Hold `Shift` to take hold**, and while you're holding on,
`W` climbs and `S` goes back down; with neither pressed he hangs where he
is rather than sliding. Let go of `Shift` and he lets go of the tree.
Walking sideways out of the tissue drops him too.

Climbing needs its own key because it used to share `W` with the jump, and
that was wrong in a way you find within a minute of playing: every trunk
you clipped at the top of a jump grabbed you and carried on lifting, so
jump-walking through a wood was closer to hovering than to walking.

The two ways off a tree do different things. Climb off the *top* with `W`
still held and he springs off the crown; simply let go and he drops from
where he was, without the hop.

A crown breaks a fall. Dropping into foliage takes speed off in proportion
to how much of him is in it — clipping the top of one barely registers,
going through the middle of one roughly halves how hard he lands — and he
still reaches the ground. Falling into a tree and catching yourself on the
way through is a thing you can do.

## Shaking a tree

Left-clicking a plant shakes it instead of cutting it. It has to be a
plant you are actually **pointing at** — a tree merely standing between
you and where you clicked doesn't take the blow, and the pick sees
straight through living wood to the rock behind, so you can dig in a wood
and dig while standing inside a tree. There's no marker beforehand; the
shake marks itself where it lands, for a moment, as it happens.

A shake does three things and takes nothing structural. Whatever was
resting on the branches — snow, sand, your own spoil — comes off them.
Leaves that were already dying come down: the shed is weighted by how
shaded a leaf is, so a healthy sunlit crown barely gives up anything while
a shaded, crowded one rains litter, and the litter piles up on the ground
underneath as real fallen material rather than simply vanishing. And a
grown tree yields seed, which falls out of the crown and can take root
where it lands. A seedling has nothing to give — you only get seed off a
tree that has some.

Cutting a tree down is not something he can do yet.

## The view

The world is bigger than the screen, and while the gnome is in it the view
is his. Run far enough toward the edge of the picture and it begins to
travel with you, keeping him near the middle; at the ends of the world it
stops rather than showing you emptiness beyond the edge, so he can walk
right up to the wall while the view stays over solid ground.

Small movements do not shift it — the picture holds still while he shuffles
about and only moves once he is genuinely going somewhere, which is what
keeps a walk from feeling like the world sliding underneath him.

With no gnome in the world the view is **yours** instead. The same four keys
that run him — `A`, `D`, `W` and `S` — scroll the map around, stopping at the
edges of the world just as they do for him, and a readout in the corner says
where you are looking. It is measured in screenfuls rather than in cells, so
the picture slides past at the same speed however far in or out you are
zoomed.

It also **starts gently and speeds up**. A tap nudges the view a little, which
is what you want when you are lining something up; keep the key down and it
picks up over about a second, so holding it carries you from one end of the
world to the other in roughly six seconds. Let go, or change your mind and
press the opposite key, and it starts gently again — so correcting an
overshoot is a nudge rather than a lurch back the other way.

There is nothing to switch between. Summon him and the view is his again;
dismiss him and it comes back to you. The two can never both have it, which
is why the keys can be the same ones.

## Sand, water, and falling rock

Loose material is not a floor — it's something he's *in*. Walking into a
drift of sand or a spread of his own rubble, he sinks to about the knee
and keeps going, noticeably slower for as long as any of it is around him.
Deeper than that and it stops being wading: material up to his chest holds
him where he is, and material all around him is the burial described
below.

What stops him is a *bank*, though, not a grain. A few loose cells lying
across his chest — dirt caught in a canopy, the spatter from a dig, the
litter of a forest floor — he shoulders past without slowing. It takes
several of them abreast, the way the face of a drift is several abreast,
before they read as something to stop against. Before that distinction
existed a single grain of soil lodged in a tree could pen him in
completely, and in a grown wood that happened constantly.

Deep banks in a forest floor are still a wall he has no way over: he
sinks to the knee and stops, and he cannot climb *onto* loose material
the way he steps up onto rock. Dig through it, or go round.

Water he swims in. Falling in, he goes under with whatever speed he
arrived with, the water eats that speed quickly, and how he behaves after
that is the water feel you've picked — by default he sinks unless you swim.
Under water `W` is a stroke upward and `S` pulls him down — each a distinct
pull with a beat between, not a continuous thrust.

**Getting out is one continuous press.** Hold `W` to stroke up, and the
moment his head breaks the surface that same held key becomes a hop that
puts him on the bank — a smaller thing than a standing jump, sized to
clear a lip rather than to launch him out of the pond. He can also pull
himself up a low bank while still in the water, without jumping at all.
This used not to work: holding `W` was the only way to surface and was the
one input guaranteed to have nothing left by the time he got there, so he
bobbed at the edge.

Rock that is falling can be stood on. When a shelf gives way beneath him
he goes down with it, riding the slab rather than being left hanging in
the air above it — right up until it tips, which slabs do, at which point
he comes off it like anything else would.

## Digging

Summoning him puts you in his dig tool, and a yellow ring shows exactly
where the next bite will land and how big it will be. Left-click to cut.
(Point at a living plant instead and there's no ring at all — that is
the shake, described above, on the same button, and the absence is the
tell: no ring means no bite is coming.) He faces the way you send
him, and leans into a swing when he cuts, so a blow reads as a blow rather
than as something the cursor did.
He works the **first rock face along the line you point at** — not the
spot under the cursor — so pointing deep into a hillside cuts the near
wall of it, the way swinging a pick would, and pointing past what his
arms reach cuts at arm's length in that direction. A tree standing in the
way is not a rock face: his aim goes straight through living wood to the
stone behind, so you can dig in a wood, and dig while standing inside a
trunk. Loose material doesn't
stop the aim: he'll reach over a heap of his own spoil to the stone
behind it rather than chewing the muck again.

A bite doesn't erase rock, it breaks it: the stone cracks, comes apart,
throws an impulse, and leaves rubble. But breaking alone can never open a
cave — broken rock takes up the same room the solid rock did — so some of
what he cuts is pulverised to dust and gone, and the rest stays as spoil
underfoot. Roughly a third stays by default. You can change that balance
while playing, from "nothing is lost, and you can barely dig" to "the
rock simply goes"; the middle settings are the ones that leave you
walking over your own diggings in a tunnel that still opens.

That balance is not his alone — it is what *mining* does, whoever is
doing it. The sandbox cut (`H`) digs by the same setting, so a hole you
make with the cursor and a hole he makes with a pick behave the same way
in the same rock. They used to disagree: his bites opened tunnels while
the cursor's only ever loosened rock in place, which meant the tool you
tested a cave with was not the tool that dug it.

He bites at a steady rate rather than continuously, so holding the button
reads as a series of blows rather than a beam. Consecutive bites are joined
into one another, so working a face as you walk cuts a **corridor** rather
than a string of round chambers — which matters for more than looks, since
a chain of circles pinches between each pair and he is very nearly as tall
as his own bore. Before this he could dig a tunnel he then could not walk
into.

Tunnelling has consequences. Rock that was holding up an overhang stops
holding it up, and enough of it will bring the roof down — undermine a
cliff for long enough and it will come apart above and behind you.

## Being buried

Sand poured on his head usually just carries him up on top of the pile:
material that ends up inside him shoves him out of the way, upward for
preference. Bury him properly, though — under a dumped heap, or a
collapse — and he sticks, with no movement and no jumping. Knee-deep is
wading; buried to the chest is stuck.

Digging is the way out. While he's buried, clicking digs *upward*
regardless of where the cursor is, and he burrows out through the top of
the heap in a few seconds, throwing the material above him out to the
surface as he climbs. There is a limit: get him deep enough that there is
no open space anywhere near, and there is nowhere for what he removes to
go, so he stays put. Being at the bottom of a hill is genuinely bad news.

He is a visitor in the simulation rather than a part of it: sand keeps
falling as if he weren't there. Two current gaps — a root threading a
tunnel can't be cut out of the way (it no longer *stops* him, since he
walks through growing things, but the pick won't take it), and spoil that
has nowhere solid to land stays in the bore rather than being thrown into
the air.

## Changing how he feels

Four things are hard to judge except by playing, so each is a named set
you can cycle mid-game, with the active one shown in the title bar:

- **Jump feel** — how heavy he is in the air, from long floating hangs to
  short sharp hops.
- **Water feel** — whether the water lifts him, holds him where he is, or
  lets him sink so that staying up is something you do.
- **Spoil** — how much of the rock he cuts survives as rubble.
- **Tree depth** — whether he weaves through a stand of trees, draws over
  all of it, or passes behind all of it.

Underneath, every individual number — run speed, jump height, fall speed,
step height, dig reach, bite size, wade depth, buoyancy — is adjustable
live in the tunables panel (`O`, then page to PLAYER), the same way
explosions are tuned.

Underneath that, climb speed, how far he can reach to shake something, how
readily a shaken tree sheds and sows, how big a hop leaving the water is,
and how high a lip he can mantle are all individual numbers in the same
panel.

**Cave formations are walk-through too**, on the same principle as the
trees and for the same reason: a gallery hung with stalactites should read
as somewhere you can go, not as a portcullis. He weaves in front of some
and behind others, and the side a given formation puts him on is fixed for
that formation, so it does not flicker as he walks. Unlike a tree they are
not climbable — there is nothing to take hold of on wet stone — and unlike
a tree they mine: the pick takes a stalagmite the same as it takes the wall
behind it.

One housekeeping note: the gnome took over the movement keys, so two
older debug tools moved — dig is now `H` (it was `D`), and planting a
worm is now `J` (it was `W`). Those same keys now also scroll the map
whenever he is not in the world — see *The view* above.
