# External cloud compute for brain training: cost and benefit (2026-10-18, research only, nothing started or bought)

## What a job costs in machine time today
- Fetch+trail suite, pop 100, 8 layouts, 4 threads: 115 s/gen (fetch3 mind-s1, gen 11), 148 s/gen avg on fetch2 (runner F log), so ~130 s/gen. 100 gens = ~3.6 h = ~14 vCPU-hours.
- Scarce+trail (forage-big) was slower: 29 gens in 2 h 10 min = ~270 s/gen.
- Map skip (1.4-1.65x, brain lane, not yet merged): 100 gens ~2.4 h. Early stop on flat runs (not before gen 40): assume a typical run ends at about half, ~1.2 h. The early-stop saving is a guess.
- Scoring splits pop x layouts (800 tasks/gen) over `threads` workers (src/train/mod.rs:722-760), and the code guards that the score does not depend on thread count. So one job should speed up on a bigger box. NOT measured: needs a 20-gen test at threads=4 vs 16+.

## What the Haiku runner pool costs now (get_session usage.cost_usd, token-equivalent; may be plan-covered, not a separate bill)
- Runner A $4.55 (13 h), B $4.58 (12.6 h), C $2.59 (14.5 h), F $0.28 (0.9 h). D, E not checked. About $0.2-0.35 per runner-hour; 4 runners ~ $1.2/h.
- Driver: a 300k-token context re-read every 9-minute poll, billed at the >100k rate ($0.05/M cache read). A fresh runner with a small context would cost a fraction.
- The machine itself is included. Per vCPU-hour that is ~$0.075 (4 vCPU per runner), a job today ~$1.1 in tokens.
- For scale: the Opus brain lane has cost $234 in the same period. Compute is not where the money goes.

## External options (prices checked 2026-10-17/18)
| Option | Price | 4-vCPU job today / with skip / skip+early stop |
|---|---|---|
| GitHub Actions standard Linux runner, public repo (repo is public) | $0; 20 concurrent jobs on Free, 6 h per job | $0 |
| AWS c7a.xlarge (4 vCPU, no hyperthreads) spot, us-east-1 | $0.077/h (range 0.075-0.100, interruption rate >20%) | $0.28 / $0.19 / $0.09 |
| same, on-demand | $0.205/h | $0.74 / $0.49 / $0.25 |
| Hetzner CCX23 (4 dedicated vCPU) | EUR 0.138/h (~$0.16); prices rose 2.2x on 2026-06-15 | $0.58 / $0.38 / $0.19 |
| Hetzner CX33 (4 shared vCPU) | EUR 0.0136/h (~$0.016) | $0.06 / $0.04 / $0.02 (noisy neighbours) |
Not priced: Google, Azure, Oracle free tier, prepaid GPU/CPU marketplaces. ARM (Hetzner CAX, AWS Graviton) skipped: floats may differ from the x86 baseline.

## What $25 buys
- AWS spot: ~325 four-vCPU box-hours = 90 / 135 / 270 jobs (today / skip / skip+stop). On-demand: ~120 box-hours = 34 / 51 / 100 jobs.
- One big box: 32-vCPU is ~8x the xlarge, so ~$0.62/h spot (derived, not looked up), ~25-40 h of it for $25. If scaling holds near 80%, a 100-gen job takes ~30-35 min instead of 3.6 h for about the same $/job.

## What more machines would do
- Parallel seeds: the default bar is 12 seeds. Today 5 jobs run at once (4 runners + brain lane), so 12 seeds is 3 rounds x 3.6 h = ~11 h. With 12 boxes it is one round.
- Faster single job (needs the scaling test): the lane has restarted fetch runs 3 times in 3 hours; each restart is a lost 3.5 h run. Shorter loops are the real win.
- Money saved is small. The win is hours.

## Catches
- Sandbox must reach the machine: environment network policy, an AWS/Hetzner token as an environment secret, and results copied back (the /mnt/project-files mount is not visible from outside). Not tested. Read_documentation topics environment.network / environment.secrets.
- AWS/Hetzner bill after the fact; no hard $ cap. A forgotten instance keeps charging. Needs an auto-shutdown at job end.
- GitHub Actions: same 20-job pool that PR CI uses, so heavy training would queue CI. Terms of service limit Actions to the repo's own build/test; training the game's brain is arguably that but not certain, and heavy sustained use could be throttled. Runner spec (4 vCPU, 16 GB) is from memory, not re-checked on the docs page. Results come back via a data branch or artifact (500 MB Free storage).
- Spot interruption: a run lost at hour 3 is lost unless the trainer resumes from a checkpoint (champions/ folder suggests best-genome restarts exist; not checked).

# Part 2 (03:40Z): which provider, spend caps, Claude integration. Research only.

## Spend caps, as of Oct 2026 (the hard requirement)
| Provider | Real hard cap? | Source notes |
|---|---|---|
| Modal | YES. "Spend limit" = monthly cap on net charges after credits; at the limit Modal stops workloads that would cost real money. Default limit = usage limit minus credits, so out-of-pocket can be $0. Workspace budget = "hard outer cap". Environment budgets need the Team plan ($250/mo). | modal.com/docs/guide/budgets. Page does not say how fast it bites. |
| AWS | NOT YET for most. New "Spend Limits" (per project, project paused for the month, alerts at 50/75/90%) is in limited release, "not yet generally available". Doesn't say whether EC2/spot is stopped. AWS Budgets alone only alerts. | builder.aws.com article 2026-10-07; aidev.news; Simon Willison 2026-10-03 |
| Google Cloud | Partial. Spend caps in public preview since 2026-07-29, one service per cap; capped Gemini API calls returned 403. Not confirmed for Compute Engine. | medium.com/google-cloud article |
| Hetzner | No cap found; billed after the fact. | costgoat |
| E2B / Daytona | Prepaid starter credits ($100 one-time / $200). Fly's comparison page lists no dollar caps. E2B Hobby sessions last 1 h (too short); Daytona first tier meters 10 vCPUs total (about 2 jobs at once). | fly.io/learn/ai-sandbox-pricing |

## Price per 4-vCPU job (100 gens, 3.6 h today)
- Modal Functions: $0.0000131 per physical core (= 2 vCPU) per second = $0.047/core-h, memory $0.008/GiB-h (modal.com/pricing). 4 vCPU + 4 GiB ~ $0.126/h, about $0.45/job today, $0.30 with the map skip. (Modal *Sandboxes* are ~3x dearer, ~$0.071/vCPU-h: use Functions.) Starter plan: $0 + usage, **$30/month free credits**, 100 containers.
- AWS spot c7a.xlarge $0.077/h = $0.28/job; on-demand $0.205/h = $0.74/job.
- Modal costs about the same as AWS on-demand-ish to 1.6x spot, and is ~2x cheaper than a Haiku runner in tokens (~$1.1/job).
- Modal's $30 monthly credit alone covers ~65 jobs/month today, ~100 with the map skip. Nothing leaves the card unless we raise the limit.

## Claude integration
- AWS: official managed "AWS MCP Server" (https://aws-mcp.us-east-1.api.aws/mcp), listed as a Claude connector. Claude Code for web can add it by URL with OAuth; needs an IAM user/role with the AWS MCP sign-in policy. Gives Claude documentation + AWS API calls (can launch instances), but we would still own AMI/user-data/auto-shutdown. Strongest integration on paper, most setup.
- Modal: Claude Science (a separate Claude desktop product) has a first-party Modal integration: per-job approval card with machine spec, per-second billing and max time shown; default container timeout 12 h (max 23); concurrent-jobs cap. NOT available in these project sessions as far as I can see. Here it would be Modal's Python CLI (`pip install modal`) with a token in the environment secrets, from a runner session. Untested; needs the cloud environment's network policy to allow modal.com.
- GitHub Actions: already reachable from these sessions through the GitHub tools (actions_run_trigger).
- Hetzner: no Claude integration found.

## Recommendation
1. GitHub Actions now (free; add a workflow_dispatch workflow that builds `train`, runs the args, pushes logs/curves/champions to a results branch). Keep training at <=10 of the 20 slots because PR CI queues on the same pool (ci-wall-time memory, 2026-10-03).
2. Modal as the paid/burst option on Starter ($30 free credits, spend limit set to the credits, so the hard cap is real). Use for 12-seed batches and big-core single runs.
3. AWS only if Modal turns out too slow or too dear; wait for Spend Limits to go GA before relying on it as a cap.

## Not verified
Modal spend-limit enforcement delay; Modal max cores per container; Modal reachability from the runner environment; Modal Starter container limits (the "100 containers" line is from the pricing page summary); AWS Spend Limits behaviour for EC2; Actions runner spec (4 vCPU / 16 GB, from memory).

# Part 3 (03:58Z): what was actually run
- GitHub Actions (PR 718, run 39093938467): planner, build (3 min cold), training and results-branch push all work. Runner = AMD EPYC 7763, 4 vCPU = 2 cores x 2 threads. Same 6-gen trail job (pop 100, 8 layouts), identical scores at threads=2 and 4. Total gen seconds: threads2 804, threads4 683 (1.18x from 2 to 4 threads); ~114 s/gen at 4 threads. threads=1 still running.
- Modal (Scott's account, token in the project cloud environment): a 1-core function ran in ~6 s wall incl. startup, cost well under a cent. Needs `pip install 'modal[api-proxy-support]'` (plain `modal` cannot get through the sandbox proxy) and allowed hosts api.modal.com and input-plane.us-east.modal.com (or *.modal.com). Sessions started before the environment change do not see the token or hosts.
- Final Actions thread timings (6 gens, sum of per-gen secs, same scores in all three): threads=1 1595 s, threads=2 804 s, threads=4 683 s. 1->2 threads = 1.98x (near linear), 2->4 = 1.18x (the 4 vCPU are 2 physical cores). Whole run incl. hold-out checks: 1710 s at 1 thread. Build 3 min cold.
- Bug found and fixed (commit on PR 718): the 15-minute snapshot exited silently when no pic-gen files existed yet (pictures come every 5th gen), so a slow run showed no results branch until the end. Fixed with find; the pipeline was checked, not re-run on a real long job.
- Modal scaling test (commit ed4815b3, 6-gen trail probe, seed 1, image build ~140 s): per-gen sum 2 cores/4 threads 578.6 s (wall 643 s, est. $0.022); 8 cores/16 threads 113.0 s (wall 156 s, est. $0.018). 8c/16t is 5.1x faster than 2c/4t (4x the cores) and ~6x faster than the GitHub 4 vCPU run (683 s). Scores are identical to the GitHub 4-thread probe at every generation (gens 0-5 best/median/control). 16 cores/32 threads (run after Scott's go, 05:18Z): sum 43.0 s (wall 75 s, ~32 s overhead), est. $0.017; 13.5x faster than 2c/4t on gen time (more than the 8x core ratio because the 2c/4t baseline is thread-bound) and 2.6x faster than 8c for the same cost. Scores identical at all gens. Rough 100-gen scaling from wall time (trail probe, a fetch job may scale differently): 2c ~3 h/$0.37, 8c ~43 min/$0.30, 16c ~21 min/$0.28. (An earlier $0.12 figure for 8c was wrong: it was $0.018 per 6 gens, so about $0.30 per 100.) Total for the three runs about $0.06 plus image builds. Costs are my formula (cores x wall x $0.0000131 + 4 GiB x wall x $0.00000222), not read from Modal's billing page. lscpu on Modal reports the host (18 and 24 CPUs), so the CPU type is unknown. Also seen on Actions: the hosted runner CPU varies (EPYC 7763 about 44 s/gen vs EPYC 9V45 about 17 s/gen on the same job type).

# Part 4 (2026-10-19 02:00Z): Modal vs AWS vs Google, comparable VMs, best tier (Scott's ask)
Prices read 10-19 from modal.com/pricing, calculator.holori.com (AWS, updated 10-18), gcloud-compute.com (GCP us-central1). Not read from the providers' own price pages for AWS/GCP; list prices, no discounts or credits.
Best tier = 16 physical cores (Modal "16 cores" = 32 vCPU-equivalent threads). Measured on Modal (6-gen trail probe, same scores everywhere): 2c 579 s, 8c 113 s, 16c 43 s -> per-100-gen job ~3 h/$0.37, 43 min/$0.30, 21 min/$0.28. Cost per job is flat while wall time falls 8x. 32 cores NOT measured (800 scoring tasks/gen should still split, but the serial part and hold-out checks will start to show).
| 16-core class | $/h on-demand | $/h spot | 100-gen job (21 min, assumed equal speed) |
|---|---|---|---|
| Modal Functions 16 cores + 4 GiB | 0.79 | none | $0.28 (measured) |
| AWS c7a.4xlarge (16 vCPU = 16 real cores, no SMT) | 0.821 | 0.312 (>20% interruption) | $0.29 / $0.11 |
| GCP c4d-highcpu-32 (32 vCPU = 16 cores SMT, 60 GB) | 1.272 | 0.512 | $0.45 / $0.18 |
Bigger, unmeasured: AWS c7a.8xlarge (32 real cores) 1.642 / 0.616; c7a.16xlarge 3.285 / 0.942. Smaller: GCP c3d-highcpu-16 (8 cores) 0.600 / 0.146; Modal 8 cores ~0.40.
Caps: Modal hard spend limit (verified earlier); AWS Spend Limits not GA; GCP caps in preview, not confirmed for Compute Engine. AWS/GCP also need account, credentials as env secret, network allow-list, launch + auto-shutdown + copy-back (none built). Spot loss on a 21-min job is small (rerun).
