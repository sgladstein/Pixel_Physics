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

He is a visitor in the simulation rather than a part of it: sand keeps
falling as if he weren't there, and material that ends up inside him
shoves him out of the way — usually upward, so a stream of sand poured on
his head tends to carry him up on top of the pile. If he's ever sealed in
completely, with nowhere to be pushed, he simply sticks: no movement, no
jumping, until something opens the way out. (Digging himself free is
planned; for now the brush or the mining key is the rescue.)

How he *feels* — run speed, jump height, how floaty a fall is, how tall a
step he clears — is all adjustable live in the tunables panel (`O`, then
page to PLAYER), the same way explosions are tuned.

One housekeeping note: the gnome took over the movement keys, so two
older debug tools moved — dig is now `H` (it was `D`), and planting a
worm is now `J` (it was `W`).
