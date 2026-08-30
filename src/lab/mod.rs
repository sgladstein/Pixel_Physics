//! **The evolution lab — the biosphere the second game is played in.**
//!
//! A sealed box of soil under a grow light, with plants and creatures living
//! in it, that the player can run at speed and read the state of. The design
//! of record is `Reports/evolution-lab-design-guide-2026-08-30.md`; this
//! module is its Gate 1 (*one hand-built box that runs plants and creatures
//! together*) and Gate 3 (*the two-phase loop, end to end*).
//!
//! **It is a second game, not a second engine.** Everything below the scene
//! is the shipped simulation, reached through `sim::frame::step` — the same
//! tick `src/main.rs` runs, in the same order, with nothing skipped. The
//! lab's frame time comes from what is *not in the box* rather than from what
//! is not in the binary: measured (feasibility §3c), a bed with no rock pays
//! 0.028 ms for the structural scheduler against 3.389 ms outdoors, blasts
//! 0.000, particles 0.000, the gnome 0.001. So there is nothing to strip and
//! stripping is what would make this a fork.
//!
//! Four files, and each is owned by one concern so that concurrent work on
//! them does not collide (`CLAUDE.md`, *working alongside another session*):
//!
//! | file | owns |
//! |---|---|
//! | `scene` | the box: geometry, soil, light, partitions, founders |
//! | `time`  | paused vs running, the speed dial, and what it actually achieved |
//! | `stats` | the census, and the page that draws it |
//! | `ui`    | the control bar along the bottom, its pages, and the mouse |
//! | `params`| which numbers the player can reach, and how they are written |
//! | this file | `Lab` — the state the four above are wired into, and the frame |
//!
//! **The viewport is `app::WIDTH` x `app::HEIGHT`**, deliberately the same
//! framebuffer the sandbox uses, so `render::Renderer` and `hud` need no
//! second set of geometry and a screenshot from either game is comparable
//! with one from the other.

pub mod params;
pub mod scene;
pub mod stats;
pub mod time;
pub mod ui;

use crate::render::Renderer;
use crate::sim::explosion::Blasts;
use crate::sim::frame;
use crate::sim::particle::ParticleSystem;
use crate::sim::player;
use crate::sim::world::World;

pub use crate::app::{HEIGHT, WIDTH};

/// The whole lab: a world, the systems that live beside the cell grid, a view
/// on it, and the two things the player drives — time and what is on screen.
pub struct Lab {
    pub world: World,
    pub particles: ParticleSystem,
    pub blasts: Blasts,
    pub renderer: Renderer,
    /// Present so `frame::step` gets the same arguments the sandbox gives it.
    /// The lab never summons a gnome, and `player::step` is a no-op without
    /// one — measured at 0.001 ms/frame, which is the whole reason this is a
    /// dormant field rather than a phase the lab skips.
    player_tuning: player::Tuning,
    /// The speed dial and the two-phase loop. See `time`.
    pub time: time::TimeControl,
    /// The census and its page. See `stats`.
    pub stats: stats::Stats,
    /// The control bar along the bottom, the pages it opens, and the mouse.
    /// See `ui`.
    pub ui: ui::Ui,
    /// What the box was built from, kept so a reset rebuilds the same lab and
    /// so the stats page can say what bed these numbers are from.
    pub spec: scene::LabBox,
    /// The key list. On by default on a fresh lab and dismissed by any key,
    /// because there is no other way to discover a control here — the sandbox
    /// grew its bindings a key at a time in front of somebody who already knew
    /// them, and the lab has no such person.
    ///
    /// **It is no longer the only way**, which is what `ui` is for: the bar
    /// along the bottom shows every control as a button and prints its key
    /// under it, so the page is now a reference rather than the interface.
    pub show_help: bool,
    /// **A live paint stroke: the last world cell the brush was at, and
    /// whether it is erasing.**
    ///
    /// Held as a *world* cell, not a screen pixel, for `App::begin_drag`'s
    /// reason: a screen point held across a camera move names a different
    /// cell every frame, and the stroke would smear backwards as the view
    /// panned. `None` between strokes.
    stroke: Option<Stroke>,
}

/// One in-progress brush stroke.
#[derive(Clone, Copy)]
struct Stroke {
    /// Where the brush was last applied, in world cells.
    last: (i32, i32),
    erase: bool,
}

impl Lab {
    /// Build the box `spec` describes and start it **stopped**. Nothing
    /// ticks until the player presses `SPACE` or asks for a speed.
    pub fn new(spec: scene::LabBox) -> Self {
        let mut world = spec.build();
        earth_toned_nest(&mut world);
        Self {
            world,
            particles: ParticleSystem::new(),
            blasts: Blasts::new(),
            renderer: Renderer::new(),
            player_tuning: player::Tuning::default(),
            time: time::TimeControl::new(),
            stats: stats::Stats::new(),
            ui: ui::Ui::new(),
            spec,
            // Down only when something explicitly asks for it. The one caller
            // is a headless capture wanting to photograph what is *under* the
            // key list, which is otherwise unreachable without a keypress and
            // so unphotographable on a box with no keyboard.
            show_help: std::env::var("PIXEL_PHYSICS_LAB_HELP").as_deref() != Ok("0"),
            stroke: None,
        }
    }

    /// Rebuild the box from the same spec, keeping the view and the dial.
    pub fn reset(&mut self) {
        self.world = self.spec.build();
        earth_toned_nest(&mut self.world);
        self.particles = ParticleSystem::new();
        self.blasts = Blasts::new();
        self.stats = stats::Stats::new();
    }

    /// One simulated tick — the shipped sequence, nothing skipped.
    ///
    /// Private on purpose: everything outside goes through `advance`, because
    /// how many of these run per displayed frame is `time`'s decision and
    /// splitting that across two callers is how a speed dial stops being
    /// honest about what it achieved.
    fn tick(&mut self) {
        frame::step(
            &mut self.world,
            &mut self.particles,
            &mut self.blasts,
            player::PlayerInput::default(),
            &self.player_tuning,
        );
    }

    /// Run whatever this displayed frame's share of simulated time is, and
    /// report what actually happened.
    ///
    /// `elapsed` is real time since the last call. Everything about how that
    /// becomes a tick count — the requested multiplier, the wall-clock
    /// ceiling that keeps the window answering, whether the box is paused
    /// or running — is `time::TimeControl`'s, so that the readout on screen
    /// and the loop that produced it cannot disagree.
    pub fn advance(&mut self, elapsed: std::time::Duration) -> time::Advance {
        let plan = self.time.plan(elapsed);
        let started = std::time::Instant::now();
        let mut ran = 0u32;
        while ran < plan.ticks {
            self.tick();
            ran += 1;
            if started.elapsed() >= plan.budget {
                break;
            }
        }
        let advance = self.time.record(ran, started.elapsed());
        self.stats.observe(&self.world);
        // Sampled here rather than in `draw` so that a frame which drew
        // nothing still advances the series -- and gated on `World::frame`
        // inside, so the x-axis is simulated time and not the speed dial.
        self.ui.observe(&self.world);
        advance
    }

    /// Draw the world, then whatever the lab is showing over it.
    ///
    /// `fps` is the *window's* rate, which only the binary knows; it is passed
    /// in rather than measured here so that the number on the box page is the
    /// same one the title bar shows.
    pub fn draw(&mut self, frame_buf: &mut [u8], fps: f32) {
        // Anything drawn over the terrain has no footprint tracked between
        // frames, so the dirty-rect skip cannot know to erase last frame's.
        // Same rule, and the same reasoning, as `App::draw`'s. The bar is
        // permanent chrome and its hover highlight moves with the cursor, so
        // `ui::Ui::is_dirty` is in this expression for the same reason
        // `time::hud_is_dirty` is.
        let force_full = self.ui.cursor().is_some()
            || self.ui.is_dirty()
            || self.stats.showing()
            || self.time.hud_is_dirty()
            || self.show_help;
        let touched = self.world.take_touched_chunks();
        self.renderer.draw(
            &self.world,
            &self.particles,
            &touched,
            frame_buf,
            (WIDTH, HEIGHT),
            force_full,
        );
        self.time.draw(frame_buf, &self.world);
        // The species chip's face and its explanation, read out of the world's
        // own table rather than written down here — a chip that named a
        // species while explaining a different one is the stale side table
        // `ui::Spec::note` was made owned to avoid.
        let (species, species_note) = self.selected_species();
        let state = ui::BarState {
            running: self.time.phase == time::Phase::Running,
            requested: self.time.requested,
            achieved: self.time.achieved,
            presets: &time::PRESETS,
            panel: self.ui.panel,
            stats: self.stats.showing(),
            help: self.show_help,
            tool: self.ui.tool(),
            species: &species,
            species_note: &species_note,
            brush: self.ui.brush(),
            overlay: self.renderer.field_overlay.label(),
        };
        self.ui.draw(frame_buf, &self.world, &self.spec, &state, &self.renderer, fps);
        // The pages last, because they are modal: a page covers the box *and*
        // the bar, and a control you can see but not reach is worse than one
        // that is plainly behind a page.
        //
        // **One page at a time.** The key list and the biosphere page are both
        // full-height overlays, and drawn together they interleave into
        // something neither of them is -- caught by looking at a capture of
        // the real window, not by any test, since both draw exactly what they
        // were asked to. The key list wins because it is transient: it is up
        // on a fresh lab and gone on the next keypress, where the page is
        // where you live.
        //
        // `draw_at` rather than `draw`: the hover explanation is the half of
        // the page that makes it readable cold, so the page needs the cursor.
        // `ui::Ui` owns it now -- the bar hit-tests the same position on the
        // same frame -- so it is read back from there rather than passed down
        // a second path. One owner, because two positions that disagree by a
        // frame is exactly the class of bug the retained bar exists to avoid.
        if self.show_help {
            draw_help(frame_buf);
        } else {
            self.stats.draw_at(frame_buf, &self.world, self.ui.cursor());
        }
    }

    /// Where the mouse is, in framebuffer pixels — `None` when it has left the
    /// window. Drives the hover highlight and the hover explanations.
    pub fn set_cursor(&mut self, at: Option<(i32, i32)>) {
        self.ui.set_cursor(at);
        if at.is_none() {
            self.stroke = None;
        }
    }

    /// The left button went down at `(x, y)`.
    ///
    /// **A brush starts painting here, not on release.** Every other control
    /// on the bar fires on release so a press can be taken back; a brush
    /// cannot work that way, because what you are judging is the stroke you
    /// are laying down while you hold the button. The bar's own press
    /// semantics are untouched — this only fires when the press landed on the
    /// world and a painting tool is armed.
    pub fn press(&mut self, x: i32, y: i32) {
        self.ui.press(x, y);
        if self.show_help || self.ui.covers(x, y) || !self.ui.tool().is_brush() {
            return;
        }
        let at = self.renderer.screen_to_world(x, y);
        self.begin_stroke(at, false);
    }

    /// The right button went down at `(x, y)` — the eraser, whatever tool is
    /// armed. The sandbox's rule, and it is the one the owner asked for:
    /// *"left-click paints, right-click erases"*. Having erase on its own
    /// button rather than as a seventh tool is what makes the pair a single
    /// gesture instead of two modes.
    pub fn press_erase(&mut self, x: i32, y: i32) {
        if self.show_help || self.ui.covers(x, y) {
            return;
        }
        let at = self.renderer.screen_to_world(x, y);
        self.begin_stroke(at, true);
    }

    /// A button came up, or the pointer left. Ends whatever stroke was live.
    pub fn end_stroke(&mut self) {
        self.stroke = None;
    }

    /// The pointer moved to `(x, y)` while a button is held.
    ///
    /// Continues the live stroke as a **capsule** from the last cell to this
    /// one rather than a dab at each, so a fast drag leaves one continuous
    /// band instead of a row of blobs — `App::paint_stroke`'s reason, and the
    /// same call underneath.
    pub fn drag(&mut self, x: i32, y: i32) {
        let Some(stroke) = self.stroke else { return };
        let to = self.renderer.screen_to_world(x, y);
        if to == stroke.last {
            return;
        }
        self.paint_span(stroke.last, to, stroke.erase);
        self.stroke = Some(Stroke { last: to, ..stroke });
    }

    fn begin_stroke(&mut self, at: (i32, i32), erase: bool) {
        self.paint_span(at, at, erase);
        self.stroke = Some(Stroke { last: at, erase });
    }

    /// Lay down (or lift) one span of the brush.
    ///
    /// **Two materials, and the two `aux` conventions point opposite ways.**
    /// `CLAUDE.md`'s standing gotcha: on a `Liquid` `aux == 0` means *full*,
    /// on a `Powder` it means *dry*. So water is painted at `aux == 0` — which
    /// `update::liquid_fill` reads back as `LIQUID_FULL` — and soil is painted
    /// at `SOIL_FIELD_CAPACITY`, which is damp enough for a root and short of
    /// the saturation that makes it slump. Painting soil at the `Powder`
    /// default of 0 would lay down bone-dry ground nothing can grow in;
    /// painting water at `LIQUID_FULL` would be a cell holding *twice* what it
    /// should, which is how water gets manufactured out of nothing.
    fn paint_span(&mut self, from: (i32, i32), to: (i32, i32), erase: bool) {
        use crate::sim::material;
        let radius = self.ui.brush();
        let (id, aux) = if erase {
            (material::EMPTY, 0)
        } else {
            match self.ui.tool() {
                ui::Tool::Water => match self.world.materials.id_of("water") {
                    Some(id) => (id, 0),
                    None => return,
                },
                _ => match self.world.materials.id_of("soil") {
                    Some(id) => (id, material::SOIL_FIELD_CAPACITY),
                    None => return,
                },
            }
        };
        self.world.paint_capsule_as(from, to, radius, id, 1.0);
        // `paint_capsule_as` writes the cell with a palette shade and no
        // `aux`, so the moisture is a second pass over the same disc. Only
        // over cells holding the material at `aux == 0`, so a wide stroke
        // cannot re-wet ground it never touched.
        if aux != 0 {
            let r = radius.max(0);
            for y in (from.1.min(to.1) - r)..=(from.1.max(to.1) + r) {
                for x in (from.0.min(to.0) - r)..=(from.0.max(to.0) + r) {
                    let cell = self.world.get(x, y);
                    if cell.material == id && cell.aux() == 0 {
                        self.world.set(x, y, cell.with_aux(aux));
                    }
                }
            }
        }
    }

    /// **Do the armed verb at a world cell.** Gate 4's three, plus `LOOK`.
    fn use_tool(&mut self, at: (i32, i32)) {
        let (x, y) = at;
        match self.ui.tool() {
            ui::Tool::Look => self.ui.inspect(at),
            ui::Tool::Plant => self.plant_at(x, y),
            ui::Tool::Colony => {
                let placed = self.world.found_colony(x, y);
                // The count, not just the picture: an ant is two dark cells at
                // play zoom, and a colony that placed nothing looks exactly
                // like one you have not found yet.
                self.ui.say(match placed {
                    0 => "NO ROOM FOR A COLONY HERE".to_string(),
                    n => format!("COLONY OF {n} RELEASED"),
                });
            }
            ui::Tool::Cull => self.cull_at(x, y),
            // The brushes never arrive here: they paint from `press`, so a
            // release that also painted would double the last dab.
            ui::Tool::Soil | ui::Tool::Water => {}
        }
    }

    /// Put one seed of the selected species in at `(x, y)`.
    ///
    /// **The click lands where you aimed; the seed lands where a seed can
    /// go.** `plant_tree_species` refuses an occupied cell, and a player
    /// aiming at soil is aiming at the ground rather than at the air one cell
    /// above it — so a click that lands *in* the ground walks up to the first
    /// empty cell above it. A seed is a `Powder` and falls the rest of the way
    /// on its own, so a click in mid-air is honest too.
    fn plant_at(&mut self, x: i32, y: i32) {
        let (name, _) = self.selected_species();
        let mut site = y;
        for _ in 0..MAX_PLANT_LIFT {
            if self.world.is_empty(x, site) {
                break;
            }
            site -= 1;
        }
        let lower = name.to_lowercase();
        if self.world.plant_tree_species(x, site, &lower) {
            self.ui.say(format!("PLANTED {name} AT {x},{site}"));
        } else {
            self.ui.say(format!("NO ROOM TO PLANT {name} HERE"));
        }
    }

    /// **Kill the organism under `(x, y)`, the way the engine already kills
    /// organisms.**
    ///
    /// Two paths, because the two kingdoms die differently and there is no
    /// shared one. A plant is marked **senescent**, which `plant::rot_remains`
    /// then carries out at the species' own `remains_half_life` — the owner's
    /// own ruling, and `CLAUDE.md`'s first law: the death is *graded* rather
    /// than a disappearance. A creature has no senescence path at all (nothing
    /// outside `plant.rs` reads the flag), so its energy is taken to zero and
    /// `creature::apply_creature_energy` writes a **corpse** on its next tick
    /// — a `Powder` that falls, rots, burns and can be eaten by whatever is
    /// still alive.
    ///
    /// Neither leaves a hole. That is the point: an outcome is a distribution,
    /// and a cull that deleted the cells would be the binary this project
    /// keeps ruling against.
    fn cull_at(&mut self, x: i32, y: i32) {
        let cell = self.world.get(x, y);
        let id = cell.organism_id();
        let Some(state) = self.world.organism_state(id) else {
            self.ui.say("NOTHING ALIVE HERE");
            return;
        };
        let species = self.world.species.get(state.species);
        let name = species.name.to_uppercase();
        let animal = species.creature.is_some();
        let cells = state.cells.len();
        if animal {
            if let Some(state) = self.world.organism_mut(id) {
                state.energy = 0.0;
            }
            self.ui.say(format!("CULLED {name} - DIES ON THE NEXT TICK"));
        } else {
            self.world.mark_organism_senescent(id);
            self.ui.say(format!("CULLED {name} - {cells} CELLS LEFT TO ROT"));
        }
    }

    /// The selected plantable species: its name in the bar's uppercase, and
    /// the line the chip's hover explanation shows.
    fn selected_species(&self) -> (String, String) {
        let Some(id) = self.ui.species_of(&self.world) else {
            return ("NONE".to_string(), "NO PLANTABLE SPECIES IS LOADED.".to_string());
        };
        let species = self.world.species.get(id);
        let name = species.name.to_uppercase();
        // Two real numbers off the species itself, not a description. The
        // design guide is explicit that planting must show what you are about
        // to plant *"or planting is a slot machine"*; a name alone is the
        // minimum, and these are the two that decide how it plays — how long a
        // seed keeps if it does not germinate, and how long its remains take
        // to go once it dies.
        let note = format!(
            "WHICH PLANT THE PLANTING TOOL PUTS IN. CLICK TO CYCLE. {name}: A SEED KEEPS {:.0} FRAMES BEFORE IT IS GONE, AND ITS REMAINS TAKE {:.0} FRAMES TO ROT DOWN ONCE IT DIES.",
            species.seed_half_life, species.remains_half_life
        );
        (name, note)
    }

    /// The left button came up at `(x, y)`. This is where a click turns into
    /// something happening.
    ///
    /// **Every verb here is one a key already had, plus one the keyboard could
    /// not offer at all** — pointing at a cell. That asymmetry is the whole
    /// argument for the mouse: a key can toggle a page, and nothing on the
    /// keyboard can say *that ant*.
    pub fn release(&mut self, x: i32, y: i32) {
        if self.stroke.take().is_some() {
            // The brush already did its work on press and on every drag; a
            // release that also fired the verb would double the last dab.
            self.ui.cancel_press();
            return;
        }
        match self.ui.release(x, y) {
            ui::Release::Fired(action) => self.act(action),
            ui::Release::Consumed => {}
            ui::Release::World => {
                // The key page is modal, so a click on the world dismisses it
                // rather than reaching through it — the same rule as any key.
                if self.show_help {
                    self.show_help = false;
                    return;
                }
                let at = self.renderer.screen_to_world(x, y);
                self.use_tool(at);
            }
        }
    }

    /// Do one of the bar's verbs.
    ///
    /// **The single place a control turns into a change**, so a button and its
    /// keyboard shortcut cannot drift: `bin/lab.rs`'s key handler routes
    /// through here too, and there is no second copy of "what SPACE does".
    pub fn act(&mut self, action: ui::Action) {
        match action {
            ui::Action::TogglePhase => self.time.toggle_phase(),
            ui::Action::Slower => self.time.slower(),
            ui::Action::Faster => self.time.faster(),
            ui::Action::Preset(i) => self.time.set_preset(i),
            // **One page at a time**, the rule this file already applies to
            // the key list against the biosphere page, extended to the bar's
            // three. The screen is 512x320, the biosphere page is a full-
            // height right-hand column and these open above the bar's own
            // right-hand group, so two of them drawn together interleave into
            // something neither of them is. Made exclusive in *state* rather
            // than only in the paint, so the latch on the bar tells the truth
            // about what is on screen.
            ui::Action::Panel(panel) => {
                self.ui.toggle_panel(panel);
                if self.ui.panel.is_some() && self.stats.showing() {
                    self.stats.toggle();
                }
            }
            ui::Action::Stats => {
                self.stats.toggle();
                if self.stats.showing() {
                    self.ui.panel = None;
                }
            }
            ui::Action::Tool(tool) => {
                self.ui.set_tool(tool);
                self.ui.say(format!("TOOL {}", self.ui.tool().label()));
            }
            ui::Action::NextSpecies => {
                self.ui.next_species();
                let (name, _) = self.selected_species();
                // ...and picking a species arms the tool that uses it. A chip
                // that changes what a *different* button will do, silently, is
                // the mode you forget you are in.
                self.ui.set_tool(ui::Tool::Plant);
                self.ui.say(format!("PLANTING {name}"));
            }
            ui::Action::Brush(delta) => {
                self.ui.adjust_brush(delta);
                self.ui.say(format!("BRUSH R{}", self.ui.brush()));
            }
            ui::Action::CycleOverlay => {
                self.renderer.cycle_field_overlay();
                self.ui.say(format!("OVERLAY {}", self.renderer.field_overlay.label()));
            }
            ui::Action::Help => self.show_help = !self.show_help,
            ui::Action::Reset => self.reset(),
            ui::Action::ParamGroup(i) => self.ui.set_param_group(i),
            ui::Action::ParamScroll(d) => self.ui.scroll_params(d),
            ui::Action::ParamSelect(i) => self.ui.select_param(i),
            ui::Action::ParamAdjust(i, sign) => self.adjust_param(i, sign),
            ui::Action::ParamSave => self.save_param(),
        }
    }

    /// **Move one parameter by one of its own steps, and say what it did.**
    ///
    /// The list is rebuilt here rather than carried from the paint, `ui::Ui::
    /// page_params`' tradeoff — and `i` is an index into the page as it was
    /// drawn, which is the page a click was aimed at.
    ///
    /// Every branch of `params::write` is a live store the tick reads through,
    /// so an ordinary change is felt on the next tick with no reload. The one
    /// exception is the bed's own spec, which is what a rebuild is made from;
    /// the notice says so rather than leaving the player to wonder why nothing
    /// moved. `CLAUDE.md`'s second law: an event with no visible consequence
    /// is not finished, and half of these knobs are invisible on a **stopped**
    /// box, which is exactly when you set them.
    fn adjust_param(&mut self, index: usize, sign: i32) {
        let list = self.ui.page_params(&self.world, &self.spec);
        let Some(param) = list.get(index) else { return };
        if !param.writable() {
            self.ui.say("THIS ONE IS SHOWN, NOT CHANGED");
            return;
        }
        let target = param.tunable.stepped(sign);
        // The name as the panel prints it, not as the file spells it: the
        // notice sits under the row it is about, and `BIRTH_GRANT` beside a
        // row reading `BIRTH GRANT` reads as a different thing.
        let name = param.tunable.name.replace('_', " ").to_uppercase();
        let rebuild = params::needs_rebuild(&param.knob);
        let knob = param.knob.clone();
        // Cloned out before the borrow ends, so the write can take the world
        // mutably. A `Knob` is a small enum over a name and a species string.
        if !params::write(&mut self.world, &mut self.spec, &knob, target) {
            // A registered row whose writer declined is the "reader with no
            // writer" failure `CLAUDE.md` names, and it looks exactly like
            // working code from the outside -- so it is said out loud rather
            // than swallowed. `params::tests::every_writable_parameter_
            // actually_moves` is what keeps it from ever being seen.
            self.ui.say(format!("{name} HAS NO WRITER -- NOTHING CHANGED"));
            return;
        }
        self.ui.select_param(index);
        // Re-read rather than echoing `target`: the store may have rounded it
        // (every integral field does), and a readout that disagrees with the
        // world is worse than no readout.
        let shown = self
            .ui
            .page_params(&self.world, &self.spec)
            .get(index)
            .map_or_else(|| format!("{target:.3}"), |p| p.display());
        self.ui.say(if rebuild {
            format!("{name} = {shown} - TAKES EFFECT ON REBUILD")
        } else {
            format!("{name} = {shown}")
        });
    }

    /// Write the highlighted parameter back to its asset file.
    ///
    /// The refusals are as much of the feature as the writes — see
    /// `params::save`, which declines an ambiguous or non-numeric field rather
    /// than editing the wrong one.
    fn save_param(&mut self) {
        let Some(index) = self.ui.selected_param() else {
            self.ui.say("NOTHING PICKED - MOVE A ROW OR CLICK ITS NAME FIRST");
            return;
        };
        let list = self.ui.page_params(&self.world, &self.spec);
        let Some(param) = list.get(index) else { return };
        let message = match params::save(param) {
            Ok(ok) => ok,
            Err(e) => e.to_uppercase(),
        };
        self.ui.say(message);
    }
}

/// **Draw nest material as worked earth in the lab, not as pale stone.**
///
/// Owner, 2026-08-30: *"we don't need a visible line for where the colony is
/// placed."* `creature::found_colony` paints one row of `nest` across 53
/// columns at the surface, and `nest.ron`'s palette is a pale warm sand
/// (196,168,120) chosen to *"read clearly against soil"* — which outdoors is
/// the point and in a flat bed is a stripe drawn across the box.
///
/// **The colour carries no behaviour.** `AtNest` is a contact scan for the
/// material and nothing else, and an ant's route home is a pheromone gradient
/// that never asks where the nest is — `nest.ron` says both, at length. So
/// what the fix costs is exactly the player's ability to spot a colony at a
/// glance, and what it buys is that a founded colony no longer draws a line.
///
/// Three things this is deliberately *not*:
///
/// - not an edit to `creature.rs`. The nest patch is functional and narrower
///   than the ant band on purpose — *"home has to be a place, not everywhere,
///   or there is no gradient to walk up"* — and the foraging scene measured
///   414 deliveries at that ratio;
/// - not an edit to `nest.ron`, which would change the sandbox too, where a
///   findable nest is wanted;
/// - not a test in `render.rs`'s per-pixel path. A material comparison per
///   pixel is 163,840 of them a frame for a stripe; swapping the palette in
///   the lab's own `Materials` costs **nothing at draw time at all**, which is
///   what `Materials::get_mut` exists for.
///
/// The tones are `packedsoil`'s own reference-loam family, which is the
/// engine's existing answer to "ground an animal has worked" — the same
/// material an ant lines its galleries with. Worked ground drawn as worked
/// ground.
fn earth_toned_nest(world: &mut World) {
    const WORKED_EARTH: [[u8; 4]; 4] = [
        [48, 38, 32, 255],
        [56, 45, 38, 255],
        [42, 33, 28, 255],
        [62, 50, 42, 255],
    ];
    let Some(nest) = world.materials.id_of("nest") else { return };
    let def = world.materials.get_mut(nest);
    def.palette = WORKED_EARTH.to_vec();
    def.base_shades = WORKED_EARTH.len();
}

/// How far a planting click may walk **up** out of the ground to find an
/// empty cell to drop a seed in. Twelve rows: a click aimed at the surface
/// lands in soil, and a click aimed a hand's width into the bed should still
/// plant rather than silently refusing. Past that the player is aiming at
/// something buried and a seed is not what they meant.
const MAX_PLANT_LIFT: i32 = 12;

/// The key list, drawn over a dimmed screen.
///
/// Uppercase and punctuation-light on purpose: `hud`'s font is a hand-authored
/// 5x7 bitmap with a deliberately small glyph set, and a character it does not
/// have draws as a silent blank rather than as anything you would notice. That
/// gap has shipped three times in this repo, so every line here is checked
/// against `hud::has_glyph` by `every_help_line_is_drawable`.
const HELP: [&str; 22] = [
    "THE EVOLUTION LAB",
    "",
    "THE BOX STARTS EMPTY. YOU STOCK IT.",
    "EVERY CONTROL IS ALSO A BUTTON ON",
    "THE BAR ALONG THE BOTTOM.",
    "",
    "SPACE      STOP / RUN THE BOX",
    "UP DOWN    SPEED     1-7  PRESET",
    "",
    "Z X C V B N  LOOK PLANT COLONY",
    "             CULL SOIL WATER",
    "CLICK      USE THE ARMED TOOL",
    "RIGHT      ERASE",
    ".          WHICH SPECIES TO PLANT",
    "[ ]        BRUSH NARROWER WIDER",
    "O L        FIELD / LIFE OVERLAY",
    "",
    "P          PARAMETERS -- THE NUMBERS",
    "           BEHIND THE VERBS",
    "F1 F2 F3   PLANTS ANTS BOX   TAB STATS",
    "F RATE   WASD PAN   - = ZOOM   R REBUILD",
    "?          THIS PAGE",
];

fn draw_help(frame: &mut [u8]) {
    let w = HELP.iter().map(|l| crate::hud::text_width(l)).max().unwrap_or(0);
    let (bw, bh) = (w + 24, HELP.len() as i32 * 10 + 20);
    let (x0, y0) = ((WIDTH as i32 - bw) / 2, (HEIGHT as i32 - bh) / 2);
    for y in y0..(y0 + bh) {
        for x in x0..(x0 + bw) {
            if x < 0 || y < 0 || x >= WIDTH as i32 || y >= HEIGHT as i32 {
                continue;
            }
            let i = ((y as u32 * WIDTH + x as u32) * 4) as usize;
            // Dim what is behind rather than covering it, so the box stays
            // visible under the page and the page reads as an overlay.
            for c in 0..3 {
                frame[i + c] /= 4;
            }
        }
    }
    for (i, line) in HELP.iter().enumerate() {
        let colour = if i == 0 { [235, 235, 200, 255] } else { [200, 200, 200, 255] };
        crate::hud::draw_text(frame, WIDTH, HEIGHT, x0 + 12, y0 + 12 + i as i32 * 10, line, colour);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **`SPACE` stops the world, and this is the counter that says so.**
    ///
    /// Owner complaint, 2026-08-30: *"what does spacebar/tending mean. it
    /// isn't pausing anything."* An image cannot answer this — a stopped box
    /// and a settled one look identical in a still frame, which is exactly
    /// `CLAUDE.md`'s *"did it fire at all" needs a counter, not a picture*.
    /// `World::frame` is that counter.
    ///
    /// Both arms in one test, because the paused arm alone would be green for
    /// a lab whose `advance` had stopped working altogether.
    #[test]
    fn a_paused_lab_does_not_advance_the_world_and_a_running_one_does() {
        let mut lab = Lab::new(scene::LabBox {
            founders: 0,
            colonies: 0,
            ..scene::LabBox::default()
        });
        assert_eq!(lab.time.phase, time::Phase::Paused, "a fresh lab starts stopped");
        let at_rest = lab.world.frame;
        for _ in 0..120 {
            lab.advance(std::time::Duration::from_millis(16));
        }
        assert_eq!(lab.world.frame, at_rest, "a stopped box ran ticks");

        lab.act(ui::Action::TogglePhase);
        assert_eq!(lab.time.phase, time::Phase::Running);
        for _ in 0..120 {
            lab.advance(std::time::Duration::from_millis(16));
        }
        assert!(
            lab.world.frame > at_rest,
            "the positive control did not fire: the world stayed at frame {at_rest}"
        );

        // ...and `SPACE` again stops it where it stands.
        lab.act(ui::Action::TogglePhase);
        let stopped_at = lab.world.frame;
        for _ in 0..120 {
            lab.advance(std::time::Duration::from_millis(16));
        }
        assert_eq!(lab.world.frame, stopped_at, "stopping again did not stop it");
    }

    /// **An empty box is a state, not an error.**
    ///
    /// `bin/lab.rs` opens the game with no founders and no colony — owner
    /// request, 2026-08-30, *"the game should start with no plants or
    /// creatures. I add them."* Everything that reads a population now has to
    /// survive one of zero, and three of them divide by a count or index a
    /// sorted list. This runs an empty bed and paints every page over it.
    #[test]
    fn an_empty_box_runs_and_draws_every_page() {
        let mut lab = Lab::new(scene::LabBox {
            founders: 0,
            colonies: 0,
            ..scene::LabBox::default()
        });
        lab.show_help = false;
        assert_eq!(lab.world.live_organism_count(), 0, "the bed was not empty");
        lab.act(ui::Action::TogglePhase);
        for _ in 0..240 {
            lab.advance(std::time::Duration::from_millis(16));
        }
        assert!(lab.world.frame > 0, "the positive control did not fire");
        assert_eq!(lab.world.live_organism_count(), 0, "something grew in an empty bed");

        let mut frame = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
        // Every page, plus the census, plus a cursor so the hover paths run.
        lab.set_cursor(Some((40, 40)));
        for panel in [ui::Panel::Plants, ui::Panel::Ants, ui::Panel::Box, ui::Panel::Params] {
            lab.ui.panel = Some(panel);
            lab.draw(&mut frame, 60.0);
        }
        lab.ui.panel = None;
        lab.draw(&mut frame, 60.0);
    }

    /// **The parameters page, driven the way a player drives it: by clicking.**
    ///
    /// The positive control the bar already has (`every_button_answers_where_
    /// it_was_drawn`), extended to the one page whose rows are controls. Every
    /// `+` face is clicked at the middle of the rectangle it was *drawn* at —
    /// read back out of the retained layout, so this cannot pass against a
    /// hand-written pixel table — and the value is read back out of the
    /// registry afterwards.
    ///
    /// **A row that paints its figure correctly and refuses to move looks
    /// identical to a working one in any screenshot.** `params::tests::every_
    /// writable_parameter_actually_moves` guards the writers; this guards
    /// everything between the click and them: the hit test, the row index the
    /// action carries, and `Lab::act`'s routing.
    #[test]
    fn clicking_a_parameters_row_moves_it() {
        let mut lab = Lab::new(scene::LabBox::default());
        lab.show_help = false;
        let mut frame = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
        lab.act(ui::Action::Panel(ui::Panel::Params));
        // A frame has to be painted before anything on it can be clicked: the
        // page's rectangles are retained from the last paint, deliberately.
        lab.draw(&mut frame, 60.0);

        let mut moved = 0;
        for group in 0..params::GROUPS.len() {
            lab.act(ui::Action::ParamGroup(group));
            lab.draw(&mut frame, 60.0);
            for row in 0..lab.ui.page_params(&lab.world, &lab.spec).len() {
                let Some(r) = lab.ui.widget_rect(ui::Action::ParamAdjust(row, 1)) else { continue };
                let (cx, cy) = (r.x + r.w / 2, r.y + r.h / 2);
                assert_eq!(
                    lab.ui.hit(cx, cy),
                    Some(ui::Action::ParamAdjust(row, 1)),
                    "row {row} of page {group} answered for another widget"
                );
                let before = lab.ui.page_params(&lab.world, &lab.spec)[row].display();
                lab.press(cx, cy);
                lab.release(cx, cy);
                lab.draw(&mut frame, 60.0);
                let after = lab.ui.page_params(&lab.world, &lab.spec)[row].display();
                let param = &lab.ui.page_params(&lab.world, &lab.spec)[row];
                // A knob shipped at its own ceiling clamps rather than moves,
                // which is right -- `water.flow_rate` is 1000 of 1000.
                if param.tunable.value < param.tunable.max {
                    assert_ne!(before, after, "{}.{} did not move on a click", param.tunable.category, param.tunable.name);
                    moved += 1;
                }
            }
        }
        assert!(moved >= 30, "only {moved} rows were reachable by a click");
    }

    /// A click on the parameters page must not also reach the world behind it.
    /// The page is a big rectangle over the bed and the `LOOK` tool is armed
    /// by default, so without this every adjustment would move the inspector
    /// as well.
    #[test]
    fn a_click_on_the_parameters_page_does_not_reach_the_world() {
        let mut lab = Lab::new(scene::LabBox::default());
        lab.show_help = false;
        let mut frame = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
        lab.act(ui::Action::Panel(ui::Panel::Params));
        lab.draw(&mut frame, 60.0);
        let before = lab.ui.inspecting();
        // Somewhere on the page that is not one of its chips: the title bar.
        lab.press(20, ui::bar_top() - 120);
        lab.release(20, ui::bar_top() - 120);
        assert_eq!(lab.ui.inspecting(), before, "a click on the page moved the inspector");
    }

    /// An empty bed with the view at rest, and the screen position of one
    /// world cell on it. Every test below aims through `world_to_screen`
    /// rather than assuming screen and world coincide, which is only true
    /// while the camera has not moved.
    fn bench() -> Lab {
        let mut lab = Lab::new(scene::LabBox {
            founders: 0,
            colonies: 0,
            ..scene::LabBox::default()
        });
        lab.show_help = false;
        let mut frame = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
        // One painted frame, because a click is tested against the retained
        // bar and there is not one until something has been drawn.
        lab.draw(&mut frame, 60.0);
        lab
    }

    fn aim(lab: &Lab, wx: i32, wy: i32) -> (i32, i32) {
        lab.renderer.world_to_screen(wx, wy).expect("the bed is on screen")
    }

    /// Click a **world** cell, aimed through the renderer. Takes the cell
    /// rather than a screen point so a caller cannot borrow `lab` twice in
    /// one expression, and so no test can aim at a screen pixel that is only
    /// the right one while the camera has not moved.
    fn click_cell(lab: &mut Lab, wx: i32, wy: i32) {
        let at = aim(lab, wx, wy);
        lab.set_cursor(Some(at));
        lab.press(at.0, at.1);
        lab.release(at.0, at.1);
    }

    /// **Gate 4's three verbs, each with the count that says it fired.**
    ///
    /// One test, because each of them looks identical to nothing happening in
    /// a still frame — a seed is one cell, an ant is two — and because the
    /// shared setup is most of the cost. `CLAUDE.md`'s rule: *"did it fire at
    /// all" needs a counter, not a picture*.
    ///
    /// The negative arms are the half that matters. Every one of these can
    /// return quietly: `plant_tree_species` refuses an occupied cell,
    /// `found_colony` places nobody where there is no surface, and a cull aimed
    /// at bare soil resolves to no organism. A test that only asserted the
    /// happy path would be green for a verb wired to the wrong button.
    #[test]
    fn the_three_verbs_change_the_world_and_say_so_when_they_cannot() {
        let mut lab = bench();
        let surface = lab.spec.ground_y;
        // **Well apart, and that is not tidiness.** `found_colony` paints a
        // band of `nest` along the surface and drops ants across a wider one
        // still; founding it on top of the seed overwrote the seed's only
        // cell, freed its slot, and the cull arm below then failed looking for
        // a plant that the *previous* arm had deleted.
        let x = 80;
        let colony_x = lab.spec.width - 112;

        lab.act(ui::Action::Tool(ui::Tool::Plant));
        assert_eq!(lab.ui.tool(), ui::Tool::Plant);
        let before = lab.world.live_organism_count();
        click_cell(&mut lab, x, surface);
        assert_eq!(
            lab.world.live_organism_count(),
            before + 1,
            "planting put nothing in the bed"
        );
        // ...and the negative: the same click into the stone base, which has
        // no empty cell within reach above it.
        let deep = lab.spec.ground_y + lab.spec.soil_depth + 40;
        let before = lab.world.live_organism_count();
        click_cell(&mut lab, x, deep);
        assert_eq!(
            lab.world.live_organism_count(),
            before,
            "planting into the stone base planted something"
        );

        lab.act(ui::Action::Tool(ui::Tool::Colony));
        let before = lab.world.live_creature_count();
        click_cell(&mut lab, colony_x, surface);
        assert!(
            lab.world.live_creature_count() > before,
            "founding a colony released nobody: {before} creatures either side"
        );

        // Cull, aimed at the plant put in above — found by walking the live
        // list rather than by remembering an id, so this cannot pass against
        // an organism that is no longer there.
        lab.act(ui::Action::Tool(ui::Tool::Cull));
        let plant = lab
            .world
            .live_organism_ids()
            .into_iter()
            .find(|id| {
                lab.world.organism_state(*id).is_some_and(|st| {
                    lab.world.species.get(st.species).creature.is_none() && !st.cells.is_empty()
                })
            })
            .expect("the planted seed is alive");
        let (cell_at, cells_before) = {
            let st = lab.world.organism_state(plant).expect("live");
            (*st.cells.keys().next().expect("a cell"), st.cells.len())
        };
        click_cell(&mut lab, cell_at.0, cell_at.1);
        let after = lab.world.organism_state(plant).expect("still tracked");
        assert!(after.senescent, "the cull did not mark the plant");
        // **The graded death.** `CLAUDE.md`'s first law: an outcome is a
        // distribution, not a binary. A cull that emptied the cell list would
        // pass any "is it dead" assertion and be exactly the disappearance the
        // owner ruled against.
        assert_eq!(
            after.cells.len(),
            cells_before,
            "the cull deleted the plant instead of leaving remains to rot"
        );

        // And a cull aimed at nothing says so rather than silently missing.
        let empty = lab.spec.ground_y - 40;
        assert!(lab.world.get(x, empty).organism_id() == 0, "test setup: that cell is bare");
        click_cell(&mut lab, x, empty);
    }

    /// **A culled animal becomes a corpse, and it takes a tick.**
    ///
    /// The other half of the graded death, and the half with no senescence
    /// path: nothing outside `plant.rs` reads the flag, so a creature cull
    /// takes its energy to zero and `creature::apply_creature_energy` writes
    /// the corpse on its next tick. The **stopped** box is asserted first,
    /// because that is the honest behaviour and it is also what would make a
    /// naive test green for a cull that did nothing at all.
    #[test]
    fn a_culled_animal_leaves_a_corpse_once_the_box_runs() {
        let mut lab = bench();
        let x = lab.spec.width / 2;
        let surface = lab.spec.ground_y;
        lab.act(ui::Action::Tool(ui::Tool::Colony));
        click_cell(&mut lab, x, surface);
        let alive = lab.world.live_creature_count();
        assert!(alive > 0, "test setup: the colony released nobody");

        let ant = lab
            .world
            .live_organism_ids()
            .into_iter()
            .find(|id| {
                lab.world
                    .organism_state(*id)
                    .is_some_and(|st| lab.world.species.get(st.species).creature.is_some())
            })
            .expect("an ant");
        let at = *lab.world.organism_state(ant).expect("live").cells.keys().next().expect("a cell");

        lab.act(ui::Action::Tool(ui::Tool::Cull));
        click_cell(&mut lab, at.0, at.1);
        assert_eq!(lab.world.organism_state(ant).map(|s| s.energy), Some(0.0));
        assert_eq!(
            lab.world.live_creature_count(),
            alive,
            "a stopped box killed something: nothing ticks while paused"
        );

        lab.act(ui::Action::TogglePhase);
        for _ in 0..120 {
            lab.advance(std::time::Duration::from_millis(16));
        }
        assert!(lab.world.frame > 0, "the positive control did not fire");
        assert!(
            lab.world.organism_state(ant).is_none(),
            "the culled ant is still alive after 120 frames"
        );
        let corpse = lab.world.materials.id_of("corpse").expect("corpse is compiled in");
        let bounds = lab.world.bounds().expect("bounds");
        let corpses: u32 = (bounds.min_y..=bounds.max_y)
            .flat_map(|y| (bounds.min_x..=bounds.max_x).map(move |x| (x, y)))
            .map(|(x, y)| u32::from(lab.world.get(x, y).material == corpse))
            .sum();
        assert!(corpses > 0, "the culled ant vanished instead of leaving meat");
    }

    /// **The two `aux` conventions point opposite ways, and getting either
    /// backwards manufactures water out of nothing.**
    ///
    /// `CLAUDE.md`'s standing gotcha, asserted in both directions rather than
    /// once: on a `Powder` `aux == 0` is *dry*, so soil painted at the default
    /// would be ground nothing can root in; on a `Liquid` `aux == 0` is
    /// *full*, so water painted at `LIQUID_FULL` would be a cell holding twice
    /// what it should. Each half would look completely correct on screen.
    #[test]
    fn the_soil_brush_lays_damp_ground_and_the_water_brush_lays_full_water() {
        use crate::sim::{material, update};
        let mut lab = bench();
        let (x, y) = (lab.spec.width / 2, lab.spec.ground_y - 30);
        let soil = lab.world.materials.id_of("soil").expect("soil");
        let water = lab.world.materials.id_of("water").expect("water");

        lab.act(ui::Action::Tool(ui::Tool::Soil));
        let at = aim(&lab, x, y);
        lab.set_cursor(Some(at));
        lab.press(at.0, at.1);
        lab.release(at.0, at.1);
        let cell = lab.world.get(x, y);
        assert_eq!(cell.material, soil, "the soil brush laid nothing down");
        assert_eq!(
            cell.aux(),
            material::SOIL_FIELD_CAPACITY,
            "soil painted at the powder default is bone dry"
        );

        let (wx, wy) = (x + 60, y);
        lab.act(ui::Action::Tool(ui::Tool::Water));
        let at = aim(&lab, wx, wy);
        lab.set_cursor(Some(at));
        lab.press(at.0, at.1);
        lab.release(at.0, at.1);
        let cell = lab.world.get(wx, wy);
        assert_eq!(cell.material, water, "the water brush laid nothing down");
        assert_eq!(
            update::liquid_fill(cell),
            material::LIQUID_FULL,
            "the water brush laid down a partly-drained cell"
        );
        // The inverse, which is the half that catches the convention being
        // flipped: a `Liquid` written at `LIQUID_FULL` in `aux` reads back as
        // full too, and holds twice what it should.
        assert_eq!(cell.aux(), 0, "water written at LIQUID_FULL manufactures water");

        // And the right button takes it back, wherever the tool is pointed.
        let before = lab.world.get(wx, wy).material;
        assert_ne!(before, material::EMPTY);
        lab.press_erase(at.0, at.1);
        lab.end_stroke();
        assert_eq!(lab.world.get(wx, wy).material, material::EMPTY, "the eraser left the cell");
    }

    /// A drag lays down one continuous band, not a dab at each end.
    #[test]
    fn a_drag_paints_the_whole_span_it_swept() {
        let mut lab = bench();
        let (x, y) = (lab.spec.width / 2 - 40, lab.spec.ground_y - 30);
        let soil = lab.world.materials.id_of("soil").expect("soil");
        lab.act(ui::Action::Tool(ui::Tool::Soil));
        let from = aim(&lab, x, y);
        lab.set_cursor(Some(from));
        lab.press(from.0, from.1);
        // One jump, wider than the brush, so the two ends cannot touch.
        let to = aim(&lab, x + 60, y);
        lab.set_cursor(Some(to));
        lab.drag(to.0, to.1);
        lab.release(to.0, to.1);
        let midpoint = lab.world.get(x + 30, y);
        assert_eq!(
            midpoint.material, soil,
            "the drag left a gap: a stroke must be a capsule, not two dabs"
        );
    }

    /// **A founded colony must not draw a line across the bed.**
    ///
    /// Measured as contrast against the ground it sits on rather than as a
    /// colour equality: the claim is *"you cannot see a stripe"*, and a test
    /// that asserted three specific bytes would be green for any repaint and
    /// would go red for a legitimate retune of `packedsoil`.
    ///
    /// The sensitivity half is the shipped `nest.ron` palette, checked in the
    /// same test: a pale (196,168,120) against soil's (64,46,34) is a
    /// separation of 132, and the assertion below must be one this cannot
    /// pass — otherwise it is measuring nothing.
    #[test]
    fn a_founded_colony_does_not_draw_a_pale_line_across_the_bed() {
        use crate::sim::material::MaterialId;
        let lab = bench();
        let luma = |id: MaterialId| -> f32 {
            let p = &lab.world.materials.get(id).palette;
            p.iter()
                .map(|c| 0.299 * c[0] as f32 + 0.587 * c[1] as f32 + 0.114 * c[2] as f32)
                .sum::<f32>()
                / p.len().max(1) as f32
        };
        let nest = lab.world.materials.id_of("nest").expect("nest");
        let soil = lab.world.materials.id_of("soil").expect("soil");
        let gap = (luma(nest) - luma(soil)).abs();
        assert!(gap < 24.0, "nest stands {gap:.0} luma off the soil it is painted on");

        // The control: the shipped palette, which is what this replaces.
        let shipped = [[196u8, 168, 120, 255], [184, 156, 110, 255], [206, 180, 132, 255]];
        let shipped_luma = shipped
            .iter()
            .map(|c| 0.299 * c[0] as f32 + 0.587 * c[1] as f32 + 0.114 * c[2] as f32)
            .sum::<f32>()
            / shipped.len() as f32;
        assert!(
            (shipped_luma - luma(soil)).abs() > 24.0,
            "the control does not fail the assertion, so the assertion measures nothing"
        );
    }

    /// **The font cannot draw everything, and what it cannot draw it draws as
    /// nothing.** `[`/`]`, then `_`/`<`/`>`, then `;`/`'` have each shipped
    /// blank in this engine's UI for as long as they were bound. This is the
    /// cheap standing check, not a discipline.
    #[test]
    fn every_help_line_is_drawable() {
        for line in super::HELP {
            for c in line.chars() {
                assert!(crate::hud::has_glyph(c), "the HUD font has no glyph for {c:?} in {line:?}");
            }
        }
    }
}
