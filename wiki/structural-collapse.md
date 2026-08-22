# Structural Collapse

*Current as of: 2026-08-21. Where something breaks — at the neck, across
a section, sharing load between supports — is settled behavior, and a cut
into a building now takes the corner it was made in rather than the whole
building, and a very large collapse now arrives in stages that spread out
from the break rather than in one frame. Loose material piled on rock now
weighs on it, which is new and is what makes a blasted cave roof drop its
own muck through — expect to find that in play before anywhere else.
Digging into open ground no longer spreads: a hole is a hole,
and the rock around it stops where the damage stops. What is still in flux
is **how big a room you can build**: a wide, thin-walled one can still
fail to hold its own roof up the moment you finish it, and the exact width
at which that starts is not yet where anyone wants it. Also unsettled, and
the thing to watch when playing: whether collapse now happens **too
rarely** on open terrain, having spent a long time happening far too much.
And the answer to what was the open question at the top of this page last
build — *does the hillside settle around a wound, or quietly eat itself?* —
came back as **eating itself**, and is now fixed. **Rock cut loose deep
inside solid stone still cracks where it stands, but it cracks along the
same grain a blast finds rather than along wandering lines of its own.**
Two things follow that are worth watching in play. The breaks a collapse
draws are now one cell wide, straight and closed, matching the web a blast
leaves rather than sitting on top of it as a darker tangle. And the world
**goes quiet**: what came away at the moment of the bang is unchanged, but
the trickle of small, apparently random collapses that used to go on for
ten or twenty seconds afterwards has stopped — a charge settles a few
seconds after the flash and then holds still. The trade is real and is the
thing to judge: rather less rock comes away *in total* than in the build
before this one, because much of that total used to arrive long after
anyone was still watching. A piece smaller than one block of the grain has
no joint inside it and so cracks nothing at all, which is the same rule as
before wearing different clothes. **The setting that decides how far
damage travels (`F9`) now actually does something, which it largely did
not before** — see the last section, which is rewritten. Three things
changed there: the setting survives loading a new world (it used to
quietly go back to the default while the title bar kept naming what you
picked, so anyone comparing settings across a re-roll was comparing the
default with itself); it now bounds *what a collapse eats* rather than
only where the question gets asked; and trees obey it, which they never
did at any setting. The thing to judge in play is the trade the second of
those buys, which is real and is described below: at the tighter settings
a collapse can now stop part way and leave rock standing that is holding
nothing up.*

Any solid structure needs an unbroken path of connected material leading
back down to the ground, or out to the edge of the world, to stay up. Cut
that path and whatever's on the far side of the cut is no longer supported
— but being connected isn't the whole story.

What actually decides whether something holds is how much is hanging off
a given point, and how far out it reaches, weighed against how much that
point can actually carry. A tower's own weight sits directly above its
base no matter how tall it gets, so a column can stand at heights nothing
about its footprint would suggest. A shelf or overhang reaching sideways
is different — the farther it reaches, and the more weight sitting at the
far end of that reach, the more strain lands on wherever it's rooted. Rock
doesn't fail wherever happens to be geometrically farthest from support —
it fails specifically at the point carrying the most strain, which is
usually a narrow neck holding up something much bigger than itself.

Weight can also be shared: a structure resting on two separate legs
actually splits its load between them, rather than the whole weight
routing through whichever leg the game happened to pick first.

Walls are not judged the same way as the roof they carry. A wall passes
weight downward, and it is asked how thick *it* is, not how far the thing
on top of it reaches — so a wall does not become fragile just because the
roof above it is ambitious, and a chip taken out of one halfway up does
not bring the building down. What the roof's reach strains is the roof.
Wide flat spans are therefore the thing to watch when building: a long
roof wants to be thick, or to be an arch, or to come down on a wall part
way along.

A crack that goes all the way *round* something now genuinely cuts it off,
including deep inside solid rock where there is no open face anywhere near
it — which is what the paragraph below has always claimed and, until
recently, was not quite true of a crack drawn at an angle.

Hitting rock scores fissures into it that run well past the material the
blow actually removes, and they stay. Cracked rock carries less than
intact rock, and striking the same spot again drives the existing fissures
deeper rather than scribbling fresh ones somewhere else — so a span you
can't chew through can still be *worked* until it gives. A crack is also a
place support cannot cross, so a fissure carried all the way through
something separates it.

When something does give way, a whole section breaks free at once rather
than peeling off one thin sliver at a time, and it reads as chunks rather
than a spray of grit. Different kinds of rock also come apart into
differently sized pieces.

A really big collapse does not arrive all at once. It starts where the
rock actually gave way — the neck, the undercut, wherever you were
working — and eats outward from there over the next half second, in
several bites rather than one. A hundred-cell overhang therefore reads as
something *coming down*, with the near end going first and the far end
following it, instead of the whole span blinking out of existence in a
single frame. Once it has started it always finishes: nothing stops
half way and leaves rock hanging in the air — **except where you have
asked it to.** At the default that "except" never fires. At the tighter
`F9` settings below it can, and that is the price of them; see that
section.

**What you build is sound until something happens to it.** Placed stone is
braced the same way the world's own rock is — the way a real cliff face is
held up by the mass behind it — so a structure doesn't need to be
engineered to stand, and won't quietly fall over because you didn't think
about buttresses. You can put up a long span or a wide roof and walk away
from it.

Damage is what takes that away, and only locally. A blow, a blast, a cut,
or a crack run through it all leave the rock around them no longer braced,
and *that* rock has to earn its support the ordinary way. So a wall you
haven't touched holds; a wall you've hit has to answer for itself. This is
why working at one spot repeatedly is worth doing — see below.

None of this has to be guessed at: there's a view that tints every
load-bearing cell by how close it is to giving way, green at rest through
to red at its limit. Rock that isn't carrying anything, or that's buried
too deep to care, isn't tinted at all. It's the quickest way to see why
something stood or didn't — a beam shows the strain concentrated along the
face that's in tension, and a span between two legs shows the load
splitting toward both of them.

Broken stone leaves rubble behind in stone's own colour (see
[Powders](powders.md)), so a collapsed span visibly reads as "this used to
be part of the structure" rather than a material swap.

Rubble is something rock can stand *on*. Loose material holds a piece up
the way a gravel bed holds up a paving slab — it takes weight, and it takes
it wherever the piece actually touches down — but it cannot hold anything
*steady*. A piece whose weight sits over its footing rests on rubble
indefinitely, however heavy it is; one reaching out past the edge of what
it's standing on tips off, however light. Nothing in between is
special-cased, and the same rule covers a boulder settling into a scree
slope and a ledge that overhangs the pile beneath it.

Rubble is also something rock can be *crushed by*. A pile standing on a
span is weight on that span, exactly as more rock would be, and a deep one
counts for a lot — so muck left sitting on a roof you have just blasted is
a load that roof did not have before, and a thin one over a cave will give
way under it and pour through. This is the mining beat: blow the roof,
wait, and the broken stuff comes down of its own accord a moment later.
Only what is directly *on top* counts — sand heaped against the side of a
beam, or lying under it, is not a burden — and past about a dozen cells
deep a pile stops getting heavier, the way real fill arches over itself
instead of pressing its whole weight down.

One rough edge worth knowing: a pile that grows very slowly, a grain at a
time, may not be noticed by the rock underneath until something else
happens nearby. Anything that *arrives* — a slab landing, a blast, a
blow — is noticed immediately.

What that rules out is the thing rubble used to do: a slab lying on debris
from its own collapse was treated as though the debris were what held it
together, not merely what it rested on, so each piece that came down
weakened its neighbours and a single dig could eat outward across the
world. Digging a cave and dropping the spoil on the floor is now simply
digging a cave.

Building precisely is its own thing, separate from simply painting
material in a loose freehand stream. There's a way to drag out a shape
instead — setting a starting point and dragging to preview exactly where
it will land before it's placed — in a few forms: a solid block, a
straight beam, and a **hollow room**, four walls around an empty middle
that you can actually go inside. For the room, the brush size sets how
thick the walls are. This exists because freehand painting can't produce a
clean straight edge or a repeatable size — fine for sculpting terrain, not
for building something on purpose.

Taking material away precisely is its own verb too, and deliberately not
the same as erasing. Erasing simply removes matter; a *cut* takes a bite,
loosens the rock around it, scores short cracks, and shoves the air — so
carving a doorway through a wall is something the wall above it finds out
about. It's the quiet, aimed counterpart to striking, which is the heavy
swing.

## How far damage travels

Breaking something does not only break that thing. Cut away what a shelf
was standing on and the shelf comes down a moment later; undermine a cliff
for long enough and it comes apart above and behind you. That delay is the
point — it is what gives you time to see it coming and get out, or to put
a support in.

It can also be too much. A heavy blow on a hillside used to keep finding
new things to break for a thousand frames afterwards, well away from where
it landed, which reads less like a collapse than like the world quietly
rotting.

So how far consequences may travel from what you actually hit is a setting,
cycled with `F9`, named in the title bar whenever it is not the default:

- **SPREAD** (default) — damage travels as far as the structure says it
  should. Nothing is held back.
- **LOCAL** — consequences stay within a wide room of the wound.
- **TIGHT** — the wound and its surroundings only. Undermining still brings
  things down, but a hillside stops unravelling.
- **NONE** — only what you struck is ever destroyed. Nothing collapses
  afterwards, at all.

**The distance is measured from the edge of what you hit, not from the
middle of it.** A big charge makes a big wound, and its own crater and the
seams it opens are always inside its own allowance however tight the
setting is — otherwise TIGHT would crack rock and never break it, which is
the opposite of the complaint it exists for. So the leash is on *the chain
beyond the wound*: what the tool does is the tool's business, and what
happens next is what the setting governs. A heavier tool therefore gets a
longer leash, which is the same thing as saying a bigger bang is allowed a
bigger consequence.

The setting now survives loading a new world, which it did not before — it
used to go quietly back to SPREAD on every re-roll while the title bar
went on naming what you had chosen. It also reaches a collapse that has
already started: switching to a tighter setting mid-cave-in stops the rest
of it arriving, rather than only affecting the next one. And it applies to
trees, which previously ignored it entirely — a blast could take a limb off
a tree fifty cells away at any setting, including NONE.

The whole range is one number, so it is worth trying the ends rather than
reasoning about them. Note what the tighter settings cost, and there are
two costs, not one. A delayed cave-in is a mechanic, and **NONE** removes
it entirely — you can undercut a mountain and it will sit there. And at
**LOCAL** or **TIGHT** a collapse that runs into the edge of its allowance
simply stops there, so a long span can lose the part near the blast and
leave the far part standing on nothing. That is the setting doing what it
was asked, and it is also the thing most likely to look wrong: rock in the
air is a strong signal, and whether it is a worse artifact than the
unravelling hillside it replaces is the open question on this page.

