export const meta = {
  name: 'world-review',
  description: 'Multi-agent review of the generated world: six lenses over rendered strips, then a prosecutor against the findings record',
  phases: [
    { title: 'Lens review', detail: 'six independent lenses, each grounded in the rendered images first and code second' },
    { title: 'Prosecute', detail: 'one agent reads the raw Reports and hunts already-reverted ideas, unfalsifiable claims and cost violations' },
  ],
}

// The orchestrating session renders these before invoking the workflow (one
// release build, then straight binary runs). Every lens must ground its
// findings in these images; a finding with no image citation is presumed
// speculative. The paths are stable so the workflow is re-runnable.
const ROOT = '/home/user/Pixel_Physics'
const IMG = ROOT + '/target/filmstrips/review'

const STRIPS = [
  'rolling-s1.png', 'terraced-s1.png', 'canyon-s1.png',
  'wetland-s1.png', 'arid-s1.png', 'flat-s1.png',
  'rolling-s2.png', 'rolling-s7.png', 'rolling-s13.png',
  'canyon-s2.png', 'canyon-s7.png', 'canyon-s13.png',
]
const MOOD = ['rolling-rain.png', 'rolling-night.png', 'canyon-mined.png']

const BRIEF = [
'You are one lens of a multi-agent review of the WORLD of a falling-sand physics sandbox (Rust,',
'2D side-view, Noita-class ambition). The owner has asked for a review of the current world',
'generation and a roadmap toward a world that is beautiful, varied, and satisfying to be in.',
'',
'=== VALUES (owner-stated, binding; violating these makes a finding worthless) ===',
'1. EVERYTHING SHOULD FEEL SATISFYING -- ranked above correctness of any mechanic. Graded outcomes',
'   beat binary ones; every destructive event owes debris, impulse, and a mark.',
'2. "Looks good and realistic, IN MOTION, AT PLAY SCALE -- without ruining performance."',
'3. EXACTNESS IS NOT A GOAL. A mechanism whose only advantage is precision buys nothing.',
'4. FRAME COST IS A HARD CONSTRAINT, not a tiebreaker. The chunk-sleeping + dirty-rect render skip',
'   is the entire cost model: a settled world costs ~nothing, and anything that keeps chunks',
'   permanently awake or forces full redraws is charged its real price. Precedent: a steady global',
'   wind was reverted for costing 3.55 ms standing on every scene, forever.',
'5. Simple rules, emergent outcomes. The outcome is FORBIDDEN as a target, simple rules are not:',
'   you may not hardcode "a hoodoo at x=400"; you may absolutely use a tuned weighted local rule',
'   whose side effect is hoodoos (Reports/design-philosophy.md section 2b).',
'',
'=== THE WORLD AS IT EXISTS ===',
'World: 2048x640 cells, bounded, fixed size (streaming is planned last). Viewport 512x320, camera',
'follows a playable gnome who runs, jumps, swims, digs corridors, and rides falling slabs.',
'Worldgen (src/worldgen/, ~2700 lines) is a decide/realise split: region.rs assigns 2-5 regions',
'per 512-cell window, each a Character { elev, relief, aridity, resistance, sediment }; column.rs',
'plans every column (elevation = regional wave + hills + strata-space terracing + detail + dunes;',
'soil depth tapering with slope; water table; bedrock); passes.rs realises ten passes in order:',
'stone_massif, bedrock_floor, soil_blanket, brows (overhanging stone lips at cliff edges), talus',
'(scree aprons), pockets (sealed sand/gravel lenses inside stone), ponds, soil_moisture,',
'moisture_init, life_scatter (tree seeds + moss, clustered). Parameters are data:',
'assets/worldgen.ron, 33 knobs, six presets (rolling terraced canyon wetland arid flat),',
'hot-reloadable. Only 7 of the engine\'s 23 materials are ever placed by worldgen: stone, bedrock,',
'soil, sand, gravel, water, seed/moss. Materials are data too (assets/materials/*.ron, id order',
'append-only, compiled in via include_str!).',
'Exactly three passes read the whole world (ponds, soil_moisture, moisture_init) -- an assertion',
'pins the list; they are the acknowledged streaming blockers. Every other pass declares a finite',
'column margin.',
'Guarantees the tests enforce (tests/worldgen.rs): generated terrain arrives AT REST (zero cells',
'move in 120 frames), the world sleeps within 45 frames, same seed = same world, pools have level',
'surfaces, life arrives clustered not scattered.',
'Weather exists and is a pure function of (seed, frame): rain (which CREATES water cells, capped',
'at 24 columns/frame), snow, wind gusts, lightning. Evaporation exists and DELETES water,',
'unbanked. So water is already deliberately non-conserved in both directions. There is a day/night',
'cycle (1 minute per day), stars, a moon. Sky light is drawn only where sky existed at genesis:',
'"underground" is stored once, never inferred, and a dug cavity is dark at noon. There are no',
'local light sources anywhere.',
'',
'=== NOT BUILT YET (stated in wiki/the-world.md and src/worldgen/mod.rs) ===',
'Caves, erosion, rivers, springs, rain-and-evaporation as a closed cycle, world age, streaming.',
'"The lower half of a world is a quarry rather than a destination."',
'',
'=== DO-NOT-RETRY AND MEASURED FACTS (each cost real effort; do not re-derive or re-propose) ===',
'D1. The liquid heightfield body subsystem (src/sim/liquid.rs, ~2000 lines) never runs in',
'    production. Automatic promotion was implemented and REVERTED; the subsystem was measured',
'    O(width^2), 1.2-2.6x SLOWER than the plain cellular automaton it was built to replace. The',
'    standing instruction (Reports/open-bugs-handoff.md section 6) is to settle what it is FOR',
'    before spending anything more on it. Do not propose building on it.',
'D2. Water has NO hydrostatic pressure: it cannot siphon, rise, or push. Only a 1% overfill',
'    compressibility. Anything needing water to flow uphill is proposing a new mechanism',
'    (sketched in Reports/liquid-simulation-research-r2.md section 6, unimplemented) and must say so.',
'D3. Whiskers: a spreading liquid front sheds one-cell-tall sheets that draw as a comb of detached',
'    ledges (open bug section 1). Three movement-rule fixes were measured and rejected; the standing',
'    diagnosis is that the honest fix is in how a one-cell-thick sheet is DRAWN. A river/waterfall',
'    presents exactly this geometry, permanently.',
'4. Levelling is O(width^2): a 1024-wide pool takes ~19 minutes wall clock to flatten and sleep.',
'    Stop-early knobs exist (min_transfer trades residual tilt; HORIZONTAL_TRANSFER_REACH is the',
'    accuracy-free lever, capped by MAX_REACH=32 which parallel.rs\'s safety proof is pinned to).',
'D5. The world edge is sealed in three places on purpose: out-of-bounds reads return a bedrock',
'    sentinel, the field solver treats the edge as a wall, and the ponds pass treats the edge as',
'    the tallest barrier (worlds used to drain dry when a region put low ground at an edge).',
'D6. A cap must BOUND WORK, never GATE whether something happens. Written twice: fracture once',
'    declined regions LARGER than its cap, so the biggest collapses got the least behaviour.',
'D7. Four structural support models failed the same way before the current one: geometry cannot',
'    distinguish a mountain from a stacked wall; the difference must be stored as data.',
'D8. An animated grain mode looked free in moving scenes and cost ~10 ms/frame on a SETTLED world,',
'    because what it defeats is the dirty-rect skip, and a settled world is where that skip works.',
'    Five grain modes sit behind the G key awaiting an owner decision; a settled pool changes 431',
'    cells of fill per step with ZERO occupancy changes, so its interior genuinely cannot animate.',
'D9. Editing an asset .ron does nothing until rebuild (include_str!); identical output across sweep',
'    settings means the knob was never connected.',
'D10. One dig into generated terrain currently removes far too much world (live bug, another',
'    session\'s territory: src/sim/load.rs, structural.rs, rigid.rs are READ-ONLY to this review).',
'',
'=== OWNER DIRECTIVES FOR THIS REVIEW (verbatim intent) ===',
'- Rivers/waterfalls: wanted if feasible; the world may be imagined as a 2D slice of a 3D world',
'  with water entering and leaving the frame. Reports/worldgen-design.md sections 0 and 5a already',
'  design this and record the open decision: off-plane flux REAL (a coarse (x,z) drainage map',
'  computes it) vs PLAUSIBLE (water appears at the boundary at a believable rate).',
'- Beauty means BOTH how the world is generated AND how it is drawn. Not just palettes.',
'- VARIABILITY is first-class: different biomes; the world should not all look the same.',
'- Boulders and rock formations of different types and shapes depending on the environment;',
'  prefer formations that EMERGE from a process (simulated geological history: erosion,',
'  weathering, deposition over a world age) over authored shape generators, the same',
'  evolution-not-authoring philosophy the project applies to plants and creatures. Full realism',
'  may be too ambitious -- scope it honestly.',
'- Caves: wanted, but NOT one massive continuous network (a la Noita). Rare, hidden, sealed',
'  chambers found by mining down, awesome and beautiful, "like you found a cool secret."',
'- Plants: vegetation shapes the world\'s look, but this track does NO plant development. Worldgen',
'  owns placement only (the life_scatter pass). Wants for new species go to the plant workstream',
'  as recommendations in the plant system\'s own terms.',
'',
'=== METHOD (from CLAUDE.md; binding) ===',
'Look before you measure: ground EVERY finding in a named image and region of it first, code',
'second. A finding with no image citation is presumed speculative and will be discarded. For every',
'opportunity you propose, state its DEMONSTRABILITY: the single image or printed counter that',
'would show it working, and what that artifact reads when nothing is wrong. State what each idea',
'COSTS (frame time, chunks kept awake, new knobs). Say plainly when you are uncertain.',
'',
'=== IMAGES (contact sheets: each row is a 4-shot viewport traverse of one 2048x640 world) ===',
'Preset strips at seed 1, plus seed variation on rolling and canyon:',
STRIPS.map(s => '  ' + IMG + '/' + s).join('\n'),
'Mood shots (same worlds under rain, at night, and after mining shafts):',
MOOD.map(s => '  ' + IMG + '/' + s).join('\n'),
'Per-pass cell counts for these worlds are in ' + IMG + '/pass-counts.txt.',
'',
'TOOLS: Read the images and any file under ' + ROOT + ' (source, Reports/, wiki/, assets/).',
'DO NOT run cargo build/test/run -- everything you need is pre-rendered, and concurrent builds',
'thrash the machine. Do not write or edit any file.',
].join('\n')

const FINDING_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['verdict_summary', 'findings', 'opportunities', 'assigned_answers', 'open_questions'],
  properties: {
    verdict_summary: { type: 'string', description: 'One paragraph: the state of this dimension of the world, as seen in the images.' },
    findings: {
      type: 'array',
      description: 'Problems and notable observations, each anchored in an image.',
      items: {
        type: 'object',
        additionalProperties: false,
        required: ['what', 'evidence_image', 'evidence_region', 'severity', 'also_in_code'],
        properties: {
          what: { type: 'string' },
          evidence_image: { type: 'string', description: 'Filename of the image that shows it.' },
          evidence_region: { type: 'string', description: 'Where in the image (row/shot number, left/right, feature).' },
          severity: { type: 'string', enum: ['blocker', 'major', 'minor', 'observation'] },
          also_in_code: { type: 'string', description: 'File/function that explains it, or empty if not traced.' },
        },
      },
    },
    opportunities: {
      type: 'array',
      description: 'Concrete improvements, each with demonstrability and cost.',
      items: {
        type: 'object',
        additionalProperties: false,
        required: ['name', 'what_player_sees', 'demonstrability', 'cost_estimate', 'risks', 'delegable'],
        properties: {
          name: { type: 'string' },
          what_player_sees: { type: 'string' },
          demonstrability: { type: 'string', description: 'The single image or counter that would show it working, and what it reads when nothing is wrong.' },
          cost_estimate: { type: 'string', description: 'Frame time, chunks kept awake, new knobs, blast radius.' },
          risks: { type: 'string' },
          delegable: { type: 'string', enum: ['cheap-model-safe', 'needs-frontier', 'mixed'], description: 'Could a cheaper implementation model execute this from a written spec, verified by rendered images?' },
        },
      },
    },
    assigned_answers: { type: 'string', description: 'Direct answers to the specific questions in your assignment.' },
    open_questions: { type: 'array', items: { type: 'string' } },
  },
}

phase('Lens review')

const lenses = await parallel([
  () => agent(BRIEF + '\n\n=== YOUR ASSIGNMENT: LANDFORM & BIOME VARIABILITY ===\n' +
    'You own the "world should not all look the same" mandate and the rock-formation question.\n' +
    'Walk each strip left to right and judge: does the country change character region to region,\n' +
    'or is it the same hills at different heights? Do escarpment transitions between regions read as\n' +
    'places meeting, and are they beautiful? Do 4 seeds of one preset feel like 4 places? Judge the\n' +
    'geology: strata legibility, talus, brows, dunes -- believable? Then answer:\n' +
    '(1) Should the six presets become BIOMES coexisting in one world (region Character driving\n' +
    'per-region identity: materials, formation vocabulary, water, vegetation placement, palette)\n' +
    'rather than world-global parameter sets? What exactly would that change in region.rs/column.rs?\n' +
    '(2) What environment-dependent rock-formation vocabulary is missing (freestanding boulders,\n' +
    'boulder fields, hoodoos/spires, arches, mesas, karst), and which regions should get which?\n' +
    '(3) Formation METHOD -- evaluate the spectrum: (a) extending the current pass vocabulary with\n' +
    'noise-based forms, (b) generation-time process passes (thermal/hydraulic erosion and deposition\n' +
    'run over the real materials during generation, parameterized by a world age, so forms are side\n' +
    'effects of a mechanism), (c) live in-sim erosion (flag cost; do not pursue). The owner prefers\n' +
    '(b)-style process over authored shapes if it is affordable -- assess honestly, including whether\n' +
    'a bounded process pass can keep the at-rest and 45-frame-sleep guarantees and whether its\n' +
    'freestanding outputs survive the structural model on frame 1 (D10 caution: a formation the load\n' +
    'model immediately fells is a collapse animation, not a landform).\n' +
    '(4) Read Reports/plant-substrate-v2-design.md and Reports/plant-species-authoring.md and return\n' +
    'in assigned_answers a plant_workstream_asks list: what biome identity needs from vegetation,\n' +
    'expressed in the plant system\'s real authoring levers -- recommendations only, no plant\n' +
    'development in this track.\n' +
    'Files: src/worldgen/column.rs, passes.rs, region.rs, assets/worldgen.ron, wiki/the-world.md.',
    { label: 'landform+biomes', phase: 'Lens review', schema: FINDING_SCHEMA }),

  () => agent(BRIEF + '\n\n=== YOUR ASSIGNMENT: GRAPHICS & ART DIRECTION ===\n' +
    'You own how the world is DRAWN -- actual rendering quality, not just palette/mood.\n' +
    'From the images: how does each material read per-cell (shade variation, grain)? Do surfaces\n' +
    'have any edge treatment (top-surface highlights, darkening under overhangs and at cavity\n' +
    'mouths), or is a material one flat texture? How does water read (fill-dimming toward black,\n' +
    'the one-cell-sheet whisker draw problem D3)? How do sky and light interact with terrain in the\n' +
    'day, night and rain shots -- and what is missing (no local light, no cast canopy shadows: the\n' +
    'plants respond to light but the PICTURE does not show shading)? Is there any background depth\n' +
    'behind the terrain (parallax layers), and what would one cost given the camera moves?\n' +
    'Answer specifically:\n' +
    '(1) Rank the render-side changes by beauty-per-cost, each priced against the dirty-rect skip\n' +
    '(D8 is the precedent: animation on settled water defeats the skip). Candidates to assess plus\n' +
    'your own: surface-top highlight pass, ambient-occlusion-style darkening near edges/overhangs,\n' +
    'monolayer water draw fix (D3), waterfall spray/foam via the existing particle system, canopy\n' +
    'shadow, region-keyed palette and sky tint, parallax background, the parked M6 items (bloom,\n' +
    'light falloff). Note render.rs computes colour per cell at draw time from material + shade +\n' +
    'fill + sky -- say which ideas fit that pipeline cheaply.\n' +
    '(2) The grain-mode decision (D8): five modes exist behind G. From the stills, what would you\n' +
    'recommend the owner look at, and what is the honest cost table?\n' +
    '(3) What single change most improves how a generated world reads at play scale?\n' +
    'Files: src/render.rs, src/sim/sky.rs, wiki/world-cycles.md, wiki/the-world.md.',
    { label: 'graphics', phase: 'Lens review', schema: FINDING_SCHEMA }),

  () => agent(BRIEF + '\n\n=== YOUR ASSIGNMENT: HYDROLOGY -- RIVERS AND WATERFALLS ===\n' +
    'You own the owner\'s direct question: are rivers and waterfalls possible here?\n' +
    'Ground yourself first: in the strips, where does water actually sit today (wetland ponds,\n' +
    'canyon basins)? Find in the rendered terrain the actual cliff/valley shapes that could host a\n' +
    'waterfall and a stream -- name the image and shot.\n' +
    'Facts you build on (verified this session): water is ALREADY non-conserved by design (rain\n' +
    'creates cells at a 24-column/frame cap; evaporation deletes unbanked). The edge is sealed in\n' +
    'three places (D5). No pressure (D2): everything is gravity-fed, so a source must sit above its\n' +
    'sink. A flowing channel keeps its chunks permanently awake and is all-surface, so the\n' +
    'buried-cell scan skip does not help; the wind revert (3.55 ms standing) is the cost precedent.\n' +
    'Waterfalls themselves are cheap: free fall is unthrottled and in-flight water is exempt from\n' +
    'evaporation scheduling. Whiskers (D3) are exactly river geometry.\n' +
    'Reports/worldgen-design.md sections 0 and 5a already design rivers as a slice of a 3D drainage\n' +
    'map and pose the REAL vs PLAUSIBLE off-plane-flux decision. Read them.\n' +
    'Deliver in assigned_answers:\n' +
    '(1) A concrete PROTOTYPE SPEC for plausible flux: where the source sits (edge column vs perched\n' +
    'spring on an existing brow), the emission budget (pattern-match rain\'s cap), where the sink is,\n' +
    'which existing scene/example harness renders it, and the settle/measure procedure.\n' +
    '(2) The pre-registered KILL CRITERION: what standing worst-frame delta vs the same-session\n' +
    'baseline kills the approach (not the tuning), measured how.\n' +
    '(3) The honest failure modes: pooling in the first basin (is a chain of pools linked by falls\n' +
    'acceptable or even better? judge from the terrain in the images), flooding, whisker sheets,\n' +
    'evaporation-in-transit steady state.\n' +
    '(4) What the REAL-flux upgrade needs later (coarse (x,z) map) and what of the prototype\n' +
    'survives that upgrade.\n' +
    '(5) Which of the three edge seals (D5) must open for an edge sink, and what happens to the\n' +
    'nothing_escapes_the_world test.\n' +
    'Files: Reports/worldgen-design.md, Reports/weather-handoff.md, src/sim/weather.rs,\n' +
    'src/sim/evaporation.rs, src/worldgen/passes.rs (ponds), src/sim/update.rs (liquid rules).',
    { label: 'hydrology', phase: 'Lens review', schema: FINDING_SCHEMA }),

  () => agent(BRIEF + '\n\n=== YOUR ASSIGNMENT: COST & STREAMING READINESS ===\n' +
    'You own pricing. Every idea the other lenses will propose must survive you.\n' +
    'From code (read, do not run): explain the actual cost model concretely -- chunk sleeping\n' +
    '(chunk.rs dirty rects, sweep_region, is_settled), the liquid reach widening\n' +
    '(Material::sweep_reach = 24 for liquids), the dirty-rect render skip (render.rs), field-tile\n' +
    'sleeping. Then produce in assigned_answers:\n' +
    '(1) The permanent-wakefulness math for a river: given CHUNK_SIZE=64 and a 2048-wide world (32\n' +
    'chunk columns), estimate what a flowing channel of N chunks costs per frame relative to the\n' +
    'settled baseline, and what fraction of the wind-revert precedent (3.55 ms standing) a modest\n' +
    'river band represents. State assumptions plainly; this is an estimate to be measured, not a\n' +
    'claim.\n' +
    '(2) What exactly the three GLOBAL worldgen passes block for streaming, and what localizing each\n' +
    'needs (the coarse (x,z) map is the stated fix -- check Reports/worldgen-design.md section 5).\n' +
    '(3) A cost verdict per likely roadmap item: water-cycle closure, edge-source river, secret\n' +
    'sealed chambers (generation-time only -- should be free at runtime; verify), generation-time\n' +
    'erosion passes (build-time cost only -- what is the budget? worldgen build is ~385 ms today),\n' +
    'region-keyed palettes (render-time), parallax background (render-time, camera moves), canopy\n' +
    'shadows.\n' +
    '(4) Which proposals would defeat the dirty-rect skip or keep chunks awake, D8-style.\n' +
    'Files: src/sim/chunk.rs, src/sim/world.rs (chunks_to_sweep, active_chunk_count),\n' +
    'src/render.rs, src/sim/parallel.rs (READ-ONLY, do not propose changing it), examples/ascii.rs.',
    { label: 'cost+streaming', phase: 'Lens review', schema: FINDING_SCHEMA }),

  () => agent(BRIEF + '\n\n=== YOUR ASSIGNMENT: UNDERGROUND & SECRET CAVES ===\n' +
    'You own the owner\'s cave directive: caves are wanted, but NOT one massive continuous network.\n' +
    'Rare, hidden, sealed chambers found by mining down, awesome and beautiful, "like you found a\n' +
    'cool secret." Ground yourself in canyon-mined.png (shafts cut into a settled world) and the\n' +
    'strips: what does the underground look like today? (Expect: uniform stone with strata shading,\n' +
    'sand/gravel pockets, bedrock. wiki calls it "a quarry rather than a destination.")\n' +
    'Answer in assigned_answers:\n' +
    '(1) The chamber mechanism: the pockets pass already places sealed ellipse lenses inside stone\n' +
    'with a one-cell rind check. What does growing that machinery into rare LARGER sealed chambers\n' +
    'need -- shapes beyond ellipses (geode-like vugs, low wide grottos, vertical chimneys), interior\n' +
    'content (crystal/mineral linings as new .ron materials, standing water inside sealed chambers,\n' +
    'gravel floors), and what keeps a chamber ceiling within the structural model\'s tolerance so\n' +
    'discovery does not trigger collapse (bounded chamber size sidesteps the unsolved noise-ceiling\n' +
    'span problem -- verify that claim against stone.ron\'s span rules and say what the max safe\n' +
    'ceiling span is).\n' +
    '(2) The reward: what makes a discovered chamber read as a payoff in THIS renderer, given sky\n' +
    'light is drawn only where sky existed at genesis -- a breached chamber is dark at noon. Is\n' +
    'dark-then-reveal a feature (torch question deferred), or do secret caves force the first local\n' +
    'light source? If crystal materials could glow, what would that cost (the renderer computes\n' +
    'colour per cell; a self-luminous material that does not light its surroundings is cheap -- a\n' +
    'true light source is not)? Be honest about which options are demonstrable now.\n' +
    '(3) Rarity and discovery: how rare is "secret" (chambers per world), what depth band, and what\n' +
    'hints (strata deformation around a chamber, mineral veins leading to it) make finding one feel\n' +
    'earned rather than random? Veins are a natural pockets-pass extension -- assess.\n' +
    '(4) What is cheap-model-delegable here vs frontier work.\n' +
    'Files: src/worldgen/passes.rs (pockets, strata_shade), assets/materials/*.ron examples,\n' +
    'wiki/the-world.md, wiki/world-cycles.md (underground light), Reports/worldgen-design.md\n' +
    'section 5 (caves).',
    { label: 'underground+caves', phase: 'Lens review', schema: FINDING_SCHEMA }),

  () => agent(BRIEF + '\n\n=== YOUR ASSIGNMENT: THE PLAYER JOURNEY ===\n' +
    'Images only -- deliberately no code. You are the player\'s advocate.\n' +
    'Walk every strip left to right as a player who just spawned: the gnome walks, jumps, swims and\n' +
    'digs; walking one screen takes about a minute of play. For each strip: what would make you\n' +
    'stop and look? What would you screenshot? Where would you dig, and what do you expect to find?\n' +
    'Where does the image promise something the world cannot deliver, and where is it just dull?\n' +
    'Compare presets: which worlds would you rather explore, and what specifically earns that?\n' +
    'Compare seeds of one preset: does a new seed feel like a new place or a reshuffle?\n' +
    'Then the mood shots: does rain change how the world feels? Does night? Does the mined shot\n' +
    'make you want to dig, or does the hole look like a mistake?\n' +
    'In assigned_answers: (1) the three most memorable places across all strips (image + where),\n' +
    '(2) the three dullest stretches and what each is missing, (3) your ranked wish list as a\n' +
    'player, ignoring cost entirely -- the other lenses will price it.',
    { label: 'player-journey', phase: 'Lens review', schema: FINDING_SCHEMA, model: 'sonnet' }),
])

const good = lenses.filter(Boolean)
log(good.length + ' of 6 lenses returned; sending to the prosecutor')

phase('Prosecute')

const CRITIQUE_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['verdict_summary', 'dead_ends', 'unfalsifiable_claims', 'conflicts_between_lenses', 'cost_concerns', 'strongest_ideas', 'recommended_order'],
  properties: {
    verdict_summary: { type: 'string' },
    dead_ends: {
      type: 'array',
      description: 'Findings or opportunities that re-tread something already tried, reverted, or measured false. Cite the source.',
      items: {
        type: 'object',
        additionalProperties: false,
        required: ['lens', 'claim', 'why_dead', 'source'],
        properties: {
          lens: { type: 'string' },
          claim: { type: 'string' },
          why_dead: { type: 'string' },
          source: { type: 'string', description: 'Where in CLAUDE.md or Reports/ this is recorded.' },
        },
      },
    },
    unfalsifiable_claims: { type: 'array', items: { type: 'string' } },
    conflicts_between_lenses: { type: 'array', items: { type: 'string' } },
    cost_concerns: { type: 'array', items: { type: 'string' } },
    strongest_ideas: { type: 'array', items: { type: 'string' } },
    recommended_order: { type: 'array', items: { type: 'string' } },
  },
}

const critique = await agent(
  'You are the FINDINGS-LOG PROSECUTOR for a review of the generated world of a Rust falling-sand\n' +
  'sandbox. Six lens agents reviewed rendered images of generated worlds from a COMPRESSED brief.\n' +
  'You are the only agent reading the RAW record, and your job is to catch what compression lost.\n\n' +
  'READ THESE FIRST, IN FULL:\n' +
  '  ' + ROOT + '/CLAUDE.md (the method; every rule was paid for)\n' +
  '  ' + ROOT + '/Reports/open-bugs-handoff.md (open bugs, and rejected fixes WITH their numbers)\n' +
  '  ' + ROOT + '/Reports/worldgen-design.md (the M10 redesign: slice topology, rivers, caves, age)\n' +
  '  ' + ROOT + '/Reports/weather-handoff.md (water cycle state; the wind revert; "and then it stops" tests)\n' +
  '  ' + ROOT + '/Reports/design-philosophy.md (settles constants/hardcoding/scope arguments)\n' +
  '  ' + ROOT + '/Reports/next-session-handoff.md (the live dig-damage bug and worldgen churn warning)\n' +
  'Also skim Reports/liquid-simulation-research-r2.md section 6 (hydrostatic pressure sketch) and\n' +
  'Reports/prior-art-worldgen-slicing.md if a hydrology claim needs checking.\n' +
  'You may read any source file under ' + ROOT + ' to verify a claim.\n' +
  'DO NOT run cargo build/test/run. Do not write or edit any file.\n\n' +
  'YOUR JOB, in priority order:\n' +
  '1. DEAD ENDS: every place a lens re-proposes something tried-and-reverted, measured false, or\n' +
  '   forbidden by a decision (heightfield bodies, standing global costs, whisker movement-rule\n' +
  '   fixes, exactness-justified mechanisms, caps that gate, geometry-inferred support). Cite the\n' +
  '   recorded source for each.\n' +
  '2. UNFALSIFIABLE OR SCENE-MEASURING CLAIMS: demonstrability statements that cannot fail, or\n' +
  '   metrics that would read the same when nothing is wrong, or that measure the scene rather\n' +
  '   than the mechanism.\n' +
  '3. CONFLICTS between lenses on fact or design; say which is right or what measurement settles it.\n' +
  '4. COST: anything that defeats the dirty-rect skip, keeps chunks awake, adds per-cell per-frame\n' +
  '   work, or introduces an untunable counterweight knob.\n' +
  '5. STRONGEST IDEAS, fairly, including underdeveloped ones worth keeping.\n' +
  '6. RECOMMENDED ORDER for the roadmap, honoring the owner priorities (variability/biomes, rivers\n' +
  '   if affordable, secret caves, graphics quality) and the repo\'s own sequencing (water cycle ->\n' +
  '   coarse map -> streaming last).\n\n' +
  'Be specific and adversarial. Do not summarise the lens reports back; assume the reader has them.\n\n' +
  '=== THE SIX LENS REPORTS ===\n' +
  good.map((p, i) => '\n\n########## LENS ' + (i + 1) + ' ##########\n' + JSON.stringify(p, null, 1)).join(''),
  { label: 'prosecutor', phase: 'Prosecute', schema: CRITIQUE_SCHEMA, effort: 'high' }
)

return { lenses: good, critique }
