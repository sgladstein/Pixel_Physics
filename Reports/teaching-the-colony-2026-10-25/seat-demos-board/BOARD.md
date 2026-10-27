# Seat demos: the shared board

Started 2026-10-25 at the owner's request (Scotttt, 07:19Z and 07:21Z):
> "for each one of these start up new threads which will adversarily review their report with a feasibility
> analysis. If the idea is deemed feasible then they write up a detailed implementation that will get us to a
> playable demo. That implementation plan should be independently reviewed. Then revised and then implemented
> to a playable demo." ... "Six seats sounds good. Make sure everyone collaborates as they are all interested
> and some could be combined."

Six threads, one per seat: **queen, keeper, guest, colony-mind, enemy, scout**.

## The designs (read your own seat in full, skim the others)
In `designs/` (also in the repo under `Reports/teaching-the-colony-2026-10-25/` once PR #898 lands):
- the teaching plan: `teaching-plan.md` (main recommendation: the queen as main seat; §2 engine facts, §7 memory layer)
- queen: `queen.md` · keeper: `keeper.md` (research notes; the plan's §3-§6 carry their design)
- guest: `seat-guest.md` · colony mind: `seat-colony-mind.md` · enemy: `seat-enemy.md` · scout: `seat-scout.md`
- each seat doc's review: `seat-*-critique.md`; the earlier review of the plan: `critique.md`
- comparison, combinations, shared engine work: `seats-overview.md`
- owner's Claude Doc of the four new seats: https://claude.ai/artifact/SQS7Efw6B47ekdLChmk4KC
- owner's Claude Doc of the teaching plan: https://claude.ai/artifact/AQmZ5E5towdrhgfXhtTJyt

## Each thread's steps
1. **Adversarial review and feasibility** of your seat's design, by an agent that did not write it. Verdict: feasible as written / feasible with changes / not feasible (and why). Use the seat's critique as a starting point, not the answer.
2. If feasible: a **detailed implementation plan to a playable demo**: something the owner can launch and play (a binary or a mode flag), with every step's proof.
3. **Independent review of the plan** (a fresh agent).
4. **Revise** the plan.
5. **Implement to a playable demo.** Put a GIF/screenshots on an Artifact page (the owner cannot see images in chat; the review queue is retired). PR to the merge desk.
If not feasible: write up why, land it, post it here, and offer your hands to a seat whose demo shares your parts.

## Collaboration rules (the owner asked for this explicitly)
- **Your status file:** `status/<seat>.md`. Only you write it. Update it at each step: current step, verdict, the plan's link, which shared pieces you need, which seats you could combine with. Read the others' before planning and before building.
- **Shared engine pieces are built once.** Before building one, look in `shared/`. If a file for it exists, the owner named in it builds it; tell them what you need. If not, create `shared/<piece>.md` naming your seat as owner, the interface you will expose, and the branch. Never edit another seat's shared file; message its thread instead (a thread may message another when its work depends on it).
  Known shared pieces (from `seats-overview.md`): a steerable non-ant body (guest, enemy); the memory layer of plan §7 (scout, keeper, mind); sending never-foraged ants without starving them (mind, scout; RECRUIT is off for a 442-vs-298 harm); reactions that read the scent model through body rules (guest, keeper, enemy); a hook for choosing who lays (queen, mind); and a common demo launcher, so all six demos start the same way.
- **Likely combinations; talk early:** queen -> scout -> colony mind (the life-cycle arc); queen + enemy ("switch sides" seasons); keeper + guest (the keeper as symbiont). If two demos turn out to be one demo, say so in both status files and merge efforts.
- **Do not change the shipped lab's behaviour by default** for a demo: use a separate binary or a mode flag, off in the lab unless measured neutral.

## Project rules that apply
Merge desk thread merges everything and watches CI; lanes open a PR and hand off. Land findings in `Reports/` before resolving. Pictures go on Artifact pages. Subagents: Opus by default, model passed explicitly. One physics for every world; one brain design, no hand-made castes; ants dig and forage all game; every verb needs a visible consequence. Owner rejected "walk to food and the colony follows you" as too simple.
