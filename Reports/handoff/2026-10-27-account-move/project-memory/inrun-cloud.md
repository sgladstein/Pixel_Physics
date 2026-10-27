---
name: inrun-cloud
description: How the teacher lane runs in-run selection batches on Google Cloud (owner offered 48 free cores, 2026-10-21)
metadata:
  type: project
  modified: 2026-10-22T04:55:00Z
---

Owner (2026-10-21 ~23:00Z): "we now have 48 cores free whenever you need them on Google cloud" — standing go for in-run batches.

**Setup:** branch `claude/project-thread-8y5ak0` (compute branch, never for main, no PR) = main + in-run harness + nest score (PR #770 crater fix d8770ccb) + pheromone speedup (PR #775 e1294c7f). Local worktree /home/user/cloud.
- `scratch/cloud/inrun_cloud.py start NAME jobs=scratch/inrun/jobs/X.txt --ref claude/project-thread-8y5ak0` (one c4d-highcpu-48 spot VM, --hours 2), `status`, `pull NAME DEST`, `delete NAME` (only after pull shows DONE). Reuses Reports/brain-training/cloud-compute/gcp/gcp_train_job.py; needs `pip install google-cloud-compute`.
- Jobs line = rounds.sh args: `OUT SEED ROUNDS FRAMES SEL GENOME args...`. Each job's folder goes to bucket runs/NAME/jobs/OUT.tgz. `scratch/inrun/armsum.py DIR` summarises arms (food per round shows whether selection is improving).
- A jobs file must be committed and pushed before start (the VM builds from the branch).
- Full event logs ARE kept in the bucket (runs/NAME/jobs/*.tgz, `pull --which jobs`); default `pull` gets small/ copies without events.csv. `scratch/inrun/evreduce.py BATCH OUT.csv` streams them to one row per colony-round; `scratch/inrun/pairfood.py` pairs arm vs control by seed (McNemar on extinction, sign test on home food per colony-round over rounds both reached).
- Replay page: https://claude.ai/artifact/HFEskdEuRrzvSsSEYU7LeK (`film=N`, scratchpad film/data.py). `trace=N` writes trace.csv per ant.

- Main vs compute branch (training-speed thread, 2026-10-22 05:58Z): main's perf commits are bit-identical and give no speed gain here; main also carries 3 result-changing commits (drainage 0.25->1.0, mind energy sense reads species tank in training arenas, drier soil -> smaller plants). Merge main only at an experiment boundary and rebaseline controls then.

- Long runs (2026-10-22): long60b filled the VM's 60 GB disk (64 jobs x 60 rounds of events.csv) and every later job crashed instantly and read as EXTINCT. Use `--disk 200`; rounds.sh now gzips each finished round's events past 10 rounds. A job whose r0 holds only mutants.txt crashed, it did not die out.

- **Harness on main (2026-10-22 ~22:00Z, thread "Training harness on main")**: branch `claude/project-thread-39xi6a` = compute head 75593154 + main d4a6c590, PR #833 at the merge desk. Hooks re-threaded onto main's mind::tick (context_at/decide_planned); hA1 byte-identical to the compute branch (8 seeds x 2x200k, veto and PP_HOMEDIR arms too) except nest.csv (main's newer nestscore). Teacher lane instead merged it into the compute branch (743157ab, 0 behind main d4a6c590) and keeps launching from `claude/project-thread-8y5ak0`; its commits after #833 (colsel8, cut_join) need a follow-up PR to main.

Results per batch: [[inrun-batches]]. See [[inrun-selection]].
- Colony selection (colsel.py) runs rounds in lockstep, so VM CPUs sit at 60-70% (owner noticed 2026-10-22 20:05Z). Oversubscribe: ~30 processes per arm (P=30 x 3 arms on 64 vCPUs).
- **Compute limits (owner 2026-10-23 03:06Z):** Google Cloud = 64 cores total, one 64-CPU spot VM at a time; queue batches. Extra compute: GitHub Actions or Haiku cloud-session runners.
