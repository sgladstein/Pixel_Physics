# PF4 recheck (2026-10-28)

Report: `Reports/brain-training/evolvable-pheromones-2026-10-25.md` §9.13. Picture page:
https://claude.ai/artifact/NcXbVeF8P1ndpiojacYnj7. Scratch scripts; they take their run directory as an argument.

| File | What |
|---|---|
| `probe.py`, `jobs1.json` | replay jobs: the PF4 bests and the founder on worlds 5,000,200-215, normal and `PP_EMIT_FREE=1`, `events.csv` kept |
| `founder.raw.txt` | std_pool member 0 with the third emit row filled as PF4 built it (seed 41) |
| `an.py`, `fresh.py`, `pair.py`, `an.json` | per-run measures (pickups at the heap vs near the door, kJ, alive) and paired sign tests |
| `loop.py` | **the re-carry tracer**: each delivery traced to the pickup that began it, tagged by what last put food on that cell. The positive control for any food-home fix |
| `origin.py` | the same, split by where the pickup was (heap / near the door) |
| `heapdel.py` | deliveries whose trip began at the heap; the harness's `scratch/pheromones/heaphome.py` is this |
| `examloop.py`, `exam_clean1.txt`, `exam_std.txt` | the exam garden worlds 500001-500016 (exam.py's settings), events kept; loop counts for clean1_k3c23 and std_pool |
| `gif.py` | the door animation on the page |
