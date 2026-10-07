//! **Every plant's life and death, one individual at a time** -- the plant
//! line's counterpart of `deeptrace`, which records every decision of every
//! ant.
//!
//! Built 2026-10-06 for the owner's question whether the ant line's method --
//! trace individuals, never a colony statistic -- would help the plant code.
//! Its first run answered with a defect: the game booked **93-97% of all plant
//! deaths as FELLED** on a bed with no animals and no player, because "left
//! owning no cells, never declared dead" was assumed to mean a felling. Nearly
//! all of it was seeds rotting on their own clock. The real fellings were
//! hidden in the pile, and once labelled they turned out to be a finding of
//! their own: **about 1,500 small plants a run, mostly grass seedlings, cut
//! loose by the support check** (`Reports/open-bugs-handoff.md` §B2) -- not
//! shaded or dried out, which is what "their cells went empty" had suggested
//! before anything was labelled.
//!
//! **Deaths, always**: every death in the run, read from `World::death_log`
//! -- which this turns on -- rather than inferred. A dormant seed gets no
//! grave (owner's choice, 2026-10-06), and inferring a cause from whichever
//! `deaths_by_cause` counter moved in a frame is ambiguous when several seeds
//! rot in the same frame, so the log is the only complete record. Writes:
//!
//! - **`deaths.csv`**: one row per death: frame, id, born frame, species,
//!   lineage, kind (`seed` = never germinated, `plant`, `animal`), cause,
//!   whether a rule declared it (`declared`) or it is what took the last cell,
//!   whether it got a grave, and where.
//! - **`hash.txt`**: a digest of the whole grid and the organism turnover every
//!   `hashevery=` frames, the shape of the lab's own `grid_hash`. Two runs
//!   that differ only in a recorder must match line for line: `log=0` runs the
//!   same bed with the death log off, and `life=0` against `life=1` does the
//!   same for the ledger below.
//!
//! Prints the deaths by kind and cause, the UNKNOWN residue with the species
//! it came from (a removal no site labels: a gap to close, not a cause), and
//! reconciles every cause against the world's own `deaths_by_cause`.
//!
//! **`life=1`: the life ledger, every plant from seed to grave** (step 2 of
//! the plant-tracing plan, 2026-10-07). Each life is keyed by `(id, born)`,
//! the pair `OrganismState::born_frame` says is collision-proof. Adds:
//!
//! - **`lives.csv`**: one row per life, seed or plant: species, generation,
//!   lineage, parent (`OrganismState::parent`, with its born frame), whether
//!   and when it germinated and where, its peak size, the shoot size it needs
//!   before it may set seed (`maturity`: `plant::seed_maturity_of`, or `est=`)
//!   and when it got there (`established`), its first seed, seeds set, its
//!   offspring (set, germinated, established -- counted from the children's
//!   own rows), the longest it went starving, when a rule marked it dying and
//!   why, and its death.
//! - **`plants.csv.gz`**: every germinated plant every `every=` frames (500),
//!   read straight off `OrganismState`: income, upkeep and unpaid upkeep,
//!   starving ticks, water status/uptake/demand, nutrients, roots in contact
//!   with soil, shoot/root/organ cells, the breeding fund, anchorage, whether
//!   it is dying, its growing tips (shoot and root), the light at its crown
//!   **with the day divided out** (`field::noon_equivalent_light` -- a raw
//!   reading taken at night reads every plant as shaded), and what sits on
//!   top of the crown. `species=` keeps one species.
//! - **`spells.csv`**: every starving spell (starving ticks above zero): when
//!   it began and ended, its worst, how it ended -- `recovered`, the cause the
//!   plant died of, or `ongoing` at the end of the run -- and whether it began
//!   on a dormant seed (`kind`), which starves on the grown plant's clock
//!   (`Reports/open-bugs-handoff.md` §V5).
//! - **`events.txt`**: GERMINATED, ESTABLISHED, FIRST_SEED, MARKED and GONE
//!   for every plant that germinated. A seed that never came up has its row
//!   and its death and nothing between.
//!
//! **Exact and late.** Germination (the plant's own `germination_frame`) and
//! death (the log) are exact. Established, first seed, marked and the spells
//! are read every `track=` frames (30, a third of the lab's plant tick), so
//! each is late by less than that.
//!
//! **`cull=F` is the known-answer control**: at frame `F` it culls every
//! other germinated plant not already dying, through the player's own verb
//! (`World::mark_organism_senescent`), and once they have rotted the run's
//! CULLED count must equal the number it culled. It changes the world, so it
//! is a different run from the same seed without it.
//!
//! Read with `scripts/planttrace.py` (`funnel`, `deaths`, `spells`, `life`).
//!
//! ```text
//! cargo run --release --example planttrace -- seed=1 frames=100000 colony=0 out=planttrace-out
//! cargo run --release --example planttrace -- seed=1 frames=100000 life=1 out=planttrace-out
//! python3 scripts/planttrace.py funnel planttrace-out
//! ```

use pixel_physics::lab::scenario::{Placement, Scenario};
use pixel_physics::lab::Lab;
use pixel_physics::sim::cell::OrganismId;
use pixel_physics::sim::organism::{self, CellType, DeathCause, DEATH_CAUSES, DEATH_CAUSE_LIST};
use pixel_physics::sim::world::{DeathRow, World};
use pixel_physics::sim::{field, plant};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::Write;

fn arg<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix(&format!("{key}=")).and_then(|v| v.parse().ok()))
}

/// An order-sensitive digest of the whole grid and the organism books, the
/// shape of `lab::tests::grid_hash` and `tests/determinism.rs`: what a
/// recorder could disturb moves cells or organisms, so a census of counts
/// would miss it.
fn grid_hash(w: &World) -> u64 {
    fn fnv1a(h: u64, v: u64) -> u64 {
        (h ^ v).wrapping_mul(0x0000_0100_0000_01b3)
    }
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    if let Some(b) = w.bounds() {
        for y in b.min_y..=b.max_y {
            for x in b.min_x..=b.max_x {
                let c = w.get(x, y);
                h = fnv1a(h, c.material.0 as u64);
                h = fnv1a(h, c.aux() as u64);
                h = fnv1a(h, c.organism_id() as u64);
            }
        }
    }
    h = fnv1a(h, w.live_organism_count() as u64);
    let (born, died) = w.organism_turnover();
    h = fnv1a(h, born);
    fnv1a(h, died)
}

/// System `gzip`, as `deeptrace` writes its big tables.
fn gzip_to(path: &str) -> (std::process::Child, std::io::BufWriter<std::process::ChildStdin>) {
    let mut child = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("gzip -4 > '{path}'"))
        .stdin(std::process::Stdio::piped())
        .spawn()
        .expect("gzip");
    let w = std::io::BufWriter::with_capacity(1 << 20, child.stdin.take().expect("gzip stdin"));
    (child, w)
}

/// `Some(v)` as `v`, `None` as an empty CSV field.
fn opt<T: std::fmt::Display>(v: Option<T>) -> String {
    v.map_or(String::new(), |v| v.to_string())
}

/// One plant's life, seed to grave.
struct Life {
    species: String,
    generation: u16,
    lineage: u32,
    parent: OrganismId,
    parent_born: u64,
    germinated: Option<u64>,
    origin: Option<(i32, i32)>,
    peak_cells: usize,
    maturity: Option<u32>,
    established: Option<u64>,
    first_seed: Option<u64>,
    seeds_set: u32,
    max_starving: u16,
    /// The starving spell under way: when it began, its worst so far, and
    /// whether the life was still a dormant seed when it began (§V5: a leafy
    /// species' seed starves on the grown plant's clock).
    spell: Option<(u64, u16, bool)>,
    marked: Option<(u64, DeathCause)>,
    /// Frame, cause, declared, buried.
    died: Option<(u64, DeathCause, bool, bool)>,
    /// Culled by `cull=`, the known-answer control.
    culled_here: bool,
}

struct Ledger {
    lives: BTreeMap<(OrganismId, u64), Life>,
    alive: BTreeSet<(OrganismId, u64)>,
    sample_every: u64,
    track_every: u64,
    est: Option<u32>,
    species: Option<String>,
    plants: (std::process::Child, std::io::BufWriter<std::process::ChildStdin>),
    events: std::io::BufWriter<std::fs::File>,
    spells: std::io::BufWriter<std::fs::File>,
    /// `organism_turnover().0` at the last scan: a new life can only have
    /// appeared if it moved.
    last_born: u64,
    /// Lives first seen in the death log -- born and gone between two scans,
    /// so their parent is not known. Printed, because a large number means
    /// the scan is missing what it exists to catch.
    seen_only_dead: u64,
    culled: u64,
}

impl Ledger {
    fn new(out: &str, every: u64, track: u64, est: Option<u32>, species: Option<String>) -> Self {
        let mut plants = gzip_to(&format!("{out}/plants.csv.gz"));
        writeln!(
            plants.1,
            "frame,id,born,species,cells,shoot,root,organs,income,upkeep,unpaid,starving,water,uptake,demand,nutrients,root_contact,fund,anchor,dying,tips,root_tips,light,above"
        )
        .unwrap();
        let mut spells =
            std::io::BufWriter::new(std::fs::File::create(format!("{out}/spells.csv")).expect("spells.csv"));
        writeln!(spells, "id,born,species,kind,start,end,worst,outcome").unwrap();
        Ledger {
            lives: BTreeMap::new(),
            alive: BTreeSet::new(),
            sample_every: every,
            track_every: track,
            est,
            species,
            plants,
            events: std::io::BufWriter::new(std::fs::File::create(format!("{out}/events.txt")).expect("events.txt")),
            spells,
            last_born: u64::MAX,
            seen_only_dead: 0,
            culled: 0,
        }
    }

    /// Every plant or seed the world holds that the ledger has not met yet.
    /// Scans only when the turnover's birth count moved since the last scan.
    fn discover(&mut self, w: &World) {
        let born = w.organism_turnover().0;
        if born == self.last_born {
            return;
        }
        self.last_born = born;
        for id in w.live_organism_ids() {
            let Some(s) = w.organism(id) else { continue };
            if w.species.get(s.species).creature.is_some() || s.brood.is_some() {
                continue;
            }
            let key = (id, s.born_frame);
            if self.lives.contains_key(&key) {
                continue;
            }
            self.lives.insert(
                key,
                Life {
                    species: w.species.get(s.species).name.clone(),
                    generation: s.generation,
                    lineage: s.lineage,
                    parent: s.parent,
                    parent_born: s.parent_born,
                    germinated: None,
                    origin: None,
                    peak_cells: s.cells.len(),
                    maturity: None,
                    established: None,
                    first_seed: None,
                    seeds_set: s.seeds_set,
                    max_starving: 0,
                    spell: None,
                    marked: None,
                    died: None,
                    culled_here: false,
                },
            );
            self.alive.insert(key);
        }
    }

    /// The `track=` read of every living plant: germination, size, maturity,
    /// first seed, starving spells, and the moment a rule marks it dying.
    fn track(&mut self, w: &World) {
        let f = w.frame;
        for &(id, born) in &self.alive {
            let Some(s) = w.organism(id).filter(|s| s.born_frame == born) else {
                continue;
            };
            let life = self.lives.get_mut(&(id, born)).expect("alive is a subset of lives");
            if life.germinated.is_none() && !s.dormant_seed {
                let g = if s.germination_frame > 0 {
                    s.germination_frame
                } else {
                    f
                };
                life.germinated = Some(g);
                life.origin = s.origin;
                life.maturity = self.est.or_else(|| plant::seed_maturity_of(w, id));
                let (x, y) = s.origin.unwrap_or((0, 0));
                writeln!(
                    self.events,
                    "{g} GERMINATED id={id} born={born} species={} x={x} y={y}",
                    life.species
                )
                .unwrap();
            }
            life.peak_cells = life.peak_cells.max(s.cells.len());
            if life.germinated.is_some()
                && life.established.is_none()
                && life.maturity.is_some_and(|m| s.shoot_cells >= m)
            {
                life.established = Some(f);
                writeln!(
                    self.events,
                    "{f} ESTABLISHED id={id} born={born} shoot={}",
                    s.shoot_cells
                )
                .unwrap();
            }
            if life.first_seed.is_none() && s.seeds_set > 0 {
                life.first_seed = Some(f);
                writeln!(self.events, "{f} FIRST_SEED id={id} born={born}").unwrap();
                // A seed is only set past the maturity gate, so a plant that
                // set one had grown up -- even if it lost shoot before this
                // read saw it big enough. Not under `est=`, whose threshold
                // is the reader's own and no gate in the box.
                if life.established.is_none() && self.est.is_none() {
                    life.established = Some(f);
                    writeln!(
                        self.events,
                        "{f} ESTABLISHED id={id} born={born} shoot={} (from its first seed)",
                        s.shoot_cells
                    )
                    .unwrap();
                }
            }
            life.seeds_set = s.seeds_set;
            life.max_starving = life.max_starving.max(s.starving_ticks);
            if s.starving_ticks > 0 {
                let (start, worst, seed) = life.spell.unwrap_or((f, 0, s.dormant_seed));
                life.spell = Some((start, worst.max(s.starving_ticks), seed));
            } else if let Some((start, worst, seed)) = life.spell.take() {
                let kind = if seed { "seed" } else { "plant" };
                writeln!(
                    self.spells,
                    "{id},{born},{},{kind},{start},{f},{worst},recovered",
                    life.species
                )
                .unwrap();
            }
            if s.senescent && life.marked.is_none() {
                life.marked = Some((f, s.senescence_cause));
                writeln!(
                    self.events,
                    "{f} MARKED id={id} born={born} cause={}",
                    s.senescence_cause.label()
                )
                .unwrap();
            }
        }
    }

    /// One row per germinated plant, every `every=` frames.
    fn sample(&mut self, w: &World) {
        let f = w.frame;
        for &(id, born) in &self.alive {
            let Some(s) = w.organism(id).filter(|s| s.born_frame == born) else {
                continue;
            };
            if s.dormant_seed {
                continue;
            }
            let name = &w.species.get(s.species).name;
            if self.species.as_ref().is_some_and(|want| want != name) {
                continue;
            }
            let (mut tips, mut root_tips, mut light) = (0u32, 0u32, 0f32);
            let mut top: Option<(i32, i32)> = None;
            for &(cx, cy) in s.cells.keys() {
                let c = w.get(cx, cy);
                match organism::cell_type(c.aux()) {
                    Some(CellType::GrowingTip) => tips += 1,
                    Some(CellType::RootTip) => root_tips += 1,
                    _ => {}
                }
                // Above ground only, as `deeptrace`'s garden record reads it.
                if !w.materials.get(c.material).reinforces_powder {
                    light = light.max(plant::ambient_light_above(w, cx, cy));
                    if top.is_none_or(|(_, ty)| cy < ty) {
                        top = Some((cx, cy));
                    }
                }
            }
            let light = field::noon_equivalent_light(light, f);
            let above = top.map_or("-".to_string(), |(tx, ty)| {
                w.materials.get(w.get(tx, ty - 1).material).name.clone()
            });
            writeln!(
                self.plants.1,
                "{f},{id},{born},{name},{},{},{},{},{:.3},{:.3},{:.3},{},{:.2},{:.3},{:.3},{:.2},{},{:.3},{:.2},{},{tips},{root_tips},{light:.3},{above}",
                s.cells.len(),
                s.shoot_cells,
                s.root_cells,
                s.organ_cells,
                s.income,
                s.maintenance,
                s.maintenance_unpaid,
                s.starving_ticks,
                s.water_status,
                s.water_uptake,
                s.water_demand,
                s.nutrient_status,
                s.contact_root_cells,
                s.reproductive_budget,
                s.anchor_status,
                u8::from(s.senescent),
            )
            .unwrap();
        }
    }

    /// A death from the log closes its life.
    fn death(&mut self, d: &DeathRow, name: &str) {
        let key = (d.id, d.born_frame);
        self.alive.remove(&key);
        let life = self.lives.entry(key).or_insert_with(|| {
            self.seen_only_dead += 1;
            Life {
                species: name.to_string(),
                generation: 0,
                lineage: d.lineage,
                parent: 0,
                parent_born: 0,
                germinated: None,
                origin: None,
                peak_cells: 0,
                maturity: None,
                established: None,
                first_seed: None,
                seeds_set: 0,
                max_starving: 0,
                spell: None,
                marked: None,
                died: None,
                culled_here: false,
            }
        });
        // Germinated and gone inside one `track=` window: the frame is not
        // known, so the death's own stands in for it.
        if life.germinated.is_none() && !d.dormant_seed {
            life.germinated = Some(d.frame);
        }
        // Declared inside the last window, before a read saw it.
        if life.marked.is_none() && d.declared {
            life.marked = Some((d.frame, d.cause));
        }
        if let Some((start, worst, seed)) = life.spell.take() {
            let kind = if seed { "seed" } else { "plant" };
            writeln!(
                self.spells,
                "{},{},{},{kind},{start},{},{worst},{}",
                d.id,
                d.born_frame,
                life.species,
                d.frame,
                d.cause.label()
            )
            .unwrap();
        }
        life.died = Some((d.frame, d.cause, d.declared, d.buried));
        if life.germinated.is_some() {
            writeln!(
                self.events,
                "{} GONE id={} born={} cause={} declared={} buried={}",
                d.frame,
                d.id,
                d.born_frame,
                d.cause.label(),
                u8::from(d.declared),
                u8::from(d.buried)
            )
            .unwrap();
        }
    }

    /// The known-answer control: cull every other germinated plant that no
    /// rule has marked yet, through the player's own verb.
    fn cull(&mut self, w: &mut World) {
        let targets: Vec<(OrganismId, u64)> = self
            .alive
            .iter()
            .filter(|&&(id, born)| {
                w.organism(id)
                    .is_some_and(|s| s.born_frame == born && !s.dormant_seed && !s.senescent)
            })
            .copied()
            .collect();
        for &(id, born) in targets.iter().step_by(2) {
            if w.mark_organism_senescent(id) {
                self.culled += 1;
                if let Some(life) = self.lives.get_mut(&(id, born)) {
                    life.culled_here = true;
                }
            }
        }
        println!(
            "  cull={}: culled {} of {} germinated plants not yet dying",
            w.frame,
            self.culled,
            targets.len()
        );
    }

    fn finish(mut self, out: &str, end: u64) {
        for (&(id, born), life) in self.lives.iter_mut() {
            if let Some((start, worst, seed)) = life.spell.take() {
                let kind = if seed { "seed" } else { "plant" };
                writeln!(
                    self.spells,
                    "{id},{born},{},{kind},{start},{end},{worst},ongoing",
                    life.species
                )
                .unwrap();
            }
        }
        // Offspring, counted from the children's own rows.
        let mut kids: HashMap<(OrganismId, u64), (u32, u32, u32)> = HashMap::new();
        for life in self.lives.values() {
            if life.parent != 0 {
                let k = kids.entry((life.parent, life.parent_born)).or_default();
                k.0 += 1;
                k.1 += u32::from(life.germinated.is_some());
                k.2 += u32::from(life.established.is_some());
            }
        }
        let mut lives = std::io::BufWriter::new(std::fs::File::create(format!("{out}/lives.csv")).expect("lives.csv"));
        writeln!(
            lives,
            "id,born,species,generation,lineage,parent,parent_born,kind,germinated,origin_x,origin_y,peak_cells,maturity,established,first_seed,seeds_set,offspring,offspring_germinated,offspring_established,max_starving,marked,marked_cause,died,cause,declared,buried,culled_here"
        )
        .unwrap();
        for (&(id, born), l) in &self.lives {
            let (set, germinated, established) = kids.get(&(id, born)).copied().unwrap_or_default();
            writeln!(
                lives,
                "{id},{born},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{set},{germinated},{established},{},{},{},{},{},{},{},{}",
                l.species,
                l.generation,
                l.lineage,
                l.parent,
                l.parent_born,
                if l.germinated.is_some() { "plant" } else { "seed" },
                opt(l.germinated),
                opt(l.origin.map(|o| o.0)),
                opt(l.origin.map(|o| o.1)),
                l.peak_cells,
                opt(l.maturity),
                opt(l.established),
                opt(l.first_seed),
                l.seeds_set,
                l.max_starving,
                opt(l.marked.map(|m| m.0)),
                opt(l.marked.map(|m| m.1.label())),
                opt(l.died.map(|d| d.0)),
                opt(l.died.map(|d| d.1.label())),
                opt(l.died.map(|d| u8::from(d.2))),
                opt(l.died.map(|d| u8::from(d.3))),
                u8::from(l.culled_here),
            )
            .unwrap();
        }
        lives.flush().unwrap();
        self.events.flush().unwrap();
        self.spells.flush().unwrap();
        self.plants.1.flush().unwrap();
        drop(self.plants.1);
        self.plants.0.wait().expect("gzip");

        // A short funnel; `scripts/planttrace.py funnel` is the full one.
        let mut by: BTreeMap<&str, [u64; 4]> = BTreeMap::new();
        for l in self.lives.values() {
            let e = by.entry(l.species.as_str()).or_default();
            e[0] += 1;
            e[1] += u64::from(l.germinated.is_some());
            e[2] += u64::from(l.established.is_some());
            e[3] += u64::from(l.first_seed.is_some());
        }
        println!("\nLIVES (seed -> germinated -> established -> set a seed)");
        for (sp, [n, g, e, s]) in &by {
            println!("  {sp:12} {n:6} -> {g:5} -> {e:5} -> {s:5}");
        }
        println!(
            "  {} lives first seen in the death log (born and gone between two scans; parent unknown)",
            self.seen_only_dead
        );
        if self.culled > 0 {
            // **Still rotting is not a miss.** A culled plant is carried out
            // cell by cell at its species' half-life (`rot_remains`), so a big
            // one can outlast the run: seed 1's 953-cell shrub still had 239
            // cells 50,000 frames after its cull. What would be a miss is a
            // culled plant booked under any other cause.
            let booked = self
                .lives
                .values()
                .filter(|l| l.culled_here && l.died.is_some_and(|d| d.1 == DeathCause::Culled))
                .count() as u64;
            let other = self
                .lives
                .values()
                .filter(|l| l.culled_here && l.died.is_some_and(|d| d.1 != DeathCause::Culled))
                .count() as u64;
            let pending = self
                .lives
                .values()
                .filter(|l| l.culled_here && l.died.is_none())
                .count() as u64;
            println!(
                "\nCULL CONTROL: culled {}, booked CULLED {booked}, booked as anything else {other}, still rotting {pending} -- {}",
                self.culled,
                if other == 0 && booked + pending == self.culled { "matches" } else { "DOES NOT MATCH" }
            );
        }
    }
}

fn main() {
    // Births inherit exactly unless `PIXEL_PHYSICS_MUTATION=on`
    // (`Reports/how-we-test.md` §1): two arms that differ by one frame
    // otherwise draw different mutations, plants as well as animals.
    pixel_physics::sim::creature::mutation_off_for_measuring();
    let seed: u64 = arg("seed").unwrap_or(1);
    let frames: u64 = arg("frames").unwrap_or(100_000);
    let scenario: String = arg("scenario").unwrap_or_else(|| "played_bed".to_string());
    // `colony=0` takes the colony out of the scenario's timeline (the garden
    // alone); absent, the scenario runs as written.
    let colony: Option<u8> = arg("colony");
    let out: String = arg("out").unwrap_or_else(|| "planttrace-out".to_string());
    let hash_every: u64 = arg("hashevery").unwrap_or(10_000).max(1);
    let log_on = arg::<u8>("log").unwrap_or(1) == 1;
    let life_on = arg::<u8>("life").unwrap_or(0) == 1;
    let every: u64 = arg("every").unwrap_or(500).max(1);
    let track: u64 = arg("track").unwrap_or(30).max(1);
    let est: Option<u32> = arg("est");
    let species: Option<String> = arg("species");
    let cull_at: Option<u64> = arg("cull");
    println!(
        "planttrace: scenario={scenario} seed={seed} frames={frames} colony={} hashevery={hash_every} log={} life={} every={every} track={track} est={} species={} cull={} out={out}",
        colony.map_or("as-written".to_string(), |c| c.to_string()),
        u8::from(log_on),
        u8::from(life_on),
        opt(est),
        species.as_deref().unwrap_or("all"),
        opt(cull_at),
    );
    if life_on && !log_on {
        eprintln!("planttrace: life=1 reads the death log; it needs log=1");
        std::process::exit(2);
    }
    std::fs::create_dir_all(&out).expect("out dir");
    let mut sc = Scenario::load(&scenario).unwrap_or_else(|e| {
        eprintln!("scenario {scenario}: {e}");
        std::process::exit(1);
    });
    sc.bed.seed = seed;
    if colony == Some(0) {
        sc.timeline
            .retain(|e| !matches!(e.what, Placement::Colony { .. } | Placement::Colonies { .. }));
    }
    let mut lab = Lab::new(sc.bed.clone());
    let msg = lab.load_scenario(sc);
    println!("  {msg}");
    if log_on {
        lab.world.death_log = Some(Vec::new());
    }
    let mut ledger = life_on.then(|| Ledger::new(&out, every, track, est, species.clone()));
    if let Some(l) = ledger.as_mut() {
        l.discover(&lab.world);
    }

    let mut deaths = std::io::BufWriter::new(std::fs::File::create(format!("{out}/deaths.csv")).expect("deaths.csv"));
    writeln!(deaths, "frame,id,born,species,lineage,kind,cause,declared,buried,x,y").unwrap();
    let mut hashes = std::io::BufWriter::new(std::fs::File::create(format!("{out}/hash.txt")).expect("hash.txt"));
    // (kind, cause) -> deaths; (cause) -> deaths, for the reconciliation;
    // (species) -> UNKNOWN deaths, for the residue.
    let mut table: BTreeMap<(&'static str, usize), u64> = BTreeMap::new();
    let mut logged = [0u64; DEATH_CAUSES];
    let mut unknown_by_species: BTreeMap<String, u64> = BTreeMap::new();
    let (mut rows, mut buried) = (0u64, 0u64);
    for f in 1..=frames {
        lab.tick_for_harness();
        if let Some(l) = ledger.as_mut() {
            l.discover(&lab.world);
        }
        let w = &mut lab.world;
        if let Some(log) = w.death_log.as_mut() {
            let drained = std::mem::take(log);
            for d in drained {
                let kind = if d.creature {
                    "animal"
                } else if d.dormant_seed {
                    "seed"
                } else {
                    "plant"
                };
                let name = w.species.get(d.species).name.clone();
                writeln!(
                    deaths,
                    "{},{},{},{},{},{kind},{},{},{},{},{}",
                    d.frame,
                    d.id,
                    d.born_frame,
                    name,
                    d.lineage,
                    d.cause.label(),
                    u8::from(d.declared),
                    u8::from(d.buried),
                    d.at.0,
                    d.at.1
                )
                .unwrap();
                *table.entry((kind, d.cause.index())).or_insert(0) += 1;
                logged[d.cause.index()] += 1;
                if d.cause == organism::DeathCause::Unknown {
                    *unknown_by_species.entry(format!("{kind} {name}")).or_insert(0) += 1;
                }
                rows += 1;
                buried += u64::from(d.buried);
                if !d.creature {
                    if let Some(l) = ledger.as_mut() {
                        l.death(&d, &name);
                    }
                }
            }
        }
        if let Some(l) = ledger.as_mut() {
            if f.is_multiple_of(l.track_every) {
                l.track(&lab.world);
            }
            if f.is_multiple_of(l.sample_every) {
                l.sample(&lab.world);
            }
            if cull_at == Some(f) {
                l.cull(&mut lab.world);
            }
        }
        if f % hash_every == 0 || f == frames {
            writeln!(hashes, "{f},{:016x}", grid_hash(&lab.world)).unwrap();
        }
    }
    deaths.flush().unwrap();
    hashes.flush().unwrap();

    let w = &lab.world;
    println!("\nDEATHS BY KIND AND CAUSE ({rows} deaths, {buried} buried)");
    for kind in ["seed", "plant", "animal"] {
        let line: Vec<String> = DEATH_CAUSE_LIST
            .iter()
            .filter_map(|c| table.get(&(kind, c.index())).map(|n| format!("{} {n}", c.label())))
            .collect();
        if !line.is_empty() {
            println!("  {kind:6} {}", line.join(" | "));
        }
    }
    println!("\nUNKNOWN RESIDUE (a removal no site labels -- a gap, not a cause)");
    if unknown_by_species.is_empty() {
        println!("  none");
    }
    for (k, n) in &unknown_by_species {
        println!("  {k:30} {n}");
    }
    if log_on {
        let mut ok = true;
        for c in DEATH_CAUSE_LIST {
            let (mine, world) = (logged[c.index()], w.deaths_by_cause[c.index()]);
            if mine != world {
                ok = false;
                println!(
                    "  MISMATCH {}: death_log {mine} against deaths_by_cause {world}",
                    c.label()
                );
            }
        }
        println!(
            "\nRECONCILED with deaths_by_cause: {}",
            if ok { "yes, every cause" } else { "NO -- see above" }
        );
    }
    if let Some(l) = ledger {
        l.finish(&out, frames);
    }
    println!(
        "planttrace: done, {rows} deaths -> {out}/deaths.csv, digests -> {out}/hash.txt{}",
        if life_on {
            ", lives -> lives.csv, plants.csv.gz, spells.csv, events.txt"
        } else {
            ""
        }
    );
}
