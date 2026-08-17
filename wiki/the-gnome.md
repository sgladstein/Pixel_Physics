# The Gnome

*Current as of: this build.*

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

He treats the world the way you'd expect from standing in it. Solid rock,
plants and creatures hold him up, shallow bumps and rubble (up to a couple
of cells) are stepped over without jumping, and anything taller is a wall.
The world's edge is a wall too.

## The view follows him

The world is bigger than the screen, and while the gnome is in it the view
is his. Run far enough toward the edge of the picture and it begins to
travel with you, keeping him near the middle; at the ends of the world it
stops rather than showing you emptiness beyond the edge, so he can walk
right up to the wall while the view stays over solid ground.

Small movements do not shift it — the picture holds still while he shuffles
about and only moves once he is genuinely going somewhere, which is what
keeps a walk from feeling like the world sliding underneath him. With no
gnome in the world, the view stays where it is and the mouse works on
whatever is under it.

## Sand, water, and falling rock

Loose material is not a floor — it's something he's *in*. Walking into a
drift of sand or a spread of his own rubble, he sinks to about the knee
and keeps going, noticeably slower for as long as any of it is around him.
Deeper than that and it stops being wading: material up to his chest holds
him where he is, and material all around him is the burial described
below.

Water he swims in. Falling in, he goes under with whatever speed he
arrived with, the water eats that speed quickly, and he floats back up to
the surface rather than walking along the bottom. Under water `W` is a
stroke upward and `S` pulls him down — each a distinct pull with a beat
between, not a continuous thrust. At the surface his head comes clear and
ordinary rules resume, and there's a brief window in which `W` is a proper
jump instead of a stroke, which is how he gets out onto a bank.

Rock that is falling can be stood on. When a shelf gives way beneath him
he goes down with it, riding the slab rather than being left hanging in
the air above it — right up until it tips, which slabs do, at which point
he comes off it like anything else would.

## Digging

Summoning him puts you in his dig tool, and a yellow ring shows exactly
where the next bite will land and how big it will be. Left-click to cut.
He works the **first rock face along the line you point at** — not the
spot under the cursor — so pointing deep into a hillside cuts the near
wall of it, the way swinging a pick would, and pointing past what his
arms reach cuts at arm's length in that direction. Loose material doesn't
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

He bites at a steady rate rather than continuously, so holding the button
reads as a series of blows rather than a beam.

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
falling as if he weren't there. Two current gaps — a plant root stops a
tunnel, since he can't yet cut through growing things, and spoil that has
nowhere solid to land stays in the bore rather than being thrown into the
air.

## Changing how he feels

Three things are hard to judge except by playing, so each is a named set
you can cycle mid-game, with the active one shown in the title bar:

- **Jump feel** — how heavy he is in the air, from long floating hangs to
  short sharp hops.
- **Water feel** — whether the water lifts him, holds him where he is, or
  lets him sink so that staying up is something you do.
- **Spoil** — how much of the rock he cuts survives as rubble.

Underneath, every individual number — run speed, jump height, fall speed,
step height, dig reach, bite size, wade depth, buoyancy — is adjustable
live in the tunables panel (`O`, then page to PLAYER), the same way
explosions are tuned.

One housekeeping note: the gnome took over the movement keys, so two
older debug tools moved — dig is now `H` (it was `D`), and planting a
worm is now `J` (it was `W`).
