# Design: `noprune=` -- named slots selection may tune but never prune

*Built as `noprune=` (renamed from `noprune=`: `keep=` already names kept arenas). An independent review (2026-10-21) found one MAJOR gap -- a kept scorer input still dies when its unit's output weight is pruned -- and two guards that could not fail; all fixed as listed at the end.*

**Why.** The teacher lane copies a lesson into a brain by gradient descent and revives links that selection had pruned
to 0 (slot 1498, Walk's job reading its own best step; and `DEPTH` into live scorer units: 654, 699, 789, 834, 924,
1059, 1149, 1239). A selection run from that copy (owner, 2026-10-21: copy-then-select, tested against selection from
s171) would prune them again: `train::mutate` zeroes a live slot with probability `prune` whenever the slot mutates, and
the wire cost charges every live link. Offline (2026-10-21, 50 children of s171 per arm), pruning was the main wrecker of
children; a revived link is exactly what it would take away.

**What.** A setting `noprune=<slot,slot,...>` (or `noprune=@file`, one slot per line) on `examples/train.rs`, carried
as `Settings::no_prune: Vec<usize>`.

- `mutate` gains one argument, `keep: &[bool]` (indexed by slot, empty = none): for a kept slot the prune branch is
  skipped, so the slot still steps by `sigma` when it mutates (selection can tune it) but is never set to 0.
- **Not frozen** (the alternative, removing the slots from `trainable_for` as `protect=` does): rejected, because the
  copy's values were fitted offline and selection should be free to adjust them.
- **The wire cost still charges them** (0.001 a link, nine links: negligible, and exempting them would make `wired`
  depend on the list).
- **Refused at start** if any listed slot is out of range or reads 0 in the start genome (a wrong list, or a list for
  another layout, fails loudly instead of protecting nothing).
- **Crossover** cannot reintroduce a 0: every genome descends from the start, and with the slot never pruned no parent
  carries a 0 there (a step landing exactly on 0.0 is measure-zero, and the clamp is +-40).
- **Resume:** ` noprune=<count>:<hash of the sorted list>` goes into the arena string when the list is non-empty, so
  a state saved with another list (or none) never resumes under this one.
- Default empty: every run so far is bit-identical (same RNG draws: the prune branch's `rng.unit()` is still drawn for
  a kept slot and its result ignored, so draws do not shift).

**Guards.**
1. At `prune` 1 and `pm` 1, a listed live slot is never 0 in 1,000 children, while unlisted live slots are pruned
   (positive control: the same run without the list zeroes the listed slot).
2. An empty list gives children identical to today's `mutate` on the same seed (no drift in RNG draws).
3. A listed slot reading 0 in the start genome panics with a message naming it.

**Not in scope.** Choosing the list (the teacher lane supplies it per copy); protecting `ROOF_DROP` links (not needed:
the trained brains already carry them, and they are not being revived).

## Review and what changed (2026-10-21)

- **MAJOR, fixed:** a listed scorer-unit input brings that unit's output weight (`mind::scorer_output`) into the list
  automatically; both must be live at start.
- **MAJOR, fixed:** the run guard is end to end (`no_prune_slots_survive_a_run_that_pays_for_pruning_them`: `train()` at
  pm 1, prune 1, cross 1, through a resume, on an arena that pays for pruning the slots; control: unlisted, both pruned).
- **MAJOR, fixed:** the empty-list guard compares against a fingerprint recorded on `main` before the change
  (`an_empty_no_prune_list_mutates_as_before`; red when the prune branch is altered).
- **MINOR, taken:** "draws unchanged" holds for an empty list only; the arena-string field is added in
  `RunState::arena` (library side), FNV over the sorted list; the zero check runs on the genome `train` receives; a
  step landing under 1e-6 is not taken for a kept slot; a listed slot outside the trainable set is reported as frozen.
