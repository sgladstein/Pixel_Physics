# Day, Night, and Decay

*Current as of: 2026-09-06 (bodies rot instead of lying where they fell for
ever, and what rots underground goes back into the ground it pushed aside — so
a bed with plants and animals in it stops quietly running out of earth).
Before that: 2026-08-31 (a sealed room no longer dries out — water the sun
takes out of the ground now humidifies the air above it, and that humidity is
what stops the sun taking more, so one plant no longer parches a whole bed).*

*Before that: soil grades between wet and dry now — it used to
show only three values with nothing in between; and broken branches rot,
where they used to be the one thing a dead plant left that stayed for ever.*

*Before that: a world can now be a
room instead of a landscape — the air inside a sealed box is drawn as a lit
interior, with grow lights you can see in the ceiling and pools of light on
the wall beneath them, and it dims when the light schedule does; and the light
grid is half as fine, so caves stay lit about twice as deep.*

*Before that: a world whose pace knobs
are not at normal now says so on screen when it starts, instead of only on the
title bar; and break the top off a hill and you now see sky through the gap
rather than a black slab — how dark a broken-open space is depends on how much
sky it can actually see, not on how deep it is.*

*Before that: you can now stop the sun at a chosen time of day and leave it
there, and the options menu that holds that setting has been rebuilt.*

*Before that pass: rotting leaf litter mostly disappears now instead of all
becoming soil, so a wood no longer buries itself; and how long a day lasts
is a setting, as are the paces of growth, weather, creatures and the gnome —
five separate knobs, none of which changes how fast things fall; and
daylight now reaches into what you dig, so an overhang no longer hangs a
dark rectangle in the sky beneath it.*

## How fast the world runs

A full day used to take one minute of real time and could not be changed.
It is a **setting** now, and so are four other paces, each independent of the
others: how long a **day** lasts, how fast **plants grow**, how fast the
**weather** changes its mind, how fast **creatures** act, and how fast the
**gnome** moves. They live under `O` → WORLD (`Tab` cycles between the
panel's menus), and each one is a whole multiple of its normal speed — 8
means eight times slower. The day is named on the title bar at every
setting.

**And if any of the other four has been moved off normal, the game says so
on screen when it starts.** A world running its creatures at a quarter speed
looks exactly like a world whose creatures are broken, and the title bar is
not where anyone looks when the thing they are watching is on the canvas.
The setting is saved, so it outlives the session that made it; the notice is
what stops a knob somebody set last week from being mistaken for a fault.

**Slowing the world does not slow the world's physics.** Sand falls at the
same rate, water flows at the same rate, a collapse comes down at the same
rate, whatever these are set to. What changes is how fast the world *ages*.
That separation is real rather than approximate: a scene run at every
setting comes out cell-for-cell identical.

The day ships at **eight minutes**. Half of any day is night, and night is a
flat, unchanging dark — so a longer day is a proportionally longer night
too, and eight minutes means four minutes of it. That is the trade the
setting makes; if what you want is for dawn and dusk to *linger* rather than
for the cycle to come round less often, this is not the knob for it.

## Stopping the sun

Slowing the day is not the same as picking an hour, and no setting of the
speed knob will ever hold one: at its slowest a day still takes half an hour
and still goes round. **Time of day** is a separate control, the first row of
the same menu, and it holds the sun where you put it — dawn, noon, dusk or
midnight, or LIVE to let it run again. Let it run and it carries on from
where it stopped rather than jumping to wherever it would have got to.

Held is not paused. Only the sun stops: the weather still comes and goes,
plants still grow, the gnome still runs, sand still falls. A world pinned at
midnight is a world that will be dark for as long as you leave it, with
everything else going on as usual — which also means it behaves like night
rather than merely looking like it. Standing water evaporates fastest at
noon, slowly at dawn and dusk, and at midnight the world very slightly gains
water back.

Dawn and dusk sit exactly on the horizon, which is where the sky is most
strongly coloured — and also where the ground is as dark as it gets, because
daylight is measured from the sun being *up*. Expect a spectacular sky over
a night-dark landscape rather than a golden hour.

Because a world stuck at midnight looks exactly like a world that happens to
be at midnight, anything held says so: a badge in the top right of the
screen, and again on the title bar.

One thing to know before turning the growth knob up: a plant grown slowly is
not simply the same plant arriving later. Measured across eight worlds, a
tree at four times slower ends up anywhere between **a sixth of its normal
size and a third larger than it**, for the same amount of growing — usually
smaller, occasionally larger, and which way depends on the world. The knob
changes what grows, not only how fast it does.

## The sky

Open air is drawn as sky, and the sky changes through the day. It runs on the
same clock as the daylight that plants read, so what you see and what they
respond to are never out of step.

Overhead is deeper than the low sky, which is paler — that falloff is what
makes it read as air rather than as a backdrop. Around **sunrise and sunset**
the low sky goes warm and the overhead turns violet, so dawn and dusk are
events you can watch rather than moments the lighting passes through. Sunset
runs oranger and sunrise pinker, so you can tell which way the day is going
without a clock. Night is dark but never black — there is always a little
starlight, the same way the light itself never falls to nothing.

At night the sky fills with **stars** — sparse, of varying brightness, and
thinning out toward the horizon where the sky is palest. They come out as dusk
deepens and are gone before dawn has finished.

The **moon** rises once the sun is well down, climbs an arc across the sky
through the middle of the night, and sets before sunrise. It carries a faint
halo, so it lights the piece of sky it is in rather than sitting on top of it
like a sticker.

Sky is only drawn where there is actually sky. Open space *inside* the ground
— a cave, a blast cavity, a sealed chamber nobody has broken into — is unlit
rock, dark at noon as much as at midnight. Anything else would mean daylight
showing through the middle of a mountain.

**Underground means below where the ground was when the world began**, and
nothing you do afterwards moves it. That much is fixed once and never
argued with again: a mine does not become outdoors because you widened a
tunnel by one more swing, and a cavern you hollow out under a mountain is a
cavern rather than a courtyard however big it gets.

What *can* change is how much light gets in — see further down. A space
still counts as underground and can still be full of daylight, if the
daylight can reach it; the two are separate questions and used to be one.

The same rule from the other side: **nothing standing in the air makes it
dark underneath**. A tree, a bridge, a roof, a stray block left floating —
none of them turn the space below into a cave, because none of them are
ground. So you see sky between the leaves of a tree, and the space under a
platform you build reads as outdoors, because it is.

**A room, though, is now its own kind of place.** A world can say it is
indoors rather than outdoors, and where it does, the air in it stops being
sky altogether: you get a wall behind everything, panelled in bands down its
height, dimmest right under the ceiling and again where it meets the bench,
and dug space below the bench line reads as bare earth rather than as more
wall. The **grow lights** are fixtures in the ceiling you can see, and each
throws a pool of cold light down the wall beneath it that fades out before it
reaches the floor, so where the lights are is something you read off the room
rather than being told. Turn the light schedule down and the whole room dims
with it — the pools go first and hardest, the walls keep enough to see by.
Nothing else in the room goes warm as it dims, either: an unlit surface
indoors settles toward the wall's own cool grey rather than toward whatever
the sky outside is doing, so a dark laboratory reads as switched off and not
as evening.

What the fixtures are *not* is where the crop's light comes from. That still
arrives through the shell from the world's own daylight, and how thick the
shell is decides how much gets in — a ceiling three rows thicker than it
needs to be costs nearly half the light on the bench, and the only sign of
it is that everything growing there grows less.

This is what the sealed laboratory of the second game is made of; the
outdoor world never declares itself a room and is drawn exactly as it always
was.

That now holds for the landscape too, and it used to be the loudest thing
wrong with it. **Stand under a cliff's overhanging lip and you are outdoors.**
The sky beneath a brow, the air behind a leaning rock, the gap under a natural
arch — all of it is open sky, because you can walk out of it sideways without
moving a stone. Before, anything with rock above it in the same column read as
cave whether or not it was one, so every overhang in the world hung a
hard-edged dark rectangle in the sky beneath it, and a boulder standing over a
pond drew a dark band straight down through the water to the bottom.

**Ground you remove is still ground — but the light now finds its way in,
and what decides how much is how much sky the place can actually see.** Break
the top off a hill and the hollow you leave reads as sky, because from down
in it most of the sky is still overhead; the shaded side, where a taller
slope stands over it, goes dimmer, the way the far wall of a real cutting
does in the afternoon. Drive a shaft down instead and it goes dark within a
room's depth however wide you make it, because a slot admits a sliver of sky
and nothing more. A tunnel in from a cliff face keeps its soft wedge of
daylight a little way past the mouth and no further.

None of that is a rule about shape or about width — the same question is
asked everywhere, and a quarry and a mine answer it differently because
standing in one you can see the sky and standing in the other you cannot.
Depth on its own decides nothing: what matters is what is over your head.

So the old trade is gone in both directions. A mine does not fill with
daylight as you widen it, and a hilltop you have broken open no longer draws
as a black slab beside the sky it is level with — which it did until this
pass, however wide the cut, because the only thing being asked was how far
the light had travelled from the last place that was open.

**Rock is lit evenly at every depth.** A cliff face, the wall of your shaft,
the cutaway the screen always is — the stone reads the same near the surface
as it does deep down, and the strata layering stays fully legible all the way
to the bottom of the world.

It was not always. The ground used to be brightest in a band at the surface
and dim gradually with depth, on the reasoning that a cross-section wants a
vertical light axis or it reads as flat wallpaper. Played, the even version
won and it was not close, so that is the one you get. `F10` switches the
depth shading back on if you want to compare.

**Being underground comes on gradually.** Go in under the surface and the
light does not switch off at the doorway: it dims sharply in the first row
or two and keeps falling for about the height of a small room before it is
properly dark. So a shallow scrape stays a lit place you can see into, the
mouth of a cave reads as an opening rather than a hole cut out of the
picture, and only depth is actually black.

**That lit depth doubled on 2026-08-30**, when the light grid was made half
as fine. Light is dimmed by how many *grid blocks* of rock it has crossed
rather than by how many cells, so making a block twice as tall makes a given
thickness of rock half as opaque. Measured straight down through solid stone,
what used to be dark at about forty cells is now dark at about eighty: a
shallow cave is a brighter place than it was, a deep one is unchanged because
it was black either way, and the mouth of a tunnel now reads as an opening
for roughly twice as far in. Nothing about the shape of the falloff changed —
it is the same curve stretched.

Trees do shade each other, and plants respond to that, but it is not
something the picture shows yet: nothing on screen darkens because a tree is
above it.

**The ground is lit by the time of day too.** Rock, soil and water all darken
through the evening and brighten at dawn, and they pick up the colour of the
sky while they do it — everything goes warm under a sunset and cold under a
night sky. It is a whole-world effect rather than a local one: the underground
is not separately dark, and a torch does not light its surroundings. Making
depth and light sources matter is a different feature, and a much bigger one.

Loose material is lit the same way. A slab that breaks off and the grit
thrown up with it stay the colour of the rock they came from for the whole of
their fall, so a collapse at dusk is a dusk-coloured collapse — rather than
each piece flashing pale as it comes loose and darkening again as it lands.

## Day, night, and decay

The world moves through a full day and night, with light rising and
falling smoothly rather than switching between two fixed states — a
gradual dawn, a bright midday peak, a gradual dusk, and then a dim night
that never goes fully black before the cycle begins again.

**The air warms and cools with it.** Days are warmer than nights, rising to
their warmest around midday and falling to their coldest in the middle of
the night, with the change spread smoothly across the hours in between
rather than arriving as a switch at dusk. It is weather, not an event: a
mild difference you would notice on a thermometer, not one that freezes a
pond or sets anything alight.

It is a surface effect, and it fades as you go down. Open air feels the
whole swing; the ground takes a weaker version of it at its surface; and a
little way underground there is no day and night at all — deep rock sits at
the same temperature at midnight as at noon, which is what makes a cave feel
like a different place from the hillside above it.

**One thing acts on it, and exactly one: standing water dries faster when
it is warm.** A puddle out in the open loses roughly four times as much
across a warm afternoon as across the night either side of it, so pools
visibly shrink through the day and hold nearly still overnight — and the
sky's reserve of water fills on the same rhythm. See
[Weather](weather.md). Over a whole day it comes to the same total as
before; what the warmth changes is *when* the water goes.

Everything else still deliberately ignores the time of day. What melts,
what catches fire, when a worm flees the heat, how fast damp ground itself
darkens and dries — all of them read the same value at every hour, so
nothing else behaves differently at night than at noon. That is a choice
rather than an omission: a threshold that quietly meant something
different every hour would be impossible to reason about, and drying was
picked to go first because following the sun is the whole of what it is
supposed to do.

Destruction isn't always permanent, either. Ash left behind by a fire
doesn't stay ash forever — over time it slowly weathers into soil, faster
in damp conditions and much more slowly in dry ones, and it will do this
whether the fire was real or you painted the ash there yourself.
Freshly-formed soil occasionally gives new growth a chance to take root
nearby, so a burned patch of ground doesn't necessarily stay bare forever.

Fallen leaves rot on the same schedule, but they mostly rot away to
**nothing**. Only about one cell of litter in twenty leaves any soil behind.
That is deliberate and it is roughly what happens outdoors: rotting is
mostly the leaf being breathed away by the things eating it, and what little
survives is a fraction of the volume you started with. Before this, every
shed leaf became a permanent cell of soil, and a mature wood would slowly
bury itself — the floor climbing the trunks year after year until the trees
stood waist-deep, then shoulder-deep, in their own leaf fall.

**It is slower now rather than stopped.** A long-lived stand on old ground
will still raise its floor eventually, because leaves keep arriving and soil
has no way to weather back out of the world. Soil at the world's edge can
spill away, but that is no match for a canopy. If you leave a forest running
long enough, expect the ground under it to creep upward.

(Growing roots used to be listed here as the other drain, and they are not one
any more — see *what rots underground goes back into the ground*, below. A root
now borrows a cell of earth and returns it, where it used to consume nineteen
in twenty of everything it touched.)

## Wet ground and dry ground, and the ground in between

Soil holds water, and how much it holds is meant to shade smoothly from
sodden at the water table to parched at the surface. **It did not.** Ground
showed up as one of three things and nothing else: soaked, or the ordinary
damp it settles to after rain, or the dry it reaches when roots have taken
what they can. There was no in-between and no visible movement — a wet patch
and a dry patch could sit side by side indefinitely without either affecting
the other.

Now water seeps between them, and the ground shades. Dig a hole and fill it
in, drop a mat of anything on wet ground, or water one patch and not the
next, and the wetness spreads and levels out over time instead of standing in
blocks. **Fresh ground is still dry when it arrives** — a pile of anything is
dry until the ground under it soaks in — but it now soaks, over a while,
instead of staying dry for ever.

**Sun-dried ground dries more slowly the drier it gets**, which is the other
half of the same change and the reason the first half is safe. Damp ground
gives water up to the air readily; ground that has already dried gives it up
very slowly, because there is less left to give and it is held more tightly.
Without that, ground that could now be resupplied from below would simply
wick itself dry — the sun pulling at the surface, more water arriving to
replace it, for ever, until the whole bed reached the parched end. With it,
the ground settles into a standing profile: dry at the top, damper as you go
down, holding there instead of running away.

**And the air holds what the ground gives up**, which is what makes it a loop
rather than a leak. Water leaving the soil arrives in the air directly above
it and makes that air humid — and humid air is what stops evaporation. So a
sealed room does not dry out. The first ground to dry dampens the air over it,
that air pushes back, and the bed settles into its standing profile and stays
there: sampled at three times the interval it takes to settle, the bed is not
merely slowing down but genuinely stopped.

**A single plant used to break exactly this.** Put one plant in a full bed and
the surface began drying beside it, then spread outwards — one side faster
than the other — until the whole top of the world was parched and nothing
dropped on it would germinate. The plant was not drinking it: it had taken
about a two-hundredth of the water that went missing. What it did was *start*
the drying, and the drying then fed itself, because the water the ground gave
up went somewhere the air could not see. Now it goes into the air, the air
answers, and one plant costs the bed roughly what one plant drinks.

## What a dead plant leaves, and how long it lies there

A plant does not go all at once, and the pieces it leaves behind do not go at
the same speed as each other. That spread is the point: a wood that has just
been through a bad season should look different from one that went through
it a year ago, and it does, because the floor is made of different things at
each stage.

Shed leaves are the fastest. Damp ones are among the quickest things in the
world to disappear; dry ones sit and are what carries a ground fire from one
stand to the next. **Broken branches are next, and they take roughly ten
times as long as the leaves around them** — long enough that a snapped limb
is still recognisably a limb while the leaf fall of two seasons has come and
gone over it. A fallen bole is slowest of all, and will lie for what is
effectively for ever unless something burns it.

A branch does not turn into soil directly. It crumbles into the same leaf
litter that falls from above, and *that* rots on into ground — so a branch
pile passes through being leaf mould on its way to being earth, and is worth
eating to an ant for the whole of that middle stretch, which it was not
while it was still wood. If you clear a bed and come back much later, the
brown you find is not the brown you left.

**Branches used to be the exception, and it read as one.** Everything else a
plant left behind went somewhere eventually; a broken branch simply stayed
put, for the whole life of the world. After a die-back — a hard winter, a
fire, or you cutting a stand down yourself — roughly a third of everything
that died would still be lying exactly where it fell no matter how long you
waited, and in a sealed box it was matter that could never be used again.
Now every part of a dead plant is on its way somewhere, even if most of it is
on its way to nothing.

**Most of it is still on its way to nothing, and that is on purpose.** Only
about a twentieth of what rots leaves any ground behind; the rest is breathed
away. So "everything decays now" does not mean "everything comes back" — it
means the floor keeps moving instead of silting up with something permanent.

**Bodies are on their way somewhere now too.** A dead animal used to be the
last permanent thing in the world: it fell where it died, and nothing could
take it apart. In a sealed box that was ground you could never get back, and
underground it was worse than that — a corpse is loose matter, so it settled
into whatever burrow its owner died in and plugged it for good. Carrion rots
now, fast in damp ground and slowly in dry, and leaves the same twentieth
behind that a leaf does. Anything hungry still gets first claim: a body is
food before it is anything else, and only what nobody eats rots.

**And what rots underground goes back into the ground.** A root does not fall
anywhere when it dies — there is nowhere for it to fall — so buried tissue
crumbles where it lies and the earth closes over it. That is not the same
transaction as a leaf landing on the floor, and it should not pay the same
price: a root got where it is by *pushing a cell of earth out of the way*, so
what it hands back when it rots is that cell, not a twentieth of it. The
bookkeeping is strict — the ground can never get back more than it lent out,
and tissue with air beside it still falls and rots as litter like anything
else.

**What that fixes is a bed quietly running down.** Every generation of roots
was taking earth and returning a twentieth of it, so ground that had had
plants growing in it for a long time simply had less ground in it than when
you started — badly enough to see in a sealed box, where nothing new can
arrive. It costs something in the open, though, and it is worth knowing:
ground that has had a stand turning over on it is *looser* than ground that
never did, because it still has all its earth and a dead root leaves a loose
grain where it used to leave nothing at all. Undercut a grassy bank and more
of it runs out than you might expect. What holds the surface is the living
roots, and that is unchanged.
