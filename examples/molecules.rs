//! **What a candidate scent molecule looks like on the ground** -- the
//! evolvable-pheromones lane's picture of the proposed palette
//! (`Reports/evolvable-pheromones-2026-10-25.md`).
//!
//! One ant-walk's worth of trail (a [`pheromone::DEPOSIT`] on each of 120
//! cells, laid once and never again) is put down on a separate plane per
//! molecule, and the planes are snapshotted as they fade and spread. Each
//! molecule is a (fade per pass, blend per pass) pair under the engine's own
//! unchanged plane physics; the alarm row is the shipped alarm plane hit by
//! one bite, for scale.
//!
//! ```text
//! cargo run --release --example molecules -- out=/tmp/mol
//! python3 scripts/molecules_sheet.py /tmp/mol out.png
//! ```
//!
//! Writes `<out>/<row>_<frame>.u16` (little-endian, W x H) and prints per
//! snapshot the peak, the trail's width at mid-length (rows over one old
//! `u8` unit) and the total left on the plane.
use pixel_physics::sim::chunk::Rect;
use pixel_physics::sim::pheromone::{self, Channel, Pheromones, Scent, Spread};
use std::io::Write;

const W: i32 = 160;
const H: i32 = 48;
const TRAIL_Y: i32 = H / 2;
const X0: i32 = 20;
const LEN: i32 = 120;
const FRAMES: [u64; 5] = [0, 60, 180, 1200, 4800];

/// How a molecule moves: lying on the ground (the trail planes' conserving
/// blend) or in the air (the alarm plane's active space around a source).
#[derive(Clone, Copy)]
enum Law {
    /// (fade per pass, blend per pass)
    Ground(f32, f32),
    /// (fade per pass, loss per cell of distance, of 255)
    Air(f32, u16),
}

const ROWS: [(&str, Law); 7] = [
    ("road", Law::Ground(0.0, 0.05)),
    ("today_A", Law::Ground(pheromone::TRAIL_A_RHO, pheromone::DIFFUSE)),
    ("today_B", Law::Ground(pheromone::DECAY_RHO, pheromone::DIFFUSE)),
    ("fresh", Law::Ground(0.10, 0.25)),
    ("flash", Law::Ground(0.25, 1.0)),
    ("alarm", Law::Air(pheromone::ALARM_RHO, 12)),
    ("call", Law::Air(0.10, 4)),
];

fn main() {
    let mut out = String::from("molecules-out");
    for arg in std::env::args().skip(1) {
        if let Some(v) = arg.strip_prefix("out=") {
            out = v.into();
        } else {
            eprintln!("molecules: unknown argument {arg}");
            std::process::exit(2);
        }
    }
    std::fs::create_dir_all(&out).expect("out dir");
    println!("molecules: out={out} trail {LEN} cells x DEPOSIT {} laid once; interval {}", pheromone::DEPOSIT / pheromone::SCALE, pheromone::PHEROMONE_INTERVAL);
    println!("{:<10} {:>6} {:>7} {:>6} {:>10}", "molecule", "frame", "peak", "width", "total");
    for (label, params) in ROWS {
        let mut p = Pheromones::new(Rect::new(0, 0, W - 1, H - 1));
        let ch = match params {
            Law::Ground(rho, diffuse) => {
                p.set_channel_rho(Channel::A, rho);
                p.set_channel_diffuse(Channel::A, diffuse);
                for x in X0..X0 + LEN {
                    p.deposit(Channel::A, x, TRAIL_Y, pheromone::DEPOSIT);
                }
                Channel::A
            }
            Law::Air(rho, fall) => {
                p.set_alarm_rho(rho);
                p.set_alarm_spread(Spread::ActiveSpace { fall: fall * pheromone::SCALE });
                p.deposit(Channel::Alarm, X0 + LEN / 2, TRAIL_Y, pheromone::ALARM_DEPOSIT);
                Channel::Alarm
            }
        };
        let mut frame = 0u64;
        for &snap in &FRAMES {
            while frame < snap {
                frame += 1;
                p.step(frame, pheromone::PHEROMONE_INTERVAL);
            }
            let mut bytes = Vec::with_capacity((W * H * 2) as usize);
            let mut peak: Scent = 0;
            let mut total = 0u64;
            for y in 0..H {
                for x in 0..W {
                    let v = p.sample(ch, x, y);
                    peak = peak.max(v);
                    total += v as u64;
                    bytes.extend_from_slice(&v.to_le_bytes());
                }
            }
            let width = (0..H).filter(|&y| p.sample(ch, X0 + LEN / 2, y) >= pheromone::SCALE).count();
            std::fs::File::create(format!("{out}/{label}_{snap}.u16")).and_then(|mut f| f.write_all(&bytes)).expect("write");
            println!("{label:<10} {snap:>6} {:>7.1} {width:>6} {:>10.0}", peak as f32 / pheromone::SCALE as f32, total as f64 / pheromone::SCALE as f64);
        }
    }
}
