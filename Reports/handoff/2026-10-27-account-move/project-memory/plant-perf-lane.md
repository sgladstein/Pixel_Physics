---
name: plant-perf-lane
description: Plant performance thread (2026-10-21): benchmark scenes, where a plant bed's time goes, the mister-width finding, PR 771
metadata:
  type: project
  modified: 2026-10-21T23:19:18.694Z
---

Plant performance thread, one of four performance lanes started 2026-10-21 (lab world, creatures, plants, training). Branch `claude/project-thread-e0jd3i`.

- Ground rules (coordinator, from the owner's ask): speed changes must be bit-identical (training determinism); profile first; own your files; message the owning lane before editing theirs; PRs go to the merge desk. Haiku runners: 4 total across all four perf lanes, via the coordinator. This lane uses 0 so far.
- File ownership agreed: plant lane holds plant.rs, organism.rs, decay.rs. Lab world thread holds frame/parallel/update/liquid/field/chunk/structural/load.rs and the lab tick path; it approved the soil_partner inline in update.rs. Plant community thread owns rain.rs's tick and src/train_plant/.
- Benchmark: `examples/plantcost.rs` scene=community (512-wide roles4 bed, at=60000, ~770 organisms, ~1.1 ms/frame single-thread) and scene=rung (trainer rung-2 bed, ~2.1 s a bed). Three hashes: cells, field, orgs. Lab world thread runs scripts/perfbench.sh across all scenes.
- Where time goes: rung bed: soil-water pass 73%. Community bed (callgrind, 600 frames): soil water 29%, field 24%, plants (step_organisms + organism_tick) ~24%, CA sweep 17%. Plant code costs are spread thin, no single hotspot.
- Finding: the mister rains a fixed amount whatever the width, so a 48-wide bed gets ~10x per-column rain; 85% of soil changes are 1-2 units. Handed to the plant community thread; owner chose rain per column (PR #773, halves a training bed's cost).
- PR #771 (merged 2026-10-21): plantcost + soil_partner #[inline(always)] + MoistureView::is_inner, -10.5% instructions, wall 6/6 pairs ~10% faster on the rung bed. PR #776: update_soil_water_cell + inlined mark_moist_neighbours edge test, total -14.2% vs before #771; shipped box 20k pre-run ~9% faster; hashes equal. After #776 the soil pass belongs to the lab world thread.
- Tree bed (plantcost mix=roles, ~19k plant cells): CA sweep 35% (16% fire.rs inlined), soil water 25%, field 17%, plants 14%. Plant code is not the bottleneck at these stand sizes.
- Tried and dropped: Box<[Cell; CHUNK_AREA]> (lab world thread: no wall gain, pre-run 8% slower). Skipped: per-material table for the soil pass (instructions only).

- Soil-water review lane (Fable, branch claude/project-thread-mblj3z, write-up /mnt/project-files/soil-water-perf/soil-water-performance-ideas-2026-10-21.md): the cost is the one-unit drainage conveyor ((surplus*0.25) as u16). Bit-identical work tops out ~1.3x on the pass; the real lever is the drainage rule (changes results, with the owner as of 2026-10-21 23:27Z).
- Fast-drainage growth gate (2026-10-22, owner asked "will it affect plant growth?"): drain 0.25->1.0 + schedule-on-infiltration shows no growth effect beyond a 0.26 noise-control arm (rungs x4 species, community 4 seeds, dry spells). Top soil slightly WETTER (0.542->0.569), moisture writes ~1000->70/tick, rung beds 40-45% faster. Write-up /mnt/project-files/plant-perf/fast-drainage-growth-check-2026-10-22.md; scratch harness /tmp/claude-0/fast (SCRATCH_DRAIN_RATE, SCRATCH_SCHEDULE_INFILTRATION). Reported to owner; implementation belongs to the soil-water review lane / lab world thread. Re-checked 01:10Z: root-mat site included, on main+#777 with dry station (48 layouts: noise), and the real commit 80295015 (branch claude/project-thread-pppbyd) vs 65a166ac7: pass, sent to lab world thread. Trap: dry station at 12 layouts has only ~9 established, so p50 jumps (tree 423->240 was noise).
- PR #784 (2026-10-22, with merge desk): MaterialRegistry/SpeciesRegistry by_name -> FxHashMap. Dormant seeds do 5 id_of lookups a tick (plant.rs bury_or_unearth, seed_pile_below, pip check); SipHash was 1.6% of the community bed on main. -0.9% instructions, hashes identical, wall in noise. Finding call sites: #[track_caller] counter on id_of in a scratch build (callgrind can't attribute inlined hashbrown lines).

**Why:** the next session on performance needs the scenes, the split and who owns what.
**How to apply:** measure with plantcost hashes before and after; rank changes by callgrind instruction counts, but confirm every gain with interleaved paired wall-time runs (6 pairs), because instruction cuts have shown no wall gain here. Related: [[plant-community-lane]].
