# Nest raider: handoff from the lone predator lane (2026-10-25)

**Ask (owner Scotttt, 2026-10-24/25):** new predator types with different behaviours ("I like your ideas for new predators"), the nest raider first, in its own thread while the spider lane works on coexistence. Owner rules that apply: one physics for every world; features default on unless a measured harm; land reports in Reports/ via the Merge desk before resolving; lanes open PRs and hand them to the Merge desk (thread cmsg_01Mei3dwkcdzPxa5Jmuqi2bBBjZgzija4YCh6ciFtqnPbN), never watch CI; no review queue (show pictures in chat).

**The idea.** A predator that goes *into* the nest rather than picking off foragers on the surface. Its body must fit the ants' 2-wide shaft (the spider's 5-cell rigid body, 3 wide x 2 tall, does not). It is the predator that makes nest defence (guards at the door, alarm in the tunnels) worth learning.

**Reuse, all on main (PR #844, #852, #871, #872, #879):**
- Species file pattern: `assets/species/spider.ron` (comments explain every number: tick_interval, start_energy, bite = dig_force vs ant flesh 0.25, crop, carnivore gut traits[0]=1.0, its own scent, sight_range). New `.ron` + matching `assets/materials/<name>.ron` (food_energy = body_energy).
- **Set `handling: true`** (`CreatureDef::handling`): without it a predator inside a nest eats at bite speed and wipes the colony (2 spiders -> 23 in 1,500 frames).
- Minds: `src/train/predator.rs` (`VERBS`, `spider_mind`, `place_spider`, `BREEDING` genes in `NN.body.txt`). A raider probably also wants Dig? (decide; spider has none).
- Training: `examples/predtrain.rs` (lone in-run selection beside a fixed colony; `founder=hunter|blank`, `pool=`); co-evolution on the Teacher lane's bed: `examples/inrun.rs spiders=POOL:N spider_mut= spider_out= spider_score=total`, `scratch/inrun/colsel.py spiderevo= spiderhold= spiderlab=`. These are spider-named; generalise by species rather than copy.
- Sight: `PIXEL_PHYSICS_MIND_SIGHT` reads the nearest prey on the home senses for minds with `sight_range > 0` (`options::sensed_home`). Underground, sight is blocked by soil: a raider may need scent instead (ants' scent planes).
- Lab: `examples/spiderlab.rs` (population test in a lab box, nothing refilled; `pool=lab`, `snap=`), and the lab game hookup in `src/lab/mod.rs` (`give_spider_minds`, `assets/minds/spider_pool.txt`, `NOT_STOCKABLE_IN_LAB` in `src/lab/ui.rs`).
- Judge: `scratch/inrun/predjudge.py`.

**Read first:** `Reports/lone-predator-2026-10-24.md` (stages one to four: what worked, what evolved backwards, the elitism fix, handling time), `Reports/how-the-ant-works.md` §12 (switches), memory file `predator-lane`.

**Lessons that cost time:** single-life selection with no elitism made spiders worse (keep parents, judge repeatedly); a per-frame-alive score bred spiders that avoid prey (use total food); build a lab population test before training long; run judge dirs out of the scratchpad (~70 MB a run, the disk fills); never `pkill -f`.

**Open question for the raider lane:** with handling time the trained ants kill spiders off; the spider lane's coexistence batch (spiderhold + spiderlab selection) may give the answer every predator needs. Check `predator-lane` memory for its result.
