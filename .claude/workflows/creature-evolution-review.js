export const meta = {
  name: 'creature-evolution-review',
  description: 'Design review: how do creatures evolve rather than being hand-authored',
  phases: [
    { title: 'Propose', detail: 'four independent architecture proposals from a compressed constraints brief' },
    { title: 'Prosecute', detail: 'one agent reads the raw findings log and hunts for already-reverted ideas' },
  ],
}

const ROOT = 'C:/Users/Scott/Code/Pixel Physics/.claude/worktrees/creatures'

const BRIEF = [
'You are reviewing the architecture of a falling-sand physics sandbox (Rust) that has creatures.',
'The owner wants to stop hand-authoring creatures. Today "ant" and "beetle" are .ron files. The goal',
'is a framework where such creatures are OUTCOMES of evolution, not files.',
'',
'FIRST TARGET (decided, do not re-litigate): GUIDED DIVERGENCE. One or two authored ancestral species,',
'and evolution differentiates them into niches. OPEN-ENDED (a single ancestor from which predator,',
'herbivore and burrower all emerge) is the stated direction, and you ARE asked to assess its',
'feasibility and name what it would additionally require -- but not to assume it as the first milestone.',
'',
'KEYSTONE (decided by the owner, build on it, do not re-argue it): energy becomes a property of FOOD,',
'not of the eater. Today eat_energy is a constant on the EATING species, so food has no nutritional',
'identity and herbivore-vs-carnivore specialisation is unreachable. Fixing this also closes an energy',
'pump and is the same work as putting plants into the energy ledger.',
'',
'=== HARD CONSTRAINTS (violating these makes a proposal worthless here) ===',
'1. DETERMINISM IS REQUIRED, same-build. No transcendentals (exp, tanh, sin, pow) anywhere a decision',
'   reads them -- they are the named cross-platform determinism trap. The activation function is',
'   squash(x) = x / (1 + abs(x)). The choice function squares rather than exponentiates. Headings come',
'   from an 8-entry table. Any proposal needing softmax, Gaussian sampling or tanh must say how it',
'   stays deterministic.',
'2. FRAME COST IS A HARD CONSTRAINT, not a tiebreaker. Say what your proposal costs. Measured today:',
'   creatures are essentially FREE (55 ants + 30 trees worst 21.3ms/mean 1.50ms against a 0-ant control',
'   at 24.1/1.59). The world (trees, cellular automata, fields) dominates. Per-cell per-frame work is',
'   the expensive kind; per-creature per-tick work is cheap (creatures tick every 6-8 frames).',
'3. EXACTNESS IS NOT A GOAL. The project optimises for "looks good and realistic, in motion, at play',
'   scale". A mechanism whose only advantage is numerical precision buys nothing. A graded outcome',
'   beats a binary one. Stopping work early is a legitimate optimisation.',
'4. IT MUST FEEL SATISFYING. Above correctness of any individual mechanic. Legible feedback beats',
'   exactness.',
'',
'=== THE ARCHITECTURE AS IT EXISTS ===',
'BODY. A creature is 1..N connected cells on a shared "organism substrate" (the same substrate plants',
'use). Two body plans: Chain(n), where the head picks a move and the body follows snake-fashion (so it',
'flows over any terrain and is exactly one cell wide); and Rigid(offsets), where every cell translates',
'by the same offset (any shape, but it cannot enter a gap narrower than itself). Rotation was rejected',
'as an unsolved problem in a falling-sand grid; facing is a MIRROR of the authored template only.',
'The beetle being unable to follow an ant into a one-cell tunnel falls out of Rigid passability with no',
'hiding logic anywhere. Organism slots: 4095 max (12-bit index + 4-bit generation), with a free list.',
'',
'BRAIN. A fixed-topology network whose weights ARE the genome. 16 inputs, 4 hidden units (each with a',
'self-recurrence reading last tick), 9 outputs. Stored as one flat Vec<f32> of 248 weights in four',
'row-major blocks: [0..144) input->output as input*9+output, [144..208) input->hidden, [208..212)',
'hidden self-recurrence, [212..248) hidden->output. A weight below W_EPS (0.01) means NO CONNECTION:',
'not evaluated and not charged for. That is how evolution deletes a wire. Each live wire costs',
'synapse_cost energy per tick, which is the sparsity pressure.',
'  Topology evolution (NEAT-style) was explicitly REJECTED: variable-length graph genome, speciation',
'  machinery, hours-of-noise bootstrap, illegible results. Topology is what got caged; weights evolve',
'  freely. A proposal may revisit the STORAGE FORMAT but must not smuggle topology evolution back in',
'  without saying so plainly and defending it.',
'  THE 16 INPUTS: Bias(always 1.0), PheroAFront, PheroALateral, PheroBFront, PheroBLateral,',
'  MoistureFront, MoistureLateral, LightHere, TempAboveAmb, FoodAdjacent(0/1), AtNest(0/1),',
'  Energy(fraction), Carrying(0/1), Crowding, PheroAAlong, PheroBAlong.',
'  THE 9 OUTPUTS: Turn, Move(P of stepping), EmitA, EmitB, Dig, Drop, Persist, Tumble, Caution.',
'',
'POSITIONAL LAW AND THE MIGRATION HAZARD. Slot indices are the meaning; a stored genome is unlabelled',
'numbers. Because the input->output block is row-major (input * BRAIN_OUTPUTS + output), APPENDING AN',
'INPUT IS SAFE (it appends whole rows) but APPENDING AN OUTPUT IS NOT (it changes the row stride and',
'silently renumbers every weight with input >= 1). This already happened: 168 -> 188 (inputs, lawful)',
'-> 248 (outputs, unlawful). Harmless only because NOTHING PERSISTS A GENOME YET. The moment genomes',
'are heritable this is data corruption. A sparse (input_id, output_id, weight) encoding with stable ids',
'has been floated; its costs were identified as: it invents a mutation-operator design problem (perturb',
'vs add vs delete, and their relative rates) that dense uniform mutation does not have; it biases search',
'toward refining over exploring; the positional law relocates to the enum rather than disappearing; and',
'an unordered list is a determinism hazard unless canonically sorted or expanded to dense before',
'evaluation. Treat that as an open decision to be resolved, not a settled one.',
'',
'FIELDS AND CHANNELS. Two "pheromone" planes at cellular-automaton resolution, meaning-free by',
'construction (nothing in the engine knows what channel A means), with deposit/diffuse/decay and tile',
'sleeping. Separately, coarse fields at 1/8 resolution: moisture, light, temperature. The light channel',
'swings 20:1 over a day/night cycle BY DESIGN, so any decision reading it must divide the oscillator out',
'(there is a noon_equivalent_light helper); temperature will need the same treatment the day anything',
'gates on it.',
'',
'SPECIES DATA. A species .ron authors: body plan, tick_interval, start_energy, idle_cost, move_cost,',
'synapse_cost, eat_energy, hunger_fraction, food (a list of MATERIAL NAME STRINGS), nest material,',
'dig_force, nest_memory, sensor_offset, and the brain weights as a sparse named wiring list.',
'  Notably general already: the beetle eats ants with NO predation code, because "ant" is a material and',
'  food is a name list. Digging is gated by the target material penetration_resistance against the',
'  species dig_force -- never a name whitelist -- so a softer stone becomes diggable with no code change.',
'  Notably NOT general: diet is a list of hardcoded strings; every creature senses identically (sensor',
'  acuity is not a species trait, only sensor_offset is); anatomy is authored, never inherited.',
'',
'PLANTS. Share the organism substrate. Have their own node economy. Moss spreads by division on damp,',
'shaded stone (poikilohydric -- it stalls on dry sunlit ground, which is correct). Trees regrow leaves',
'continuously. Plants have NO energy account visible to the creature ledger, and reclaiming a fully dead',
'plant slot is a known open gap wanting a BFS-from-roots liveness check.',
'',
'ENERGY. An EnergyLedger tracks granted / eaten / metabolized / moved / synapse_tax / died_holding. It',
'is an ACCOUNTING IDENTITY, NOT A CONSERVATION LAW: granted, eaten and died_holding are free terms',
'defined as whatever happened, so they move both sides together by construction and the ledger cannot',
'detect energy creation. There is a live pump: a corpse cell is worth the eater full eat_energy no',
'matter what the dead creature had, so an ant granted 900 that starves at exactly 0 leaves two corpse',
'cells worth 120 each, and a colony sustains itself on its own dead indefinitely. The property that',
'actually matters is weaker than conservation: NO LINEAGE MAY EXTRACT UNBOUNDED ENERGY FROM A CYCLE IT',
'CONTROLS.',
'',
'=== MEASURED FINDINGS. DO NOT RE-DERIVE THESE. THEY WERE EXPENSIVE. ===',
'F1. Lateral pheromone sensors read EXACTLY ZERO on a horizontal surface: they sample ahead-left and',
'    ahead-right at the full sensor offset, which on a surface puts both in open air. The Jones/Physarum',
'    sensor triad assumes agents in open 2D; this is a side-view world where creatures walk on surfaces.',
'    Homing is therefore run-and-tumble chemotaxis (run while an along-heading gradient improves,',
'    re-orient at random when it does not), NOT steering. MoistureLateral has identical geometry and is',
'    predicted (untested) to read a large SPURIOUS value -- one sample in the air, one in the ground.',
'F2. NO CREATURE CAN PERCEIVE ANOTHER CREATURE KIND. The Crowding input counts undifferentiated',
'    MaterialKind::Creature within radius 2. An ant cannot tell an ant from a beetle, at any range.',
'F3. PREDATION HAS NEVER BEEN A SELECTIVE PRESSURE. Runs with 0 beetles and 9 beetles came out',
'    BIT-IDENTICAL over 6000 frames -- no beetle touched an ant all run. A beetle has no scent',
'    instincts, so it run-and-tumbles at random and essentially never finds prey.',
'F4. The Dig output gates BOTH digging AND eating/pickup. Evolution cannot separate "excavate" from',
'    "feed"; they are one gene.',
'F5. THE HIGHEST-LEVERAGE CHANGES HAVE REPEATEDLY BEEN ENVIRONMENTAL, NOT MECHANICAL. Three times the',
'    answer was the ecology rather than the creature: the food source (a finite corpse pile vs regrowing',
'    leaves), the cost structure, and reachability. Prefer adding pressures and ecology over adding',
'    creature machinery. Prefer moving hardcoded policy into the genome over tuning it.',
'F6. AN ABLATION IN A BROKEN ECONOMY MEASURES NOTHING. Eight of ten authored instincts read',
'    bit-identical in a degenerate scene.',
'F7. Flat ground is degenerate (an ant there usually has exactly one legal step and is not deciding).',
'    Hand-built stand-ins for generated terrain have been unrepresentative THREE times. Use the real',
'    generator.',
'F8. synapse_cost is expressed in absolute energy but only means anything RELATIVE TO start_energy.',
'    Cutting the budget 10x once made brains cost 80% of a life and invalidated a whole sweep.',
'F9. "IMMOBILITY WINS" WAS A HORIZON ARTIFACT. An idle ant always starves at tick 900 whatever the run',
'    length, so mean-population-over-a-run is a function of the horizon: doing nothing scores 0.90 over',
'    1000 ticks and 0.30 over 3000. Runs had been 1000 ticks, i.e. 1.1 idle lifetimes -- the single most',
'    favourable horizon for doing nothing that still kills it. At 3000 ticks foraging wins (+0.247).',
'F10. IDENTICAL OUTPUT ACROSS SETTINGS HAS TWO CAUSES, not one: the knob was never connected, OR the',
'    knob is saturated. eat_energy 120 vs 700 read bit-identical because a single 120 meal already',
'    outlived the run.',
'F11. Outcomes here have ENORMOUS SPREAD. One genome across 8 world seeds: a forager scored mean 0.504',
'    with sd 0.116 (23% of itself) while a do-nothing genome scored sd 0.002. Any comparison needs',
'    paired arms and enough seeds; a bar set from one run will flake.',
'F12. A metric can measure the SPAWN LAYOUT rather than behaviour. This has happened three times. Most',
'    recently: 52 ants were requested, terrain silently accepted only 23, and the outcome was still',
'    divided by 52. The zero-genome control did NOT catch it, because both arms carry the same handicap',
'    so it CANCELS in the difference and survives in every absolute number.',
'F13. A rule must state which OBJECT it evaluates -- a cell, a section, or a whole piece -- and the',
'    quantities it needs must be defined for that object. Getting this wrong has cost real rework twice.',
'F14. When a rule must tell apart two things that can look identical, STATE THE DIFFERENCE AS DATA (a',
'    bit on the cell), do not infer it from shape. Four successive support models tried to infer it from',
'    geometry and every one was either too strong or too weak, never both.',
'F16. THE GENOME SPACE IS NOT DEGENERATE -- measured, 400 random genomes x 8 seeds, completed after this',
'    brief was first drafted. Behaviour-space coverage 26 of 81 cells. Survival spread 0.103 to 0.541',
'    against a do-nothing baseline of 0.300, so HALF of all random genomes are WORSE than doing nothing',
'    and a few are clearly better. The hand-authored ant scores 0.504 and IS BEATEN by random genome',
'    r029 at 0.541 -- the authored animal is measurably suboptimal, which is exactly the gradient',
'    selection needs. At least three DISTINCT successful strategies appeared: a short-range forager',
'    (travelled 10, feeding 0.55), a long-range directed commuter (travelled 30, commute 0.88), and a',
'    SESSILE grazer that never moves yet still feeds (travelled 0.000, feeding 0.370, survival 0.447),',
'    presumably sitting on moss. The WORST genomes all travel far and never feed (travelled 52, feeding',
'    0.000, survival 0.103), so movement without food is strictly worse than stillness. The median',
'    random genome never moves at all. Selection therefore has real structure to act on TODAY, before',
'    any work proposed in this review. The open question is not "can evolution find anything" but',
'    "what does it need in order to find DIVERGENT things".',
'F15. A size cap must BOUND WORK, never GATE WHETHER SOMETHING HAPPENS. An "if too_big { return }" is a',
'    claim that the largest cases deserve the least behaviour.',
'',
'=== WHAT YOU MUST PRODUCE ===',
'A design proposal. For EVERY substantive claim or mechanism you propose, you must state:',
'  (a) WHAT WOULD BE MEASURED to know it works -- a specific number from a specific harness, and what',
'      it reads when nothing is wrong (a metric that cannot be sanity-checked against a known-good case',
'      is not acceptable here);',
'  (b) WHAT WOULD FALSIFY IT;',
'  (c) WHAT IT COSTS in frame time and in new tuning knobs (a knob nobody can tune in either direction',
'      is a counterweight for a bug, not a model);',
'  (d) WHICH EXISTING FINDING (F1-F15) OR DECISION IT MIGHT CONTRADICT, named explicitly. If you believe',
'      a finding should be overturned, say so and say what measurement would justify it.',
'Break the work into STAGED MILESTONES, each independently shippable and judgeable BY EYE, in order.',
'Prefer the smallest change that could produce visible divergence. Be concrete about data structures',
'and where code would live. Say plainly when you are uncertain rather than writing confident prose.',
'',
'TOOLS: You may READ files under ' + ROOT + ' to ground yourself (src/sim/creature.rs, brain.rs,',
'organism.rs, world.rs, assets/species/*.ron, examples/creature_space.rs are the relevant ones).',
'DO NOT run cargo build, cargo test, or cargo run: five agents building concurrently thrash a 4-core',
'machine and a design review does not need to compile anything. Read source instead.',
'Do not write or edit any file.',
].join('\n')

const PROPOSAL_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['title', 'core_claim', 'design', 'milestones', 'risks', 'contradictions', 'open_questions'],
  properties: {
    title: { type: 'string' },
    core_claim: { type: 'string', description: 'One paragraph: the single most important thing you are asserting.' },
    design: { type: 'string', description: 'The full proposal, in detail, including data structures and where code lives.' },
    milestones: {
      type: 'array',
      items: {
        type: 'object',
        additionalProperties: false,
        required: ['name', 'what_ships', 'measurement', 'falsifier', 'cost'],
        properties: {
          name: { type: 'string' },
          what_ships: { type: 'string' },
          measurement: { type: 'string', description: 'The specific number, from which harness, and what it reads when nothing is wrong.' },
          falsifier: { type: 'string' },
          cost: { type: 'string', description: 'Frame time, new tuning knobs, blast radius across files.' },
        },
      },
    },
    risks: { type: 'array', items: { type: 'string' } },
    contradictions: {
      type: 'array',
      description: 'Named findings F1-F15 or decisions this might contradict, and why you think it is justified.',
      items: { type: 'string' },
    },
    open_questions: { type: 'array', items: { type: 'string' } },
  },
}

phase('Propose')

const proposals = await parallel([
  () => agent(BRIEF + '\n\n=== YOUR ASSIGNMENT ===\n' +
    'GENOME ENCODING AND ANATOMY. You own one question: what goes in the genome, and how is it encoded?\n' +
    'Today the genome carries BEHAVIOUR ONLY (248 brain weights). Body plan, size, tick rate, dig force,\n' +
    'sensor reach, metabolism and diet are all authored constants in the .ron. So evolution can change\n' +
    'what a creature does but never what it IS -- which is the single largest obstacle to the owner goal.\n' +
    'Address: (1) Resolve the dense-vs-sparse storage question, weighing the costs already identified,\n' +
    'and give a recommendation with a migration path that is safe BEFORE heritable genomes exist.\n' +
    '(2) Which anatomical traits should become genes, and what is the MINIMUM set that produces visibly\n' +
    'different animals? Body plan is discrete (Chain(n) vs Rigid(shape)) -- how does a discrete,\n' +
    'structural trait mutate sensibly without producing nonsense bodies or breaking the physics?\n' +
    '(3) Should sensor acuity/range/which-channels be heritable, and what does that cost given that\n' +
    'sense() currently computes all 16 inputs unconditionally for every creature every tick?\n' +
    '(4) How do behavioural genes and anatomical genes coexist in one representation, given that a body\n' +
    'change may invalidate the brain wiring that assumed the old body?',
    { label: 'genome+anatomy', phase: 'Propose', schema: PROPOSAL_SCHEMA }),

  () => agent(BRIEF + '\n\n=== YOUR ASSIGNMENT ===\n' +
    'ENERGY AND TROPHIC STRUCTURE. You own the keystone, which the owner has already decided to adopt:\n' +
    'energy becomes a property of FOOD rather than of the EATER.\n' +
    'Address: (1) Where does the energy live? Per material? Per cell (there is a per-cell aux byte, but\n' +
    'it already carries two opposite-facing conventions: on a Liquid aux==0 means FULL, on a Powder\n' +
    'aux==0 means DRY)? Per organism? Say exactly, and say what it costs in memory and per-cell work.\n' +
    '(2) How do PLANTS enter the ledger? They have their own node economy and photosynthesis is the\n' +
    'largest free energy source in the world, entirely outside the current accounts. A closed ledger is\n' +
    'unreachable until this is resolved.\n' +
    '(3) How does DIET become an evolvable trait rather than a list of hardcoded material-name strings?\n' +
    'The engine has a strong existing pattern here: digging is gated by material penetration_resistance\n' +
    'against species dig_force, never a name whitelist. Is there an equivalent for digestion? What\n' +
    'material property would a herbivore and a carnivore specialise on, such that specialising is a real\n' +
    'trade-off rather than a free lunch?\n' +
    '(4) Kill the corpse pump, and state the invariant that replaces "the ledger balances" -- remembering\n' +
    'that the property actually needed is "no lineage may extract unbounded energy from a cycle it\n' +
    'controls", which is weaker than conservation and must be TESTABLE.\n' +
    '(5) What does this unlock for guided divergence specifically? Be concrete about the first visible\n' +
    'result: what would a player SEE that they cannot see today?',
    { label: 'energy+trophic', phase: 'Propose', schema: PROPOSAL_SCHEMA }),

  () => agent(BRIEF + '\n\n=== YOUR ASSIGNMENT ===\n' +
    'ENERGY AND TROPHIC STRUCTURE -- SECOND OPINION, ADVERSARIAL. Another agent is designing the\n' +
    '"energy belongs to food" architecture. YOUR JOB IS TO ATTACK THAT PREMISE and find what it breaks or\n' +
    'what it will cost that its advocates are not seeing. The owner has approved it, so you are not\n' +
    'trying to veto it -- you are trying to make sure it is adopted with eyes open, and to find the\n' +
    'cheaper or safer variant if one exists.\n' +
    'Specifically consider: (1) What does making energy a per-food-cell quantity cost in memory and in\n' +
    'per-cell per-frame work, given that frame cost is a HARD constraint and per-cell work is the\n' +
    'expensive kind? Is there a cheaper encoding (per material, per organism, statistical) that buys\n' +
    'most of the benefit? (2) Does conserved energy actually produce BETTER-LOOKING behaviour, or just\n' +
    'more correct bookkeeping -- remembering that exactness is explicitly not a goal here and that a\n' +
    'mechanism whose only advantage is precision buys nothing? (3) What NEW degenerate attractors does a\n' +
    'closed energy economy create? Naive ecologies go extinct; a closed loop can collapse to a single\n' +
    'trophic level or oscillate to extinction. (4) Is there a way to get evolvable diet and kill the\n' +
    'corpse pump WITHOUT a full trophic accounting -- a cheaper 80% solution? (5) What is the strongest\n' +
    'argument that this work should be DEFERRED behind something else, and is that argument right?\n' +
    'Produce a real proposal (your own best design), not just objections.',
    { label: 'energy-devils-advocate', phase: 'Propose', schema: PROPOSAL_SCHEMA }),

  () => agent(BRIEF + '\n\n=== YOUR ASSIGNMENT ===\n' +
    'SPECIATION, SELECTION AND ENVIRONMENT. You own the question of what actually makes one lineage\n' +
    'become two, and what environment can sustain several viable strategies at once.\n' +
    'Context you must respect: nothing yet reproduces or inherits -- creatures are hand-placed and the\n' +
    'genome is per-species, not per-individual. Stage 4 (queen, eggs, worker hatching, corpse on death,\n' +
    'slot reclamation, genome inheritance with mutation) is planned but unbuilt. Population target for\n' +
    'anything colony-visible is 50+; stigmergy has a minimum viable population.\n' +
    'Address: (1) The minimum machinery for heritable variation with selection, given the organism\n' +
    'substrate and the 4095-slot ceiling. What is the reproduction model, and what does mutation look\n' +
    'like concretely (rates, per-weight vs global scale -- note the ant authored brain has a hand-derived\n' +
    'gate living at +/-30 while every other authored weight is between 0.2 and 2.5, so a single global\n' +
    'mutation step size will either never move the gate or shred everything else)?\n' +
    '(2) GUIDED DIVERGENCE, the first target: given one or two authored ancestors, what makes them\n' +
    'differentiate into niches rather than converging on one strategy? What environmental structure is\n' +
    'required -- spatial heterogeneity, multiple food types, refuges, seasonality? Remember F5: the\n' +
    'highest-leverage changes here have repeatedly been environmental, not mechanical.\n' +
    '(3) OPEN-ENDED: assess feasibility honestly. What would a single ancestor need for predator,\n' +
    'herbivore and burrower to emerge from selection alone? Name the specific blockers. Is it reachable\n' +
    'in this engine, and if so roughly how far away? A confident "not yet, and here is precisely why" is\n' +
    'more useful than optimism.\n' +
    '(4) How do we avoid the sessile-freeloader attractor (doing nothing beating doing something), which\n' +
    'has already dominated three separate experiments, most recently as a pure horizon artifact (F9)?\n' +
    '(5) Selection pressure requires predation to work, and F3 says it currently does not fire at all.\n' +
    'What is the minimum fix -- a third scent channel as fear/prey scent has been proposed.',
    { label: 'speciation+environment', phase: 'Propose', schema: PROPOSAL_SCHEMA }),
])

const good = proposals.filter(Boolean)
log(good.length + ' of 4 proposals returned; sending to the findings-log prosecutor')

phase('Prosecute')

const CRITIQUE_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['verdict_summary', 'dead_ends', 'unfalsifiable_claims', 'conflicts_between_proposals', 'cost_concerns', 'strongest_ideas', 'recommended_order'],
  properties: {
    verdict_summary: { type: 'string' },
    dead_ends: {
      type: 'array',
      description: 'Proposals that re-tread something already tried, reverted, or measured false. Cite the source.',
      items: {
        type: 'object',
        additionalProperties: false,
        required: ['proposal', 'claim', 'why_dead', 'source'],
        properties: {
          proposal: { type: 'string' },
          claim: { type: 'string' },
          why_dead: { type: 'string' },
          source: { type: 'string', description: 'Where in CLAUDE.md or creature-direction.md this is recorded.' },
        },
      },
    },
    unfalsifiable_claims: { type: 'array', items: { type: 'string' } },
    conflicts_between_proposals: { type: 'array', items: { type: 'string' } },
    cost_concerns: { type: 'array', items: { type: 'string' } },
    strongest_ideas: { type: 'array', items: { type: 'string' } },
    recommended_order: { type: 'array', items: { type: 'string' } },
  },
}

const critique = await agent(
  'You are the FINDINGS-LOG PROSECUTOR for a design review of a Rust falling-sand physics sandbox.\n\n' +
  'Four agents have proposed architectures for making creatures EVOLVE rather than be hand-authored.\n' +
  'They worked from a COMPRESSED brief. You are the only agent reading the RAW source of truth, and\n' +
  'your job is to catch what compression lost.\n\n' +
  'READ THESE FIRST, IN FULL:\n' +
  '  ' + ROOT + '/CLAUDE.md  (the method; every rule in it was paid for)\n' +
  '  ' + ROOT + '/Reports/creature-direction.md  (especially section 13, subsections 13a through 13o,\n' +
  '      which is the findings log from the sessions that built this and CONTRADICTS the design above it\n' +
  '      in about a dozen places, each with the measurement that forced the change)\n' +
  '  ' + ROOT + '/Reports/design-philosophy.md  (settles arguments about constants and scope)\n\n' +
  'You may also read source under ' + ROOT + ' to check a claim.\n' +
  'DO NOT run cargo build/test/run -- reading is enough, and builds are slow here. Do not edit any file.\n\n' +
  'YOUR PRIMARY JOB, in priority order:\n' +
  '1. DEAD ENDS. Find every place a proposal re-proposes something already TRIED AND REVERTED, or\n' +
  '   already MEASURED FALSE, or forbidden by a decision. This is the single highest-value thing you\n' +
  '   can produce: generic design reasoning confidently re-invents dead ends, and this codebase has an\n' +
  '   unusually rich record of them. Cite where each is recorded.\n' +
  '2. UNFALSIFIABLE CLAIMS. Every proposal was required to state a measurement and a falsifier. Flag any\n' +
  '   that are vague, that cannot fail, or whose metric would read the same when nothing is wrong.\n' +
  '   Note especially metrics that would measure the SCENE or the SPAWN LAYOUT rather than behaviour --\n' +
  '   that specific error has occurred three times.\n' +
  '3. CONFLICTS. Where do the four proposals contradict each other on a question of fact or design? Say\n' +
  '   which is right, or what measurement would settle it.\n' +
  '4. COST. Anything that would cost the dirty-rect render skip, keep chunks awake, add per-cell\n' +
  '   per-frame work, or introduce a tuning knob nobody can tune in either direction.\n' +
  '5. STRONGEST IDEAS. Be fair: name the ideas most worth keeping, including any that are\n' +
  '   underdeveloped but promising.\n' +
  '6. RECOMMENDED ORDER. Given the owner target of GUIDED DIVERGENCE first, with energy-as-food-property\n' +
  '   as the approved keystone, what is the right sequence of work?\n\n' +
  'Be specific and adversarial. Do not summarise the proposals back; assume the reader has them.\n\n' +
  '=== THE FOUR PROPOSALS ===\n' +
  good.map((p, i) => '\n\n########## PROPOSAL ' + (i + 1) + ': ' + p.title + ' ##########\n' + JSON.stringify(p, null, 1)).join(''),
  { label: 'findings-log-prosecutor', phase: 'Prosecute', schema: CRITIQUE_SCHEMA, effort: 'high' }
)

return { proposals: good, critique }
