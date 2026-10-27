# Lone predator creature (thread cmsg_01Mei3dwkcdzPxa5Jmuqi2bBPYNpvazZ94GT2uVUdjhkJz) — hand-off, 2026-10-27

**What it did:** trained a spider (`assets/species/spider.ron`) with a mind by the colony's in-run selection, put it on the Teacher lane's bed against the colony, co-evolved both, added sight, handling time (a meal takes the gut's time; a full spider does not bite), evolved breeding genes, a lab population test (`examples/spiderlab.rs`), and the spider in the lab game (COLONY chip, `assets/minds/spider_pool.txt` = coex-r10). PRs #844, #852, #871, #872, #879, #891 merged. Full record: `Reports/lone-predator-2026-10-24.md` (stages one to five, plus "Where it stands").

**Unmerged:** nothing. PR #950 (pools, judged rows, GIFs into Reports/lone-predator/, "Where it stands" section) merged 2026-10-27 as 9216aa2.

**Open to-dos** (also in the report's "Where it stands"):
1. Re-measure spider pools at the new brain price (PIXEL_PHYSICS_LINK_PRICE 0.25, landed after batch ce4) and after #887 (no reach-budding from living prey).
2. More seeds before ranking pools: coexistence was 2 of 4 seeds.
3. Carry evolved breeding genes (NN.body.txt) into the lab pool file; today a lab spider breeds at the species bar.
4. Explain the evo-r10 pool's two lab runs where the hand-built colony died after only 2-4 spider kills.
5. spiderhold never fired at K=30 (median kills 7-16); try lower K.
6. No colony defence learning shown in any batch; Teacher lane wanted a 64-colony defence run once spiders persist.
7. Other predators (owner liked them; agreed to hold until the spider coexists): ambush hunter, pack hunter, jumping spider (hopper leap), wasp (real sustained flight; owner: the flitter's flight is "a floaty jump", low priority). Nest raider has its own thread/report.

**Pending owner decisions:** none.

**Files outside the repo:** all worth keeping are in PR #950. /mnt/project-files/predator/nest-raider/ (29 MB) belongs to the nest raider thread. The scratchpad holds only batch run dirs and binaries (ce/ce2/ce4, judge dirs); the kept results are in the PR.

**Jobs running:** none (all local, all finished). No VM jobs.

**Working practices learned here (not all in the repo):**
- The owner cannot see images attached to chat messages: put pictures on a page and link it.
- Judge-run directories are ~70 MB each: delete them after writing rows.csv, or the disk fills.
- Never `pkill -f`/`pgrep -f` (it killed this lane's own shell once).
- A force-push was refused by the classifier: use merge commits.
