# How far the plant substrate can reach — sunflowers, tomatoes, vines (2026-08-23)

**Status: design note, answering a direct owner question; all three §7
calls signed off by the owner, 2026-08-23, same day.** From the
four-woody-species card (`20260823T094729940Z-55bc8b`, 2026-08-23):

> "Different-ish. The biggest differences are still size and color. That
> really different morphology. I think the issue is in the base design of the
> random walk growth. At some point we should consider flowers and fruits to
> add more variety, but could we ever realistically get to a tomato plant or
> sunflower or climbing vines with our implementation?"

Written by the review/integrator session from the full plant record
(`plant-project-review-2026-08-23.md` and everything it cites). No code was
written for this note; every "exists today" claim below was verified in
source during that review. Its sibling is the physical-trees design
(commissioned the same day, separate report) — that one answers "can a tree
*behave* physically"; this one answers "can a plant *be shaped* like
something other than a tree".

---

## 1. The short answer

**Yes — and the random walk is not the ceiling.** The walk is the engine's
*variation* mechanism: it decides where the next cell of an axis goes, biased
by light, crowding, heading and gravity. What it cannot supply — and what a
sunflower, a tomato and a vine are actually made of — is *identity*: axes
that know when to stop, organs that terminate them, and (for the vine) a way
to hold onto something. Those are three bounded additions to the
species-as-data layer, not a growth-engine rewrite.

The diagnosis pattern is the one this project has already paid for three
times (`plant-appearance-design.md` §5, the CLAUDE.md "which pixels" rule):
the trees look alike because of **composition and allocation**, not because
the walk fundamentally cannot draw other shapes. A tomato plant differs from
a tree in *what its cells are* (fruit, flower, truss) and *when its axes
stop* — both of which are exactly the kind of thing this engine already
expresses as species data.

## 2. What the archetypes need, against what exists

The engine already has, verified in source: sympodial growth as a heritable
allele (**a tomato is the textbook sympodial plant**); per-order behaviour
tiers (`ByOrder`); plastochron-scheduled leaf sprays; orthotropic and
plagiotropic tropism per tier; a prostrate creeper species with its own
niche; heritable palette bands wired to real loci; turgor height budgets
that make small plants cheap (grass and shrub prove the scale works); the
climbable-material flag the gnome already uses on grown wood; wind in the
field; and seeds that carry a genome and an endowment.

What is missing is exactly four primitives, each small:

**2a. Organ cell types: `Flower` and `Fruit`.** `CellType` has spare space
(`Segment` sits unused today). An organ cell is *inert* — no per-tick
behaviour beyond decay and being eaten — so it costs the scheduler nothing.
A `Behavior::Flower { cost, trigger }` on a tip converts it into a small
organ cluster instead of another metamer. Fruit ripens by staging its `aux`
(a colour readout for free, in the same spirit as the bark/leaf bands),
then drops: a `Powder` that carries the seed's genome, or converts to seed
plus litter where it lands. This is also the least speculative appearance
lever on the board — an organ is colour and shape concentrated in one
place, i.e. precisely "pixels that move", where the architecture levers
measurably were not.

**2b. Determinate axes.** Today every axis is indeterminate: a tip extends
until staleness, turgor or the economy stops it, which is why every species
is a variation on "tree". Real herbaceous form is mostly *determinate* —
an axis grows N metamers and then terminates in an organ. `ByOrder` already
carries per-tier parameters; a `determinate: [n]` entry per tier, counted
by the plastochron machinery that already hands lineage state parent→child,
gives: the sunflower's single stalk (order 0, determinate, terminal
flower), the tomato truss (determinate order-1 laterals ending in fruit
clusters), and — worth noticing — a *safe* direction for the frontier
economy: every recorded growth catastrophe here came from too much
frontier, and determinacy only ever removes frontier.

**2c. Rosette / whorl placement.** A sunflower is a basal rosette plus one
determinate leader. The whorl mechanism is already queued (C3, the conifer
tiers lever, priced in the verification report); a basal-rosette variant is
the same mechanism pointed at order 0 near the collar. Nothing new to
invent — sequencing only.

**2d. A climbing tropism plus an attachment bit.** The vine is the only
archetype needing a genuinely new *read*: a tropism term that scores
adjacency to solid, non-organism cells, so the axis hugs surfaces instead
of standing free. The read is cheap where it belongs (the `Grow` dispatch
already holds the neighbourhood, per the guard-at-the-call-site rule) and
it is tip-only, so the scheduler never pays for the mature vine. The subtle
half is support: a climber pressed against a wall should count as anchored.
That must be **stated as data, not inferred from shape** — the
mountain-vs-tower lesson is explicit in CLAUDE.md — so it is a species
`attaches` flag feeding `anchor_support` one extra anchor rule, not a
geometric guess. The creeper is the natural base species; today it runs a
superseded root-branching path (`open-bugs-handoff.md` §W1a) and would take
this upgrade in the same touch.

## 3. The bill of materials, per archetype

| archetype | needs from §2 | already has | size |
|---|---|---|---|
| **sunflower** | organs (2a) + determinate leader (2b) + basal rosette (2c) | orthotropic tier, small-plant economy, colour bands | S–M |
| **tomato** | organs (2a) + determinate trusses (2b) | **sympody (built, heritable)**, ByOrder, moisture-gated germination | S–M |
| **climbing vine** | climbing tropism + `attaches` (2d); organs optional | creeper species, climbable wood to climb, plagiotropy | M |
| annual herbs generally | 2a + 2b + a post-fruiting senescence flag | grass's fast generations | S on top of the above |

All four are species-`.ron` work plus at most two new `Behavior` variants
and two `CellType`s. None of them touches the walk itself; none of them
adds per-cell per-frame work (organs are inert; the climbing read is
tip-only). The frame-cost exposure is the usual one — more small organisms
in the world — which is the same budget item the sowing work (PR #20)
already priced.

## 4. What this buys beyond looks

- **Evolution gets its missing halves.** Fruit *is* dispersal (Arc A5: the
  seed travels in a material that falls, rolls, floats and gets carried)
  and *is* the plant→creature coupling the ecology design keeps asking for
  (ants already eat leaves and litter; fruit is food worth walking to —
  the cycle-not-chain rule from the creature line). A post-fruiting
  senescence flag makes annuals — which is **turnover** (Arc A2) arriving
  in the herb layer for free, with generation times measured in one run.
- **The genome gets loci that visibly matter**: flower colour bands, fruit
  size against seed endowment (the deferred `LOCUS_SEED_STRATEGY` finally
  has something to spend on), determinate-N as a heritable integer with an
  unmissable phenotype.
- **A real answer to "the species are all one plant"** that does not fight
  the composition problem: a sunflower next to a tree is not a smaller
  tree, whatever the palettes do.

## 5. What this does **not** fix, deliberately

Tree-vs-tree sameness. The four-looks verdict ("the biggest differences
are still size and color") is about the woody archetype, and its levers
are the ones already queued: crown recession (C1), whorls (C3), and the
allocation split (λ) behind them. Organs and determinacy add *new
archetypes* beside the tree; they do not make two oaks read differently.
Both lines are needed; neither substitutes for the other.

## 6. Sequencing

1. **Behind A2 (the generation loop).** New small species multiply organism
   counts, and today seeds are immortal, grass cannot die, and the
   4,095-slot ceiling is a debug-only check. Land P3 first or the herb
   layer becomes a slot leak with flowers on it.
2. **2b + 2a together as one package** (determinate axes + organs) — the
   sunflower is the acceptance artifact: a card of a rosette, a stalk, and
   a flower head that reads as *not a tree* at a glance. Blind against a
   shrub.
3. **2d (climbing) as its own package** — it touches `anchor_support` and
   deserves the structural care; pairs naturally with the physical-trees
   work since both amend what "supported" means.
4. **Whorls (C3)** stay where the queue has them; the rosette variant rides
   with whichever lands second.

## 7. The three calls — all DECIDED, owner, 2026-08-23

1. **Does fruit carry the seed, or convert in place?** Carrying gives real
   dispersal (and creature-borne spread); converting is simpler and still
   reads as fruit. Recommendation: carry — dispersal is the evolution
   arc's missing spatial half, and a falling fruit is a satisfying object
   in a way an in-place conversion is not.
   **DECIDED: fruit carries the seed.**
2. **May a vine attach to player-built walls?** The `attaches` bit makes
   this a data decision, not an emergent accident. Recommendation: yes —
   a wall slowly greening over is exactly "the world responds to you".
   **DECIDED: yes, vines may attach to player-built walls.**
3. **Are annuals wanted?** Post-fruiting death gives the herb layer
   seasons-like turnover without seasons. Recommendation: yes, on the
   herb archetypes only.
   **DECIDED: yes, annuals on the herb archetypes.**

All three land as the organ package's `.ron` defaults. A consequence worth
naming now that annuals are in: the generation-loop package (A2/P3) rises
in the P-lane's priority — annuals *are* turnover, fruit-borne seeds *are*
dispersal, and both dead-end into the immortal seed bank and the
debug-only organism-slot ceiling until that package lands.
