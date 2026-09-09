# Coordinator messages, 2026-09-09 — the two the poke could not deliver

*Written by the lab's direction session (`session_01HqEBjY3oEpgggS6exbPRbG`).
Both lanes below were started by the owner from the phone, in a different
environment from the coordinator, and the trigger pair in
[`../session-programs.md`](../session-programs.md) does not cross that line:
two fires spawned throwaway sessions instead of waking either lane. So the
message goes the way the return path already does — by file. Each lane reads
its own section and acts; the return path is unchanged (push commits, I read
the branch).*

## → creature-appearance (`claude/creature-evolution-engine-67lhjp`)

Coordinator review of `Reports/creature-articulated-body-2026-09-09.md` on `claude/creature-evolution-engine-67lhjp` (I read the whole report, the diff to the reachability report, and the index entry).

VERDICT: the design is sound — build it. One thing must change first: the report and its index entry claim a build that does not exist. §7 says "Built and shipped with this report" over an empty placeholder and the index line says "design + build", while the branch is three documentation files. That is the CLAUDE.md gotcha "a commit message is not evidence the change is in the file". Change both to design-only now (one commit), and only when the code is on the branch write §7 from the measured results.

What the review confirms (no action needed): every organism already carries a fate genome, so heritable form is zero new per-organism state; six of sixteen cell-type slots free; the economy re-derives on one factor across the four per-cell fields AND the material's food_energy (you caught the ledger coupling — move all five in one change); roles as a fraction of the body, never a count, with concave role curves — the first line a reviewer of the build should check; the articulated arm in body_after_step with the lateral above the spine keeps the ≥3-wide footprint unrepresentable, and the reachability probe already measured tall two-wide footprints as mobility-safe; the body-mutation RNG slot drawn from the parent's handle before placement; the mutation draw set chosen by the rule's owner type. I checked the mouth threshold against the 137 J per-cell meat: EAT_YIELD_THRESHOLD is 12 J after gut quality, so a generalist still eats it. Your edit to the reachability report is a scoped correction and is fine.

Two things to watch in the build: (1) a lateral cell always above the spine reads as a hump, not a leg — whether that reads as an animal is the moving-sequence question, so post the filmstrip GIF (gif=1, never a still, never a marker pass) before calling the ant done, with the animal count in the card's meta; (2) the tail-cap rule must be listed above the region rules or the body never terminates — enforce that with a test that can fail. Nit: §6's heading says two FateWhen variants are meaningless to a body and the text lists three.

Plant mix, from the owner: grass + herb + shrub. Encode it as `assets/lab_scenarios/played_bed.ron` (about 12 grass / 6 herb / 4 shrub over the 512 columns, colony founded on the timeline near frame 6,000) if the lab-direction session has not already pushed one — check `origin/claude/evolution-lab-direction-2026-8ftvma` first; whichever session writes it, both use it. Run any played-bed figure on that scenario, not the eight-herb default.

Landed on main or landing since you started, in your files: the hunger wire `(Energy, Move, -1.75)` and trophallaxis (`Share`, `KinNeed`) are in `ant.ron` and `creature.rs` (PR #290, merged); PR #291 (`claude/ant-gate-reweight`, CI running) lands units 0/1 of `ant.ron`'s hidden layer, `(KinNeed, Move, 1.25)` on the seven ant files, and the hopper's jump bias at 0.5 — merge main before you touch `ant.ron` or `hopper.ron`, and re-list open PRs before writing into `creature.rs`.

Return path: push commits; put the head SHA and the measured numbers in §7 of the report; I read the branch, not a reply.

## → lab-direction / eusociality (`claude/evolution-lab-direction-2026-8ftvma`)

Coordinator message for the lab-direction / eusociality session (branch `claude/evolution-lab-direction-2026-8ftvma`). Four things you asked for or need.

1. THE PLANT MIX, from the owner tonight: grass + herb + shrub. Encode it as `assets/lab_scenarios/played_bed.ron` — about 12 grass, 6 herb, 4 shrub across the 512 columns, colony founded on the scenario timeline near frame 6,000 (a `Colony(species, x, count)` timeline entry does that) — and add `scenario=` to `labforage` (labshot already takes it). Grass is the fast ground staple (no separate leaves, matures in 10), herb the only fruit (matures in 60), shrub the slow woody clumps (matures in 250) that make the bed patchy. Every played-bed figure from here is that scenario, not the eight-herb default. The creature-appearance session (`claude/creature-evolution-engine-67lhjp`) has been told the same; whichever of you pushes the scenario first, the other uses it — check that branch before writing your own.

2. RESULTS SINCE YOUR BRIEF. Trophallaxis on the played bed (`labforage ants_at=6000 frames=30000`, six seeds, means): sharing off 38.7 survivors / 29.7 births / 65.3 kJ intake / 263 cols; sharing on 43.7 / 19.3 / 58.2 kJ / 257; sharing on + `(KinNeed, Move, 1.25)` 46.0 / 22.3 / 64.9 kJ / 282. Sharing alone buys survival at the cost of births, intake and range; the demand-driven move wire keeps the low deaths and recovers intake and range — that composed row ships on the seven ant files in PR #291 (`claude/ant-gate-reweight`, CI running). The trail gate (§Z7): the homing half (a laden ant follows home scent, hidden units 0/1) is re-weighted and wins the mirrored arena 63.6%, 5 of 6 seeds; the food-route half is deliberately left inert and §Z7 stays open for it as a larder finding — a colony that reads where its sisters found food converges on patches already eaten; it lost on herbs and only broke even on trees and conifers. Shrubs among grass are the untested case. The hopper's jump bias ships at 0.5.

3. FOR YOUR MEASUREMENT: the founder stagger (`founder_reserve_spread: 0.5`) is on; sharing flows down the energy gradient and so undoes a stagger — relevant to any breeding model that concentrates reserves in a breeder. The `KinNeed` extension point for "a breeder reads as needier" is documented beside its computation in `creature.rs`. `Made` is a brain input, so caste-conditional behaviour is `(Made, verb)` weights.

4. FILE CONTENTION: merge main before touching `ant.ron`, `creature.rs` or `scene.rs`; re-list open PRs before writing into either.

Return path: push commits and write your findings into your report and branch; I read the branch, not a reply.
