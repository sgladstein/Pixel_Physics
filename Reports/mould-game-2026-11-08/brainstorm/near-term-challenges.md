# Near-term challenges for the Mould demo

*Game designer brief: challenges buildable in days. Written 2026-11-08.*

## The best three, to build first

1. **Behind the Wall.** The food sits behind a salt wall the level drew. Oats aimed straight at it pull the mould into the wall, so you have to read the layout and lead the mould round. It is the cheapest dish to build, it gives a first-time player a clear win, and it builds the level format every other dish needs.
2. **Beat the Ruler.** Three foods in a big triangle and a dotted "ruler" line: the shortest way to join them using only straight food-to-food tubes. You win by using *less* tube than the ruler, and the only way to do that is a meeting point in empty space. The owner's favourite thing, the little bridge where there was no food, becomes the goal.
3. **Running Dry.** Every pile runs out in about 35 seconds, and new food turns up elsewhere. The mould has to keep moving house. That fills the dead middle minute by design, and the owner's two laws show on screen: old tubes go grey and die back slowly as their food runs out, and you are always the one deciding where the mould goes next.

Below: what the model can and can't do, the eight dishes in order, the shared code, then the dish select screen.

---

## What the model allows (read from `src/mould/mod.rs`, checked with two probe runs)

These facts decide which challenges are real:

- **The leash.** A walker more than 70 cells of tube (`feed_reach`) from food it is eating is unfed. It can't divide, and it dies in about 5 s on average. So on its own the mould fills about 70 cells round its food and stops. **Any target further than that by path cannot be reached by doing nothing.** That is our main tool for making "do nothing" fail without a timer.
- **Oats are food,** so each oat moves the leash. An oat holds 400 food, and 40 eaters drain it in about 3 s. A relay made of oats is short-lived. Real piles hold 20,000, enough for about 140 s at a full table.
- **Each pile feeds 40 eaters,** which works out to about 300 walkers per joined pile (the traces show 600, 900, 1,200 exactly).
- **Salt blocks scent too.** `spread` neither reads nor writes salt cells, so food behind a wall can't be smelled through it, only round it.
- **The push lamp never breaks a tube** (measured: 0 of 5 trunks lost). It bends tubes and stops growth under it. It does not cut.
- **Thrift is measured against the shortest food-to-food tree** (Prim's tree over the piles). A meeting point in empty space (a "Steiner point") can beat that tree by up to 13%, so a score under 1.0 is possible and means a real bridge.
- **The storm** cuts the one tube whose loss costs the most food. A loop shrugs it off.

**What I ran** (the existing binary, no rebuild, no edits):
- `mouldprobe players=idle,plan seeds=1,2,3`: do nothing joined 4, 2 and 3 of 6 piles. The plan joined 6/6 every time, at a thrift ratio of 1.66, 1.52 and 1.05.
- The same run for the plan with `feed_reach=40`: still 6/6 on all three seeds, but starved walkers rose from 51-110 to 750-1,244. So a shorter leash does not lock out a player who uses oats. It just costs more mould. That makes it a fair difficulty dial for later dishes, and it shows that oats relays work.

**Weighed and left out: fixed lamps.** The push lamp never cuts a tube. A fixed lamp would only be a soft wall, a vaguer kind of salt. A maze of salt says the same thing more clearly.

---

## The run: eight dishes after the lesson

Each dish gives only the tools it needs, so the tool bar itself tells you what the dish is about.

### 1. Behind the Wall *(easy, 90 s)*

```
 .-----------------------------.
(  H          |                 )
(  @          |           F     )      H = home   F = food
(             |                 )      | = level salt wall
 (            |                )       gap at the bottom
  '-----------      ----------'
```

- **Goal on screen:** "REACH THE FOOD BEHIND THE SALT."
- **Twist:** the level draws the salt. Home at (40,80), food at (210,80). A wall at x=128 runs from the top rim down to y=125, leaving a gap at the bottom.
- **Doing nothing fails:** the path through the gap is about 207 cells, three leashes long. The mould fills its 70 cells and stops.
- **A random hand fails:** oats dropped at random mostly land on the wrong side of the wall or too far ahead to smell.
- **The obvious move fails:** oats in a straight line pull the front into the wall, where walkers bunch up. Scent doesn't cross salt, so the far oats are never felt.
- **Intended solve:** about four oats down the wall, through the gap and back up. Stars by time: reach it at all (1), by 1:00 (2), by 0:45 (3).
- **Tools:** lamp, oats 5 (regrowing).
- **Code:** the `Level` struct with fixed piles and pre-laid salt (below). Win = the existing "reach" test. **About 0.5 day, most of it the struct.**

### 2. Three Oats *(easy-medium, 75 s)*

- **Goal:** "JOIN ALL FIVE FOODS. YOU HAVE THREE OATS. THEY DON'T COME BACK."
- **Twist:** a fixed stock, no regrowth. Home sits in a line of four foods. The first gap is 55 cells, inside the leash, so the mould gets there alone in about 10 s. The next three gaps are about 90 cells each, just past the leash.
- **Doing nothing fails:** it joins home and food A, then stops (2/5).
- **A random hand fails:** three random oats are three wasted oats.
- **The obvious move fails:** "oat toward the nearest unjoined food" spends the first oat on gap A, which never needed it. The player then lacks one oat at the end. An oat dropped too far ahead of the yellow edge is never smelled, and it rots unused.
- **Intended solve:** wait for A, then drop each oat about 15 cells beyond the yellow front of each long gap. This dish teaches the leash. The yellow/grey edge is the thing to read, and the second line of the brief says so: "GREY MOULD IS TOO FAR FROM FOOD."
- **Code:** `oats_stock` and `oats_regrow: Option<u32>` on the level, in place of the two constants. **About 1 hour on top of dish 1.**

### 3. Beat the Ruler *(medium, 100 s)*

```
              B
             / \          ruler (dotted): A-B and A-C, two sides
            /   \         best: three tubes meeting at the middle,
           /  *  \        where there is no food (*)
          A-------C
```

- **Goal:** "JOIN ALL THREE WITH LESS TUBE THAN THE RULER."
- **Twist:** a new win condition, plus a drawn ruler. The foods sit at (63,134), (193,134) and (128,22), a triangle with sides of about 130 cells. Each food is about 69 cells, rim to rim, from the middle. The ruler is the two shortest sides, drawn dotted. The HUD shows a live number: "TUBE 1.12 x RULER".
- **Doing nothing fails:** B and C are about 118 cells away, past the leash.
- **A random hand fails:** loose spurs and loops push the number well above 1.
- **The obvious move fails:** leading food to food along the sides makes the ruler itself, 1.0 at best. You can't beat the ruler by building the ruler.
- **Intended solve:** one oat in the empty middle, the bridge. Home's mould reaches it, because it is just inside the leash. Then oats outward from that junction to B and to C, not from home. The perfect answer is 0.87. At the end, the perfect three-way junction is drawn faintly over your network, so you see how close you got.
- **Stars:** joined (1), under 1.0 (2), under 0.93 (3). The bars must be set from a probe run on this layout, with headroom, before the dish ships. **Risk:** network length is tube area divided by 12, and junction blobs and fat tubes add noise. If plain mould scores a scatter of ±0.1, the 3-star bar moves to 0.95.
- **Code:** a `Win::UnderRuler { bars }` rule, which reuses `measure()` and `Score::ratio()`. The ruler overlay draws the Prim edges already computed in `measure` (expose them). The ghost Steiner junction comes from Weiszfeld's iteration over three points, about 15 lines. **About 0.5 day.**

### 4. Overgrown *(medium, 75 s)*

- **Goal:** "TRIM THE WEB TO 1.25 x THE RULER. KEEP EVERY FOOD."
- **Twist:** the dish opens already grown. Four foods, all joined, wrapped in loops and stray fingers, at a ratio of about 1.6. Snip is the main verb, with 8 snips, the lamp and no oats. This is the dish that finally explains Snip, which the owner said he didn't understand.
- **Doing nothing fails:** the web stays branchy. Do nothing sat at 1.16-1.37 even on small networks, and the plan at 1.5-1.7 on big ones.
- **A random hand fails:** random cuts land on trunks. The far side goes grey and dies, and a food is lost.
- **The obvious move fails:** "cut everything that looks extra" cuts a loop at its join, which leaves two fed spurs that take time to pull in. Worse, it cuts the one tube that is the only route to a food.
- **Intended solve:** cut loops in the middle, cut strays at their base, and use the lamp to straighten what's left. Watch each cut part go grey and shrink. The graded death is the feedback.
- **Code:** a `Start::Grown { secs }` option, which runs the existing `Plan` bot for 20 s of play at dish creation. That is 2,400 steps at about 1.3 ms, roughly 3 s, shown as a fast-forward "overnight" with the dish visible. Win = all joined and ratio under the bar. **About 0.5 day. Needs a probe run first** to check that cut spurs actually pull in within the time limit. Jones networks do retract dead ends, but no one has timed it here.

### 5. Storm Season *(medium, 120 s)*

- **Goal:** "KEEP ALL FIVE FOODS JOINED THROUGH THREE STORMS."
- **Twist:** storms at 0:45, 1:15 and 1:45. Each one is aimed at your worst tube with the 5 s purple warning, and every scar stays. Five foods sit in a loose ring.
- **Doing nothing fails:** it never joins five.
- **A random hand fails:** it builds a tree at best, and each storm takes a branch.
- **The obvious move fails:** join each food once, which is the habit dish 3 just rewarded. A tree loses a food at every storm. **This dish deliberately turns dish 3's lesson around:** loops cost tube, and loops are what survive.
- **Intended solve:** a ring, then answer each warning with a bypass (an oat beside the purple line), so that the next storm finds no single weak tube. Graded: one star per storm survived with nothing lost.
- **Code:** `storm_at: u64` becomes a schedule (`Vec<u64>`). After each storm's watch, `phase_tick` goes from `Storm` back to `Growing` instead of `Over`, with a running tally of foods lost per storm. **About 0.5 day. Measure first:** three cuts at once were never survived (0 of 20), but spaced cuts with regrowth between them have never been tried. If no hand survives two, drop to two storms.

### 6. Running Dry *(medium-hard, 120 s)*

```
 0:00  H*  a*              (* = small pile, ~35 s of food)
 0:20            b*        (dotted ring 5 s before each arrives)
 0:40                 c*
 1:00          d*          ... each new pile further on
```

- **Goal:** "KEEP THE MOULD ALIVE. THE FOOD WON'T LAST."
- **Twist:** small piles. Home and two others hold 5,000 food each, about 35 s at a full table. A new small pile arrives every 20 s, each 60-90 cells from the last, announced with the existing dotted ring. The score is graded: mould alive at the bell, with stars at 300, 600 and 900 walkers (one, two or three fed piles' worth).
- **Doing nothing fails:** the mould eats its neighbourhood bare by about 0:50, goes grey all over, and is gone by 1:10. It never reaches the later piles.
- **A random hand fails:** oats buy seconds but go nowhere.
- **The obvious move fails:** "join everything near you" makes a big, happy mould sitting on food that is about to run out. When the piles empty together, it has no front pointed at the next arrival.
- **Intended solve:** always have a finger heading for the next dotted ring before the current pile empties. Snip the long dead tubes behind you, since unfed walkers still cost upkeep from the store. The picture is the reward: a mould that walks across the dish, a slowly greying trail behind it, the front always yellow.
- **Code:** per-pile food in the level (`PILE_FOOD` becomes a field on each pile spec). Use the existing `pending` arrivals list with level-given times. `Win::AliveAtBell { bars }` counts `walkers.len()` at the end. **About 0.5 day. Measure first:** whether the plan bot, pointed at the next ring, survives. If it doesn't, lengthen the piles' life.

### 7. Feed the Sprout *(hard, 120 s)*

```
 H      c1          c3
 @----.  *        *  .---- ?      S = sprout (eats, gives nothing)
       '-.  c2 .-'          S     c = crumbs, off the straight line
           '-*-'
```

- **Goal:** "KEEP THE SPROUT FED FOR 30 SECONDS."
- **Twist:** the target is a sink. Home sits far left, the sprout far right, 190 cells apart. The sprout smells like food and counts as fed while fed tube touches it, but it feeds nothing. So it must sit within 70 cells of tube from real food the whole time. Three small crumb piles lie above and below the straight line.
- **Doing nothing fails:** the sprout is nearly three leashes away.
- **A random hand fails:** nothing random holds a 190-cell tube.
- **The obvious move fails:** a straight line of oats reaches the sprout, but each oat is eaten in a few seconds. The far half goes grey and the sprout wilts. **The shortest tube is the wrong one.**
- **Intended solve:** zigzag through the crumbs. That makes a longer network, but every stretch is fed from both ends, so the sprout stays green. The sprout grows while fed and wilts while unfed: a graded meter in the world, not in the HUD.
- **Code:** a `PileKind::Sprout` that gives off scent, isn't a feeding source in `census`, and doesn't pay the store in `economy` (about 30 lines). A fed-seconds counter, read from `fed` at the sprout's rim. A grow/wilt ring in `render.rs`. **About 0.5-1 day.**

### 8. The Rival *(hard, stretch: 2-3 days, build last)*

- **Goal:** "HOLD MORE FOOD THAN THE BLUE MOULD AT THE BELL."
- **Twist:** a second mould, played by the existing `Plan` bot with its own oats, starting in the opposite corner.
- **Why plain play fails:** the rival is a decent planner. Doing nothing loses on food count, and racing it pile for pile is a coin toss.
- **Intended solve:** use salt to wall the rival off from the middle foods, and grab the contested pile first.
- **Code:** a team tag on walkers, a second trail field so each team follows its own trail, a per-team feeding search and table, and a second colour. That roughly doubles the field work (sim 1.3 ms → about 2.2 ms a frame, still inside budget). It is the only dish here that touches the core step, so it goes last.

---

## The shared code (what makes all of this "days")

**`src/mould/level.rs` (new, about 200 lines):**

```rust
pub struct Level {
    pub name: &'static str,
    pub goal: [&'static str; 2],      // the on-screen sentence, ~40 chars a line
    pub piles: Vec<PileSpec>,         // x, y, r, food, kind (Food | Sprout), arrives: Option<secs>
    pub salt: Vec<Segment>,           // pre-laid walls, drawn with the existing stroke_dab
    pub tools: Tools,                 // oats, oats_regrow: Option<secs>, salt, snips
    pub seconds: u32,
    pub storms: Vec<u32>,             // seconds; empty = none
    pub start: Start,                 // Fresh | Grown { secs }
    pub win: Win,                     // JoinAll{by:[u32;3]} | UnderRuler{bars} | KeepJoined
                                      // | AliveAtBell{bars} | SproutFed{secs}
}
pub fn run() -> Vec<Level>            // the eight dishes, hand-placed, no seed jitter
```

**Changes in `mod.rs`:**
- `Dish::from_level(&Level)`: the inoculum and grid code is shared with `with_params`. Seeded dishes stay as they are (free play).
- Make the tool stock constants into fields.
- `storm_at` becomes a schedule.
- `phase_tick` scores through `match level.win`.
- **About 150 lines changed, 0.5 day.**

Every pile in a level is placed by hand with no seed jitter. A dish should be a puzzle you can learn, not a dice roll.

**`mouldprobe level=N`:** runs do nothing, random and the plan on each level, plus one scripted "intended" hand per dish, printing stars. **The bar for shipping a dish: do nothing scores 0, random scores 1 at most, the intended hand scores 3.** This is the same test the current dish passes. Set every star bar from this run with headroom, never by guess. About 0.5 day for the harness, then one run per dish.

**After-dish card, one line of why:** the owner said "I didn't really know what I was doing." The score card must say *why* in world words, computed from the dish. For example: "THE STORM CUT YOUR ONLY TUBE TO THE TOP FOOD. A LOOP WOULD HAVE HELD." Or "YOUR LAST OAT WAS TOO FAR AHEAD TO SMELL." Or "TUBE 1.04 x RULER: TRY MEETING IN THE MIDDLE." One message per win rule, picked by the failed condition. **About 0.5 day.** This is worth more than any single dish.

**Total for the top three plus the frame:** about 3 days (struct, harness, why-card, three dishes, select screen). Dishes 2, 4 and 5 add about half a day each, dishes 6 and 7 about a day, and the Rival 2-3 days.

---

## The dish select screen

- **One screen of nine cards** in a 3x3 grid that reads on a phone: Lesson, then dishes 1-8. Each card shows the dish name, its goal sentence, and best stars (three dots). It is drawn with the existing `panel` and `text` helpers in `render.rs`.
- **Unlocking:** the next dish opens when you earn at least one star on the one before. The lesson counts as done when it is finished. With 8 dishes, unlocking everything at once would bring back the "what am I meant to do?" problem. A locked card still shows its name and goal, so the player can see where the run is going.
- **Keys:** M for the menu at any time, number keys or a click to pick, R to retry, N for the next dish. The free-play seeded dishes stay behind a ninth "FREE DISH" option, so nothing is lost.
- **Before each dish:** the briefing card shows only the goal, the one twist line, and that dish's tools. **After it:** stars, the graded number (time, ratio, fed seconds or mould alive), and the one-line why.
- **Saving:** best stars per dish go in a small text file beside the executable (`mould-progress.txt`, one line per dish). It is read at start, and a missing file means a fresh run. That is about 30 lines in `src/bin/mould.rs`.

**Why these three first.**
- **Behind the Wall** is mostly the level format that everything else needs. It is also the first "you did it" a newcomer gets.
- **Beat the Ruler** is the owner's own pleasure, the bridge where there was no food, turned into the goal. It is the only dish where an idea beats effort.
- **Running Dry** makes the middle minute busy by design rather than by patching. It shows the graded grey die-back the house rules ask for.

Together they ask three different questions: where can the mould go, what shape should it take, and when should it move. Storm Season is the obvious fourth: it is cheap and it turns dish 3's lesson around.
