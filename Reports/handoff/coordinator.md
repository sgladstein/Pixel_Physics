# Pixel Physics: coordinator handoff (2026-10-02)

Owner: Scottt (GitHub sgladstein). Repo: https://github.com/sgladstein/Pixel_Physics.
Main is e8adc960 (PR 547, merged 22:23). PRs 529 to 547 have merged.

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

## Scottt's goals and standing decisions

- **Colony goal: a stable colony, not a big one.** Smaller colonies are fine, even
  intended. He wants a colony that breeds less, builds up a food supply and survives
  long term, not one that booms and busts. Judge ant changes by long-run survival
  and food stock, not by colony size or fruit taken.
- **Structure must emerge from ant rules.** We do not draw rooms or castes; the
  layout is "a problem for the ants to solve".
  - Behaviours should be able to evolve between species, so prefer genome or
    brain-weight dials over fixed constants.
  - Ants are where they are because of their work, not because something pulls them
    to a spot. Nest workers stay in the nest because their tasks are there: dig,
    nurse, move food from the entrance to the store, and carry spoil out.
- **No guessing on biology.** Never answer "how real ants do it" from memory.
  1. Check the repo's research reports first: Reports/nest-biology-2026-09-19.md,
     the biology table in Reports/ant-breeding-plan-2026-09-29.md, and
     Reports/ant-sim-literature-review-external-2026-09-19.md.
  2. Then do web research.
  3. Cite sources either way.
- **New features default to ON unless there is a real trade-off.** His words
  (2026-10-01): "You can turn anything on by default that you want".
- **Never start a new thread without Scottt's explicit yes in his own words.**
  - A question from him is not a yes.
  - "Up to you whether to continue or start a new thread" is a yes.
  - Small code questions go to an existing lane.
- **He watches usage.**
  - Track token use and surface ideas for making the process cheaper.
  - Read targeted doc sections only. README is about 72k tokens; CLAUDE.md was
    slimmed to about 10k in PR 529.
  - When a lane's context gets very large, it finishes its current step and asks him
    whether to continue in a fresh thread.
- **Where he talks.** He talks to threads directly about their own work, and uses
  the project chat for cross-thread work and status. Relays to threads carry his
  message verbatim plus the coordinator's instructions.

## How work flows

- **Lanes.** Three are active: Ant breeding plan (breeding), Nest rooms from piles
  (nest), and the Merge desk. Foraging throttle and top-up is finished. Scottt
  expects nest and foraging to merge into one lane eventually.
- **Permissions.** Any thread may push branches and open draft PRs without asking.
  Lanes never merge; all merges go through the merge desk, one at a time.
- **Hand-off.**
  - A lane hands its PR to the coordinator as soon as the work is done, not after
    CI is green.
  - The coordinator forwards it to the desk. The desk marks it ready, merges when
    green, and sends red CI back through the coordinator.
  - A lane saying it handed a PR to the desk does not mean the desk got it. Check.
- **Pre-PR testing: no full suite.**
  - Run fmt, clippy and the touched area's tests. When a change touches a list that
    other code loops over, add that area's integration tests.
  - Open the draft and keep working. CI runs on drafts and on every claude/** push,
    and takes about 18 minutes.
  - Lanes slip into running the full suite. Correct them.
- **Lane test runs** take 5 to 8 minutes each, 4 at a time. A check of 3 versions on
  12 seeds takes 45 to 50 minutes.
- **Who owns what.**
  - Breeding owns lay_egg and brood carrying in brood.rs, `creature::kin_deficit`
    and `brood::nurse`.
  - Nest owns where nest workers go, and the larva-hunger scent. The scent must
    read hunger through kin_deficit, and only for the Larva stage.
  - Cross-lane changes go through the coordinator.

## Where things stand

- **On by default now:**

  | Feature | PR | Notes |
  |---|---|---|
  | Brood | 543 | |
  | Egg and pupa stages | 544 | |
  | Graded fertility, 1.25x | 545 | PIXEL_PHYSICS_BREEDING_MAX |
  | Lay only at the nest | 546 | BUD_SITE=nest |
  | Kin footing | 541 | |
  | Nest roof, DIG_ROOF=6 | 539 | |
  | Foraging colony throttle | 540 | |
  | Dug nest is home; food piling | 547 | PIXEL_PHYSICS_NEST_HOME=material and PIXEL_PHYSICS_STOREROOM restore the old behaviour |

  The share top-up is built but off.
- **Known breakage on main.** Laying only at the nest kills the lab box: births
  329 to 4, and 10 of 12 boxes died. Scottt chose to ship it anyway.
  - The dug home did not rescue it. The lab colony stays small, and rich ants sit
    out on the surface.
  - Walk home to lay and three other laying fixes did not rescue it
    (12 seeds: births median 4 to 7, 10 of 12 boxes still died). They are on
    the breeding branch, not merged. The lab problem is still open.
- **Nest lane.**
  - PR 547 (merged, e8adc960) made the dug home and food piling the defaults,
    based on food-box results. With piling, the colony falls 3-7% from its peak, against 35-59%
    today. It does not fix the lab.
  - The larva-hunger cue crashed colonies and was pulled.
  - Tether removal exists only as an off switch.
- **There is no food store yet.**
  - Food is banked as energy inside ants with no ceiling.
  - 96-98% of eating happens away from home.
  - "Fed ants store" and a food-keyed breeding brake are the breeding lane's open
    items.
  - The existing appetite gate (eating cap) made things worse on main.
- **Open question for Scottt:** should undisturbed soil hold game-wide? It is the
  real lever for tunnels refilling, and it is an engine change.
- **Rule watch.** /mnt/project-files/rule-watch/log.md logs misses of the rules
  trimmed from CLAUDE.md. The tally is due around 2026-10-15, and the new desk must
  post it itself.

## Working habits to keep

- Stamp every measurement with the main commit it ran on.
- Run a fix before announcing a cause. Check the lab box before saying, or
  predicting, that a fix works.
- Re-check a claim when moving it to a new bed.
- Trace individual ants before naming a cause from an aggregate counter.
- Name what a counter actually counts before quoting it. "Stored food" turned out to
  be surface crumbs, and "at the nest" was only the door strip.
- Confirm a cause on enough seeds before relaying it.
- Put results in commit messages.
- Run any change to where ants walk on a bed with food.
- **Baselines that do not compare:** anything measured before 5244c858, and lab
  numbers on 0738a8ca or later against 6a8dacd4.

## Gotchas

- The board marks a thread "waiting on you" as soon as it posts a PR link, even when
  nothing waits on Scottt.
- PR 521 is an adversarial review of earlier work. Use what is relevant.
- The project-files folder (/mnt/project-files) and its reports may not carry over
  to a new account. Anything the new team needs should be in this file or the repo.
