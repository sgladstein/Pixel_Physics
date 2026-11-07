//! **The mine air**: a buoyant gas on the fine grid, carried by a small
//! incompressible draught.
//!
//! Why a fluid solve and not a diffusing field. The brief's first sketch was
//! "rises, spreads" -- a diffusing scalar with an upward bias. That cannot do
//! the one thing the level is built round: gas under a *sloping* roof has to
//! run uphill along it to the high end, where lamp A hangs. Diffusion spreads
//! as the square root of time (about 30 cells in 30 s at the stable limit),
//! so a gas that only diffuses never crosses a 200-cell gallery inside a
//! level. A buoyancy force plus a pressure projection turns "up, but the roof
//! is in the way" into "along the roof, uphill" for free, which is the
//! ceiling-jet real firedamp does, and it is also what makes Blow look like
//! a draught rather than a smear: the push is a velocity, the gas rides it,
//! and the tunnel walls steer it.
//!
//! The grid is a staggered (MAC) one: `u` lives on the vertical faces between
//! cells, `v` on the horizontal ones, the pressure and the gas in the cells.
//! A face is open when both cells beside it hold air. The only face to the
//! outside is the top of the air shaft (`vent`), held at pressure zero, so it
//! is the one place gas leaves the mine.
//!
//! `engine.rs`'s air grid (`src/sim/field.rs`) is not used: it works in 16x16
//! blocks, too coarse to steer anything down an 8-cell shaft (brief,
//! "Engine facts already checked").

pub const W: usize = 256;
pub const H: usize = 160;

/// The burn band, in the gas's own concentration units. Below `THIN` the
/// gas is drawn pale blue and will not light; between them pink, and a flame
/// runs through it; above `THICK` deep purple, too rich to burn. The units
/// are arbitrary (real firedamp's band is about 5-15% in air); what matters
/// on screen is only that the three colours sit where these do.
pub const THIN: f32 = 0.05;
pub const THICK: f32 = 0.15;
/// Below this nothing is drawn at all.
pub const TRACE: f32 = 0.03;

/// Buoyancy: velocity gained per tick per unit of gas, upward.
const BUOYANCY: f32 = 0.03;
/// Extra buoyancy per unit of `heat`, so a burn pulls a plume up behind it.
const HEAT_BUOYANCY: f32 = 0.02;
/// Fraction of velocity kept each tick.
const DAMPING: f32 = 0.99;
/// Plain mixing between neighbouring cells, per tick.
const MIX: f32 = 0.006;
/// The largest speed a face may carry, cells per tick. Donor-cell advection
/// is only stable below 0.5; a little under it leaves margin for two faces
/// draining one cell.
const MAX_SPEED: f32 = 0.42;
/// SOR sweeps of the pressure solve per tick. Warm-started from last tick's
/// pressure, which is why so few are enough for a flow that changes slowly.
const SOLVE_ITERS: usize = 24;
const OMEGA: f32 = 1.75;

pub fn idx(x: usize, y: usize) -> usize {
    y * W + x
}

pub fn band(c: f32) -> Band {
    if c < TRACE {
        Band::Clear
    } else if c < THIN {
        Band::Thin
    } else if c <= THICK {
        Band::Burns
    } else {
        Band::Thick
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Band {
    Clear,
    Thin,
    Burns,
    Thick,
}

impl Band {
    /// What the screen calls it. Plain world words: the brief bans jargon.
    pub fn words(self) -> &'static str {
        match self {
            Band::Clear => "CLEAN AIR",
            Band::Thin => "TOO THIN TO BURN",
            Band::Burns => "WILL BURN",
            Band::Thick => "TOO THICK TO BURN",
        }
    }
}

pub struct Air {
    /// The cell can hold gas: not rock, not water.
    pub open: Vec<bool>,
    /// Top-row cells whose upper face opens to the sky.
    pub vent: Vec<bool>,
    /// Gas concentration.
    pub c: Vec<f32>,
    /// Heat left by a burn, decays; adds buoyancy.
    pub heat: Vec<f32>,
    /// Volume added this tick (a burn expands the air), cleared each step.
    pub expand: Vec<f32>,
    /// `u[y*(W+1)+x]` is the face between cells x-1 and x.
    pub u: Vec<f32>,
    /// `v[y*W+x]` is the face between cells y-1 and y.
    pub v: Vec<f32>,
    p: Vec<f32>,
    div: Vec<f32>,
    /// Open-face counts per cell, cached with the mask.
    nopen: Vec<u8>,
    /// The open cells by colour for the red-black solve: index, the four
    /// neighbours (the spare cell `W*H` for a closed one), 1/open faces.
    sor: [Vec<(u32, [u32; 4], f32)>; 2],
    u_tmp: Vec<f32>,
    v_tmp: Vec<f32>,
    c_tmp: Vec<f32>,
    /// Gas that left through the vent, total.
    pub vented: f64,
    /// Open cells above this row are the air shaft's upper reach, where the
    /// mine's own draught carries gas off to the surface: each loses
    /// `SKY_DRAW` of its gas a tick, counted as vented. 0 turns it off.
    ///
    /// **Why not leave it to the vent face alone.** With one opening, gas
    /// can only leave by an exchange flow (gas up one side of the shaft, air
    /// down the other), and measured on the vent test that drains about a
    /// fifth of a side room in 100 s -- far too slow for "gas blown into the
    /// air shaft escapes", which is the whole point of blowing it there.
    pub sky_row: usize,
    /// Gas lost because its cell filled with water or rock, total -- the
    /// one place gas leaves the books other than the vent, a burn or the
    /// mould. Printed by the probe so the books can be seen to balance.
    pub buried: f64,
}

const SKY_DRAW: f32 = 0.03;

impl Air {
    pub fn new() -> Self {
        Air {
            open: vec![false; W * H],
            vent: vec![false; W],
            c: vec![0.0; W * H],
            heat: vec![0.0; W * H],
            expand: vec![0.0; W * H],
            u: vec![0.0; (W + 1) * H],
            v: vec![0.0; W * (H + 1)],
            p: vec![0.0; W * H + 1],
            div: vec![0.0; W * H],
            nopen: vec![0; W * H],
            sor: [Vec::new(), Vec::new()],
            u_tmp: vec![0.0; (W + 1) * H],
            v_tmp: vec![0.0; W * (H + 1)],
            c_tmp: vec![0.0; W * H],
            vented: 0.0,
            sky_row: 0,
            buried: 0.0,
        }
    }

    /// Re-read which cells hold air. Gas in a cell that has just filled with
    /// water or rubble is lost, and the faces round it close.
    pub fn set_open(&mut self, open: Vec<bool>) {
        self.open = open;
        for y in 0..H {
            for x in 0..W {
                let i = idx(x, y);
                if !self.open[i] {
                    self.buried += self.c[i] as f64;
                    self.c[i] = 0.0;
                    self.p[i] = 0.0;
                }
            }
        }
        for x in 0..W {
            self.vent[x] = self.open[idx(x, 0)];
        }
        for y in 0..H {
            for x in 0..=W {
                if !self.u_open(x, y) {
                    self.u[y * (W + 1) + x] = 0.0;
                }
            }
        }
        for y in 0..=H {
            for x in 0..W {
                if !self.v_open(x, y) {
                    self.v[y * W + x] = 0.0;
                }
            }
        }
        for y in 0..H {
            for x in 0..W {
                let mut n = 0;
                if self.u_open(x, y) {
                    n += 1;
                }
                if self.u_open(x + 1, y) {
                    n += 1;
                }
                if self.v_open(x, y) {
                    n += 1;
                }
                if self.v_open(x, y + 1) {
                    n += 1;
                }
                self.nopen[idx(x, y)] = n;
            }
        }
        let spare = (W * H) as u32;
        self.sor = [Vec::new(), Vec::new()];
        for y in 0..H {
            for x in 0..W {
                let i = idx(x, y);
                if !self.open[i] || self.nopen[i] == 0 {
                    continue;
                }
                let nb = |ok: bool, j: usize| if ok && self.open[j] { j as u32 } else { spare };
                let n4 = [
                    nb(x > 0, i.wrapping_sub(1)),
                    nb(x + 1 < W, i + 1),
                    nb(y > 0, i.wrapping_sub(W)),
                    nb(y + 1 < H, i + W),
                ];
                self.sor[(x + y) % 2].push((i as u32, n4, 1.0 / self.nopen[i] as f32));
            }
        }
    }

    pub fn is_open(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && (x as usize) < W && (y as usize) < H && self.open[idx(x as usize, y as usize)]
    }

    fn u_open(&self, x: usize, y: usize) -> bool {
        x > 0 && x < W && self.open[idx(x - 1, y)] && self.open[idx(x, y)]
    }

    fn v_open(&self, x: usize, y: usize) -> bool {
        if y == 0 {
            return self.vent[x];
        }
        y < H && self.open[idx(x, y - 1)] && self.open[idx(x, y)]
    }

    pub fn at(&self, x: i32, y: i32) -> f32 {
        if self.is_open(x, y) {
            self.c[idx(x as usize, y as usize)]
        } else {
            0.0
        }
    }

    /// Mean gas over the open cells within `r` of a point.
    pub fn mean_near(&self, x: i32, y: i32, r: i32) -> f32 {
        let (mut s, mut n) = (0.0, 0);
        for dy in -r..=r {
            for dx in -r..=r {
                if dx * dx + dy * dy <= r * r && self.is_open(x + dx, y + dy) {
                    s += self.c[idx((x + dx) as usize, (y + dy) as usize)];
                    n += 1;
                }
            }
        }
        if n == 0 {
            0.0
        } else {
            s / n as f32
        }
    }

    pub fn total(&self) -> f64 {
        self.c.iter().map(|&c| c as f64).sum()
    }

    /// **Blow**: push the air along a stroke. Adds velocity in a disc round
    /// `(x, y)` in direction `(dx, dy)` (any length; normalised here). The
    /// projection then turns it into a draught the tunnel walls steer, which
    /// is why a blow along a gallery carries gas well past the brush.
    pub fn blow(&mut self, x: f32, y: f32, dx: f32, dy: f32, radius: f32, strength: f32) {
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1e-4 {
            return;
        }
        let (nx, ny) = (dx / len, dy / len);
        let r = radius.ceil() as i32;
        for oy in -r..=r {
            for ox in -r..=r {
                let d2 = (ox * ox + oy * oy) as f32;
                if d2 > radius * radius {
                    continue;
                }
                let fx = x as i32 + ox;
                let fy = y as i32 + oy;
                if !self.is_open(fx, fy) {
                    continue;
                }
                let w = strength * (1.0 - d2.sqrt() / (radius + 1.0));
                let (fx, fy) = (fx as usize, fy as usize);
                if self.u_open(fx, fy) {
                    self.u[fy * (W + 1) + fx] += nx * w;
                }
                if self.u_open(fx + 1, fy) {
                    self.u[fy * (W + 1) + fx + 1] += nx * w;
                }
                if self.v_open(fx, fy) {
                    self.v[fy * W + fx] += ny * w;
                }
                if self.v_open(fx, fy + 1) {
                    self.v[(fy + 1) * W + fx] += ny * w;
                }
            }
        }
    }

    /// One tick of the air.
    pub fn step(&mut self) {
        self.forces();
        self.advect_velocity();
        self.project();
        self.advect_gas();
        self.mix();
        for i in 0..self.sky_row * W {
            if self.open[i] && self.c[i] > 0.0 {
                let d = self.c[i] * SKY_DRAW;
                self.c[i] -= d;
                self.vented += d as f64;
            }
        }
        for i in 0..W * H {
            self.expand[i] = 0.0;
            self.heat[i] *= 0.96;
        }
    }

    fn forces(&mut self) {
        for y in 1..H {
            for x in 0..W {
                if !self.v_open(x, y) {
                    continue;
                }
                let a = idx(x, y - 1);
                let b = idx(x, y);
                let lift = BUOYANCY * 0.5 * (self.c[a] + self.c[b]) + HEAT_BUOYANCY * 0.5 * (self.heat[a] + self.heat[b]);
                self.v[y * W + x] -= lift;
            }
        }
        for u in self.u.iter_mut() {
            *u *= DAMPING;
        }
        for v in self.v.iter_mut() {
            *v *= DAMPING;
        }
    }

    /// Velocity at a point in cell coordinates (cell centres at +0.5).
    fn sample_u(u: &[f32], x: f32, y: f32) -> f32 {
        // u faces sit at integer x, cell-centre y.
        let fx = x.clamp(0.0, W as f32);
        let fy = (y - 0.5).clamp(0.0, (H - 1) as f32);
        let x0 = (fx.floor() as usize).min(W - 1);
        let y0 = (fy.floor() as usize).min(H - 2);
        let tx = fx - x0 as f32;
        let ty = fy - y0 as f32;
        let g = |x: usize, y: usize| u[y * (W + 1) + x];
        let a = g(x0, y0) * (1.0 - tx) + g(x0 + 1, y0) * tx;
        let b = g(x0, y0 + 1) * (1.0 - tx) + g(x0 + 1, y0 + 1) * tx;
        a * (1.0 - ty) + b * ty
    }

    fn sample_v(v: &[f32], x: f32, y: f32) -> f32 {
        let fx = (x - 0.5).clamp(0.0, (W - 1) as f32);
        let fy = y.clamp(0.0, H as f32);
        let x0 = (fx.floor() as usize).min(W - 2);
        let y0 = (fy.floor() as usize).min(H - 1);
        let tx = fx - x0 as f32;
        let ty = fy - y0 as f32;
        let g = |x: usize, y: usize| v[y * W + x];
        let a = g(x0, y0) * (1.0 - tx) + g(x0 + 1, y0) * tx;
        let b = g(x0, y0 + 1) * (1.0 - tx) + g(x0 + 1, y0 + 1) * tx;
        a * (1.0 - ty) + b * ty
    }

    /// Semi-Lagrangian self-advection: what gives a blow its swirl and lets
    /// a ceiling jet carry on past a corner rather than stopping dead.
    fn advect_velocity(&mut self) {
        for y in 0..H {
            for x in 0..=W {
                let k = y * (W + 1) + x;
                if !self.u_open(x, y) {
                    self.u_tmp[k] = 0.0;
                    continue;
                }
                let px = x as f32;
                let py = y as f32 + 0.5;
                let vx = self.u[k];
                let vy = Self::sample_v(&self.v, px, py);
                self.u_tmp[k] = Self::sample_u(&self.u, px - vx, py - vy);
            }
        }
        for y in 0..=H {
            for x in 0..W {
                let k = y * W + x;
                if !self.v_open(x, y) {
                    self.v_tmp[k] = 0.0;
                    continue;
                }
                let px = x as f32 + 0.5;
                let py = y as f32;
                let vx = Self::sample_u(&self.u, px, py);
                let vy = self.v[k];
                self.v_tmp[k] = Self::sample_v(&self.v, px - vx, py - vy);
            }
        }
        std::mem::swap(&mut self.u, &mut self.u_tmp);
        std::mem::swap(&mut self.v, &mut self.v_tmp);
    }

    fn project(&mut self) {
        for y in 0..H {
            for x in 0..W {
                let i = idx(x, y);
                if !self.open[i] {
                    continue;
                }
                let d = self.u[y * (W + 1) + x + 1] - self.u[y * (W + 1) + x] + self.v[(y + 1) * W + x] - self.v[y * W + x];
                self.div[i] = d - self.expand[i];
            }
        }
        // Red-black SOR over the open cells only, each with its four
        // neighbours resolved once in `set_open`: a closed or outside
        // neighbour points at the spare cell past the end, whose pressure is
        // always zero. Most of the mine is rock, so this is about three
        // times cheaper than sweeping the whole rectangle.
        let ncell = W * H;
        for _ in 0..SOLVE_ITERS {
            for colour in 0..2 {
                for k in 0..self.sor[colour].len() {
                    let (i, nb, inv) = self.sor[colour][k];
                    let i = i as usize;
                    let s = self.p[nb[0] as usize] + self.p[nb[1] as usize] + self.p[nb[2] as usize] + self.p[nb[3] as usize];
                    let target = (s - self.div[i]) * inv;
                    self.p[i] += OMEGA * (target - self.p[i]);
                }
            }
        }
        debug_assert_eq!(self.p[ncell], 0.0);
        for y in 0..H {
            for x in 1..W {
                if self.u_open(x, y) {
                    self.u[y * (W + 1) + x] -= self.p[idx(x, y)] - self.p[idx(x - 1, y)];
                }
            }
        }
        for y in 0..H {
            for x in 0..W {
                if y == 0 {
                    if self.vent[x] {
                        self.v[x] -= self.p[idx(x, 0)];
                    }
                } else if self.v_open(x, y) {
                    self.v[y * W + x] -= self.p[idx(x, y)] - self.p[idx(x, y - 1)];
                }
            }
        }
        for u in self.u.iter_mut() {
            *u = u.clamp(-MAX_SPEED, MAX_SPEED);
        }
        for v in self.v.iter_mut() {
            *v = v.clamp(-MAX_SPEED, MAX_SPEED);
        }
    }

    /// Flux-limited (minmod) upwind advection on the faces: conservative, so
    /// every unit of gas that leaves the mine leaves through the vent and is
    /// counted there.
    ///
    /// **Plain donor-cell was tried first and smeared the layering away**:
    /// its numerical diffusion is about u/2 cells² a tick, three times the
    /// deliberate mixing at gallery speeds, and the first look at "Two
    /// Lamps" (2026-11-07) showed the dome as an even pale-blue haze that
    /// never reached the purple band at all. The limiter keeps a pocket's
    /// edge sharp, which is what makes the three colours readable.
    fn face_value(&self, up: usize, down: usize, far: Option<usize>, speed: f32) -> f32 {
        let cu = self.c[up];
        let Some(far) = far else { return cu };
        let a = cu - self.c[far];
        let b = self.c[down] - cu;
        let slope = if a * b <= 0.0 {
            0.0
        } else if a.abs() < b.abs() {
            a
        } else {
            b
        };
        cu + 0.5 * (1.0 - speed) * slope
    }

    fn advect_gas(&mut self) {
        self.c_tmp.copy_from_slice(&self.c);
        for y in 0..H {
            for x in 1..W {
                let f = self.u[y * (W + 1) + x];
                if f == 0.0 {
                    continue;
                }
                let (l, r) = (idx(x - 1, y), idx(x, y));
                let flux = if f > 0.0 {
                    let far = (x >= 2 && self.open[idx(x - 2, y)]).then(|| idx(x - 2, y));
                    f * self.face_value(l, r, far, f)
                } else {
                    let far = (x + 1 < W && self.open[idx(x + 1, y)]).then(|| idx(x + 1, y));
                    f * self.face_value(r, l, far, -f)
                };
                self.c_tmp[l] -= flux;
                self.c_tmp[r] += flux;
            }
        }
        for y in 0..=H {
            for x in 0..W {
                let f = self.v[y * W + x];
                if f == 0.0 {
                    continue;
                }
                if y == 0 {
                    // The vent: outflow carries gas away, inflow is clean air.
                    if f < 0.0 {
                        let flux = -f * self.c[idx(x, 0)];
                        self.c_tmp[idx(x, 0)] -= flux;
                        self.vented += flux as f64;
                    }
                    continue;
                }
                if y == H {
                    continue;
                }
                let (a, b) = (idx(x, y - 1), idx(x, y));
                let flux = if f > 0.0 {
                    let far = (y >= 2 && self.open[idx(x, y - 2)]).then(|| idx(x, y - 2));
                    f * self.face_value(a, b, far, f)
                } else {
                    let far = (y + 1 < H && self.open[idx(x, y + 1)]).then(|| idx(x, y + 1));
                    f * self.face_value(b, a, far, -f)
                };
                self.c_tmp[a] -= flux;
                self.c_tmp[b] += flux;
            }
        }
        std::mem::swap(&mut self.c, &mut self.c_tmp);
        for c in self.c.iter_mut() {
            if *c < 0.0 {
                *c = 0.0;
            }
        }
    }

    fn mix(&mut self) {
        self.c_tmp.copy_from_slice(&self.c);
        for y in 0..H {
            for x in 0..W {
                let i = idx(x, y);
                if !self.open[i] {
                    continue;
                }
                if x + 1 < W && self.open[i + 1] {
                    let d = MIX * (self.c[i] - self.c[i + 1]);
                    self.c_tmp[i] -= d;
                    self.c_tmp[i + 1] += d;
                }
                if y + 1 < H && self.open[i + W] {
                    let d = MIX * (self.c[i] - self.c[i + W]);
                    self.c_tmp[i] -= d;
                    self.c_tmp[i + W] += d;
                }
            }
        }
        std::mem::swap(&mut self.c, &mut self.c_tmp);
    }
}

impl Default for Air {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A sealed box with one vent: a box of 1.0 gas keeps its total to
    /// within rounding while nothing leaves, and the gas collects at the top.
    fn sealed_box() -> Air {
        let mut a = Air::new();
        let mut open = vec![false; W * H];
        for y in 20..60 {
            for x in 20..120 {
                open[idx(x, y)] = true;
            }
        }
        a.set_open(open);
        a
    }

    #[test]
    fn gas_is_conserved_in_a_sealed_box_and_rises() {
        let mut a = sealed_box();
        for y in 50..60 {
            for x in 60..70 {
                a.c[idx(x, y)] = 1.0;
            }
        }
        let before = a.total();
        for _ in 0..600 {
            a.step();
        }
        let after = a.total();
        assert!((after - before).abs() / before < 1e-3, "{before} -> {after}");
        let top: f32 = (20..25).flat_map(|y| (20..120).map(move |x| (x, y))).map(|(x, y)| a.c[idx(x, y)]).sum();
        let bottom: f32 = (55..60).flat_map(|y| (20..120).map(move |x| (x, y))).map(|(x, y)| a.c[idx(x, y)]).sum();
        assert!(top > 3.0 * bottom, "top {top} bottom {bottom}");
    }

    /// The positive control for the vent: gas in a side room off an open
    /// shaft leaves through it, and is counted leaving. (A shaft full of gas
    /// from wall to wall does not: a perfectly level layer is balanced by
    /// pressure and nothing breaks the symmetry. In the level gas always
    /// arrives from one side.)
    #[test]
    fn gas_under_a_vent_escapes_and_is_counted() {
        let mut a = Air::new();
        let mut open = vec![false; W * H];
        for y in 0..60 {
            for x in 20..30 {
                open[idx(x, y)] = true;
            }
        }
        for y in 50..60 {
            for x in 30..70 {
                open[idx(x, y)] = true;
            }
        }
        a.set_open(open);
        a.sky_row = 30;
        for y in 52..60 {
            for x in 40..70 {
                a.c[idx(x, y)] = 0.3;
            }
        }
        let before = a.total();
        for _ in 0..6000 {
            a.step();
        }
        assert!(a.total() < 0.7 * before, "{} of {before} left", a.total());
        assert!((a.vented + a.total() - before).abs() / before < 0.02, "vented {} left {} of {before}", a.vented, a.total());
    }

    /// Gas under a roof that rises to the right ends up at the right.
    #[test]
    fn gas_runs_uphill_along_a_sloping_roof() {
        let mut a = Air::new();
        let mut open = vec![false; W * H];
        for x in 10..210 {
            let roof = 60 - (x as i32 - 10) * 20 / 200;
            for y in roof as usize..70 {
                open[idx(x, y)] = true;
            }
        }
        a.set_open(open);
        for y in 62..70 {
            for x in 20..40 {
                a.c[idx(x, y)] = 1.0;
            }
        }
        for _ in 0..2400 {
            a.step();
        }
        let left: f32 = (0..H).flat_map(|y| (10..60).map(move |x| (x, y))).map(|(x, y)| a.c[idx(x, y)]).sum();
        let right: f32 = (0..H).flat_map(|y| (160..210).map(move |x| (x, y))).map(|(x, y)| a.c[idx(x, y)]).sum();
        assert!(right > left, "left {left} right {right}");
    }
}
