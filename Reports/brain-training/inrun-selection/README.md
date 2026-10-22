# In-run selection: snapshot of the Teacher lane's ant-brain training (2026-10-22)

**What this is.** The ant brain is now trained by selection *inside* a running colony: 24-36 ants dig and forage in a
sealed bed, the best performers parent the newborns, and the colony that survives a round carries its pool into the
next. This folder is a frozen copy of everything that work needs to be picked up again by a later session, so that
nothing lives only on an unmerged branch or a cloud bucket. **It is a record, not live code**: nothing here is built or
run by CI.

## Where the code lives

- **Source of truth:** branch `claude/project-thread-8y5ak0`, head `4f2cefa9` at the time of writing. The harness is
  `examples/inrun.rs`; it relies on env-gated hooks in `src/sim/creature/{mind,options}.rs` and `src/train/challenges.rs`
  (all off by default).
- **The same diff, frozen:** [`harness-vs-e3a82847.patch`](harness-vs-e3a82847.patch) is `git diff` from the branch's
  merge base with `main` (`e3a82847`) to `4f2cefa9`, over `examples/` and `src/`. It does **not** apply to current
  `main`: main has since renamed the bed (`inrun` -> `colony_bed`) and restructured `mind::tick`, dropping the scratch
  hooks. Landing the harness on main means re-threading those hooks and checking hA1 stays bit-identical.
- **Older scratch tools** (copy logs, teacher probes): backup branch `claude/project-thread-8y5ak0-teacher-probes`
  (`76ea8e82`).

## How to run a batch

`tools/inrun_cloud.py start NAME jobs=FILE --ref claude/project-thread-8y5ak0 --type c4d-highcpu-64 --disk 200 --hours N`
builds the branch on a 64-core GCP VM, runs one job per line, uploads to the bucket and deletes the VM. One 64-core VM
per account (quota); job names lowercase. A job line is `NAME_SEED SEED ROUNDS FRAMES SEL GENOME args`; the standard
args are in any `jobs/*.txt` (e.g. `jobs/ha1next.txt`, arm `hA1c`).

- `tools/evreduce.py BATCH out.csv` reduces a batch to one row per colony-round straight from the bucket (never pull a
  whole batch locally; 19 GB filled the disk once). **Home food = deliveries within 8 columns of the nest**; the bed
  books any put-down beside a dug cell as delivered, so the raw count overstates.
- `tools/pairfood.py A.csv:ARM B.csv:ARM` pairs two arms by seed (sign test / McNemar for extinction).
- `tools/cg.sh` is the **common garden**: the last round's top 30 of each colony in a fixed fresh world
  (`seed*1000+99`), 200k frames, selection off. It reports home food, starved, and new-ground dig cuts within 20
  columns. The old hand-built ant runs as `nomind=1`. This is the judge to trust: in-run totals are confounded by
  which colonies survived.

## Genomes (`genomes/`)

| genome | what it is |
|---|---|
| `h1` | first in-run-selected founder (21 Oct) |
| `c1` | foraging lane's carry-home copy into h1; founder 05:40Z 22 Oct |
| `hA1` | c1 + the hungry-only DAgger lesson; **the current founder** since 10:55Z 22 Oct |
| `r4`, `r5` | crater-refusal copies into hA1 (dig lane), under test |
| `hA1noB` | hA1 with the crowd-trail input silenced (maze lane), under test |

## Results that changed the plan (64 colonies unless stated; common garden 32 seeds)

- **hA1 founder:** selection off, starved 4.2 vs 15.3 of 24 (fewer in 59/64 seeds); with selection, extinct 8 vs 17.
- **Selected hA1 vs the old ant, common garden:** home food ~3,470 vs 735; starved 4.9 vs 0; dig cuts 130 vs 326.
  Selection breeds better foragers and steadily erodes digging. **Digging is the open problem** (owner: ants must dig a
  large multi-chamber nest *and* forage all game).
- **Refuted / not shipped:** round length (6x200k vs 24x50k) does not keep digging; fixed dig weights 2-4 only delay its
  loss; `dead_w=0` and `w_hungry=30` do not cut starvation (the latter kills digging); hungry put-down (`PP_NEEDDROP`)
  no effect; rim+spoil vetoes during selection give better foragers (4,403 home) but not more digging; one long world
  is worse on everything.
- **Low mutation** keeps digging (cuts 244) and starves less (3.0, p=.007) but loses more colonies in training (20 vs 8).
- **Appetite gate** on hA1: home 8,269 vs 1,700 (62/64) but extinct 16 vs 8. Card with the owner.
- **Scoring review:** an ant's score is ~87% food, 3% digging; adaptive prices are inert. Colony-level selection gives
  3x food and a third of the starvation but zero digging, because the nest score rewards not digging. Next step: colony
  selection with digging as a gate. See `/mnt/project-files/scoring-review/` and the review doc linked in the design doc.

`results/` holds the per-colony common-garden tables these numbers came from; `jobs/` two example batch definitions (`ha1next.txt`, the standard hA1 control arm `hA1c`; `nob.txt`); every other batch file is in `scratch/inrun/jobs/` on the branch.

## The design doc

[`design-doc-2026-10-22.md`](design-doc-2026-10-22.md) is an export of the living design doc (Claude Docs) as of
2026-10-22 ~15:30Z: the original proposal, the scoring function, pilot findings, every cloud batch with its numbers, and
the open questions. The living copy keeps moving; this one is the frozen record.
