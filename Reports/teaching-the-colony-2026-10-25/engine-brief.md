# Brief: what the game and the ant are today (for outside researchers)

## The game
Pixel Physics is a Rust falling-sand style world where every pixel is simulated
(powders, liquids, gases, fire, heat, light, weather, soil moisture). The main
game is "the lab": a sealed box of soil (roughly 512x320 to 1024x640 cells)
under grow lights, where plants and an ant colony live. Also present or
being built: plants that grow from seeds, flower, seed, compete for light and
water, die and rot; bug predators (spiders, beetles, a centipede that raids
the surface); rain and water. One set of physics for every world. The world
will grow larger later (streaming).

## What one ant is
- Each tick the ant's "mind" picks a VERB and a TARGET cell from a short list
  of options: walk, turn around, rest, dig, pick up, drop, eat, feed (a hungry
  nestmate or larva), attack (not yet active).
- Each option is scored by a small neural network from ~45 features of that
  option: what is in the target cell (air, soil, spoil, food, brood, corpse,
  stone, water, ant), three scent planes there (A = home trail, B = food
  trail, alarm) as strength and gradient, a memory of where it last found food,
  can it stand/fall/climb, hardness, wetness, depth, headroom, roofed, nestmate
  or stranger, laden, hungry, larva and larva hunger, corpse, crowding, room
  size, direction home, warmth. Plus ~22 context senses (its own hunger etc).
- **Weights are fixed at birth from the genome. There is no learning during
  an ant's life yet.** Everything an ant knows was put there by evolution or
  by offline training.
- Ants are blind by default (sight is a heritable trait), they smell.
- Ants starve, age and die; they eat from crop and colony store; they lay
  eggs (laying is the ant's own decision, a gene); larvae must be fed; an egg
  inherits the layer's brain (with mutation). A colony is therefore a
  *pool of genomes*, and its surviving mixture is the product.
- Genome: ~2,800 brain weights + body genes (size, speed, sight range, etc).

## How ants are trained now (offline, not by a player)
Colony selection: many colonies run in parallel boxes, the best colonies (food
brought home, nest dug, rooms, survival) seed the next round. Lessons (scored
test situations) are stacked onto the current best founder.

## What a player can do in the lab today
Plant seeds; place/move/remove grow lamps; drag to lay food or home scent
trail; drop alarm scent; fling an ant; open any plant or animal's page
(genome read back as plain sentences, a per-ant life record: steps, digs,
deliveries, young; graves record cause of death); lineages have generated
names; a specimen shelf keeps, clones and mutates individuals. A playable
"you are the founding queen" body exists in the engine but is switched off
(walk, eat, carry, dig, feed, lay, lay trail by key). Measured: a hand-laid
trail does NOT measurably pull workers to food; a scripted player-queen
founded a colony more often than the queen's own genome did; four beetles
kill a lone queen every time.

## Known current problems
Trained colonies starve when food is far from home (they wander randomly);
boom-bust populations; fed ants idle on the mound.

## The owner's standing rules (do not design against these)
- One brain design for every ant: no hand-made castes; roles should emerge.
- Ants must dig a multi-room nest AND forage the whole game.
- Graded outcomes beat binary ones; every player verb must produce a visible
  consequence (debris, impulse, mark, behaviour change). "It has to feel
  satisfying."
- Frame cost is a hard constraint (hundreds of ants, every pixel simulated).
- Realistic, natural feel is valued. Features default on.

## The owner's question (verbatim, 2026-10-25)
"I want to really entirely deep focused session into exploring all the
possible ways that a player could interact with our ant colony to teach them
behaviors and what types of behaviors expected and unexpected for the player.
Could we potentially see that would be fun. I think you either come at this
from the outside perspective as maybe the player is like a gardener farmer
gnome who grows plants, lay seeds, Waters them and then also like tames a
colony or keeps a colony and trains them for things. Or we come at it from
the inside perspective? And the player is like the Queen Aunt and it's their
colony which has the nice tie-in for controlling genetics. Cuz then the queen
could lay all the eggs and the player as the queen could choose which members
of The colony are doing the best and lay their offspring or choose different
weights for different genetics based on how the colonies currently doing. But
again, how is the queen teaching the colony? Different behaviors in a way
that's interesting and more complex than you know. You walk to some food and
you that your colony follows you. Let's think outside the box."
