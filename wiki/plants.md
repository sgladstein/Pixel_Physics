# Plants

*What plants do, and what it looks like when they are working. No code, no
file names — see `Reports/` for why any of it is built the way it is.*

*Current as of: 2026-08-23. Written when water became a real currency and
roots started mattering; updated when the genome took over leaf economics,
wood density and seed provisioning, and again when grass arrived and plants
stopped all being made of the same stuff; again when roots stopped
destroying the water they did not drink and the forest floor stopped
keeping rain off the ground under it; and again when worlds started arriving
with four woody species in them instead of one. Updated again the same day,
when the pick and the chisel started being able to touch a tree at all — see
**Cutting a plant down**, which is new and is the part most worth playing.
One plant-economy number is unsettled and its cause is still open — see
`Reports/open-bugs-handoff.md`.*

## What a plant is

A plant is grown, not placed. A seed lands somewhere, germinates if it has
light and something solid to sit on, and from then on every cell it has was
paid for. It sends a shoot up and a root down, and everything above and below
ground is the same organism sharing one economy.

Plants are made of ordinary cells like everything else in the world. You can
dig them, burn them, bury them, and cut them — and that sentence has only
been true of *all four* since this build. See **Cutting a plant down**.

## The two things a plant needs

**Light**, caught by foliage. A leaf in open sky earns well; a leaf buried
inside its own canopy, or under a taller neighbour, earns almost nothing.

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

- **It sets how much water the plant can draw and store.** A big root system
  keeps drinking through a dry spell; a small one lives tick to tick.
- **It anchors the plant.** A root threaded through soil holds, and holds the
  soil too — root-bound ground keeps a slope that bare soil would lose.

A plant that is short of water spends its growth on roots instead of canopy,
and switches back once it is comfortable. So a thirsty plant looks
root-heavy, and a well-watered one looks top-heavy. Root systems are branched
tangles, not single taproots, and they reach most of the way down a deep bed.

## What a healthy stand looks like

Trunks are bare at the bottom and foliage sits at the top — plants shed
leaves that are too shaded to pay for themselves, so a crowded stand lifts its
own crowns. Neighbouring crowns stay mostly separate rather than merging into
one green slab. Big plants set more seed than small ones, without any rule
saying so: seed is set per mature cell, so size buys offspring.

Growth is fastest when young and tails off — a plant stops when its income
can no longer cover another cell, not because it hit a size limit.

## What trouble looks like

- **Drought.** Foliage thins from the inside out. The soil under the stand
  visibly dries first in the rooting zone.
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
  Fire steps from one plant to the one touching it, so a thin, patchy sward
  is a firebreak and a thick closed one is a fuse. Nothing says so anywhere;
  it falls out of the gaps. **In practice a grassfire is currently a slow,
  local smoulder rather than a front sweeping a meadow** — see
  `Reports/plant-implementation-plan.md` for the measurement and the two
  fire-side changes that would be needed.
- It **breeds far faster than a tree** — it is cheap to build, it sets seed
  young, and it will colonise bare ground long before anything woody does.

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

**Grass does not come with a new world yet**, though you can plant it. It
has no way to die of anything, and a world that seeded it would accumulate
grass that can never be cleared — so it waits on that rather than on
anything about how worlds are made.

Individuals of one species differ too. Every plant carries a genome drawn when
it germinates and inherited by its seed, so a population drifts and can be
selected on.

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

Rain goes **through** it. A litter blanket is loose stuff lying on the
ground, not a roof, so the soil beneath a deep forest floor still takes the
weather — and because litter also rots into soil, a wooded floor ends up
holding rather more water after a storm than bare ground does, which is
what mulch is for. It used to do the opposite and seal the ground it lay
on, and the blanket is deepest exactly where the roots are.

It does not pile up forever. Litter **rots back into soil**, faster where the
ground is damp, so a standing wood reaches a floor of roughly constant depth
instead of burying itself. That is what makes the floor a cycle rather than an
accumulator — and it is why a wood costs no more to simulate the longer it
stands.

Litter is also the fastest fuel in the world: it is the layer that carries a
ground fire between two stands across open ground. And it is **food** — the
one part of a canopy's production that ends up where a walking animal can
reach it. See `ants.md`.

## Cutting a plant down

**New this build, and the honest half of it is in the second paragraph.**
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

**What comes down is, for now, sawdust.** The crown dies where it stands,
browns, and then dissolves into a cone of loose deadwood — a handful of
pieces come away as pieces, near where you actually hit, and the other
ninety-odd per cent does not. That is the known and next thing to fix, it
is the exact failure this project has rejected before ("a uniform dissolve
into powder"), and it is written here rather than left to be discovered:
the tree *falling* works, the tree *coming apart* does not yet.

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

## Fire and death

Wood burns, and a burnt plant becomes ash. Ash decays into soil if it is damp,
and soil sometimes reseeds — so a burnt patch grows back on its own. Litter
takes the same path but much faster; it does not reseed, or every shed leaf
would be a chance at a new tree and a stand would carpet itself.

Dead and broken plant tissue becomes deadwood, which falls and piles like any
other loose material.
