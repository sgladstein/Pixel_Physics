//! **The satchel seat's demo** (`demo=satchel`, 2026-10-31): the player is a
//! hand over the box with an ordered satchel of cards, and **the colony's own
//! deaths can set the cards off**. The design is the Noita wand page and its
//! two reviews (`/mnt/project-files/game-design/noita-wands/`: the page, the
//! code-checking `review.md` and the explorer's `notch-up.md`), merged in the
//! "Satchel seat plan" doc. Borrowed as principles, not copied: cards are
//! dealt in slot order, a modifier changes the next card and deals a
//! replacement, a multicast deals two, and a trigger carries the next card as
//! its payload.
//!
//! **The lab's own twist is the trigger: the colony pulls it.** `WAKE` arms
//! its payload where you click; the first animal that dies within
//! [`BIND_R`] of it fires the payload on the body. Every effect a chain lays
//! down leaves a *zone* behind it for [`ZONE_LIFE`] frames, and a death
//! inside a zone fires the chain's payload again, one level deeper. A fire on
//! a corpse burns the ants beside it, their deaths light more fires, and the
//! chain feeds itself.
//!
//! **One visible rule stops it**, the lab's recursion limit: at depth
//! [`MAX_DEPTH`] the next death only puffs grey dust, and one chain lays at
//! most [`CHAIN_CAP`] effects. Past [`BUDGET`] effects in a [`BUDGET_EVERY`]
//! window the rest wait in a queue and are never dropped, so the budget
//! bounds work and never decides what happens (`CLAUDE.md`'s size-cap rule).
//!
//! **Why fire and soak, and not water** (the review, `review.md` high 3):
//! water drowns nothing in this engine -- there is no drowning death cause,
//! ants walk on water and brood dies only when its cell is overwritten. What
//! water *does* is fill soil's pore space until a packed gallery wall slumps
//! (`update.rs`'s `self_supporting` branch, `slumps_into`). So `SOAK` is a
//! cave-in charge: it saturates every water-holding cell in its disc, and the
//! galleries under it come down by the ordinary rules. Fire is the lab
//! fire tool's rule (`Lab::fire_at`): only what can burn catches, ants and
//! spiders included.
//!
//! **Why a spider on a clock**: a verb needs a reason. A spider lands at the
//! far end every [`Rules::raider_every`] frames and hunts the colony; your
//! cards are how you stop it, and your own chains are the colony's biggest
//! danger. **Favour** pays for cards: it regrows on a clock and every ant
//! hatched pays [`Rules::favour_per_hatch`], so killing your own colony also
//! starves your hand. Not food brought home: the hand-built ant books almost
//! none (`keeper.rs`'s stakes notes), so a delivery currency would read 0 on
//! the ant the fun hunt plays.
//!
//! **The ant**: the hand-built one unless `ant=trained` (the fun hunt's
//! decision 3, 2026-10-30). The satchel is repacked only **while paused**
//! (`Y` steps the presets), Noita's "edit at the station, not in the fight".
//!
//! Off is off: with no `demo=satchel` nothing here runs.

use std::collections::VecDeque;

use crate::sim::material;
use crate::sim::world::World;

use super::super::Lab;

/// The satchel seat's own keys, as the help line prints them.
pub const SEAT_KEYS: &str = "CLICK PLAYS THE NEXT CARD, Y REPACKS (PAUSED)";

/// Most cards a satchel holds.
pub const SLOTS: usize = 5;
/// The depth at which a chain stops: a death caused at this depth puffs.
pub const MAX_DEPTH: u8 = 3;
/// Most effects one chain lays down, all depths together.
pub const CHAIN_CAP: u32 = 24;
/// Effects run per window; the rest wait (never dropped).
pub const BUDGET: u32 = 8;
/// The budget's window, in frames (one second at 60 fps).
pub const BUDGET_EVERY: u64 = 60;
/// How far from an armed `WAKE` a death sets it off.
pub const BIND_R: i32 = 30;
/// How long an effect's zone listens for deaths it caused. A burning ant
/// takes its `burn_duration` (40) and more to die, and a cave-in buries
/// slowly, so the zone outlives both.
pub const ZONE_LIFE: u64 = 600;
/// How far past an effect's own radius its zone reaches: an ant that ran
/// burning out of the disc still counts as the fire's.
pub const ZONE_PAD: i32 = 8;
/// How long a ring stays on screen.
const RING_LIFE: u64 = 90;
/// How far from the nest door raiders land, in columns.
pub const RAIDER_GAP: i32 = 90;
/// What a raider killed by your cards adds to the score.
pub const RAIDER_SCORE: u32 = 10;
/// Most armed `WAKE`s at once.
pub const MAX_ARMED: usize = 3;

/// One card in a satchel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Card {
    /// Lights everything that can burn in a disc.
    Fire,
    /// Saturates every water-holding cell in a disc: the galleries under it
    /// slump.
    Soak,
    /// The next card is twice the size (stacks).
    Wide,
    /// Deals the next two cards at once.
    Twin,
    /// Carries the next card as a payload; the first death near it fires it,
    /// and deaths it causes fire it again, one level deeper.
    Wake,
}

impl Card {
    pub fn name(self) -> &'static str {
        match self {
            Card::Fire => "FIRE",
            Card::Soak => "SOAK",
            Card::Wide => "WIDE",
            Card::Twin => "TWIN",
            Card::Wake => "WAKE",
        }
    }

    /// Favour it costs to deal.
    pub fn cost(self) -> u32 {
        match self {
            Card::Fire => 6,
            Card::Soak => 4,
            Card::Wide => 3,
            Card::Twin => 2,
            Card::Wake => 4,
        }
    }

    /// Frames it adds to the wait before the next play.
    pub fn delay(self) -> u64 {
        match self {
            Card::Fire | Card::Soak => 30,
            Card::Wide => 10,
            Card::Twin => 0,
            Card::Wake => 20,
        }
    }

    /// Its chip colour on the HUD, by kind: effects warm, modifiers violet,
    /// the multicast blue, the trigger green.
    pub fn colour(self) -> [u8; 4] {
        match self {
            Card::Fire => [255, 120, 60, 255],
            Card::Soak => [90, 160, 255, 255],
            Card::Wide => [190, 150, 255, 255],
            Card::Twin => [120, 200, 255, 255],
            Card::Wake => [110, 230, 160, 255],
        }
    }
}

/// What a play lays down, before it is placed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Planned {
    /// A fire of this radius.
    Fire(i32),
    /// A soak of this radius.
    Soak(i32),
    /// An armed trigger carrying these.
    Wake(Vec<Planned>),
}

pub const FIRE_R: i32 = 4;
pub const SOAK_R: i32 = 5;
/// The widest any card gets, however many `WIDE`s are stacked on it.
pub const MAX_R: i32 = 16;

/// One play's worth of cards, dealt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Deal {
    pub planned: Vec<Planned>,
    pub cost: u32,
    pub delay: u64,
    /// The slot the next play starts from.
    pub next: usize,
    /// It ran off the end of the satchel: a reload follows.
    pub wrapped: bool,
    /// Modifiers with nothing after them to change.
    pub fizzled: u32,
}

/// **Deal one play off `deck` from slot `pos`**: one draw, as Noita's wand
/// with one spell per cast. A modifier changes what it is drawn into and
/// draws a replacement, so it never wastes the play; `TWIN` draws two;
/// `WAKE` draws one into its payload. Running past the last slot wraps once
/// and flags the reload; a second wrap in one play stops it, so a satchel of
/// nothing but `TWIN`s ends.
pub fn deal(deck: &[Card], pos: usize) -> Deal {
    struct St<'a> {
        deck: &'a [Card],
        p: usize,
        wrapped: bool,
        used: usize,
        cost: u32,
        delay: u64,
        fizzled: u32,
    }
    fn next(s: &mut St) -> Option<Card> {
        if s.deck.is_empty() || s.used >= 2 * s.deck.len() {
            return None;
        }
        if s.p >= s.deck.len() {
            if s.wrapped {
                return None;
            }
            s.wrapped = true;
            s.p = 0;
        }
        let c = s.deck[s.p];
        s.p += 1;
        s.used += 1;
        s.cost += c.cost();
        s.delay += c.delay();
        Some(c)
    }
    fn one(s: &mut St, wide: u32) -> Vec<Planned> {
        let Some(c) = next(s) else {
            if wide > 0 {
                s.fizzled += 1;
            }
            return Vec::new();
        };
        let r = |base: i32| (base << wide.min(2)).min(MAX_R);
        match c {
            Card::Fire => vec![Planned::Fire(r(FIRE_R))],
            Card::Soak => vec![Planned::Soak(r(SOAK_R))],
            Card::Wide => one(s, wide + 1),
            Card::Twin => {
                let mut v = one(s, wide);
                v.extend(one(s, wide));
                v
            }
            Card::Wake => vec![Planned::Wake(one(s, wide))],
        }
    }
    let mut s = St { deck, p: pos.min(deck.len()), wrapped: false, used: 0, cost: 0, delay: 0, fizzled: 0 };
    let planned = one(&mut s, 0);
    let wrapped = s.wrapped || s.p >= deck.len();
    Deal { planned, cost: s.cost, delay: s.delay, next: if wrapped { 0 } else { s.p }, wrapped, fizzled: s.fizzled }
}

/// The ready-made satchels `Y` steps through while paused.
pub const PRESETS: [(&str, &[Card]); 4] = [
    ("WILDFIRE", &[Card::Wake, Card::Fire, Card::Wide, Card::Fire]),
    ("CAVE-IN", &[Card::Wake, Card::Soak, Card::Twin, Card::Fire, Card::Soak]),
    ("BONFIRE", &[Card::Twin, Card::Wake, Card::Wide, Card::Fire, Card::Fire]),
    ("PLAIN", &[Card::Fire, Card::Soak, Card::Fire]),
];

/// The seat's tunables, each a `key=value` on the command line.
#[derive(Clone, Copy, Debug)]
pub struct Rules {
    /// Frames from the opening until the round is won.
    pub round: u64,
    /// The colony under this many ants loses the round.
    pub lose_at: usize,
    pub favour_max: u32,
    pub favour_start: u32,
    /// One favour back every this many frames.
    pub favour_every: u64,
    pub favour_per_hatch: u32,
    /// The wait after a play that ran off the end of the satchel.
    pub reload: u64,
    /// A spider lands every this many frames (0: none).
    pub raider_every: u64,
    /// The first spider lands this long after the opening.
    pub raider_first: u64,
    /// What lands: `spider` (the lone predator) or `centipede` (the nest
    /// raider), by `raider=`.
    pub raider: &'static str,
    /// Wave `n` lands `n` raiders, up to this many: the pressure grows.
    pub wave_cap: u32,
    /// Frames run before the round opens (the colony lands at 6,000 on the
    /// played bed and needs a while to dig in).
    pub warmup: u64,
}

impl Default for Rules {
    fn default() -> Rules {
        Rules {
            round: 18_000,
            lose_at: 5,
            favour_max: 30,
            favour_start: 20,
            favour_every: 90,
            favour_per_hatch: 2,
            reload: 240,
            raider_every: 3_000,
            raider: "spider",
            wave_cap: 4,
            raider_first: 900,
            warmup: 9_000,
        }
    }
}

impl Rules {
    fn from_args(args: &std::collections::BTreeMap<String, String>) -> Rules {
        let mut r = Rules::default();
        let get = |k: &str| args.get(k).and_then(|v| v.parse::<u64>().ok());
        if let Some(v) = get("round") {
            r.round = v;
        }
        if let Some(v) = get("lose_at") {
            r.lose_at = v as usize;
        }
        if let Some(v) = get("favour") {
            r.favour_start = v as u32;
            r.favour_max = r.favour_max.max(v as u32);
        }
        if let Some(v) = args.get("raider") {
            r.raider = if v == "centipede" { "centipede" } else { "spider" };
        }
        if let Some(v) = get("wave_cap") {
            r.wave_cap = v as u32;
        }
        if let Some(v) = get("raider_every") {
            r.raider_every = v;
        }
        if let Some(v) = get("warmup") {
            r.warmup = v;
        }
        r
    }
}

/// An armed `WAKE` waiting for a death.
#[derive(Clone, Debug)]
pub struct Armed {
    pub at: (i32, i32),
    pub payload: Vec<Planned>,
    /// The chain it continues, if a chain armed it; else a new chain starts.
    pub chain: Option<usize>,
    pub depth: u8,
}

/// Where an effect listens for the deaths it caused.
#[derive(Clone, Copy, Debug)]
pub struct Zone {
    pub at: (i32, i32),
    pub r: i32,
    pub chain: Option<usize>,
    pub depth: u8,
    pub until: u64,
}

/// One chain: the payload every death in it fires again.
#[derive(Clone, Debug)]
pub struct Chain {
    pub payload: Vec<Planned>,
    pub effects: u32,
    pub deepest: u8,
    pub puffs: u32,
}

/// An effect waiting for the budget.
#[derive(Clone, Debug)]
pub struct Pending {
    pub at: (i32, i32),
    pub what: Planned,
    pub chain: Option<usize>,
    pub depth: u8,
}

/// A mark on the world: an effect (by depth) or a puff (grey).
#[derive(Clone, Copy, Debug)]
pub struct Ring {
    pub at: (i32, i32),
    pub r: i32,
    pub depth: u8,
    pub puff: bool,
    pub born: u64,
}

/// What the round has done, for the HUD and the page.
#[derive(Clone, Copy, Debug, Default)]
pub struct Counts {
    pub plays: u32,
    pub refused: u32,
    pub effects: u32,
    pub chains: u32,
    pub deepest: u8,
    pub puffs: u32,
    /// Ants that died inside a zone: killed by your cards.
    pub card_kills: u32,
    /// Ants that died anywhere else.
    pub natural: u32,
    pub raiders_in: u32,
    pub waves: u32,
    pub raiders_killed: u32,
    /// Raiders that died inside one of your zones.
    pub raiders_by_cards: u32,
    /// **The score**: every effect a chain lays is worth its depth (a deep
    /// one is worth three shallow), and a raider your cards kill is worth
    /// [`RAIDER_SCORE`]. Ants your cards kill cost nothing on the score:
    /// they cost the colony, which is how you lose.
    pub score: u32,
    pub hatched: u32,
    pub cells_burnt: u32,
    pub cells_soaked: u32,
    pub min_ants: usize,
}

/// How the round ended.
#[derive(Clone, Copy, Debug)]
pub struct RoundEnd {
    pub frame: u64,
    pub won: bool,
    pub ants: usize,
}

#[derive(Clone, Debug)]
pub struct SatchelDemo {
    pub rules: Rules,
    pub preset: usize,
    pub deck: Vec<Card>,
    pub pos: usize,
    pub favour: u32,
    /// No play before this frame (cast delay, or the reload).
    pub ready_at: u64,
    pub reloading: bool,
    pub armed: Vec<Armed>,
    pub zones: Vec<Zone>,
    pub chains: Vec<Chain>,
    pub queue: VecDeque<Pending>,
    pub budget: u32,
    pub rings: Vec<Ring>,
    pub counts: Counts,
    pub opened: Option<u64>,
    pub ended: Option<RoundEnd>,
    pub warming: bool,
    pub ants_at_open: usize,
    last_ants: usize,
    ant_deaths_since: usize,
    last_frame: u64,
}

impl Default for SatchelDemo {
    fn default() -> SatchelDemo {
        SatchelDemo::with_rules(Rules::default())
    }
}

impl SatchelDemo {
    pub fn with_rules(rules: Rules) -> SatchelDemo {
        SatchelDemo {
            rules,
            preset: 0,
            deck: PRESETS[0].1.to_vec(),
            pos: 0,
            favour: rules.favour_start,
            ready_at: 0,
            reloading: false,
            armed: Vec::new(),
            zones: Vec::new(),
            chains: Vec::new(),
            queue: VecDeque::new(),
            budget: BUDGET,
            rings: Vec::new(),
            counts: Counts::default(),
            opened: None,
            ended: None,
            warming: false,
            ants_at_open: 0,
            last_ants: 0,
            ant_deaths_since: 0,
            last_frame: 0,
        }
    }

    pub fn from_args(args: &std::collections::BTreeMap<String, String>) -> SatchelDemo {
        let mut s = SatchelDemo::with_rules(Rules::from_args(args));
        if let Some(name) = args.get("satchel") {
            if let Some(i) = PRESETS.iter().position(|(n, _)| n.eq_ignore_ascii_case(name)) {
                s.preset = i;
                s.deck = PRESETS[i].1.to_vec();
            }
        }
        s
    }

    /// Step to the next preset satchel, from its first slot.
    pub fn repack(&mut self) {
        self.preset = (self.preset + 1) % PRESETS.len();
        self.deck = PRESETS[self.preset].1.to_vec();
        self.pos = 0;
    }
}

fn satchel(lab: &Lab) -> Option<&SatchelDemo> {
    lab.demo.as_ref()?.satchel.as_ref()
}

fn satchel_mut(lab: &mut Lab) -> Option<&mut SatchelDemo> {
    lab.demo.as_mut()?.satchel.as_mut()
}

pub fn live_of(world: &World, species: &str) -> usize {
    let Some(sp) = world.species.id_of(species) else { return 0 };
    world.live_organism_ids().into_iter().filter(|&id| world.organism(id).is_some_and(|s| s.species == sp)).count()
}

/// **After the scenario loads**: the hand-built ant unless `ant=trained`, the
/// death log on, then the warm-up with nobody watching, then the round.
pub fn on_start(lab: &mut Lab) {
    let trained = lab.demo.as_ref().and_then(|d| d.args.get("ant")).is_some_and(|v| v == "trained");
    if !trained && lab.new_ant {
        lab.set_new_ant(false);
    }
    lab.world.death_log = Some(Vec::new());
    let warmup = satchel(lab).map_or(0, |s| s.rules.warmup);
    if let Some(s) = satchel_mut(lab) {
        s.warming = true;
    }
    for _ in 0..warmup {
        lab.tick_for_harness();
    }
    open_round(lab);
}

fn open_round(lab: &mut Lab) {
    let frame = lab.world.frame;
    let ants = live_of(&lab.world, "ant");
    lab.world.death_log = Some(Vec::new());
    if let Some(s) = satchel_mut(lab) {
        let (rules, preset, deck) = (s.rules, s.preset, s.deck.clone());
        *s = SatchelDemo::with_rules(rules);
        s.preset = preset;
        s.deck = deck;
        s.opened = Some(frame);
        s.ants_at_open = ants;
        s.last_ants = ants;
        s.counts.min_ants = ants;
        s.last_frame = frame;
    }
}

/// `Y`: repack, only while paused.
pub fn seat_key(lab: &mut Lab, key: char) {
    if key != 'Y' {
        return;
    }
    if lab.time.multiple() != 0 {
        lab.ui.say("PAUSE (SPACE) TO REPACK THE SATCHEL".to_string());
        return;
    }
    if let Some(s) = satchel_mut(lab) {
        s.repack();
        let names: Vec<_> = s.deck.iter().map(|c| c.name()).collect();
        let line = format!("SATCHEL: {} -- {}", PRESETS[s.preset].0, names.join(" "));
        lab.world.log_player_action(line.clone());
        lab.ui.say(line);
    }
}

/// A click on the box plays the next card there.
pub fn mouse(lab: &mut Lab, cell: (i32, i32), act: super::MouseAct) -> bool {
    if satchel(lab).is_none() {
        return false;
    }
    if !matches!(act, super::MouseAct::Press { erase: false }) {
        return matches!(act, super::MouseAct::Drag | super::MouseAct::Release) && lab.demo.as_ref().is_some_and(|d| d.mouse_held);
    }
    let msg = play_at(lab, cell);
    lab.ui.say(msg);
    true
}

/// **Play the next card at `cell`**: deal it, pay for it, and lay it down.
/// Returns the line the HUD says. Public for the tests and the harness.
pub fn play_at(lab: &mut Lab, cell: (i32, i32)) -> String {
    let frame = lab.world.frame;
    let Some(s) = satchel_mut(lab) else { return String::new() };
    if s.ended.is_some() {
        return "THE ROUND IS OVER -- H FOR THE CARD".to_string();
    }
    if frame < s.ready_at {
        s.counts.refused += 1;
        let what = if s.reloading { "RELOADING" } else { "NOT READY" };
        return format!("{what} -- {:.1}S", (s.ready_at - frame) as f32 / 60.0);
    }
    let d = deal(&s.deck, s.pos);
    if d.cost > s.favour {
        s.counts.refused += 1;
        return format!("NOT ENOUGH FAVOUR: THIS PLAY COSTS {}, YOU HAVE {}", d.cost, s.favour);
    }
    s.favour -= d.cost;
    s.pos = d.next;
    s.reloading = d.wrapped;
    s.ready_at = frame + if d.wrapped { d.delay.max(s.rules.reload) } else { d.delay };
    s.counts.plays += 1;
    for p in &d.planned {
        s.queue.push_back(Pending { at: cell, what: p.clone(), chain: None, depth: 0 });
    }
    let line = format!("PLAYED {} AT {},{} -- {} FAVOUR{}", describe(&d.planned), cell.0, cell.1, d.cost, if d.wrapped { " -- RELOAD" } else { "" });
    lab.world.log_player_action(line.clone());
    run_queue(lab);
    line
}

/// A play in words, for the HUD and the chronicle.
pub fn describe(planned: &[Planned]) -> String {
    let words: Vec<String> = planned
        .iter()
        .map(|p| match p {
            Planned::Fire(r) => format!("FIRE R{r}"),
            Planned::Soak(r) => format!("SOAK R{r}"),
            Planned::Wake(inner) => format!("WAKE[{}]", describe(inner)),
        })
        .collect();
    if words.is_empty() {
        "NOTHING".to_string()
    } else {
        words.join(" + ")
    }
}

/// Run queued effects while the budget lasts.
fn run_queue(lab: &mut Lab) {
    loop {
        let Some(s) = satchel_mut(lab) else { return };
        if s.budget == 0 {
            return;
        }
        let Some(p) = s.queue.pop_front() else { return };
        s.budget -= 1;
        apply(lab, p);
    }
}

/// Lay one effect down.
fn apply(lab: &mut Lab, p: Pending) {
    let frame = lab.world.frame;
    match p.what {
        Planned::Wake(payload) => {
            if let Some(s) = satchel_mut(lab) {
                if s.armed.len() >= MAX_ARMED {
                    s.armed.remove(0);
                }
                s.armed.push(Armed { at: p.at, payload, chain: p.chain, depth: p.depth });
                s.rings.push(Ring { at: p.at, r: 3, depth: p.depth, puff: false, born: frame });
            }
        }
        Planned::Fire(r) | Planned::Soak(r) => {
            let fire = matches!(p.what, Planned::Fire(_));
            if !satchel_mut(lab).is_some_and(|s| admit(s, &p, r, frame)) {
                return;
            }
            let n = if fire { burn(&mut lab.world, p.at, r) } else { soak(&mut lab.world, p.at, r) };
            if let Some(s) = satchel_mut(lab) {
                if fire {
                    s.counts.cells_burnt += n;
                } else {
                    s.counts.cells_soaked += n;
                }
            }
            lab.world.log_player_action(format!("SATCHEL {} R{r} AT {},{} DEPTH {} -- {n} CELLS", if fire { "FIRE" } else { "SOAK" }, p.at.0, p.at.1, p.depth));
        }
    }
}

/// **The book-keeping half of laying an effect**, apart from the world so
/// the caps are testable: a chain already at [`CHAIN_CAP`] puffs instead
/// (the cap is checked here as well as when a death queues the payload,
/// because one death can queue several effects at once); otherwise the
/// effect is counted, and leaves its zone and its ring.
fn admit(s: &mut SatchelDemo, p: &Pending, r: i32, frame: u64) -> bool {
    if let Some(c) = p.chain.and_then(|c| s.chains.get_mut(c)) {
        if c.effects >= CHAIN_CAP {
            c.puffs += 1;
            s.counts.puffs += 1;
            s.rings.push(Ring { at: p.at, r: 2, depth: p.depth, puff: true, born: frame });
            return false;
        }
        c.effects += 1;
        c.deepest = c.deepest.max(p.depth);
        s.counts.score += u32::from(p.depth);
    }
    s.counts.effects += 1;
    s.counts.deepest = s.counts.deepest.max(p.depth);
    s.zones.push(Zone { at: p.at, r: r + ZONE_PAD, chain: p.chain, depth: p.depth, until: frame + ZONE_LIFE });
    s.rings.push(Ring { at: p.at, r, depth: p.depth, puff: false, born: frame });
    true
}

/// **Light everything that can burn in the disc** -- the lab fire tool's
/// rule (`Lab::fire_at`) at a card's radius: bare soil and stone refuse,
/// plants, litter and animals catch, at their own flame temperature.
pub fn burn(world: &mut World, at: (i32, i32), r: i32) -> u32 {
    let mut lit = 0;
    for y in (at.1 - r)..=(at.1 + r) {
        for x in (at.0 - r)..=(at.0 + r) {
            if (x - at.0).pow(2) + (y - at.1).pow(2) > r * r || !world.in_bounds(x, y) {
                continue;
            }
            let mut cell = world.get(x, y);
            if cell.material == material::EMPTY || cell.is_burning() {
                continue;
            }
            let m = world.materials.get(cell.material);
            if m.flammability <= 0.0 || m.burn_duration == 0 {
                continue;
            }
            let hot = m.burn_temperature;
            cell.ignite(m.burn_duration);
            if hot.is_finite() {
                cell.set_temperature(hot.round() as i16);
            }
            world.set(x, y, cell);
            lit += 1;
        }
    }
    lit
}

/// **Fill every water-holding cell in the disc to its capacity.** Soil and
/// packed soil hold water in `aux`; a packed wall past field capacity
/// slumps into loose soil and the gallery under it comes in by the ordinary
/// fall rules, so the cave-in is graded by how much wall the disc reached.
pub fn soak(world: &mut World, at: (i32, i32), r: i32) -> u32 {
    let mut wet = 0;
    for y in (at.1 - r)..=(at.1 + r) {
        for x in (at.0 - r)..=(at.0 + r) {
            if (x - at.0).pow(2) + (y - at.1).pow(2) > r * r || !world.in_bounds(x, y) {
                continue;
            }
            let cell = world.get(x, y);
            if cell.material == material::EMPTY {
                continue;
            }
            let cap = world.materials.get(cell.material).water_capacity;
            if cap == 0 || cell.aux() >= cap {
                continue;
            }
            world.set(x, y, cell.with_aux(cap));
            wet += 1;
        }
    }
    wet
}

pub fn frame(_lab: &mut Lab) {}

/// After each tick: deaths fire armed cards and zones, the queue runs, the
/// favour and the spiders keep their clocks, and the round is judged.
pub fn after_tick(lab: &mut Lab) {
    let frame = lab.world.frame;
    let Some(s) = satchel(lab) else { return };
    // The warm-up runs with nobody watching: its deaths are nobody's.
    if s.warming {
        lab.world.death_log.get_or_insert_with(Vec::new).clear();
        return;
    }
    // A rebuilt box (`Lab::reset` makes a new world, whose frame restarts).
    if frame < s.last_frame || s.opened.is_none() {
        open_round(lab);
        return;
    }
    let ant = lab.world.species.id_of("ant");
    let spider = satchel(lab).and_then(|s| lab.world.species.id_of(s.rules.raider));
    let deaths = lab.world.death_log.get_or_insert_with(Vec::new).drain(..).filter(|d| d.creature).collect::<Vec<_>>();
    let Some(mut s) = lab.demo.as_mut().and_then(|d| d.satchel.take()) else { return };
    s.last_frame = frame;
    if frame.is_multiple_of(BUDGET_EVERY) {
        s.budget = BUDGET;
    }
    s.zones.retain(|z| z.until > frame);
    s.rings.retain(|r| frame.saturating_sub(r.born) < RING_LIFE);
    for d in &deaths {
        let is_ant = Some(d.species) == ant;
        if Some(d.species) == spider {
            s.counts.raiders_killed += 1;
        }
        if is_ant {
            s.ant_deaths_since += 1;
        }
        on_death(&mut s, d.at, is_ant, Some(d.species) == spider, frame);
    }
    let playing = s.ended.is_none();
    if playing && frame.is_multiple_of(s.rules.favour_every) {
        s.favour = (s.favour + 1).min(s.rules.favour_max);
    }
    if frame.is_multiple_of(30) {
        let ants = live_of(&lab.world, "ant");
        let hatched = (ants + s.ant_deaths_since).saturating_sub(s.last_ants);
        if hatched > 0 && playing {
            s.counts.hatched += hatched as u32;
            s.favour = (s.favour + s.rules.favour_per_hatch * hatched as u32).min(s.rules.favour_max);
        }
        s.last_ants = ants;
        s.ant_deaths_since = 0;
        s.counts.min_ants = s.counts.min_ants.min(ants);
        if playing {
            let opened = s.opened.unwrap_or(frame);
            if ants < s.rules.lose_at {
                s.ended = Some(RoundEnd { frame, won: false, ants });
            } else if frame >= opened + s.rules.round {
                s.ended = Some(RoundEnd { frame, won: true, ants });
            }
            if let Some(e) = s.ended {
                lab.world.log_player_action(format!("SATCHEL ROUND {} AT FRAME {} -- {} ANTS", if e.won { "WON" } else { "LOST" }, e.frame, e.ants));
            }
        }
    }
    let raider = playing
        && s.rules.raider_every > 0
        && s.opened.is_some_and(|o| frame >= o + s.rules.raider_first && (frame - o - s.rules.raider_first).is_multiple_of(s.rules.raider_every));
    if let Some(d) = lab.demo.as_mut() {
        d.satchel = Some(s);
    }
    if raider {
        land_raider(lab);
    }
    run_queue(lab);
}

/// **One death**: inside a chain's zone it fires the chain again one level
/// deeper (or puffs at the limit); inside any zone it is a card kill; near
/// an armed `WAKE` it starts a chain.
fn on_death(s: &mut SatchelDemo, at: (i32, i32), is_ant: bool, is_raider: bool, frame: u64) {
    let inside = |z: &Zone| (z.at.0 - at.0).pow(2) + (z.at.1 - at.1).pow(2) <= z.r * z.r;
    // The deepest chain zone it fell in: a death in two chains' fires
    // continues the one furthest along, which is the one nearest its stop.
    let zone = s.zones.iter().filter(|z| inside(z)).max_by_key(|z| (z.chain.is_some(), z.depth)).copied();
    if is_raider && zone.is_some() {
        s.counts.raiders_by_cards += 1;
        s.counts.score += RAIDER_SCORE;
    }
    if is_ant {
        if zone.is_some() {
            s.counts.card_kills += 1;
        } else {
            s.counts.natural += 1;
        }
    }
    if let Some(Zone { chain: Some(c), depth, .. }) = zone {
        let depth = depth + 1;
        let full = s.chains.get(c).is_none_or(|ch| ch.effects >= CHAIN_CAP);
        if depth > MAX_DEPTH || full {
            s.counts.puffs += 1;
            if let Some(ch) = s.chains.get_mut(c) {
                ch.puffs += 1;
            }
            s.rings.push(Ring { at, r: 2, depth, puff: true, born: frame });
            return;
        }
        let payload = s.chains[c].payload.clone();
        for p in payload {
            s.queue.push_back(Pending { at, what: p, chain: Some(c), depth });
        }
        return;
    }
    let near = s.armed.iter().position(|a| (a.at.0 - at.0).pow(2) + (a.at.1 - at.1).pow(2) <= BIND_R * BIND_R);
    if let Some(i) = near {
        let a = s.armed.remove(i);
        let (c, depth) = match a.chain {
            Some(c) => (c, a.depth + 1),
            None => {
                s.chains.push(Chain { payload: a.payload.clone(), effects: 0, deepest: 0, puffs: 0 });
                s.counts.chains += 1;
                (s.chains.len() - 1, 1)
            }
        };
        if depth > MAX_DEPTH {
            s.counts.puffs += 1;
            s.rings.push(Ring { at, r: 2, depth, puff: true, born: frame });
            return;
        }
        for p in a.payload {
            s.queue.push_back(Pending { at, what: p, chain: Some(c), depth });
        }
    }
}

/// **A wave of raiders lands**, wave `n` bringing `n` of them up to
/// [`Rules::wave_cap`], on the surface at alternate ends of the box, and
/// hunts. A predator thinks with its species' trained pool, which the lab
/// gives any predator within a sweep (`Lab::give_predator_minds`).
fn land_raider(lab: &mut Lab) {
    let Some((species, n)) = satchel_mut(lab).map(|s| {
        s.counts.waves += 1;
        (s.rules.raider, s.counts.waves.min(s.rules.wave_cap))
    }) else {
        return;
    };
    let w = lab.spec.width;
    let door = lab.world.nest_sites.first().map_or(w / 2, |n| n.x);
    let mut landed = 0;
    for i in 0..n {
        // Alternate sides, [`RAIDER_GAP`] columns out from the door and a
        // little further for each one after: near enough to matter, far
        // enough to be met on the surface first.
        let off = RAIDER_GAP + 8 * (i as i32 / 2);
        let x = (if i % 2 == 0 { door + off } else { door - off }).clamp(20, w - 20);
        let y = super::keeper::surface_under(&lab.world, x) - 2;
        if lab.stock_one(x, y, species).is_some() {
            landed += 1;
        }
    }
    if let Some(s) = satchel_mut(lab) {
        s.counts.raiders_in += landed;
    }
    if landed > 0 {
        let name = species.to_uppercase();
        lab.world.log_player_action(format!("SATCHEL: WAVE OF {landed} {name}"));
        lab.ui.say(format!("{landed} {name}{} LANDED -- STOP THEM BEFORE THEY REACH THE NEST", if landed == 1 { " HAS" } else { "S HAVE" }));
    }
}

const WHITE: [u8; 4] = [235, 235, 235, 255];
const DIM: [u8; 4] = [160, 160, 160, 255];
const BAD: [u8; 4] = [255, 110, 80, 255];
const GOOD: [u8; 4] = [120, 230, 140, 255];
const PUFF: [u8; 4] = [150, 150, 150, 255];

/// A depth's ring colour, white (your click) through yellow and orange to
/// red at the limit: a fixed ramp, never a blend with the cell beneath.
fn depth_colour(depth: u8) -> [u8; 4] {
    match depth {
        0 => [255, 255, 255, 255],
        1 => [255, 230, 90, 255],
        2 => [255, 150, 40, 255],
        _ => [255, 60, 40, 255],
    }
}

fn put_world(lab: &Lab, frame_buf: &mut [u8], (x, y): (i32, i32), colour: [u8; 4]) {
    let hc = super::super::ui::hud_canvas(&lab.renderer);
    if let Some((sx, sy)) = lab.renderer.world_to_logical(x, y) {
        hc.put(frame_buf, sx, sy, colour);
    }
}

fn circle(lab: &Lab, frame_buf: &mut [u8], at: (i32, i32), r: i32, colour: [u8; 4]) {
    let n = (r * 8).max(12);
    for i in 0..n {
        let a = i as f32 / n as f32 * std::f32::consts::TAU;
        let p = (at.0 + (a.cos() * r as f32).round() as i32, at.1 + (a.sin() * r as f32).round() as i32);
        put_world(lab, frame_buf, p, colour);
    }
}

/// Rings on the world: every effect by depth, puffs grey, armed `WAKE`s as
/// a pulsing green circle at their reach.
pub fn draw_world(lab: &Lab, frame_buf: &mut [u8]) {
    let Some(s) = satchel(lab) else { return };
    let frame = lab.world.frame;
    for r in &s.rings {
        let age = frame.saturating_sub(r.born) as i32;
        if r.puff {
            for k in 0..3 {
                circle(lab, frame_buf, r.at, r.r + k + age / 20, PUFF);
            }
        } else {
            circle(lab, frame_buf, r.at, r.r + 1 + age / 15, depth_colour(r.depth));
        }
    }
    for a in &s.armed {
        let pulse = ((frame / 10) % 3) as i32;
        circle(lab, frame_buf, a.at, 2 + pulse, Card::Wake.colour());
        circle(lab, frame_buf, a.at, BIND_R, [60, 140, 90, 255]);
    }
}

pub fn marks_world(lab: &Lab) -> bool {
    satchel(lab).is_some_and(|s| !s.rings.is_empty() || !s.armed.is_empty())
}

/// The panel: the satchel as chips with the next one outlined, favour, the
/// round, and the counters.
pub fn draw_hud(lab: &Lab, hc: crate::render::Hud, frame_buf: &mut [u8]) {
    let Some(s) = satchel(lab) else { return };
    let frame = lab.world.frame;
    let x0 = 4;
    let mut y = 16;
    let label = format!("SCORE {}  SATCHEL {}", s.counts.score, PRESETS[s.preset].0);
    hc.text(frame_buf, x0, y, &label, WHITE);
    let mut x = x0 + crate::hud::text_width(&label) + 6;
    for (i, c) in s.deck.iter().enumerate() {
        let w = crate::hud::text_width(c.name()) + 6;
        if i == s.pos {
            hc.fill_rect(frame_buf, x - 1, y - 2, w + 2, crate::hud::GLYPH_HEIGHT + 4, WHITE);
        }
        hc.fill_rect(frame_buf, x, y - 1, w, crate::hud::GLYPH_HEIGHT + 2, [30, 30, 36, 255]);
        hc.text(frame_buf, x + 3, y, c.name(), c.colour());
        x += w + 4;
    }
    let wait = if s.ended.is_some() {
        String::new()
    } else if frame < s.ready_at {
        format!("{} {:.1}S", if s.reloading { "RELOAD" } else { "WAIT" }, (s.ready_at - frame) as f32 / 60.0)
    } else {
        "READY".to_string()
    };
    hc.text(frame_buf, x + 4, y, &wait, if frame < s.ready_at { DIM } else { GOOD });
    y += 11;
    let label = "FAVOUR";
    hc.text(frame_buf, x0, y, label, WHITE);
    let bx = x0 + crate::hud::text_width(label) + 5;
    let bw = 90;
    hc.fill_rect(frame_buf, bx, y, bw, 7, [40, 50, 80, 255]);
    let fill = (bw as u32 * s.favour / s.rules.favour_max.max(1)) as i32;
    hc.fill_rect(frame_buf, bx, y, fill, 7, [90, 140, 255, 255]);
    let next = deal(&s.deck, s.pos);
    hc.text(frame_buf, bx + bw + 6, y, &format!("{}  NEXT: {} FOR {}", s.favour, describe(&next.planned), next.cost), if next.cost > s.favour { BAD } else { WHITE });
    y += 10;
    let ants = s.ended.map_or_else(|| live_of(&lab.world, "ant"), |e| e.ants);
    let end = s.ended.map_or(frame, |e| e.frame);
    let left = s.opened.map_or(s.rules.round, |o| (o + s.rules.round).saturating_sub(end)) / 60;
    let c = s.counts;
    hc.text(
        frame_buf,
        x0,
        y,
        &format!("ANTS {ants} (LOST UNDER {})  {}:{:02} LEFT  RAIDERS {} IN, {} KILLED", s.rules.lose_at, left / 60, left % 60, c.raiders_in, c.raiders_killed),
        if ants < s.rules.lose_at + 5 { BAD } else { WHITE },
    );
    y += 10;
    hc.text(
        frame_buf,
        x0,
        y,
        &format!("CHAINS {}  DEEPEST {}/{MAX_DEPTH}  PUFFS {}  KILLED BY CARDS {}, BY THE BOX {}", c.chains, c.deepest, c.puffs, c.card_kills, c.natural),
        DIM,
    );
    if let Some(e) = s.ended {
        y += 10;
        hc.text(frame_buf, x0, y, &format!("ROUND {} -- SCORE {} -- {} ANTS LEFT, {} RAIDERS KILLED BY CARDS", if e.won { "WON" } else { "LOST" }, c.score, e.ants, c.raiders_by_cards), if e.won { GOOD } else { BAD });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_modifier_changes_the_next_card_and_deals_a_replacement() {
        let d = deal(&[Card::Wide, Card::Fire, Card::Soak], 0);
        assert_eq!(d.planned, vec![Planned::Fire(FIRE_R * 2)]);
        assert_eq!(d.next, 2);
        assert!(!d.wrapped);
        assert_eq!(d.cost, Card::Wide.cost() + Card::Fire.cost());
    }

    #[test]
    fn twin_deals_two_and_wake_carries_the_next_card() {
        let d = deal(PRESETS[2].1, 0);
        // TWIN -> (WAKE -> WIDE -> FIRE), FIRE: the whole satchel in one play.
        assert_eq!(d.planned, vec![Planned::Wake(vec![Planned::Fire(FIRE_R * 2)]), Planned::Fire(FIRE_R)]);
        assert!(d.wrapped, "the last slot was used: a reload follows");
        assert_eq!(d.next, 0);
    }

    #[test]
    fn running_off_the_end_wraps_once_then_stops() {
        // TWIN TWIN: the second play starts at slot 1, wraps to slot 0, and
        // can wrap no further -- it ends instead of looping.
        let d = deal(&[Card::Twin, Card::Twin], 1);
        assert!(d.wrapped);
        assert!(d.planned.is_empty());
        let d = deal(&[Card::Fire, Card::Wide], 1);
        assert!(d.wrapped);
        assert_eq!(d.planned, vec![Planned::Fire(FIRE_R * 2)], "WIDE at the end modifies the wrapped-to FIRE");
        let d = deal(&[Card::Wide], 0);
        assert_eq!(d.fizzled, 1, "a modifier with nothing to change is counted, not silent");
    }

    #[test]
    fn wide_stacks_but_is_capped() {
        let d = deal(&[Card::Wide, Card::Wide, Card::Wide, Card::Wide, Card::Soak], 0);
        assert_eq!(d.planned, vec![Planned::Soak((SOAK_R << 2).min(MAX_R))]);
    }

    /// **The depth limit and the chain cap hold however many deaths pile
    /// in**: a chain fed two deaths at every effect it lays (the worst case,
    /// every fire killing two ants) goes all the way to depth 3, stops there,
    /// and never passes 24 effects; the deaths past the stop puff. Reaching
    /// depth 3 is the positive control: the chain did snowball, and the
    /// limit is what stopped it.
    #[test]
    fn a_self_feeding_chain_stops_at_the_depth_limit() {
        let mut s = SatchelDemo { opened: Some(0), ..Default::default() };
        s.armed.push(Armed { at: (100, 100), payload: vec![Planned::Fire(4), Planned::Fire(4)], chain: None, depth: 0 });
        on_death(&mut s, (100, 100), true, false, 1);
        assert_eq!(s.counts.chains, 1);
        let mut frame = 2;
        let mut popped = 0;
        while let Some(p) = s.queue.pop_front() {
            assert!(p.depth <= MAX_DEPTH, "an effect past the depth limit");
            popped += 1;
            assert!(popped < 1000, "the queue never drained");
            if admit(&mut s, &p, 4, frame) {
                on_death(&mut s, p.at, true, false, frame);
                on_death(&mut s, p.at, true, false, frame);
            }
            frame += 1;
        }
        assert_eq!(s.chains[0].deepest, MAX_DEPTH, "the chain never snowballed to the limit");
        assert!(s.chains[0].effects <= CHAIN_CAP, "{} effects", s.chains[0].effects);
        assert!(s.counts.puffs > 0, "nothing puffed: the limit was never reached");
        assert!(s.counts.card_kills > 0);
        assert_eq!(s.counts.natural, 1, "only the death that set the WAKE off was the box's own");
    }

    #[test]
    fn a_death_far_from_everything_is_natural_and_fires_nothing() {
        let mut s = SatchelDemo::default();
        s.armed.push(Armed { at: (100, 100), payload: vec![Planned::Fire(4)], chain: None, depth: 0 });
        on_death(&mut s, (100 + BIND_R + 5, 100), true, false, 1);
        assert!(s.queue.is_empty());
        assert_eq!(s.armed.len(), 1, "a far death took the armed card");
        assert_eq!(s.counts.natural, 1);
    }
}
