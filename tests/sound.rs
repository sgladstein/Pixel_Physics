//! **Sound never changes the world** -- the guards for stage 1 of
//! `Reports/sound-design-2026-10-31.md`.
//!
//! The design's promise is that sound is a reader: it cannot move a cell, a
//! counter or a random draw, so a run with the listener on is the same run
//! with it off, and training (which never builds a listener) cannot be
//! reached by it at all. Half of that is proved by the type system -- the
//! listener takes `&World` -- and half by the run below, through the real
//! lab seam (`Lab::draw` runs the listener inside `Lab::paint`, exactly
//! where `bin/lab.rs` runs it), including the tool sounds, which read the
//! world from inside a stroke.
//!
//! **The positive control** is the same run with one extra draw from the
//! world's own random stream per frame: the cheapest possible leak. If the
//! hash could not see that, it could not see the listener either.

use pixel_physics::audio::listen::{Listener, View};
use pixel_physics::audio::synth::VoiceBank;
use pixel_physics::lab::scene::LabBox;
use pixel_physics::lab::ui::{Action, Tool};
use pixel_physics::lab::Lab;
use pixel_physics::sim::world::World;

const FRAMES: u64 = 240;
const TICKS_PER_FRAME: u64 = 4;
const CHECK_EVERY: u64 = 40;

fn fnv1a(h: u64, v: u64) -> u64 {
    (h ^ v).wrapping_mul(0x0000_0100_0000_01b3)
}

/// `tests/determinism.rs`'s `lab_hash`, plus the world's random stream: a
/// leak that draws from it is the likeliest one, and the cells would only
/// show it once something had grown differently.
fn world_hash(w: &World) -> u64 {
    let b = w.bounds().expect("the lab bed sets bounds");
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for y in b.min_y..=b.max_y {
        for x in b.min_x..=b.max_x {
            let c = w.get(x, y);
            h = fnv1a(h, c.material.0 as u64);
            h = fnv1a(h, c.shade as u64);
            h = fnv1a(h, c.aux() as u64);
            h = fnv1a(h, c.organism_id() as u64);
            h = fnv1a(h, c.temperature() as u16 as u64);
        }
    }
    let (born, died) = w.organism_turnover();
    for v in [w.live_organism_count() as u64, w.live_creature_count() as u64, born, died, w.germinations] {
        h = fnv1a(h, v);
    }
    let mut probe = w.rng.clone();
    fnv1a(h, probe.next_u64())
}

struct Run {
    hashes: Vec<u64>,
    ants_heard: u32,
    observes: u64,
    verbs: u64,
    commands: usize,
}

/// The shipped bed through the lab seam: ticks, a water stroke and a soil
/// click from the mouse's own route, and a draw every frame.
fn run(listen: bool, leak: bool) -> Run {
    let mut lab = Lab::new(LabBox::default());
    lab.show_help = false;
    if listen {
        lab.enable_sound();
    }
    let (w, h) = lab.viewport();
    let mut buf = vec![0u8; (w * h * 4) as usize];
    let mut hashes = Vec::new();
    let mut commands = 0;
    for f in 0..FRAMES {
        if f == 30 || f == 120 {
            let tool = if f == 30 { Tool::Water } else { Tool::Soil };
            lab.act(Action::Tool(tool));
            lab.set_cursor(Some((200, 40)));
            lab.press(200, 40);
            for k in 1..=6 {
                lab.drag(200 + 5 * k, 40);
            }
            lab.release(230, 40);
            lab.set_cursor(None);
            lab.act(Action::Tool(Tool::Look));
        }
        for _ in 0..TICKS_PER_FRAME {
            lab.tick_for_harness();
        }
        lab.draw(&mut buf, 30.0);
        commands += lab.sound.drain().count();
        if leak {
            lab.world.rng.next_u64();
        }
        if (f + 1) % CHECK_EVERY == 0 {
            hashes.push(world_hash(&lab.world));
        }
    }
    let s = &lab.sound.stats;
    Run { hashes, ants_heard: s.ants, observes: s.observes, verbs: s.verbs, commands }
}

#[test]
fn listening_to_the_box_leaves_it_exactly_as_it_was() {
    let silent = run(false, false);
    let heard = run(true, false);
    // Vacuity first: a listener that heard nothing proves nothing.
    assert_eq!(silent.observes, 0, "the silent arm ran the listener anyway");
    assert_eq!(heard.observes, FRAMES, "the listener did not run once per drawn frame");
    assert!(heard.ants_heard > 0, "no ants heard -- the colony half of the listener is not under this test");
    assert!(heard.verbs >= 2, "the tool sounds never fired, so the stroke path is not under this test");
    assert!(heard.commands > FRAMES as usize, "the listener sent almost nothing");
    assert_eq!(silent.hashes, heard.hashes, "the world with sound on diverged from the world with it off");
}

/// The positive control: one extra random draw a frame must be seen.
#[test]
fn the_hash_sees_a_single_extra_random_draw() {
    let clean = run(false, false);
    let leaky = run(false, true);
    assert_ne!(clean.hashes[0], leaky.hashes[0], "the hash is blind to a leak through the world's random stream");
}

/// The offline render is a pure function of the box, the seed and the
/// commands -- the property that lets `examples/soundtrack.rs` stand in for
/// the speaker on the listening page.
#[test]
fn the_same_box_renders_the_same_samples() {
    let render = |seed: u64| -> (Vec<f32>, u64) {
        let mut lab = Lab::new(LabBox::default());
        lab.show_help = false;
        let mut listener = Listener::new();
        let mut bank = VoiceBank::new(48_000, seed);
        let view = View { x0: 0, y0: 0, w: lab.spec.width, h: lab.spec.height, surface_y: lab.spec.ground_y };
        let mut out = Vec::new();
        let mut block = vec![0.0f32; 1600 * 2];
        for _ in 0..120 {
            for _ in 0..TICKS_PER_FRAME {
                lab.tick_for_harness();
            }
            listener.observe(&lab.world, view, 1.0 / 30.0);
            for c in listener.drain() {
                bank.apply(c);
            }
            bank.render(&mut block, 2);
            out.extend_from_slice(&block);
        }
        (out, bank.stats.grains)
    };
    let (a, grains) = render(7);
    let (b, _) = render(7);
    let (c, _) = render(8);
    assert!(grains > 0, "no grains played -- the render is the room tone alone and proves little");
    assert!(a == b, "the same box and seed rendered different samples");
    assert!(a != c, "a different seed rendered identical samples -- the seed is not reaching the voices");
    let peak = a.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    assert!(peak < 1.0, "the limiter let the mix reach full scale ({peak})");
    assert!(peak > 0.01, "the box is all but silent ({peak})");
}
