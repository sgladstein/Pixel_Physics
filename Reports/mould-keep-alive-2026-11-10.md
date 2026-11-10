# Keep It Alive, and water and sleep (2026-11-10)

**Status: built, on by default in Keep It Alive; waits on the owner's play.**
Game: the mould probe (`cargo run --release --bin mould`), its fourth dish.

The owner picked an endless dish after the independent review
([mould-review-2026-11-10.md](mould-review-2026-11-10.md)), then agreed an
order for depth: (1) the endless dish, (2) water and sleep, so the tools work
on each other, (3) a wild second mould. This report covers (1) and (2).

## What the dish is

Keep It Alive is Running Dry at 22.5 degrees with no bell. The mould starts on
three small piles. A new pile lands every 20 s, each further from the last
(`Win::Endless`: gap 70 + 6 per pile, at most `GAP_MAX` 170 cells). You get 6
oats that never come back. The dish ends when fewer than `ENDLESS_MIN` (20)
walkers are left, and the score is how long it lived and the most mould at
once. A 15-minute safety cap exists; it is not a bell.

Why the mould dies: the stray rule (`Params::stray_reach` 24 on hand-made
dishes) means unfed walkers can't start on food far from fed tube. Once the
next pile is more than about 50 cells away, the mould wanders onto it and
starves there.

## Water and sleep (`Params::wet`)

Every cell has a wetness (`Dish::wet`) that rests at `wet_base` 0.5 and
relaxes back at `wet_relax` 0.0005 a step. The lamp dries the ground under it
(`dry_light` 0.05 x light x wet). On ground drier than `crust_wet` 0.25 a walker
falls asleep (`crust_rate` 0.05 a step). A sleeper doesn't move or eat, pays no
upkeep, isn't pushed by the lamp, and lays a faint trail (`crust_trail` 0.6).
It dies at random with a mean life of `crust_life` 20 s. Ground wetter than
`revive_wet` 0.7 wakes it (`revive_rate` 0.05 a step). For `wake_grace` 5 s
after waking it counts as fed, so it can start on food near where it woke.

The new tool is **Water** (`Tool::Water`): a drop of radius `water_r` 12 that
makes the ground wet. You have `WATER_STOCK` 2 drops, and one comes back every
`WATER_REGROW_SECONDS` 40 s. The lamp gets a second job: it puts mould to
sleep, so it can be carried through a famine, then woken with water by the next pile.
Keep It Alive turns it on through its tools (`Tools::water`); the three puzzle
dishes are dry.

**A wet dish nobody lights is the dry dish step for step**: doing nothing and
the oat-saver live exactly the same times wet and dry on 16 of 16 seeds. The
guard is `a_wet_dish_left_alone_is_the_dry_dish`, and
`the_lamp_puts_mould_to_sleep_and_water_wakes_it` goes red when waking is
disabled (0 woke).

## Numbers (16 seeds, `mouldprobe level=4`)

How long it lived, min-max (median):

| Hand | Lived | Stars (4:15 / 5:30 / 6:40) |
|---|---|---|
| Doing nothing | 1:08-3:48 (2:10) | 0 on 16 |
| Random | 1:38-4:04 (2:40) | 0 on 16 |
| Random, dry | 1:07-3:08 (2:13) | 0 on 16 |
| Oat-saver (`play::Route` "alive", no water) | 2:47-5:26 (3:34) | 1 on 3 |
| Waterer (probe `sleeper`) | 3:20-11:18 (5:05) | 0 on 4, 1 on 5, 2 on 1, 3 on 6 |

Paired, sign test: the waterer beats the oat-saver on 16 of 16 (p 0.00003),
random on 15 of 16 (p 0.0005), doing nothing on 16 of 16. The oat-saver beats
doing nothing 14-1 (p 0.001). Water doesn't help random significantly (11 of
16, p 0.21). So the bars the owner agreed hold: a good game uses three tools
(lamp, oats, water), and random still loses to a plan.

The waterer is a script, not a person. When the food under the mould is
nearly gone and no fed mould is within 35 cells of the next pile, it holds
the lamp for 1.5 s on the thickest awake mould. It waters the sleeper nearest
an unreached pile within 45 cells. It still gets no star on 4 seeds, where the
next pile lands too far for any sleeper to be near it.

Without `ENDLESS_MIN`, all of these lived longer, because a handful of
sleepers coasted for minutes. The dry numbers that the first cut's commit
quotes (doing nothing 1:21-4:08, oat-saver 3:18-5:40) were measured before
that rule.

## Tuning that was tried

At 8 seeds each:

| Water | Regrow | `crust_life` | Random | Waterer |
|---|---|---|---|---|
| 3 | 20 s | 40 s | median 4:05, cap on 4 of 16 | median 9:59 |
| 2 | 40 s | 40 s | 2:48-5:30 | median 9:59 (cap) |
| 3 | 20 s | 20 s | 1:53-5:15 | median 6:14 |
| **2** | **40 s** | **20 s** | **1:49-3:11** | **median 8:18** |

At the first setting, scattered water woke enough sleepers that random play
coasted to the ten-minute cap. Both levers were needed.

## Not yet

- The looks thread draws the wet ground, sleeping mould, the water icon and
  its preview. Until then water uses a stand-in icon, and sleepers look like
  any walker.
- Wild second mould: next.
- Finer tubes (sensor reach about 6): still to come, and they move the
  balance, so these numbers will need measuring again.
