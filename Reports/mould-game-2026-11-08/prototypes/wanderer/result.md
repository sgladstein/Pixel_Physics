# The Wanderer, prototype result (2026-11-08)

Picture page (GIF, pour strip, arrival card, paired careless/lead, the table): https://claude.ai/artifact/RrEJruyrAveXMGo6c7q5f4

**Answer first.** Leading Mabel to a crack and watching her pour through it gives a real spread, and how well you led shows in the number.

On dish 1, over 8 seeds:

| Hand | Median that arrived | Range | Seeds |
|---|---|---|---|
| Leads her with lamp and oats, waits for the trunk to thicken | 58% | 0–64% | 6 of 8 between 52% and 64% |
| Same lead, but presses Go the moment it can | 11% | 0–62% | |
| Never touches anything | starved | | 8 of 8 |

The careful lead beats the careless one on 6 of 8 seeds. Go is a verb you can see: the yellow drains out of the network along its own tubes into the crack, the HUD counts it through, and what stayed behind goes dark.

**What doesn't work yet:** carrying her over. The next dish's food resets her size, so arriving with 6 walkers or with 1,100 makes little difference to the dish after. "It's your mould" is still something the screen says, not something the next dish makes you feel.

Whether this *feels* better than scoring a dish is for Scott to say on the page of pictures. The numbers say it now has a middle, and a verb.

## What was built

A Wanderer mode of the existing game. The scored dishes and the lesson are unchanged.

- **The crack.** A glowing teal crack on the dish's right edge, labelled THE WAY ON. No food ever appears in it (the sixth pile's spot at (232, 80) is kept out of Wanderer dishes), so reaching it always takes a bridge.
- **Go (G).** Open only while fed mould is in the crack. Before you press it, the hint line reads "G: GO. 1,873 OF 1,873 JOINED TO THE CRACK", so you can see when part of her is stranded.
- **Who arrives: the pour.** On Go, every fed walker joined to the crack by tube sets off along its tubes toward the crack, one walker to a cell. Every one that gets through the rim has arrived. The line is off food, so walkers in it starve, about 7% of the line a second. A long route, or a thin bridge that makes the line queue, loses more of her; a thick, short trunk brings most of her. N counts up live: "THROUGH 497  IN LINE 1,003  DIED 393".
- **"MABEL ARRIVED WITH N OF M."** M is her peak this dish, so mould lost to dawdling counts against you too. The card adds the percentage, when she left, and how many died in the line. If every walker starves you get "MABEL DIDN'T MAKE IT / SHE STARVED AT 6:29".
- **No clock.** No storm, no stars. The food drains as before, and the HUD shows FOOD 73% as the only clock.
- **Carry-over.** N goes to the next dish (new seed), placed as a blob at its left edge around the home pile, laying a little trail so she arrives as a body.
- **Briefing card** for the Wanderer: who she is, the crack, who comes with her, the colours and the tools. A later dish's card opens with "MABEL, DISH 2: 1,134 OF HER CAME".

### How to run it

```
cargo run --release --bin mould -- wander        # dish seed 1; `-- wander 3` for seed 3
```

W from any dish also starts a journey. G: go. N after arriving: next dish with what arrived (after she died: a new journey). R: this dish again, with what came in.

To measure it:

```
cargo run --release --example mouldprobe -- wander=1 seeds=1,2,3,4,5,6,7,8 dishes=3 gif=DIR
```

### Files (branch `wanderer-proto`)

- `src/mould/wander.rs` (new)
- `src/mould/mod.rs`: hooks only. `economy()` and the store are untouched.
- `src/mould/render.rs`
- `src/mould/play.rs`: the hands `Leader` and `IdleGo`
- `src/bin/mould.rs`
- `examples/mouldprobe.rs`: `wander=1`
- `Reports/instruments.md`: the probe's row

## The counts

Each cell is arrived N of M, with M her peak that dish:
- **died:** she starved on that dish, which ends the journey.
- **–:** the journey had already ended.
- Dish k runs on seed s + 100(k − 1).
- Each stretch is capped at 10 minutes of dish.

| Hand | Seed | Dish 1 | Dish 2 | Dish 3 |
|---|---|---|---|---|
| **lead** (lamp + oats, Go when the joined count peaks) | 1 | 1,134/1,924 | 1,524/1,948 | 1,039/1,884 |
| | 2 | 1,000/1,915 | 1,241/1,897 | 1,339/1,995 |
| | 3 | 945/1,617 | 953/1,640 | 1,048/1,730 |
| | 4 | 1,131/1,965 | 932/1,557 | 981/1,689 |
| | 5 | 1,130/1,762 | 1,096/1,928 | 188/1,970 |
| | 6 | 6/1,835 | 1,151/2,164 | 1,442/1,903 |
| | 7 | 1,147/1,977 | 1,195/1,996 | 1,236/1,847 |
| | 8 | 269/1,596 | 918/1,331 | 1,231/1,965 |
| **careless** (same lead, Go the moment it opens) | 1 | 165/1,705 | 850/1,280 | 80/1,550 |
| | 2 | 202/1,867 | 903/1,391 | 4/1,386 |
| | 3 | 984/1,586 | 931/1,579 | 943/1,523 |
| | 4 | 5/1,443 | 95/2,197 | 35/1,689 |
| | 5 | 885/1,439 | 122/1,834 | 112/1,768 |
| | 6 | 57/1,825 | 12/2,107 | 1,025/1,712 |
| | 7 | 209/1,694 | 191/1,695 | 117/1,718 |
| | 8 | 9/1,514 | 4/2,067 | 3/2,260 |
| **idle, Go when open** (no hand at all) | 3 | 526/942 | died | – |
| | 4 | 514/1,216 | died | – |
| | 1, 2, 5, 6, 7, 8 | died | – | – |
| **do nothing** (never presses Go) | 1–8 | died (between 2:43 and 9:02) | – | – |

Dish 1 share that arrived:
- **lead:** median 58% (min 0%, max 64%).
- **careless:** median 11% (min 0%, max 62%).
- **idle, Go when open:** median 0%. On 2 of 8 seeds she found the crack on her own.
- **do nothing:** 0% on 8 of 8.

Walkers through the third crack: lead 188–1,442 (median about 1,140), careless 3–1,025 (median about 100). Every row with all its counts is in `sweep.txt`.

**The three times the lead failed** were the hand giving up its wait (20 s without a better joined count) with only a sliver joined: seed 6 dish 1 (6 arrived), seed 8 dish 1 (269), seed 5 dish 3 (188). A person would see "6 OF 1,800 JOINED" and not press G. The hand is a script, not a ceiling.

**Frame cost**, at the window's scale 4, measured back to back with other sessions building on the same machine:

| Dish | Hand | Mean frame | Worst frame |
|---|---|---|---|
| Wanderer dish | lead | 2.49 ms | 6.74 ms |
| Scored dish | plan | 2.67 ms | 18.45 ms |

The crack's search runs every 6 steps beside the feeding search and costs nothing visible.

## Pictures, with their counts

- **`wander-lead-seed1.gif`** (3.1 MB): one led dish from the start to Go and through the pour, seed 1. Two seconds of dish per frame while she grows, a tenth of a second per frame through the pour. Arrived **1,134 of 1,924**; 1,893 set off and 759 died in the line; Go at 1:22 with 57% of the food left; 10 oats, 36 s of lamp.
- **`lead-seed1-strip.png`**: nine frames from that GIF.
  - Growing: 965, then 1,574 walkers.
  - Go open with 1,873 of 1,873 joined.
  - The pour: through 158 / 366 / 497 / 627 / 788, died 53 / 203 / 393 / 524 / 661.
  - The arrival card.
- **`arrival-card-seed1.png`**: the arrival card. "MABEL ARRIVED WITH 1,134 OF 1,924. 59%. LEFT AT 1:22, 759 DIED IN LINE."
- **`paired-careless-vs-lead.png`**: the same seed, careless (left) against lead (right), dish 1, seeds 1–4, end of dish.
  - Careless: 165 / 202 / 984 / 5 arrived.
  - Lead: 1,134 / 1,000 / 945 / 1,131 arrived.
  - The grey network left behind on the careless side is the part of her that did not come.
- **`briefing-dish2.png`**: the briefing card at the start of dish 2, carrying 1,134.
- **`window-briefing.png`**: the real window (`--bin mould -- wander`, headless screenshot), first frame.

## Decisions, and why

- **The arrival rule went through two versions. I tried the brief's suggestion first and measured it.**
  - **First:** N fixed at Go as the fed walkers within 70 cells of tube from the crack (the food's own reach).
  - **Why it failed:** the mould in this model does not migrate, it extends, and each pile feeds a fixed number of walkers. That rule only ever took the one or two piles nearest the crack. On seeds 1 and 2: lead 203/1,894 and 203/1,902, careless 173/1,705 and 209/1,867. 11% each; leading was worth nothing.
  - **Now:** the pour. It is the theme's own line: "the thicker the trunk to the exit, the more of it makes it through".
  - **Positive control:** the same body of mould behind a 1-cell bridge loses more in the line than behind an 11-cell trunk (test `a_thin_bridge_loses_more_on_the_way`). It goes red when the line's starving is switched off (thin 1.00 vs thick 1.00).
- **The line's starving rate is a third of the dish's unfed rate.** At the full unfed rate the lead lost 77% of its line (1,456 of 1,893 on seed 1), so good and bad routes both read as "most of her died". At a third, the pour lasts about 15 s.
- **M is her peak, not her size at Go**, so "how much of her, at her best, made it".
- **Nothing left in the dish is fed after Go.** On the first try the route out stayed fed, and the 30-odd walkers left on it bred back to 1,760 during the pour, so the network never went grey.
- **One bright dot per walker in the line was tried and dropped.** Where the line queued, the dots drew a white hatched slab. The line is now drawn by its own trail.

## Note for the merge (per-piece food)

The arrival count reads only `fed` and the tube, never the store. The line's starving is a rate applied in `wander.rs`. Walkers in the line are lifted out of `Dish::walkers`, so `economy()` never sees them and is unchanged. It should mean the same under per-piece food.

Two places where the cost of leaving should come from **piece-level starvation** rather than from what I built:

1. **What stays behind after Go.** Today it is starved by setting `fed` to "nobody" from Go on. Under per-piece food, a piece cut off from food starves on its own, which is the more honest version.
2. **Arriving on a new dish.** Today the store is not scaled to the walkers carried in (left alone on purpose), so a big arrival lives off one pile's worth. Under per-piece food, the arriving body is one piece on the home pile and starves down to what that pile feeds unless led to more food quickly. That is the cost the theme describes. It is also the hook that could make carry-over matter (next steps).

## What I'd do next

1. **Make carrying her over matter.** Today a carry-in of 6 brought 1,151 through the next crack (seed 6) and a carry-in of 1,096 brought 188 (seed 5). Size does not compound, because each dish's food resets it. Candidates, cheapest first:
   - per-piece food, so a big arrival can hold several piles at once from the first second;
   - a bigger arrival starts touching more food, by placing it as a wider front;
   - the next dish's food scales with the journey, so a small mould cannot fill it.
2. **Give the drain more bite.** The lead went with 30–72% of the food left on 22 of 24 dishes (median about 55%): running out of food is a threat, but rarely the reason to leave. Smaller piles, or a drain that speeds up once she is spread, would make *when* to go a real choice.
3. **Shorten the pour, or let the player hurry it.** About 15 s is satisfying the first time. A held G that runs it at 4x is the obvious keep.
4. **Show the stranded part on the dish**, not only in the hint line: tint mould not joined to the crack, so "part of her is cut off" reads at a glance.

## What didn't work, or isn't done

- Carry-over does not compound (above). This is the main gap against the theme.
- The lead hand fails on 3 of 24 dishes by giving up its wait. That is the script, not the game, but it is in the numbers.
- No naming screen (always MABEL), no remembered tube shape, no fruiting.
- The worktree I was given was at `main` without the lesson. I branched from `claude/project-thread-cek3ly` (`ac958652`, the lesson and bug fixes), so this merges onto the thread's branch.
- Tests: `cargo test --release --lib mould`: 13 passed. `cargo clippy --release --locked --bin mould --example mouldprobe -- -D warnings`: clean.
