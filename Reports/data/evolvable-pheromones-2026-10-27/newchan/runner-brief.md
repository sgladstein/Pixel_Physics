# Runner brief: new-channel emergence run, replicate 2 (evolvable-pheromones lane)

You run ONE arm (free or deaf, given in your start message) of `scratch/pheromones/emerge.py`. Everything you need is on `main` (PR #919).

1. `git fetch origin main && git checkout origin/main` (detached is fine; you push nothing).
2. `set -o pipefail; cargo build --release --example inrun` and check that it finished.
3. Check the disk first with `df -h /`; you need at least 5 GB free.
4. Run it as a tracked background command (Bash `run_in_background: true`, timeout 7200000). **Do not use nohup**: a nohup'd process dies when the container sleeps. The script resumes from the last finished generation, so relaunch the same command every time it stops, until `gen 149` prints.

```
PIXEL_PHYSICS_ALARM_GROUND=0.03:0.05 python3 scratch/pheromones/emerge.py /tmp/nc/ARM_s2 ARM 16 150 100000 assets/minds/std_pool.txt P=4 SEED=2 EVALS=2 SCORE=stored ZERO=2410-2444 FILL=0.05 CHAN=alarm ALARM=train >> /tmp/nc/ARM_s2.log 2>&1
```
(replace ARM with free or deaf; `mkdir -p /tmp/nc` first)

5. Expect about 1.3 min per generation, so ~3.5 h in all. Every ~30 generations, copy `gens.csv`, `listen.csv` and the log to `/mnt/project-files/evolvable-pheromones/newchan/ARM_s2/`. Copy them again at the end, together with the last generation's `c00.raw.txt` and `c01.raw.txt` from `g149`.
6. If a run log shows `No space left on device`: free some space, delete the spoiled generation directories and their rows in `gens.csv`, then resume.
7. When finished, post in your own thread: "done, files in newchan/ARM_s2". Don't analyse the results; the lane does that.
