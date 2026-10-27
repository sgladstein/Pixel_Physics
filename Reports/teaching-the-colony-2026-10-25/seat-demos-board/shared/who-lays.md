# Shared piece: choosing who lays (and from which genome)

Owner: **queen** seat. State: PROPOSED 2026-10-25 08:15Z. Needed by: queen, colony mind.

## What exists
An egg inherits the layer's brain (with mutation); laying is each ant's own gene; `PIXEL_PHYSICS_BREEDING=queen` = one breeder with succession; steer applies the queen rule to the player's colony.

## Interface (proposal, details after the queen plan)
A per-colony "father store": a small list of genomes (from chosen workers) that the laying queen's eggs draw from, so "who lays" becomes "whose line the next eggs carry". A hook `brood::egg_genome(world, layer, colony) -> Genome` with the default = today's behaviour exactly. Colony mind: tell me what you'd call it with (a choice of whose line, a weighting?).

## Agreed with colony mind (2026-10-25 08:22Z)
- Per colony: a weighted list of `(source, weight)` entries, where source is a living ant id or a stored genome (a dead or shelved one).
- `egg_genome` draws a second parent (the "father") from that list by weight; **empty list or all-zero weights = today's behaviour exactly** (the layer's own genome).
- A per-colony mix knob `p` in [0,1]: the chance an egg draws from the store at all; 0 = today.
- Callers update weights whenever they like (every few hundred ticks for the mind; per blessing for the queen). No per-egg callback.
- Whether a draw is a clone of the father or a crossover of layer x father: queen plan decides (crossover is what the queen design asks for); exposed as an enum so the mind can pick.


## BUILT (queen seat, PR #908, 2026-10-25)
`crate::sim::fathers`: `World::fathers: Option<Box<Fathers>>`; `Fathers { colonies: BTreeMap<u32, Store>, build, mutate, log, genomes }`;
`Store { fathers: Vec<Father{mind, genome, weight}>, p, queue, wild, queen_only }`. Hook: `try_bud` after `mind::inherit`
(not at hatch). Keyed draw. A colony-mind driver sets weights and p on its colony's Store; `queen_only=false` makes every
egg of the colony draw. `Fathers::log` lists every fathered egg (child, layer, mind, queued, wild).
