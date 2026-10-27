# Merge desk: lane hand-off note (written 2026-10-27 ~18:50Z, main 09e338c)

Facts only. The standing procedure is `Reports/handoff/merge-desk.md` (written 2026-10-02 by an earlier desk). This note
is what that file does not say, plus the state right now. Everything here is from this desk's own session, which began
2026-10-21 and ran to this date; I do not have the earlier desk's history beyond what that file records.

## State right now

- **PRs I hold: none.** Last hand-off merged was #943 (pheromones review) as `09e338c` at ~17:59Z. Main is `09e338c`.
  `readmetoc.py --check` current and `docscheck.sh` clean on main after it.
- **Merged by this desk since 2026-10-21 (the ones I can name with SHAs):** #926 docs fix `ce49832`; #933 `ed9dd21`;
  #934 `b384f43` (brain layout 6, per-unit gate, on); #935 `d8387ed` (layout 7, reach-aware gates, on); #936 `c7c975d`;
  #937 `b7ba0cf`; #938 `76e7564` (colsel elite=1, off); #939 `063810a`; #940 `0034bf5`; #941 `44410be`; #942 `eeb7611`
  (layout 8, write gate, off, trainable only under `PP_WRITE=on`, genome length 4137); #943 `09e338c`. Also merged
  between #919 and #932 (all clean, 15/15 on the exact head): #919, #922, #925, #927, #928, #929 (merge-in needed, see
  below), #931 and the rest of that run. I did not keep SHAs for those; `git log --merges --first-parent origin/main`
  has them.
- **Waiting on an owner verdict (as I know it, thread states as of ~15:00Z 2026-10-27):**
  - Seat-demo threads: Queen seat (`cmsg_01Mei3dwkcdzPxa5Jmuqi2bBFTL11Fjw5VvcaTYAb9JLd7`) and Colony mind seat
    (`...EgF6GcJFQCizK4Vq4jzPFk`, **no standalone PR was ever handed to me**) were `blocked` (need input) at last look
    (2026-10-26 listing); Lone scout (`...4BAX9fz9kWQxgEhyKnVPS3`) and Enemy seat (`...6bSGLzkCV4hUfCkREaLmTi`) show
    `working` at 18:31Z 2026-10-27, so they may hand in more PRs. Their merged PRs are already on main
    (#905, #908, #909, #910, #914, #917, #920, #922 per memory); what is open is the owner's next-step choices.
  - Plant community plan thread (`...5iFuekoJWbiCr1frmZMiqp`) shows `working` at 18:30Z 2026-10-27.
  - Brain layout 6 and 7 owner calls are done (approved 18:23Z and 22:20Z on 10-26). Layout 8 was "prototype it, off
    by default" (15:05Z 10-27).
- **Open PRs on GitHub that were never handed to this desk (leave them unless the owner or a lane says otherwise):**
  #706 (scouts search home, SCOUT_SEARCH on), #705 (meet_few walk), #702 (draft, emergence sandbox tests), #701
  (emergence diagnosis report), #686 (draft, play-as-ant slice 1), #633 (draft, SOLID_MOUND). **Old ignore list** (the
  earlier desk's ruling by the owner, still open on GitHub): #12, #140, #143, #217, #254, #348, #456, #471, #476, #478.
- **Branches carrying unmerged work with no PR that I know of** (from `git branch -r --no-merged origin/main`, 82
  `claude/*` branches in total, so most are old): `claude/project-thread-8y5ak0` is the Teacher lane's working branch
  and moves constantly (it carried #936, #937, #938; newer commits on it after #938 are not handed in);
  `claude/project-thread-8y5ak0-playtest` (2026-10-24), `-teacher-probes` (2026-10-22), `claude/project-thread-oo9r0d`,
  `claude/project-thread-mblj3z`, `claude/project-thread-eewfjj` (all Oct 20-22; I never saw a hand-off for them).
  The `claude/merge-desk-*` branches (stack-2/4/5/6/7/8, 694, 700, 729, 732) are this desk's and earlier desks' scratch
  stacks; nothing on them is wanted. `claude/project-thread-8y5ak0-near` is a stray ref in old history. I cannot delete
  branches (403); pruning is the owner's.

## What the old desk file does not tell you (procedure as actually run)

**Probe on every hand-off** (fast; do not run the test suite, CI is the suite):
```
git fetch -q origin main refs/pull/N/head:refs/tmp/pN --force
git ls-remote origin refs/heads/main refs/pull/N/head      # head must equal the hand-off SHA, main = what you expect
git merge-tree --write-tree origin/main refs/tmp/pN | grep -E 'CONFLICT|^[0-9a-f]{40}$'   # lone sha = clean
```
Then `subscribe_pr_activity` for the PR (merged PRs unsubscribe themselves). **Do not pipe the fetch/merge-tree probe
into something that needs a long timeout**: one time it moved to the background at 120s; read its output file.

**CI.** `get_check_runs` returns `total_count` 15 for one run and 30 when a superseded duplicate run (cancelled jobs) is
also listed. Count 15 `success` on the *latest* run for the exact head; ignore `cancelled` ones and a straggler
`in_progress` entry from the cancelled run (seen on #939). `check_suite.completed` can fire before every check
finishes, so re-read the check runs, re-run `git ls-remote`, and only then merge. `docscheck (informational)` and
`cargo fmt --check (informational)` are non-gating in CI: **a PR can be green and still leave docscheck red on main**
(#925 did), so the desk runs `PYTHONWARNINGS=ignore bash scripts/docscheck.sh 2>&1 | grep -E '^docscheck'` and
`python3 scripts/readmetoc.py --check` on main after every merge. A docs-only fix PR (#926) is the repair.

**Merge.** `merge_pull_request` with `merge_method: merge` and the full 40-char `expectedHeadSha`. After it, `git checkout
main && git fetch origin main && git reset --hard origin/main` in the desk's checkout (it never carries local work),
run the two checks above, then tell the owning lane the merge SHA with `message_thread`. Lanes hold pushes until told.

**Conflicts / generated files.** When a PR conflicts with main (happened on #929): `git checkout -B mdesk-N refs/tmp/pN;
git merge --no-edit origin/main`; resolve only inside the hunks. Union a TSV or index by joining both sides
(`re.sub(r'<<<<<<< [^\n]*\n(.*?)=======\n(.*?)>>>>>>> [^\n]*\n', lambda m: m.group(1)+m.group(2), s, flags=re.S)`).
Generated dead-ends files: `git checkout origin/main -- Reports/data/dead-ends-index.tsv Reports/data/dead-ends-skeleton.md`
then rerun `python3 scripts/deadendindex.py`. README TOC: `python3 scripts/readmetoc.py`. Run `cargo check --release --lib
--examples` before pushing. Commit with trailers `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>` and the
`Claude-Session:` line. Push `git push origin HEAD:<lane branch>` (find the branch with `git ls-remote origin
'refs/heads/claude/*' | grep <sha prefix>`); a merge commit, never rebase or force-push.

**docscheck requirements new content trips over** (each cost a repair): every `examples/*.rs` needs a row in
`Reports/instruments.md`; every `Reports/dead-ends.md` entry needs a verdict row in
`Reports/data/dead-ends-triage/screened.tsv` (`key<TAB>id<TAB>label<TAB>confidence<TAB>reason`); the `## section (N
entries)` counts in dead-ends.md must match; the index and skeleton are regenerated by `deadendindex.py`. Reports/README.md
is a contested file: land a PR that touches it promptly. Lanes were told to run docscheck before hand-in.

**Defaults-on rule, as the owner said it (2026-10-24 17:35Z):** "We keep developing features off by default. The default
should be on unless there is uncertainty." I read every hand-off for default-off switches. Measured harm or a stated
uncertainty in the PR is enough to leave one off; otherwise I ask the lane to flip it. I judged these fine as off:
player-triggered demo verbs, labelled SCRATCH experiment knobs and harness-only selectors that leave a run unchanged
when unset (`inrun.rs` `oldage=`, `tick=lab`; PP_GATE_FILE; PP_SHARP_MAX), switches with measured harm
(PP_HOMELOW 52 of 52 starved when ungated; PIXEL_PHYSICS_NEST_SMELL lab harm 538 vs 78), and the owner-chosen
"prototype, off by default" PP_WRITE. Regrow for the centipede (#922) is on, kill switch `PIXEL_PHYSICS_REGROW=off`.

**Ordering.** Every merge to main can conflict or restart CI for other open PRs, so merge-ins are serialised and a
pushing lane is asked to hold. I merged one PR at a time (never needed the stacked-run rule in the old file). The
Teacher lane owns the one ant line (ONE ANT rule); other lanes' ant work lands as lessons, scores, arenas and findings.

**Tools.** The GitHub MCP tools disconnect and reconnect often. After a notification, load them with
`ToolSearch select:mcp__github__pull_request_read,mcp__github__merge_pull_request` and retry; a disconnect notice is not
a failure of the PR. `mcp__hearthbot__message_thread` reopens a resolved thread, so find a thread id with `list_threads`
(filter `since`) before messaging a lane that has finished. Thread ids I used: Teacher lane
`cmsg_01Mei3dwkcdzPxa5Jmuqi2bBFFqqpwTYL4i4DtFihYcesb`; Pheromones lane `cmsg_01Mei3dwkcdzPxa5Jmuqi2bBXcUsoiyAwhaYT7FpxPWsFj`;
brain-design review `cmsg_01Mei3dwkcdzPxa5Jmuqi2bB5G1Cu2o5Nk5bUP35Fp8FQv` (resolved 16:27Z); Enemy seat, Nest raider
(`...XGhNsrMxHxZy8iwx34N6pS`), Queen seat, Colony mind seat, Lone scout as above. These ids die with the old account;
the new project will have its own.

**Owner standing rules that shape the desk (all in project memory on the old account, repeated here):**
lanes open a PR and hand off, they do not watch CI; the desk merges on its own once 15/15 are green on the exact head
and does not wait for owner review; the review-queue tool is retired, so questions and picture comparisons go to the
owner in chat with images on an Artifact page; no waiting on the Google VM; "land before resolve" (owner 2026-10-24
15:04Z): no thread resolves with reports only in `/mnt/project-files` or memory, they land in `Reports/` via the desk.

## Honest gaps

- I have no list of every PR merged before #919; use `git log --merges --first-parent`.
- I did not audit whether the 82 unmerged branches hold unique work; only the ones named above are known to be live.
- The colony mind seat demo was never handed in as a PR; its thread has been `blocked` since 2026-10-25.
