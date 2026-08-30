# Plants

*What plants do, and what it looks like when they are working. No code, no
file names — see `Reports/` for why any of it is built the way it is.*

*Current as of: 2026-08-30 (a plant's inherited growth program has no fallback
any more: a mutation that deletes a rule deletes the behaviour, rather than the
plant quietly reverting to its species' version -- see **Individuals of one
species differ too**. Before that: most of a felled tree's leaves now stay on the
branch rather than only the layer touching it -- see **Cutting a plant down**;
what breaks off a plant turns as it falls and goes over where it lands, instead of dropping in the pose it
grew in; stems draw a line instead of wandering into one --
see **How a stem is shaped**; plants flower and fruit: two new kinds of plant
stop growing on purpose and put an organ where the shoot tip used to be, the
ripe fruit falls carrying the seed inside it, and building either costs the
plant carbon it might not have; and a shed leaf no longer lodges in the
branches on its way down, nor does a drift of them climb a trunk -- see
**The forest floor**).
2026-08-27 (a plant kept dry long enough now dies of it,
and dies gradually; standing tissue costs something to keep, and a
plant that cannot pay sheds it; grass is sown into generated worlds; a dry
meadow carries fire, a wet one stops it, and a damp one burns in patches;
and a felled crown comes apart into logs).
Written when water became a real currency and
roots started mattering; updated when the genome took over leaf economics,
wood density and seed provisioning, and again when grass arrived and plants
stopped all being made of the same stuff; again when roots stopped
destroying the water they did not drink and the forest floor stopped
keeping rain off the ground under it; and again when worlds started arriving
with four woody species in them instead of one. Updated again the same day,
when the pick and the chisel started being able to touch a tree at all — see
**Cutting a plant down**, which is new and is the part most worth playing;
again when plants started dying of ordinary causes and seeds stopped lasting
for ever; again when leaf fall was slowed to a quarter across all four
woody species, so the floor stopped burying the world; and again when a
felled crown stopped turning into sawdust and started coming apart into
logs, broken wood and leaf litter. Updated most recently
when standing wood started costing carbon to keep, night started slowing
growth, and a root buried inside its own root ball stopped counting for
anything — see **What a plant pays for**, which is new. Updated again
2026-08-27, when a plant that cannot cover the bare cost of the tissue it
already has stopped being able to stand there indefinitely and started
dying of it, and when rotting leaf litter stopped mostly turning into soil
— see **What a plant pays for** and the drought entry under
**What trouble looks like**, both rewritten. That last change had landed
three days before this page mentioned it, which is why the page now says
what it does not cover as well as what it does: a plant with no foliage
left is killed by its bill rather than by thirst, the two mechanisms are
not the same one, and grass — having no separate leaves — is exempt from
both. Those limits and one unsettled plant-economy number are open; see
`Reports/open-bugs-handoff.md`.*

## What a plant is

A plant is grown, not placed. A seed lands somewhere, germinates if it has
light and something solid to sit on, and from then on every cell it has was
paid for. It sends a shoot up and a root down, and everything above and below
ground is the same organism sharing one economy.

Plants are made of ordinary cells like everything else in the world. You can
dig them, burn them, bury them, and cut them — and that sentence has only
been true of *all four* since 2026-08-24. See **Cutting a plant down**.

## The two things a plant needs

**Light**, caught by foliage. A leaf in open sky earns well; a leaf buried
inside its own canopy, or under a taller neighbour, earns almost nothing.

Light earns most at midday and least at night, so growth visibly slows after
dark and picks up again in the morning. What a plant *decides* — whether a
leaf is too shaded to keep, whether to open a new shoot — does not swing with
the hour; only how much it actually earns does.

**Water**, drawn from damp soil by roots. Foliage spends water continuously —
more in bright light, almost none at night — and the plant can only earn from
light while it has water to spend. A plant that cannot keep up closes down:
it stops growing first, then starts shedding leaves.

A root beside open water drinks from it at the same rate it drinks from
damp ground, and **takes only what it drinks**: the cell it is drinking
from goes down a little at a time and is still there afterwards, rather
than disappearing whole. A tree on a bank is a slow, visible drain on the
pond over a long dry spell — not something that empties it in an afternoon,
which is what used to happen and what nothing in the world was counting.

Neither substitutes for the other. A plant in blazing sun with nothing to
drink earns nothing at all, and this is the single most visible rule in the
system:

- **A seed that lands in a tree's branches sprouts, struggles and dies.** It
  has no soil to reach, so it never earns, and it thins away over a few
  hundred frames. You should never see a mature tree growing out of another
  tree's canopy.
- **A stand on deep soil out-grows the same stand on a thin skin of soil over
  rock.** Same seeds, different ground.

## Roots

Roots grow down and outward through soil, displacing it as they go. They
cannot penetrate stone, and they will not push through ground that is too
dense for them.

Root mass does two jobs, and they are the same number:

- **It sets how much water the plant can draw and store** — but only the
  roots that actually touch soil count. A root cell walled in on every side
  by the plant's own roots has nothing to drink from, so it buys the plant
  nothing at all, while still costing food to keep alive. Growing a solid
  ball of root is therefore a waste of a third of it, and two plants of the
  same root mass can differ by nearly two to one in how much of it is
  earning.
- **It anchors the plant.** A root threaded through soil holds, and holds the
  soil too — root-bound ground keeps a slope that bare soil would lose. How
  well anchored a plant is depends on how far its roots *spread*, not only on
  how many there are, and a plant carrying a big crown on a narrow root plate
  puts its growth into roots until it catches up. So a tall plant in the open
  builds a wide plate and a squat one does not.

A plant that is short of water spends its growth on roots instead of canopy,
and switches back once it is comfortable. So a thirsty plant looks
root-heavy, and a well-watered one looks top-heavy. Root systems are branched
tangles, not single taproots, and they reach most of the way down a deep bed.

## What a plant pays for

**Everything a plant is standing up in costs food to keep, every day, whether
or not it is earning.** A leaf earns; the wood holding it up does not, and
the thicker that wood is the more it costs — disproportionately, so a big
plant pays far more per unit of trunk than a small one. This is what stops a
plant simply filling in for ever, and it is why an old plant is not just a
bigger young one.

The visible consequences:

- **A branch that stops earning is abandoned.** When a limb loses its leaves
  to shade or drought, the wood behind it is a pure cost, and the plant lets
  it go from the tip inward. The litter falls where it stood.
- **Plants trim from the outside in, never from the middle.** A plant never
  comes apart into floating pieces; the outermost, longest-reaching tissue
  goes first and the trunk is the last thing to go.
- **Growth is what is left over.** A plant grows on the difference between
  what it earns and what its standing tissue costs, so a big plant grows
  slowly and a stressed one not at all.
- **A plant that cannot pay at all shrinks — and if it cannot pay even to
  keep what it has left, it dies.** It sheds its way down toward a size it
  can carry. There are then two different endings, and which one you get is
  worth knowing apart:
  - **Shaded out: it holds.** A plant in a genuinely poor spot — a thin
    pocket of soil, deep shade under a neighbour — settles at a stunted size
    and stays there. It is still earning something, just not enough to grow.
    A suppressed plant waiting for a gap in the canopy is doing the right
    thing, and it can wait a very long time.
  - **Earning nothing at all: it dies.** A plant that cannot cover the bare
    cost of keeping its existing tissue alive — a tree with its water cut
    off, rather than one merely shaded — runs that way for a sustained
    stretch and is then finished. It does not wink out: it stands dead and
    is taken apart gradually, at a rate that depends on the species, so what
    you see is a tree going grey and coming to pieces over a long time
    rather than a plant disappearing between one frame and the next.

  The difference is not how badly it is doing, it is whether it is earning
  *anything*. A mature plant is nearly always spending more than it earns —
  that is what stops it growing — and that alone never kills it. One bad
  spell does not either; the plant has to fail continuously, and any single
  day it manages to pay resets the clock.

## What a healthy stand looks like

Trunks are bare at the bottom and foliage sits at the top — plants shed
leaves that are too shaded to pay for themselves, so a crowded stand lifts its
own crowns. Neighbouring crowns stay mostly separate rather than merging into
one green slab. Big plants set more seed than small ones, without any rule
saying so: seed is set per mature cell, so size buys offspring.

Growth is fastest when young and tails off — a plant stops when its income
can no longer cover another cell, not because it hit a size limit.

## How a stem is shaped

A stem is not drawn along a planned course. A growing tip looks at the cells
around it, weighs each one on how well it continues the direction it is
already going, how much light lies that way, which way the wind is pushing and
which way the plant's own sense of up points — and then *picks one at random*,
with the better-scoring directions likelier to come up. That is the whole of
it, and everything a plant's outline does comes out of repeating it a few
thousand times.

The catch is that a cell has only eight neighbours, and eight directions
cannot express a lean. A shoot growing dead upright has one neighbour straight
above it and two more at the diagonals, and even at its most single-minded it
takes one of the diagonals more often than not — so it wanders off its own
line and then wanders back, and a stem came out as a wobble rather than a
stem.

**So a shoot can draw its heading rather than groping for it — and whether it
does is yours to choose.** `K` cycles it: off (what plants have always done),
each species' own setting, or forced to the maximum on everything. It changes
how plants *grow*, not how they are drawn, so pressing it does nothing to a
tree that has already grown — press `F6` for a fresh world to see it, and the
status line names whichever mode is on.

Off is the default, because asked three times whether the straighter version
looked better, the answer was "neither". It is here to be tried, not because
it won.

When it is on, it still
weighs its neighbours and still picks one at random exactly as before, but
that pick now steers where the shoot is *aimed*; the cell it actually enters
is whichever one keeps it closest to the line it is aiming along, with the
fraction of a cell it could not spend carried over to the next step. A shoot
leaning slightly off vertical comes out as a run of upright cells with a
regular step sideways every few rows — the way a drawn line is spelled on a
grid — instead of a coin toss at every row.

Two things about this are worth knowing when you look at a plant:

- **It changes how a stem is spelled, not where it wants to go.** A shoot
  leaning fifteen degrees still leans fifteen degrees, still turns toward
  light, still bends away from a crowd, still gives up and goes round
  something in its way. The plant is exactly as responsive as it was.
- **It is part of what tells the species apart.** How straight a shoot draws
  is set per species and per tier, so a fir holds a hard, legible leader with
  its branches leaving in tiers, a broadleaf's trunk is firm but its outer
  twigs keep some wander, and a shrub stays deliberately gnarled. **A creeper
  is left to wander completely** — a vine that ran in clean lines would stop
  reading as a vine.

Roots are not drawn this way and should still look gnarled and searching.
They are feeling their way through soil rather than reaching for anything, and
that is what a root looks like when it is right.

## What trouble looks like

- **Drought.** Foliage thins from the inside out. The soil under the stand
  visibly dries first in the rooting zone. **A plant kept dry for long
  enough now dies of it** rather than shrinking to a stump and standing
  there for ever, which is what used to happen. Watch a stand through a long
  dry spell and you should see individuals lost, and the survivors get
  visibly *bigger* afterwards — the ones that come through inherit the light
  and water the dead one was using.

  One honest limit, because it changes what you are looking at: what kills a
  parched plant is being unable to pay for the tissue it is standing up in,
  not thirst as such. Drying out only acts on foliage, so a plant that has
  already lost all its leaves is not being killed by the drought any more —
  it is being killed by the bill. The two arrive at the same place here, but
  they are not the same mechanism, and only one of them is finished.

  **Grass does not do this at all.** Having no separate leaves is what
  exempts it, so a meadow will not thin and die back through a drought the
  way a stand of trees will. That is a gap rather than a decision.
- **Shade.** A plant overtopped by a neighbour loses its lower and inner
  leaves and grows leggy reaching for light.
- **Being cut.** Anything no longer connected to the ground through its own
  tissue comes away and falls. Cut a trunk and everything above the cut is
  detached, not left floating.
- **Being buried.** Weight piled on a branch shortens how far it can reach
  before it breaks.

## Species

Not every plant is a tree. **Grass** grows a few rows tall instead of a
hundred, in tufts that spread into a continuous green layer over open
ground, and it is a different kind of thing to look at rather than a small
version of the same thing. It has no separate leaves — the blade *is* the
plant, so the whole of it is green from the soil up, and there is no bare
brown stem holding it there.

Three consequences follow from that, and none of them is a rule about grass:

- Its roots run **sideways through the top few rows of soil** instead of
  diving, and that mat holds loose ground together where a tree's few thick
  roots do not.
- It is the **most flammable thing that grows** — cell for cell a fire runs
  through grass about a quarter faster than through a tree canopy, and bare
  soil will not carry one at all. The roots survive underground.
  **A dry meadow now burns end to end**, as a front you can watch cross it,
  throwing flames and trailing smoke and leaving a black scar; a
  well-watered one takes light where the fire was set and goes out within a
  few paces; and a **damp** one in between burns in patches, leaving
  scorched ground and standing green interleaved. That difference is how
  wet the ground under the grass is, and it is the single thing that
  decides whether a meadow burns — see [Fire & Heat](fire-and-heat.md).
  What used to happen instead was a local smoulder at any wetness: fire
  could only step between blades actually touching, and a sward that looks
  continuous is really a scatter of separate tufts, so a fire burnt the
  tuft it was lit in and stopped. That same patchiness is what makes the
  in-between outcome a patchwork rather than a half-crossed field.
- It **breeds far faster than a tree** — it is cheap to build, it sets seed
  young, and it will colonise bare ground long before anything woody does.
- And it **keeps a different set of books**. Because the blade is the whole
  plant, a grass tussock finishes growing early and then simply stands
  there: it earns from every blade, spends the proceeds on seed, and — for
  now — has no running costs to fall behind on. What kills it is shade. A
  sward is thinned by whatever grows over it and not by drought, which is
  the reverse of what a tree faces, and it is why grass and trees are not
  competing for the same thing.

Grass cannot get into dry sand, though, and neither can a tree: sand is
simply harder to push a root through than either of them can manage.

Species differ in shape, colour and habit — how readily they branch, how
strongly they reach for light, how far apart their leaves sit, whether they
fork repeatedly into a mound or hold a single leader. Each species has its own
range of foliage and bark colours, so two species never draw the same colour
however their individuals fall.

**The four woody plants are points on a trade rather than four sizes of the
same thing.** What a plant cannot spend on height it spends on leaf area at
each node it does make, so the tallest of them — the conifer — carries the
smallest, tightest sprays, and the ground-runner, which gets nowhere near
knee height, carries the broadest leaf clusters of any of them. In between,
a tree makes larger clusters than a conifer and a shrub larger still. That
is the quickest way to tell two of them apart at a distance when the
silhouettes are both just "green mass": count how coarse the foliage is.

Where each one grows is decided by the ground — trees in damp deep soil,
conifers on the high ground, shrubs on the dry margin, creepers on a skin of
soil over rock. `wiki/the-world.md` has that half.

**Grass now comes with a new world**, on the open ground the woody plants
leave. What it waited on was a way to die: grass that could not be cleared
would have piled up in a world that kept seeding it. Now shade kills a blade
and a buried seed loses half its viability every few thousand frames, so a
sward thins where a canopy closes over it and thickens where the light
holds. `wiki/the-world.md` has where it goes.

Individuals of one species differ too. Every plant carries a genome drawn when
it germinates and inherited by its seed, so a population drifts and can be
selected on.

**A plant's growth program is part of that genome, and there is no longer a
safety net under it.** The rules that say what a growing tip turns into, what
it puts out sideways, and when an axis stops are inherited and can mutate like
anything else — and a mutation that *removes* one removes it for good. The
lineage does not quietly fall back on the way its species used to grow. That
is what lets a line of plants reach a shape its species never had; it is also
what lets a line inherit a body plan that does not work, which is a thing that
can now happen. In practice you will not see it yet: plants breed slowly
enough that a stand rarely gets past its great-grandchildren, so almost every
plant on screen is still growing to its species' original program.

## Plants that stop, and what they stop in

**Every plant described above grows until something stops it** — it runs out
of carbon, or water, or room, or it simply cannot lift water any higher. None
of them ever *finishes*. That is why they all read as versions of one thing:
a shoot that only ever ends by failing has nowhere else to go.

**Two kinds of plant now finish on purpose.** An axis counts the leaf-and-bud
units it has made, and at its own number it stops making shoot and makes a
**flower** instead. The growing tip is used up doing it — there is no
continuation, because the flower *is* what the tip became. That is the whole
of it, and it is what the two look like:

- an **erect herb** puts everything into one stem that goes straight up on a
  bare stalk with a few large leaves widely spaced along it, and finishes in a
  single flower head several cells across at the top;
- a **scrambler** makes a short run, stops, throws a side shoot that takes over
  and does the same, over and over — so it sprawls into a low thicket studded
  with small clusters of flowers and fruit at every place an axis ended.

**A flower is not a green cell with a label on it.** Petals are their own
material, in colours nothing else in the world has: yellows, oranges, reds,
pinks, magentas, violets, blues and whites. Fruit is its own material again,
darker and heavier — reds, crimsons, purples, blue-blacks, oranges and golds.
Each species draws from a band of that range and each individual takes one
colour inside its species' band, so a stand of one plant is a spread of
related colours rather than one repeated swatch.

**Flowers set fruit, and ripe fruit falls.** A flower that has been open long
enough becomes a fruit; a fruit that has finished ripening lets go of the
plant. What drops is not scenery — the seed is *inside* it. A fallen fruit is
a soft, heavy, round thing: it rolls further than a seed does, piles in
hollows, rots quickly, and is worth twice as much to eat as a leaf. So a plant
that fruits scatters its offspring away from its own shade instead of dropping
them at its feet, and a fruiting thicket puts real food on the ground beneath
it.

**None of it is free, and that is what makes it look like a plant rather than
a decoration.** Building a flower costs carbon at the moment the plant commits
to it, and filling a fruit costs again out of the same account seeds are paid
from. So the outcome is graded rather than all-or-nothing: a plant in good
light makes a full head, one in poorer light makes a small one, and one that
never gets ahead makes a bare stalk with nothing on top. A plant that fills
fruit sets fewer loose seeds that season, because both come out of the same
pocket.

The older plants are unchanged: a tree, a conifer, a shrub, a creeper and
grass all still grow until something stops them, and none of them flowers.

## Colour is a readout, not decoration

**Where a plant sits inside its species' colour range tells you what kind of
plant it is.** Both bands are inherited, both can mutate, and both name a real
trade the plant is making. This is the part to look at before reading any
number.

**Foliage tone is the leaf's price.** A plant either builds expensive leaves —
darker, earning faster in good light, and spending markedly more water every
tick — or cheap ones, paler, earning less but thrifty. Neither wins
everywhere, which is the point: expensive leaves win where light is what's
short (shade, wet ground), cheap ones win where water is (bright, dry, thin
soil). A dark tree is dark *because* its leaves cost more to run.

**Bark tone is the wood's density.** Dense wood holds a longer branch under
more piled weight before it snaps, and costs more carbon per cell to build, so
it grows slower. Light wood is the opposite bargain — a fast grower that loses
more of itself to load. On a mixed stand you can pick out which trees will
survive a burial and which will shed limbs, by colour, before anything is
piled on them.

A freshly seeded stand is mixed on both axes from the first frame, so this is
visible immediately rather than only after generations. What selection then
does to those proportions is the thing worth watching over a long run.

## Seeds carry provisions

A seed is not just a marker for where a plant might grow — it holds what its
parent paid for it, and the seedling starts life spending that stake. A bred
seedling is therefore meaningfully better off than one dropped in by hand,
which starts broke and has to survive its first few ticks on nothing.

That matters because the first moments after germination are when most
seedlings are lost: a fresh shoot has to afford its first growth step before
any income has arrived. Where a stand is dense enough that establishment
actually fails, the provisioned ones are the ones that make it.

## The forest floor

A shed leaf does not vanish. It falls — through its own crown, past its own
branches — and lands as **leaf litter** on the ground below, where it drifts
against trunks and piles into hollows. Litter is lighter than water, so it
rafts on a pond rather than sinking, and it mats rather than slumping flat:
a drift under a tree reads as a drift, not as a brown line across the world.

**Nothing catches it on the way down.** A branch here is drawn one cell
wide, but the world is one vertical slice of a wood that has depth, so a
branch is not a shelf spanning the whole thickness of the tree — a falling
leaf goes past it, the way it would outdoors. That holds however the leaf
came to be in the air: shed from the twig above, knocked loose, or already
lying somewhere in the crown. Leaves used to hang up on the first branch
under them and then collect, so a tree slowly filled with brown speckle
that never came down; now the only litter you see off the ground is a drift
banked up against a trunk, which is a pile resting on the floor and reads
as one. A snapped *branch* is a different matter — that is a solid thing,
and it can still come to rest across the limbs below it.

**And the pile spreads round a trunk rather than up it.** This is the same
argument again, pointed sideways, and it is the half that was harder to see.
Leaves that have landed still have to go somewhere as more arrive; a drift
wedged between two trunks has nowhere to spread, so it used to grow *upward*,
climbing out of the forest floor into the low branches as a narrow column.
Every way of asking "is this on the ground" said yes — it was touching the
ground the whole way up — and it still looked exactly like leaves collecting
in the tree, because that is what it was. Now a drift banks round the trunk
instead, so the floor stays a floor and only thickens. It spills only where
there is somewhere lower to spill to, so a drift still piles against a trunk
rather than running out into a flat sheet.

Rain goes **through** it. A litter blanket is loose stuff lying on the
ground, not a roof, so the soil beneath a deep forest floor still takes the
weather — and because litter also rots into soil, a wooded floor ends up
holding rather more water after a storm than bare ground does, which is
what mulch is for. It used to do the opposite and seal the ground it lay
on, and the blanket is deepest exactly where the roots are.

The *litter* does not pile up forever: it **rots away**, faster where the
ground is damp, so the visible drift under a standing wood stays roughly
constant depth. **Most of it rots to nothing** — only about one cell in
twenty leaves any soil behind. That is close to what happens outdoors,
where rotting is mostly the leaf being breathed away by whatever is eating
it, and the little that survives is a fraction of the volume it started as.

The soil that does form is a slower story, and it is still only a story
about slowing down: nothing weathers soil back out of the world, so a very
old stand does gradually raise the ground it lives on. Growing roots eat
into soil and soil at the world's edge can spill away, but a canopy
outruns both. A mature tree also sheds sparingly (a leaf has to be deep in
shade or drying out before it goes), which together with the one-in-twenty
yield keeps the rise to a slow creep rather than the burial it once was; a
wood should read as standing *on* its floor, not sinking into it. Leave one
running long enough and the ground will still climb.

That is currently true of **the tree and not yet of its neighbours**. Worlds
arrive with four woody species in them, and only the tree has had its leaf
fall slowed so far, so a conifer belt, a scrub margin or a run of creeper
still drops leaves at the older, faster rate and still builds floor under
itself the way the whole world used to. Expect the burial look to survive in
those stands until the same change reaches them.

Litter is also the fastest fuel in the world: it is the layer that carries a
ground fire between two stands across open ground. And it is **food** — the
one part of a canopy's production that ends up where a walking animal can
reach it. See `ants.md`.

## Cutting a plant down

**New on 2026-08-24, and the honest half of it is in the second paragraph.**
Until now a tree was the one thing in the world a tool could not touch. The
pick and the chisel both asked "is this rock?" before they cut, and a trunk
is not rock — so you could bore through a mountain and not through a
sapling, and the only thing in the game that could damage a tree was a
blast. Now they cut wood the way they cut anything else, and so does the
eraser.

**Cut through the foot of a trunk and the tree comes down.** A plant holds
itself up from its roots outward, so severing the bottom of it leaves
everything above with nothing to reach, and the whole crown lets go a
second or so later. It is not one blow: a grown tree's base is twenty-odd
cells across and each swing takes a bite of about a third of that, so it is
three or four swings and then a pause before anything happens. The pause is
real and is the tree standing on nothing.

**What comes down comes down in three sizes**, and this is the part that
changed most. A crown that let go used to dissolve into a cone of sawdust —
one loose grain per cell, piling at its angle of repose, which is literally
the same shape a pile of sand makes. Now it comes apart the way a tree does:

- **Logs.** Big coherent chunks of the bole and the major limbs break away
  whole, tumble, and lie where they land. They do not flow or pile — a log
  is something you can stand on and climb, and it burns and rots on its own
  slow schedule.
- **Broken wood.** Smaller breaks, and whatever a log gives way into later,
  become loose deadwood: that still piles like any other debris.
- **Leaf litter.** Foliage does not come off as chunks. It scatters, and it
  is the same litter that already carpets a forest floor — so it rots away
  quickly, and the ants will carry it off.

Which of the three a given part of the tree becomes depends on how big the
piece it belonged to was, not on a switch, so a heavy bole and a thin twig
genuinely come apart differently. Expect a few big pieces, a good deal of
loose wood, and a lot of leaf litter over the top of both.

**New on 2026-08-29: what comes down now turns on the way, and goes over
where it lands.** Before this, nothing that broke off a tree ever rotated —
not once, on any fall — so the pieces dropped in whatever pose they had been
growing in and a good half of them ended up standing on end in the heap. Now
a piece turns as it falls, at a rate set by where its weight sits relative to
the point it broke from: a twig tumbles several times on the way down, a bole
comes over about once, and a limb hanging out to one side goes over that way
rather than the other. And a piece that comes to rest with its weight
overhanging its footing tips instead of standing there, until it is seated or
until there is no room to turn into.

**Most of the leaves stay on now.** Foliage holds to a limb it is lying
on, and it holds through another leaf as well as directly — which is the
difference between a third of a crown's leaves staying put and about
three-quarters. Foliage in open air still falls; the chain has to reach real
wood.

**What it still does not do is fall over.** The crown breaks into pieces at
the moment it is cut and they drop, rather than the tree detaching from the
ground, going over because it is unstable, and *then* shedding branches where
it hits. That is the open question on this section, and it is a bigger one
than it looks: an attempt to make the pieces sweep over together was rejected
in play for reading as "an invisible force pushing things around", which is
what any imposed motion will read as until the stress that ought to cause it
exists.

**Burning a trunk out from under a crown** now brings the crown down too,
which it did not before at the tighter `F9` settings — the fire licenses
the collapse the same way a blow does. So does erasing a trunk with the
brush. See `structural-collapse.md` for what that setting does.

**How much of the tree comes down depends on `F9`**, and at the tight end
that is visible rather than academic. Cutting the same tree at each setting:
at SPREAD and LOCAL the whole crown goes; at TIGHT about half of it does and
the top stays up, standing on nothing; at NONE nothing comes down at all and
you are left with a cut trunk holding its whole canopy in the air. That is
the setting doing exactly what it says — NONE means only what you struck
ever fails — and it is the same trade the rock pages describe, now visible
on trees too. If a half-fallen tree reads worse than a slowly-unravelling
one, the setting is the lever.

**A stump is left, and it stays alive.** The roots and the last row or two
of trunk are still anchored, so they are still the plant. What they do not
yet do is regrow — a topped tree sits there indefinitely rather than
resprouting, which is a separate piece of work.

## Bending in the wind

**New on 2026-08-29.** Wind pushes on a plant, and the parts of a plant that
are too weak to hold what is hanging on them give way and swing over.

The wind is not the same everywhere. A plant on an open ridge catches the
whole of it; one tucked behind a rise barely feels it, and one growing in the
middle of a wood is sheltered by the wood. It also does not blow underground —
roots take nothing.

**What gives way is the soft stuff.** Wood does not bend. Grass and foliage
do, and they bend where the load actually is: a leafy cluster hanging off the
far end of a long branch swings, while the quiet middle of a canopy sits
still. What swings is a whole section of the limb at once, and it swings
about the point that is holding it, so the far end of it moves most and the
part nearest the branch barely moves — the limb curls rather than sliding
sideways. Keep watching and the curl travels back along the limb as the tip
runs out of room to move into.

**It stops.** A limb that has leaned over is no longer reaching out as far,
so there is less of it hanging out in the wind, so it stops leaning. A plant
does not creep across the world in a gale.

**It also never comes apart.** Whatever swings stays joined to the plant. A
leaf that leans is a leaf still on its branch — and in a wind, a plant that
can lean loses *fewer* leaves than one that cannot, because leaning is how it
takes the load off.

**What you will not see yet.** A tree does not sway. Trunks and branches are
rigid, and the reason is that a leaning trunk has no way to relieve what the
wind is doing to it — what stops a real tree in a gale is that it breaks, and
breaking is not built yet. Meanwhile the wind is already pushing on the trunk
and the number is already being kept; nothing is reading it. Foliage in a
thick canopy is also packed too tightly to move much: there is simply nowhere
for it to go.

## When a tree breaks

**New on 2026-08-30.** A tree that is carrying more than it can hold breaks,
and the break is somewhere specific rather than anywhere.

**It is about proportion, not size.** A big tree with a trunk to match is
fine. What fails is a heavy top on a thin base — a crown that grew out further
than the wood under it can carry. So the trees that come down are the badly
grown ones, and a well-built tree of the same age standing next to one will
be untouched. Roughly a quarter of trees are living close to that line at any
moment; most have room to spare.

**What it looks like.** One place gives way, not the whole trunk at once.
There is a splintered stub left where it went, and everything that was beyond
that point — the limb, its branches, its leaves — comes down as a piece and
falls. It is not a tree dissolving; it is a tree losing an arm.

**What pushes one over.** Its own weight, as the crown grows out. Wind, which
leans on everything above ground and leans hardest on a tree standing in the
open — one tucked in a wood or behind a hill feels much less. And anything
that lands on it: snow piling on a crown through a storm is enough to bring
down a limb that was managing fine the day before, and no one has to do
anything for that to happen.

**It should get rarer.** Breaking kills off the badly-proportioned trees, and
the ones left are the ones that build a thicker trunk for the crown they
carry. Their seedlings inherit that. A wood that has been standing a long
time should be losing fewer limbs than a young one — that is the intent, and
it is not yet proven over a long enough run to state as fact.

**What you will not see yet.** A trunk does not buckle — a very tall thin
tree does not fold under its own height, it only breaks if something is
hanging off it wrongly. And roots do not break at all.

## Fire and death

Wood burns, and a burnt plant becomes ash. Ash decays into soil if it is damp,
and soil sometimes reseeds — so a burnt patch grows back on its own. Litter
takes the same path but much faster; it does not reseed, or every shed leaf
would be a chance at a new tree and a stand would carpet itself.

Dead and broken plant tissue becomes deadwood, which falls and piles like any
other loose material — except for the big pieces of a tree that has been cut
down, which become logs and lie where they fall. See **Cutting a plant
down**.

**Plants also die of nothing dramatic at all.** Foliage that sits in deep
shade, or that a plant cannot supply with water, is let go of a leaf at a
time — so a sapling that came up under a closed canopy thins away instead of
standing there for ever, and a sward under a spreading crown goes back to
bare ground. This is gradual and it is graded: something a little shaded is
effectively permanent, something in the dark is on its way out, and there is
no moment where a shadow arrives and a shelf of foliage is swept off at
once.

**A plant with nothing left that can earn is a dead plant, and dead plants
rot.** Once the last of a plant's green is gone there is no route back to
income, so what remains — the bare stem, the root mat — goes to litter over
the following while and from there into the soil, the same path a shed leaf
takes. A tussock browns off and is gone quickly; a woody stem stands a good
deal longer before it goes. A plant with dormant buds, or one still in its
seed, is not dead by this reckoning and is left alone.

## The seed bank

Seeds that land somewhere too dry to germinate do not die on the spot — they
**wait**, sometimes for a long time, and sprout when rain finally wets the
ground under them. That waiting bank is what carries a species through a
season it could not otherwise survive: even where every standing plant of a
kind is gone, the ground may still be full of them.

But a seed does not wait for ever. Viability runs out gradually, so a bank
that is not being topped up thins away and a bank that is settles at a depth
set by how fast seed is arriving. The two ends of that trade are a real
difference between species: **grass seed outlasts tree seed by about
double**, which is a large part of why grass is the thing that comes back
first on ground where nothing is currently growing. A seed whose time runs
out rots where it lies, like anything else.
