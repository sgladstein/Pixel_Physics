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
plants, creatures, and piled powder all hold him up — he walks along the
top of a sand pile, and shallow bumps and rubble (up to a couple of cells)
are stepped over without jumping. Taller than that is a wall. The world's
edge is a wall too. Water doesn't hold him at all yet: he sinks through
it and walks along the bottom. Swimming is planned, not built.

## Digging

Left-click anywhere near him and he digs instead of painting. He works
the **first face along the line you point at** — not the spot under the
cursor — so pointing deep into a hillside cuts into the near wall of it,
the way swinging a pick would. Click somewhere with nothing between him
and it, and he digs there; click past what his arms can reach and he
digs at arm's length. Beyond that reach the brush paints as it always
did, so he can share the screen with the sandbox tools.

What comes out of the hole is the point. A bite doesn't erase rock, it
breaks it: the stone cracks, comes apart into rubble, and the loose
material is shoved out of the bore to whatever opening is nearest —
usually the mouth of the tunnel he is standing in. Nothing is deleted, so
the spoil piles up behind him and he ends up walking over his own
diggings. He bites at a steady rate rather than continuously, so holding
the button reads as a series of blows rather than a beam.

Tunnelling has consequences. Rock that was holding up an overhang stops
holding it up, and enough of it will bring the roof down — undermine a
cliff for long enough and it will come apart above and behind you.

## Being buried

Sand poured on his head usually just carries him up on top of the pile:
material that ends up inside him shoves him out of the way, upward for
preference. Bury him properly, though — under a dumped heap, or a
collapse — and he sticks, with no movement and no jumping.

Digging is the way out. While he's buried, clicking digs *upward*
regardless of where the cursor is, and he burrows out through the top of
the heap in a few seconds, throwing the material above him out to the
surface as he climbs. There is a limit: get him deep enough that there is
no open space anywhere near, and there is nowhere for what he removes to
go, so he stays put. Being at the bottom of a hill is genuinely bad news.

He is a visitor in the simulation rather than a part of it: sand keeps
falling as if he weren't there. One current gap — a plant root stops a
tunnel, since he can't yet cut through growing things.

How he *feels* — run speed, jump height, how floaty a fall is, how tall a
step he clears, how far and how fast he digs — is all adjustable live in
the tunables panel (`O`, then page to PLAYER), the same way explosions
are tuned.

One housekeeping note: the gnome took over the movement keys, so two
older debug tools moved — dig is now `H` (it was `D`), and planting a
worm is now `J` (it was `W`).
