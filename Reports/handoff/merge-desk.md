# Merge desk handoff (written 2026-10-02 22:05 UTC, main e8adc960)

This is for a fresh Claude that will run the merge desk for sgladstein/Pixel_Physics
under a new account.

> **Correction (owner, 2026-10-02; main `d4418bf2`).** The evolution lab
> (`cargo run --release --bin lab`) is **the main game**; the owner has not
> worked on the outdoor sandbox or the held world in months. Where this note
> says "lab box" it means the evolution lab itself; where it says "main game"
> or "main bed" it means the food box, not the game the owner plays.
> **PR 550** (merged as `d4418bf2`) made the lab's ants lay anywhere again
> (`src/lab/scene.rs:843-845`). That is a **revert** of lay-only-at-the-nest in
> the main game, not a fix, and it also moved the `trailfollow` long runs back
> to laying anywhere, because they build through `LabBox`. **The top open
> problem is making laying at the nest work in the evolution lab.**

## What the desk is

All merges go through one session, one PR at a time. Lanes never merge their own
PRs. Scottt ruled this for his project on 2026-09-30. The repo's CLAUDE.md says an
independent session may merge its own PR, but the merge-desk ruling overrides that
here. Lanes may push branches and open draft PRs without asking.

## The flow

1. **Hand-off.** A lane finishes its work and tells the coordinator. The coordinator
   relays the PR number and head SHA to the desk. A lane hands off when the work is
   done, not when CI is green (Scottt, 2026-10-01).
2. **Desk setup.** Subscribe to the PR's activity. If it is still a draft, mark it
   ready yourself; drafts handed off as finished get marked ready by the desk.
3. **Check the head.** `git ls-remote origin refs/heads/main refs/pull/N/head`.
   The PR head must equal the head named at hand-off. If the lane pushed again
   without saying so (this happened on PR 544), read the new commit before merging.
4. **Wait for CI.** It must be green on the current head. Only the latest run counts:
   15 check runs per head (since PR 559). Ignore cancelled runs from superseded pushes. Marking a
   draft ready starts a duplicate run; if a full green run already exists on that
   same head, you can merge without waiting for the duplicate (done on PR 546).
5. **If main moved since the green run:**
   - If what landed on main is docs or scripts only (nothing in src/, tests/,
     examples/, assets/, Cargo.*, .github/) and there is no conflict, merge without
     a rerun.
   - Otherwise merge main INTO the branch (never rebase or force-push), push, and
     wait for green again.
   - Fix trivial conflicts yourself: doc indexes, and generated files rebuilt with
     their scripts, then run `bash scripts/docscheck.sh`.
   - Logic conflicts go back to the lane through the coordinator.
   - **Two or more code PRs queued: stack them** (Scott, 2026-10-03, decision
     card). On the desk's own `claude/merge-desk-*` branch, reset to current
     main, merge each queued PR head in order, push; CI runs on `claude/**`
     pushes. If all 15 checks are green on that stacked head, merge the PRs
     into main back to back, checking with `ls-remote` that each PR head still
     equals what was stacked; no per-PR rerun. If red, fall back to one at a
     time to find which PR broke it. Never push the stack to a lane's branch.
     Why: the only recorded clean-merge breakages (2026-08-25) were stale docs
     indexes, docscheck catches those after every merge, and 9 of 9 desk
     reruns on 2026-10-03 were green, so a rerun per PR cost ~11 minutes each
     and caught nothing.
     **Track it:** every stack gets a row in the rule watch's `Stacks` table
     (PRs stacked, stacked head, green/red, and on red which PR or which pair
     broke it, and whether either PR alone was green). A red stack where each
     PR alone is green is the case this rule bets against; report the count
     in the 2026-10-15 tally, and if it happens, say so to Scott at once.
6. **Merge.** Use a merge commit (not squash or rebase). Pass the full 40-character
   `expectedHeadSha` to the merge tool.
7. **Red CI** goes back to the lane through the coordinator, with the failing test
   names. Do not fix lane code at the desk.
8. **After the merge:**
   - Send the coordinator one line with the PR number, merge SHA and new main.
   - Log a row in the rule watch (below).

## Defaults and permissions

- **Features default on unless there is a real, measured trade-off.** Scottt said
  this on 2026-09-30 and again on 2026-10-01 19:10: "You can turn anything on by
  default that you want."
- If a lane flags a real trade-off and Scottt has not approved it, the PR waits for
  him. PR 546's lab-box cost is one example. He approved it on a decision card.
- **Ignore these old PRs**, from before the desk existed: 12, 140, 143, 217, 254,
  348, 456, 471, 476, 478. Scottt ruled this. Only merge what the coordinator relays.

## CI and repo facts

- CI is ci.yml and takes about 18 minutes. The long pole is the debug-assert test
  job at about 16 minutes.
- CI runs on drafts and on every push to `claude/**` branches. Push and PR runs share
  a concurrency group, so superseded runs show as cancelled.
- There is no branch protection. GitHub merge queue is unavailable because this is a
  personal-account repo. Branch-protection settings return 403 to session tokens.
- Sessions cannot delete branches (every delete returns 403). Pruning is Scottt's job.
- Check main with `git ls-remote origin refs/heads/main`. `gh api <REST path>` works
  through the session proxy; other `gh` subcommands do not.

## Merge history worth knowing (2026-10-01 to 02)

| PR | What | Merge |
|---|---|---|
| 529 | CLAUDE.md slimmed to about 10k tokens; started the rule watch | e7aa3008 |
| 537 | Docs | f063949c |
| 538 | Nest roof, collar and crest switches, all off | d404cdf6 |
| 539 | DIG_ROOF=6 on | 2dd07768 |
| 540 | Forage throttle on | 44f14af1 |
| 541 | Kin footing on | 5244c858 |
| 542 | Brood, off | 34b46b64 |
| 543 | Brood on by default (Scottt's words: "turn brood on by default") | 6235fb1e |
| 544 | Brood egg and pupa stages cut to 250 frames | 8d7cfc9c |
| 545 | Graded fertility on (1.25x) | 6a8dacd4 |
| 546 | Lay only at the nest on (BUD_SITE=nest); bankdump and EATING instruments | 0738a8ca |
| 547 | The dug nest counts as home and food piling, both on | e8adc960 |

Baselines measured before 5244c858 (kin footing) do not compare with later runs.
The queue was empty after PR 547.

## Rule watch

- **What it is.** File: `/mnt/project-files/rule-watch/log.md`. It started when
  PR 529 trimmed CLAUDE.md. Every merged PR is checked against the signature table
  at the top of the file. Misses go under "Misses"; clean PRs go under "Checked" as
  `| date | PR | note |`.
- **Score so far.** Through PR 547: no CLAUDE.md-rule misses. There is one process
  miss: PR 544 pushed after hand-off without telling the desk.
- **The tally is due 2026-10-15.** The scheduled reminder
  belongs to the old account's merge-desk session and will not fire for a new
  account. **The new desk should post the tally to Scottt itself on or after
  2026-10-15.** The tally is misses per rule against PRs where the rule applied.

## Pitfalls already paid for

- **Merged before a heads-up arrived** (PR 538). A roof-flip commit was meant to
  wait. If the coordinator says a follow-up is coming, hold the merge.
- **Default flips break tests that assume the old default** (PR 543). Turning brood
  on broke 25 budding tests in sim::creature::tests. The lane pinned brood off in
  those tests. Expect this on any default-on PR.
- **"Handed to the merge desk" from a lane does not mean the desk received it**
  (PRs 541 to 544). Only coordinator relays arrive here.
- **Unannounced pushes after hand-off** (PR 544). Compare heads before merging
  (step 3).
- **Usage.** Scottt is watching usage. Keep the desk lean: no full test runs, no
  reading big docs. Never start new threads without his explicit yes in his own
  words.
