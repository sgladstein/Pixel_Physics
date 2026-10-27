# Evolvable pheromones: data, genomes and scripts (2026-10-25 to 10-27)

Copied into the repo on 2026-10-27 because the project moved accounts and the shared folder and scratchpads do not
carry over. The write-up is `Reports/brain-training/evolvable-pheromones-2026-10-25.md` (§7 first emergence test,
§9 follow-ups, §9.5 new-channel run, §9.6 add-on, §9.8 review, E1, E2 and the plan). Everything below was produced by
`scratch/pheromones/emerge.py` and the `inrun` example; the scripts carry absolute scratchpad paths from the session
that ran them and need those edited before reuse.

| Path | What it holds |
|---|---|
| `first-emergence/{free,deaf,drift}/` | §7: gens.csv and listen.csv of the first test (alarm on the air law) |
| `reevolve/{s1,s2,fill1}/` | §9: the deleted B emit row re-evolved from zero (s1, s2: failed) and from N(0, 0.05) (fill1: rebuilt) |
| `newchan/{free,deaf}_s{1,2}/` | §9.5: the 150-generation new-channel runs; each has gens.csv, listen.csv, its log and the g149 best genome |
| `addon/` | §9.6: the add-on test (channel opened on the deaf best), gens and listen per arm |
| `headtohead{1,2,3}-*.json` | §9.5: each arm's best replayed on 16 shared worlds under every control (arms = run spec, results = per world) |
| `census_*.txt` | §9.5: emit census of the best genomes by ant state (empty/laden, home/away) |
| `e1-mutation-load.txt` | §9.8 E1 summary table (output of `scripts/e1_an.py`) |
| `e2-room-for-signal.txt` | §9.8 E2 summary per genome and bed (output of `scripts/e2_an.py`) |
| `genomes/` | E1 parents, the new-channel replicate-2 bests, the founder with its third emit row filled (layout 5, `.raw.txt`) |
| `scripts/` | e1_build/e1_an (mutant load), e2_probe/e2_an (lost searchers), h2h and probe (paired replays), slots.py (genome slot map), run wrappers |
| `ant-scent-lab.html` | the picture page shown to the owner (open in a browser) |
| `plan-2026-10-25.md`, `decode-2026-10-25.md`, `adversarial-review-2026-10-25.md` | the first plan, the decode of today's scents, the first adversarial review (all summarised in the report) |
| `*.png` | the molecules sheet and a B-plane map |
