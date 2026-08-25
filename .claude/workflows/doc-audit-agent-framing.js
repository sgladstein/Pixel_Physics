export const meta = {
  name: 'doc-audit-agent-framing',
  description: 'Census tried-and-reverted knowledge; review always-loaded docs for the agent consumer',
  whenToUse: 'Grounding the documentation-overhaul plan in real data about what agents need',
  phases: [
    { title: 'Census', detail: 'extract do-not-retry knowledge from source comments and Reports' },
    { title: 'Review', detail: 'CLAUDE.md organization review + PLAN.md inbound-reference census' },
  ],
}

// Provenance: this is the harness that produced `Reports/dead-ends.md` (542
// entries after dedup, from 501 raw) and the thirteen recommendations now in
// `Reports/claude-md-recommendations.md`, run 2026-08-19 as workflow
// wf_df7aa74f-5c1. Recovered into the repo 2026-08-24; it had existed only in
// one session's local workflow directory.
//
// Before re-running it, update three things — all are 2026-08-19 snapshots and
// all have moved: the `groups` file lists (Reports/ has gained roughly a dozen
// documents since), the PLAN.md line map in `planRefsThunk` (the progress log
// was split out to PLAN-log.md in 59ceef5, so PLAN.md is ~3,300 lines, not
// 5,421), and CLAUDE.md's line count in `claudeMdThunk`. A census run against a
// stale file list silently reports on the files it was given and says nothing
// about the ones it was not.
//
// The census prompt's marker-language list and its include/exclude boundary are
// the load-bearing part and should not be loosened: the same distinction —
// tried-and-rejected versus ordinary design rationale versus an open bug — is
// what keeps dead-ends.md and open-bugs-handoff.md from claiming each other's
// knowledge.

const ROOT = 'C:/Users/Scott/Code/Pixel Physics'

const CENSUS_SCHEMA = {
  type: 'object', required: ['entries'], additionalProperties: false,
  properties: {
    entries: {
      type: 'array',
      items: {
        type: 'object', required: ['where', 'area', 'dead_end', 'condition'], additionalProperties: false,
        properties: {
          where: { type: 'string', description: 'file plus nearest enclosing symbol or heading, e.g. "src/sim/update.rs fn flow_sideways" or "Reports/open-bugs-handoff.md par.6" — never a bare line number' },
          area: { type: 'string', description: 'subsystem: liquids, powders, structural, destruction, plants, creatures, field, rendering, worldgen, weather, scheduler, parallelism, other' },
          dead_end: { type: 'string', description: 'one or two sentences: what was tried, what happened, why it must not be retried' },
          condition: { type: 'string', description: 'the condition under which the rejection held — what change would justify re-testing it. "unknown" if the record does not say.' },
        },
      },
    },
  },
}

const RECS_SCHEMA = {
  type: 'object', required: ['recommendations'], additionalProperties: false,
  properties: {
    recommendations: {
      type: 'array',
      items: {
        type: 'object', required: ['issue', 'change', 'benefit'], additionalProperties: false,
        properties: {
          issue: { type: 'string', description: 'the problem, with a short quote from the file' },
          change: { type: 'string', description: 'the specific edit, move, or addition' },
          benefit: { type: 'string', description: 'what it buys the every-session reader; token estimate where relevant' },
        },
      },
    },
  },
}

const REFS_SCHEMA = {
  type: 'object', required: ['by_target', 'log_evidence', 'split_recommendation'], additionalProperties: false,
  properties: {
    by_target: {
      type: 'array',
      items: {
        type: 'object', required: ['target', 'ref_count', 'examples'], additionalProperties: false,
        properties: {
          target: { type: 'string', description: 'what part of PLAN.md is pointed at: decisions table, invariants, a milestone spec, issues backlog, progress log, a handoff section, or whole-file/vague' },
          ref_count: { type: 'number' },
          examples: { type: 'array', items: { type: 'string' }, description: 'referencing file and what it cites' },
        },
      },
    },
    log_evidence: { type: 'string', description: 'whether any reference depends on progress-log content specifically; cite each one found' },
    split_recommendation: { type: 'string' },
  },
}

const censusPrompt = (files, extra) => `You are auditing documentation in the git repo at ${ROOT} (a Rust falling-sand physics engine developed entirely by Claude agents). The repo records hard-won "do not retry this" knowledge — approaches that were tried, measured, and reverted or rejected — scattered through source comments and design reports. A future index file will aggregate it so no session re-attempts a known dead end. Your job: extract every such entry from EXACTLY these files (and no others):

${files.map(f => '- ' + f).join('\n')}

Method: Grep each file for marker language (reverted, tried, dead end, do not, must not, instead of, abandoned, withdrew, went stale, was wrong, found wrong, retuned, removed because, backwards, turned out, gave up, rejected, superseded), then Read the surrounding context (roughly 40 lines either side) to understand each hit. For large files never read the whole file — navigate by grep. Include ONLY genuine tried-and-rejected knowledge: a mechanism, fix, tuning, constant, or model that was actually attempted or seriously evaluated and rejected for a recorded reason. EXCLUDE ordinary design rationale where no alternative was tried, TODO/not-yet-built notes, and open bugs. When a passage records the condition under which the rejection held (a measurement, an interacting bug since fixed, a world size, a specific constant), capture it — the repo's own convention is that a dead end must be re-tested when its condition changes. One entry per distinct dead end, even when the passage is long or the same dead end is mentioned twice in one file. Cite by symbol or heading, not line number (line numbers rot).${extra ? '\n\n' + extra : ''} Do not edit or write any file.`

phase('Census')
const groups = [
  { label: 'census:update-world-parallel', files: ['src/sim/update.rs', 'src/sim/world.rs', 'src/sim/parallel.rs', 'src/sim/chunk.rs', 'src/sim/surface.rs', 'src/sim/cell.rs'] },
  { label: 'census:plant-organism', files: ['src/sim/plant.rs', 'src/sim/organism.rs', 'src/sim/scheduler.rs', 'src/sim/decay.rs'] },
  { label: 'census:load-structural-rigid', files: ['src/sim/load.rs', 'src/sim/structural.rs', 'src/sim/rigid.rs'] },
  { label: 'census:misc-src', files: ['src/sim/weather.rs', 'src/sim/creature.rs', 'src/sim/brain.rs', 'src/sim/pheromone.rs', 'src/sim/liquid.rs', 'src/sim/evaporation.rs', 'src/sim/evaporation_tests.rs', 'src/render.rs', 'src/sky.rs', 'src/sim/player.rs', 'src/worldgen/passes.rs', 'src/worldgen/column.rs', 'src/sim/field.rs', 'src/sim/fire.rs', 'src/sim/explosion.rs', 'src/sim/particle.rs'] },
  { label: 'census:handoff-reports', files: ['Reports/next-session-handoff.md', 'Reports/open-bugs-handoff.md', 'Reports/weather-handoff.md', 'Reports/load-model-handoff.md', 'Reports/load-model-fit-review.md', 'Reports/load-concentration-review-response.md'] },
  { label: 'census:destruction-reports', files: ['Reports/building-rethink.md', 'Reports/destruction-plan.md', 'Reports/prior-art-destruction.md', 'Reports/fracture-mechanics-design.md', 'Reports/underground-definition.md', 'Reports/explosion-mechanics-diagnosis.md'] },
  { label: 'census:tree-reports', files: ['Reports/tree-architecture-implementation-plan.md', 'Reports/tree-architecture-research.md', 'Reports/tree-architecture-variety-review.md', 'Reports/tree-architecture-variety-review-verification.md', 'Reports/tree-diagnosis-review.md', 'Reports/tree-extension-audit.md', 'Reports/tree-extension-biology.md', 'Reports/tree-procedural-prior-art.md', 'Reports/tree-rewrite-design.md', 'Reports/tree-shape-problem-statement.md', 'Reports/plant-species-authoring.md', 'Reports/plant-substrate-v2-design.md'] },
  { label: 'census:direction-reports', files: ['Reports/creature-direction.md', 'Reports/emergent-world-architecture.md', 'Reports/worldgen-design.md', 'Reports/design-philosophy.md', 'Reports/liquid-heightfield-design.md', 'Reports/liquid-simulation-research.md', 'Reports/liquid-simulation-research-r2.md', 'Reports/granular-mechanics-research.md', 'Reports/coupling-research.md', 'Reports/ecological-lod-design.md', 'Reports/organism-substrate-design.md'] },
  { label: 'census:readme', files: ['README.md'], extra: 'README.md is a 1,595-line status document that recounts many tried-and-removed approaches inline (e.g. a field coupling in diffuse_heat that was "tried, measured, and removed"; three attempts at wall boundary conditions). Extract those.' },
  { label: 'census:plan-md', files: ['PLAN.md'], extra: 'PLAN.md is 5,421 lines; its Progress log section (roughly lines 1107-3233) records many sessions. Include only entries recording a genuinely rejected approach with a reason and ideally a measurement — not routine progress, not plans, not open questions.' },
]

const censusThunks = groups.map(g => () => agent(censusPrompt(g.files, g.extra), { label: g.label, phase: 'Census', schema: CENSUS_SCHEMA }))

const claudeMdThunk = () => agent(`Read the file ${ROOT}/CLAUDE.md in full (513 lines, ~5,100 words). Context: the project it governs is developed entirely by Claude agents; CLAUDE.md is auto-loaded into EVERY session's context (~7k tokens, every session, forever), making it the highest-leverage bytes in the repo. It is owner-curated method knowledge in a distinctive voice; you are reviewing it as infrastructure for the agent that reads it, not rewriting it, and the voice must survive any change you propose.

Assess, quoting a short passage for every claim:
1. ORGANIZATION: when an agent mid-task needs the rule relevant to what it is doing (touching a liquid metric, adding per-cell work to the sweep, writing a guard test, running a parameter sweep), can it find that rule? Are related rules adjacent? Would a short topic map near the top pay for its tokens?
2. REDUNDANCY: passages that restate each other or restate what a referenced report already holds, where a pointer would do. Estimate tokens saved per instance, and flag any proposed cut that would lose always-loaded value (a rule an agent needs BEFORE it knows which files it is touching must stay inline).
3. ALWAYS-LOADED TEST: which sections earn a place in every session's context versus content only relevant when touching one subsystem. For anything you would move out, name the destination file and what one-line pointer stays behind.
4. GAPS for the agent consumer: the repo is about to gain Reports/README.md (an index of its 40+ design reports with per-report status) and Reports/dead-ends.md (an index of tried-and-reverted approaches with the condition under which each rejection held). What routing should CLAUDE.md carry to these, and is anything else missing that an every-session file should carry for a cold agent?

Return a structured list of recommendations. Do not edit or write any file.`, { label: 'review:claude-md', phase: 'Review', schema: RECS_SCHEMA, effort: 'high' })

const planRefsThunk = () => agent(`In the git repo at ${ROOT}: PLAN.md is 5,421 lines (~66k tokens). Its h2 sections sit at these lines: Context 56, Stack 92, Non-negotiable architecture invariants 110, Milestones 124, Second phase 251, Third phase 724, Execution order 1021, Overall verification 1086, Progress log 1107 (running to ~3233 — 2,127 lines), M19 3234, Scientific accuracy 3361, Code review findings 3376, and five session-handoff sections from 3504 to the end.

Question: which parts of PLAN.md are actually READ by the rest of the repo, and which are write-only archive? Method: grep for PLAN.md (also "the plan's", "plan's own", "PLAN.md#", "see the plan") across CLAUDE.md, README.md, Reports/*.md, wiki/*.md, docs/*.md, all .rs files under src/, examples/, tests/, and scripts/*.sh. For each reference, open enough context to classify what in PLAN.md it points at: the decisions table / determinism reversal, an invariant, a specific milestone spec, the issues backlog, the progress log specifically, a handoff section, or vague/whole-file. Count per target with examples.

Then answer the decision this feeds: a proposal exists to split the 2,127-line progress log into its own file (PLAN.md would shrink ~40% and become cheap to read whole). Report every reference whose target lives ONLY in the progress log — those are the links a split must repoint — and give a recommendation with the evidence. Do not edit or write any file.`, { label: 'census:plan-refs', phase: 'Review', schema: REFS_SCHEMA })

const all = await parallel([...censusThunks, claudeMdThunk, planRefsThunk])

const census = all.slice(0, groups.length).filter(Boolean).flatMap(r => r.entries)
const claude_review = all[groups.length]
const plan_refs = all[groups.length + 1]
const failed = groups.filter((g, i) => !all[i]).map(g => g.label)
if (failed.length) log(`WARNING: census groups returned nothing: ${failed.join(', ')}`)
log(`census: ${census.length} dead-end entries; CLAUDE.md recs: ${claude_review ? claude_review.recommendations.length : 0}; PLAN.md ref targets: ${plan_refs ? plan_refs.by_target.length : 0}`)
return { census_count: census.length, census, claude_review, plan_refs }