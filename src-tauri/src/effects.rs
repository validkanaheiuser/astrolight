//! Lighting maths. `ui/effects.js` mirrors every formula here; `scripts/check-effects.cjs`
//! compares the two implementations channel by channel.
use crate::protocol::{Frame, Key, KEY_COUNT};
use serde::{Deserialize, Serialize};
use std::f64::consts::{PI, TAU};

macro_rules! modes {
    ($($variant:ident => $name:literal),* $(,)?) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum Mode { $($variant),* }
        /// Every mode name, for tests and the parity fixture.
        #[allow(dead_code)]
        pub const MODES: &[&str] = &[$($name),*];
        impl Mode {
            pub fn parse(s: &str) -> Option<Self> {
                match s { $($name => Some(Mode::$variant),)* _ => None }
            }
        }
    };
}
modes! {
    Paint => "paint", Solid => "solid", Gradient => "gradient", Wave => "wave",
    Breathe => "breathe", Rainbow => "rainbow", Aurora => "aurora", Spiral => "spiral",
    Ripple => "ripple", Meteor => "meteor", Stars => "stars", Rain => "rain",
    Fire => "fire", Checker => "checker", Beacon => "beacon", Sunset => "sunset",
    Plasma => "plasma", Nebula => "nebula", Ocean => "ocean", Comets => "comets",
    Fireflies => "fireflies", Chevron => "chevron", Diamonds => "diamonds", Helix => "helix",
    Ribbon => "ribbon", Scanline => "scanline", Heartbeat => "heartbeat", Pinwheel => "pinwheel",
    Cycle => "cycle", Prism => "prism", Vortex => "vortex", Kaleido => "kaleido",
    Bloom => "bloom", Matrix => "matrix", Lightning => "lightning", Bounce => "bounce",
    Marquee => "marquee", Mosaic => "mosaic", Swirl => "swirl", Zigzag => "zigzag",
    Lava => "lava", Candle => "candle", Police => "police", Bars => "bars",
    Snake => "snake", Sparkle => "sparkle",
    ReactPulse => "rpulse", ReactGlow => "rglow", ReactRipple => "rripple",
    ReactCross => "rcross", ReactHeat => "rheat", ReactRainbow => "rrainbow",
}
impl Mode {
    pub fn is_static(self) -> bool {
        matches!(self, Mode::Paint | Mode::Solid | Mode::Gradient)
    }
    pub fn is_reactive(self) -> bool {
        matches!(
            self,
            Mode::ReactPulse
                | Mode::ReactGlow
                | Mode::ReactRipple
                | Mode::ReactCross
                | Mode::ReactHeat
                | Mode::ReactRainbow
        )
    }
}

/// An effect assigned to one key, replacing the keyboard-wide effect at that position.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct KeyFx {
    pub mode: String,
    pub color_a: String,
    pub color_b: String,
    pub speed: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    #[serde(default)]
    pub angle: i16,
    #[serde(default = "default_scale")]
    pub scale: u8,
    pub mode: String,
    pub color_a: String,
    pub color_b: String,
    pub brightness: u8,
    pub speed: u8,
    pub layout: usize,
    pub colors: Vec<String>,
    #[serde(default = "default_fps")]
    pub fps: u16,
    /// Empty, or one optional entry per key.
    #[serde(default)]
    pub per_key: Vec<Option<KeyFx>>,
}
fn default_scale() -> u8 {
    50
}
fn default_fps() -> u16 {
    30
}
impl Default for Config {
    fn default() -> Self {
        Self {
            angle: 0,
            scale: 50,
            mode: "wave".into(),
            color_a: "#38d9bb".into(),
            color_b: "#a77aff".into(),
            brightness: 70,
            speed: 35,
            layout: 3,
            colors: vec!["#38d9bb".into(); KEY_COUNT],
            fps: 30,
            per_key: vec![],
        }
    }
}
pub fn rgb(s: &str) -> Result<[f64; 3], String> {
    if s.len() != 7 || !s.starts_with('#') || !s.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit) {
        return Err("E:bad_color".into());
    }
    Ok([
        u8::from_str_radix(&s[1..3], 16).unwrap() as f64,
        u8::from_str_radix(&s[3..5], 16).unwrap() as f64,
        u8::from_str_radix(&s[5..7], 16).unwrap() as f64,
    ])
}

#[derive(Clone, Copy, Default)]
struct Geo {
    u: f64,
    v: f64,
    r: f64,
    ang: f64,
    seed: f64,
    rain_seed: f64,
    /// Unrotated position in keyboard-width units, for distances between keys.
    gx: f64,
    gy: f64,
}
#[derive(Clone, Copy)]
struct Override {
    mode: Mode,
    a: [f64; 3],
    b: [f64; 3],
    speed: f64,
}
/// Key presses that reactive effects respond to: `(key index, clock at press)`.
pub type Hits = [(usize, f64)];

#[derive(Clone)]
pub struct Scene {
    pub config: Config,
    mode: Mode,
    a: [f64; 3],
    b: [f64; 3],
    paint: [[f64; 3]; KEY_COUNT],
    geo: [Geo; KEY_COUNT],
    overrides: [Option<Override>; KEY_COUNT],
    animated: bool,
    reactive: bool,
}

fn fract(x: f64) -> f64 {
    x - x.floor()
}
fn hash(n: f64) -> f64 {
    fract((n * 12.9898 + 4.1414).sin() * 43758.5453)
}
fn tri(x: f64) -> f64 {
    (fract(x) * 2. - 1.).abs()
}
fn smooth(lo: f64, hi: f64, x: f64) -> f64 {
    let t = ((x - lo) / (hi - lo)).clamp(0., 1.);
    t * t * (3. - 2. * t)
}
fn hue_rgb(hue: f64) -> [f64; 3] {
    let mut out = [0.; 3];
    for (j, c) in out.iter_mut().enumerate() {
        let k = ([0., 8., 4.][j] + hue.rem_euclid(1.) * 12.) % 12.;
        *c = 255. * (1. - (k - 3.).min(9. - k).min(1.).max(0.));
    }
    out
}
/// How quickly reactive light fades, from the effect speed.
fn react_rate(speed: f64) -> f64 {
    0.4 + speed / 40.
}

impl Scene {
    pub fn new(mut c: Config, keys: &[Key]) -> Result<Self, String> {
        // Legacy profiles requested 120 FPS; migrate to the stable 30 FPS default.
        if c.fps == 120 {
            c.fps = 30;
        }
        let mode = Mode::parse(&c.mode);
        if mode.is_none()
            || c.scale < 1
            || c.scale > 100
            || !(-180..=180).contains(&c.angle)
            || c.brightness > 100
            || !(1..=100).contains(&c.speed)
            || c.layout > 3
            || ![15, 30, 60].contains(&c.fps)
            || c.colors.len() != KEY_COUNT
            || !(c.per_key.is_empty() || c.per_key.len() == KEY_COUNT)
        {
            return Err("E:bad_config".into());
        }
        let mode = mode.unwrap();
        if c.per_key.iter().all(Option::is_none) {
            c.per_key.clear();
        }
        let a = rgb(&c.color_a)?;
        let b = rgb(&c.color_b)?;
        let mut paint = [[0.; 3]; KEY_COUNT];
        let mut geo = [Geo::default(); KEY_COUNT];
        let mut overrides = [None; KEY_COUNT];
        let angle = c.angle as f64 * PI / 180.;
        let (sn, cs) = angle.sin_cos();
        for (i, k) in keys.iter().enumerate() {
            paint[i] = rgb(&c.colors[i])?;
            let p = &k.layouts[c.layout];
            let x = (p.x + p.w / 2.) / 760.;
            let y = (p.y + p.h / 2.) / 270.;
            let u = (x - 0.5) * cs + (y - 0.5) * sn + 0.5;
            let s = (i as f64 * 12.9898).sin() * 43758.5453;
            let rs = ((u * 15.).floor() * 12.9898).sin() * 43758.5453;
            geo[i] = Geo {
                u,
                v: -(x - 0.5) * sn + (y - 0.5) * cs + 0.5,
                r: ((x - 0.5).powi(2) + ((y - 0.5) * 0.6).powi(2)).sqrt(),
                ang: ((y - 0.5) * 0.6).atan2(x - 0.5) / TAU,
                seed: s - s.floor(),
                rain_seed: rs - rs.floor(),
                gx: x,
                gy: y * 270. / 760.,
            };
            if let Some(Some(fx)) = c.per_key.get(i) {
                let m = Mode::parse(&fx.mode).filter(|m| *m != Mode::Paint);
                if m.is_none() || !(1..=100).contains(&fx.speed) {
                    return Err("E:bad_config".into());
                }
                overrides[i] = Some(Override {
                    mode: m.unwrap(),
                    a: rgb(&fx.color_a)?,
                    b: rgb(&fx.color_b)?,
                    speed: fx.speed as f64,
                });
            }
        }
        let used = || std::iter::once(mode).chain(overrides.iter().flatten().map(|o| o.mode));
        Ok(Self {
            animated: used().any(|m| !m.is_static()),
            reactive: used().any(Mode::is_reactive),
            config: c,
            mode,
            a,
            b,
            paint,
            geo,
            overrides,
        })
    }
    pub fn animated(&self) -> bool {
        self.animated
    }
    /// Whether any key listens for key presses.
    pub fn reactive(&self) -> bool {
        self.reactive
    }
    /// `phase` is speed-scaled time for the keyboard effect; `clock` is plain animation
    /// seconds, used by per-key effects (which carry their own speed) and by `hits`.
    pub fn frame(&self, phase: f64, clock: f64, hits: &Hits) -> Frame {
        let mut out = [[0u8; 3]; KEY_COUNT];
        let brightness = self.config.brightness as f64 / 100.;
        let scale = 0.5 + self.config.scale as f64 / 33.3333333333;
        for (i, p) in out.iter_mut().enumerate() {
            let value = match &self.overrides[i] {
                Some(o) => self.shade(
                    o.mode,
                    &o.a,
                    &o.b,
                    scale,
                    i,
                    clock * o.speed / 50.,
                    clock,
                    react_rate(o.speed),
                    hits,
                ),
                None => self.shade(
                    self.mode,
                    &self.a,
                    &self.b,
                    scale,
                    i,
                    phase,
                    clock,
                    react_rate(self.config.speed as f64),
                    hits,
                ),
            };
            for j in 0..3 {
                p[j] = (value[j] * brightness).round().clamp(0., 255.) as u8;
            }
        }
        out
    }
    #[allow(clippy::too_many_arguments)]
    fn shade(
        &self,
        mode: Mode,
        a: &[f64; 3],
        b: &[f64; 3],
        scale: f64,
        i: usize,
        phase: f64,
        clock: f64,
        rate: f64,
        hits: &Hits,
    ) -> [f64; 3] {
        let g = &self.geo[i];
        let (u, v, r, ang) = (g.u, g.v, g.r, g.ang);
        let mut t = 0.;
        let mut gain = 1.;
        let mut hue = None;
        match mode {
            Mode::Paint => return self.paint[i],
            Mode::Solid => {}
            Mode::Gradient => t = u.clamp(0., 1.),
            Mode::Wave => t = ((u * scale * TAU - phase * 2.).sin() + 1.) * 0.5,
            Mode::Breathe => gain = 0.06 + 0.94 * ((phase * 2.).sin() + 1.) * 0.5,
            Mode::Rainbow => hue = Some(u * scale * 0.5 - phase * 0.12),
            Mode::Aurora => {
                t = ((u * 5. + (v * 3. + phase).sin() * 1.2 - phase).sin() + 1.) * 0.5;
                gain = 0.2 + 0.8 * ((v * 4. - phase * 1.3 + u * 2.).sin() + 1.) * 0.5;
            }
            Mode::Spiral => hue = Some(ang + r * scale - phase * 0.18),
            Mode::Ripple => {
                let wave = (r * scale * 15. - phase * 4.).sin() * 0.5 + 0.5;
                t = r.min(1.);
                gain = 0.08 + 0.92 * wave.powi(3);
            }
            Mode::Meteor => {
                let head = (phase * 0.22).rem_euclid(1.5) - 0.25;
                let d = head - u;
                gain = if d >= 0. {
                    (-d * (6. + scale * 3.)).exp()
                } else {
                    (-d * d * 800.).exp()
                };
                t = v;
            }
            Mode::Stars => {
                let seed = g.seed;
                gain = 0.02
                    + 0.98
                        * ((phase * (0.6 + seed) + seed * 25.).sin() * 0.5 + 0.5).powf(8. - scale);
                t = seed;
            }
            Mode::Rain => {
                let seed = g.rain_seed;
                let head = (phase * (0.3 + seed * 0.35) + seed * 3.).rem_euclid(1.7) - 0.3;
                let d = head - v;
                gain = if d >= 0. {
                    (-d * 8.).exp()
                } else {
                    (-d * d * 300.).exp()
                };
                t = seed;
            }
            Mode::Fire => {
                let noise = (u * 12. + phase * 2.5).sin() * (u * 7. - phase * 3.1).cos();
                t = (v * 0.6 + noise * 0.25 + 0.2).clamp(0., 1.);
                gain = (0.35 + v * 0.6 + noise * 0.2).clamp(0., 1.);
            }
            Mode::Checker => {
                t = ((u * scale * 10. + phase).sin() * (v * scale * 6. - phase).sin() + 1.) * 0.5;
            }
            Mode::Beacon => {
                t = ((ang * TAU - phase * 2.).cos() + 1.) * 0.5;
                gain = 0.08 + 0.92 * t.powi(6);
            }
            Mode::Sunset => t = ((u + v * 0.3) * scale + phase * 0.15).sin() * 0.5 + 0.5,
            Mode::Plasma => {
                t = 0.5
                    + ((u * scale * 7. + phase).sin()
                        + (v * scale * 7. - phase * 0.8).sin()
                        + (r * scale * 12. - phase * 1.2).sin())
                        / 6.;
            }
            Mode::Nebula => {
                let cloud = (u * scale * 3. + (v * 5. - phase * 0.25).sin() + phase * 0.35).sin();
                t = cloud * 0.5 + 0.5;
                gain = 0.25 + 0.75 * ((u * 4. - v * 3. + phase * 0.4).sin() * 0.5 + 0.5);
            }
            Mode::Ocean => {
                let wave = (u * scale * 5. - phase * 1.3 + (v * 6. + phase * 0.6).sin()).sin();
                t = wave * 0.5 + 0.5;
                gain = 0.35 + 0.65 * ((v * 8. - phase + u * 2.).sin() * 0.5 + 0.5);
            }
            Mode::Comets => {
                let d = (u + v * 0.28 - phase * 0.18).rem_euclid(0.48);
                gain = (-d * (10. + scale * 4.)).exp();
                t = (v * 3. + phase * 0.25).sin() * 0.5 + 0.5;
            }
            Mode::Fireflies => {
                let seed = g.seed;
                t = seed;
                gain = 0.015
                    + 0.985
                        * ((phase * (0.35 + seed * 0.3) + seed * 30.).sin() * 0.5 + 0.5).powi(12)
                        * ((u * 5. - phase * 0.4).sin() * 0.3 + 0.7);
            }
            Mode::Chevron => {
                t = ((u + (v - 0.5).abs() * 0.8) * scale * 8. - phase * 2.).sin() * 0.5 + 0.5;
                gain = 0.15 + 0.85 * t;
            }
            Mode::Diamonds => {
                let d = (u - 0.5).abs() + (v - 0.5).abs() * 0.6;
                t = (d * scale * 12. - phase * 2.).sin() * 0.5 + 0.5;
                gain = 0.1 + 0.9 * t.powi(2);
            }
            Mode::Helix => {
                let center = (u * scale * 6. - phase * 1.5).sin() * 0.3;
                let a = (-(v - 0.5 - center).powi(2) * 100.).exp();
                let b = (-(v - 0.5 + center).powi(2) * 100.).exp();
                t = b / (a + b + 1e-9);
                gain = (a + b).min(1.);
            }
            Mode::Ribbon => {
                let center = 0.5 + (u * scale * 3. - phase * 0.7).sin() * 0.25;
                gain = 0.05 + 0.95 * (-(v - center).powi(2) * (12. + scale * 8.)).exp();
                t = (u * 3. + phase * 0.4).sin() * 0.5 + 0.5;
            }
            Mode::Scanline => {
                let head = (phase * 0.9).sin() * 0.5 + 0.5;
                gain = 0.02 + 0.98 * (-(u - head).powi(2) * (45. + scale * 30.)).exp();
                t = v.clamp(0., 1.);
            }
            Mode::Heartbeat => {
                let beat = (phase * 0.55).rem_euclid(1.);
                gain = 0.04
                    + 0.96
                        * ((-(beat - 0.2).powi(2) * 220.).exp()
                            + 0.65 * (-(beat - 0.42).powi(2) * 300.).exp())
                        .min(1.);
                t = r.min(1.);
            }
            Mode::Pinwheel => hue = Some(ang * (2. + scale) + phase * 0.15 + r * 0.6),
            Mode::Cycle => hue = Some(phase * 0.1 + u * 0.04 * scale),
            Mode::Prism => hue = Some(r * scale * 0.5 + phase * 0.14),
            Mode::Vortex => t = (ang * TAU * 2. + r * scale * 14. - phase * 2.).sin() * 0.5 + 0.5,
            Mode::Kaleido => {
                let fold = (fract(ang * 3.) - 0.5).abs() * 2.;
                hue = Some(fold * 0.5 + r * scale * 0.5 - phase * 0.1);
            }
            Mode::Bloom => {
                t = (r * scale * 8. - (phase * 0.8).sin() * 3.).sin() * 0.5 + 0.5;
                gain = 0.2 + 0.8 * ((ang * TAU * 5. + (phase * 0.7).sin() * 2.).sin() * 0.5 + 0.5);
            }
            Mode::Matrix => {
                let seed = hash((u * 19.).floor());
                let head = (phase * (0.25 + seed * 0.4) + seed * 5.).rem_euclid(1.6) - 0.2;
                let d = head - ((v * 5.).floor() + 0.5) / 5.;
                let tip = (-d * d * 900.).exp();
                t = tip;
                gain = (if d >= 0. { (-d * 5.).exp() } else { 0. }).max(tip);
            }
            Mode::Lightning => {
                let n = (phase * 1.4).floor();
                let f = fract(phase * 1.4);
                let flash = if hash(n) > 0.45 {
                    (-f * 7.).exp() * (0.6 + 0.4 * (f * 60.).sin().abs())
                } else {
                    0.
                };
                let lean = (hash(n + 0.7) - 0.5) * 0.6;
                let bolt = (-(u - hash(n + 0.5) - (v - 0.5) * lean).powi(2) * (30. + scale * 20.))
                    .exp();
                t = bolt;
                gain = 0.04 + flash * (0.25 + 0.75 * bolt);
            }
            Mode::Bounce => {
                let dx = u - tri(phase * 0.23);
                let dy = (v - tri(phase * 0.31 + 0.3)) * 0.36;
                gain = 0.05 + 0.95 * (-(dx * dx + dy * dy) * (40. + scale * 40.)).exp();
                t = tri(phase * 0.05);
            }
            Mode::Marquee => {
                let s = ((u * scale * 3. - phase * 0.35) * TAU).sin();
                gain = 0.06 + 0.94 * (s * 4. + 0.5).clamp(0., 1.);
                t = v.clamp(0., 1.);
            }
            Mode::Mosaic => {
                let z = phase * 0.5 + g.seed * 7.;
                let (n, f) = (z.floor(), smooth(0., 1., fract(z)));
                let k = i as f64;
                t = hash(k * 3.1 + n) * (1. - f) + hash(k * 3.1 + n + 1.) * f;
                gain = 0.3 + 0.7 * (hash(k * 1.7 + n) * (1. - f) + hash(k * 1.7 + n + 1.) * f);
            }
            Mode::Swirl => {
                let (sn, cs) = (phase * 0.9).sin_cos();
                let k = 18. + scale * 10.;
                let one = (-((u - 0.5 - 0.32 * cs).powi(2)
                    + ((v - 0.5 - 0.3 * sn) * 0.45).powi(2))
                    * k)
                    .exp();
                let two = (-((u - 0.5 + 0.32 * cs).powi(2)
                    + ((v - 0.5 + 0.3 * sn) * 0.45).powi(2))
                    * k)
                    .exp();
                t = two / (one + two + 1e-9);
                gain = (0.04 + one + two).min(1.);
            }
            Mode::Zigzag => {
                let center = 0.15 + 0.7 * tri(u * scale * 1.5 - phase * 0.2);
                gain = 0.06 + 0.94 * (-(v - center).powi(2) * 40.).exp();
                t = u.clamp(0., 1.);
            }
            Mode::Lava => {
                let n = (u * scale * 2.3 + phase * 0.4).sin()
                    + (v * 3.1 - phase * 0.33).sin()
                    + ((u + v) * scale * 1.7 + phase * 0.27).sin()
                    + (r * 6. - phase * 0.2).sin();
                let m = n / 8. + 0.5;
                t = smooth(0.35, 0.65, m);
                gain = 0.2 + 0.8 * smooth(0.2, 0.8, m);
            }
            Mode::Candle => {
                let seed = g.seed;
                let flicker = 0.55
                    + 0.25 * (phase * 5.3).sin()
                    + 0.12 * (phase * 12.7 + seed * 6.).sin()
                    + 0.08 * (phase * 23.1 + seed * 40.).sin();
                gain = (flicker * (0.6 + 0.4 * v)).clamp(0., 1.);
                t = (1. - v * 0.8 + 0.1 * (phase * 3. + seed * 10.).sin()).clamp(0., 1.);
            }
            Mode::Police => {
                let beat = fract(phase * 0.6);
                let strobe = ((beat * TAU * 6.).sin() * 3. + 0.5).clamp(0., 1.);
                let left = u < 0.5;
                gain = if left == (beat < 0.5) {
                    0.15 + 0.85 * strobe
                } else {
                    0.03
                };
                t = if left { 0. } else { 1. };
            }
            Mode::Bars => {
                let seed = hash((u * 14.).floor());
                let level = 0.15
                    + 0.85
                        * (0.5 + 0.5 * (phase * (1.2 + seed * 2.2) + seed * 20.).sin())
                        * (0.6 + 0.4 * (phase * 0.7 + seed * 9.).sin());
                let height = 1. - v;
                gain = 0.04 + 0.96 * ((level - height) * 8. + 0.5).clamp(0., 1.);
                t = height.clamp(0., 1.);
            }
            Mode::Snake => {
                let row = (v * 5.).floor().clamp(0., 4.);
                let along = if row % 2. == 0. { u } else { 1. - u };
                let d = fract(fract(phase * 0.12) - (row + along.clamp(0., 1.)) / 5.);
                gain = 0.03 + 0.97 * (-d * (6. + scale * 4.)).exp();
                t = (d * 3.).min(1.);
            }
            Mode::Sparkle => {
                let z = phase * 3. + g.seed * 10.;
                let glint = if hash(i as f64 * 7.3 + z.floor()) > 0.82 {
                    (fract(z) * PI).sin()
                } else {
                    0.
                };
                t = glint;
                gain = 0.25 + 0.75 * glint;
            }
            Mode::ReactPulse
            | Mode::ReactGlow
            | Mode::ReactRipple
            | Mode::ReactCross
            | Mode::ReactHeat
            | Mode::ReactRainbow => return self.react(mode, a, b, scale, i, clock, rate, hits),
        }
        if let Some(h) = hue {
            return hue_rgb(h);
        }
        [0, 1, 2].map(|j| (a[j] * (1. - t) + b[j] * t) * gain)
    }
    /// Idle keys rest on a dim `b`; pressed keys light towards `a`.
    #[allow(clippy::too_many_arguments)]
    fn react(
        &self,
        mode: Mode,
        a: &[f64; 3],
        b: &[f64; 3],
        scale: f64,
        i: usize,
        clock: f64,
        rate: f64,
        hits: &Hits,
    ) -> [f64; 3] {
        let g = &self.geo[i];
        let mut level = 0.;
        let mut color = [0.; 3];
        for &(key, at) in hits {
            let age = (clock - at) * rate;
            if age < 0. || key >= KEY_COUNT {
                continue;
            }
            let h = &self.geo[key];
            let (dx, dy) = (g.gx - h.gx, g.gy - h.gy);
            let d = (dx * dx + dy * dy).sqrt();
            match mode {
                Mode::ReactPulse => {
                    if key == i {
                        level += (-age * 2.2).exp();
                    }
                }
                Mode::ReactGlow => {
                    let sigma = 0.05 + scale * 0.03;
                    level += (-age * 2.).exp() * (-d * d / (2. * sigma * sigma)).exp();
                }
                Mode::ReactRipple => {
                    level += (-((d - age * 0.55) / 0.05).powi(2)).exp() * (-age * 1.3).exp();
                }
                Mode::ReactCross => {
                    let along = if dy.abs() < 0.02 {
                        Some(dx.abs())
                    } else if dx.abs() < 0.035 {
                        Some(dy.abs() * 3.)
                    } else {
                        None
                    };
                    if let Some(along) = along {
                        level += (-((along - age * 0.9) / 0.07).powi(2)).exp() * (-age * 1.4).exp();
                    }
                }
                Mode::ReactHeat => {
                    level += 0.45 * (-age * 0.35).exp() * (-d * d / 0.0098).exp();
                }
                _ => {
                    let w = (-((d - age * 0.55) / 0.06).powi(2)).exp() * (-age * 1.2).exp();
                    let tint = hue_rgb(fract(at * 0.37 + key as f64 * 0.113));
                    for j in 0..3 {
                        color[j] += tint[j] * w;
                    }
                    level += w;
                }
            }
        }
        let level = level.min(1.);
        match mode {
            Mode::ReactHeat => {
                [0, 1, 2].map(|j| (b[j] * (1. - level) + a[j] * level) * (0.12 + 0.88 * level))
            }
            Mode::ReactRainbow => {
                [0, 1, 2].map(|j| (b[j] * 0.1 * (1. - level) + color[j]).min(255.))
            }
            _ => [0, 1, 2].map(|j| b[j] * 0.12 * (1. - level) + a[j] * level),
        }
    }
}
pub fn blend(from: &Frame, to: &Frame, t: f64) -> Frame {
    let t = t.clamp(0., 1.);
    let t = t * t * (3. - 2. * t);
    let mut o = *to;
    for (i, p) in o.iter_mut().enumerate() {
        for (j, c) in p.iter_mut().enumerate() {
            *c = (from[i][j] as f64 * (1. - t) + to[i][j] as f64 * t).round() as u8;
        }
    }
    o
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::keys;
    fn scene(mode: &str) -> Scene {
        let mut c = Config::default();
        c.mode = mode.to_string();
        Scene::new(c, keys()).unwrap()
    }
    #[test]
    fn migrates_legacy_rate() {
        let mut c = Config::default();
        c.fps = 120;
        assert_eq!(Scene::new(c, keys()).unwrap().config.fps, 30);
    }
    #[test]
    fn all_animations_move() {
        for mode in MODES {
            let scene = scene(mode);
            if scene.reactive() {
                let hits = [(20, 0.2)];
                assert_ne!(
                    scene.frame(0.37, 0.3, &hits),
                    scene.frame(2.19, 0.9, &hits),
                    "{mode}"
                );
            } else if scene.animated() {
                assert_ne!(
                    scene.frame(0.37, 0.37, &[]),
                    scene.frame(2.19, 2.19, &[]),
                    "{mode}"
                );
            }
        }
    }
    #[test]
    fn reactive_rests_without_hits() {
        for mode in MODES {
            let scene = scene(mode);
            if scene.reactive() {
                assert_eq!(scene.frame(1., 1., &[]), scene.frame(9., 9., &[]), "{mode}");
                assert_eq!(
                    scene.frame(1., 1., &[]),
                    scene.frame(500., 500., &[(3, 1.)]),
                    "{mode}"
                );
            }
        }
    }
    #[test]
    fn rejects_bad_config() {
        let mut c = Config::default();
        c.colors[0] = "no".into();
        assert!(Scene::new(c, keys()).is_err());
        let mut c = Config::default();
        c.fps = 1000;
        assert!(Scene::new(c, keys()).is_err());
        let mut c = Config::default();
        c.per_key = vec![None; 3];
        assert!(Scene::new(c, keys()).is_err());
        let mut c = Config::default();
        c.per_key = vec![None; KEY_COUNT];
        c.per_key[0] = Some(KeyFx {
            mode: "paint".into(),
            color_a: "#ffffff".into(),
            color_b: "#000000".into(),
            speed: 50,
        });
        assert!(Scene::new(c, keys()).is_err());
    }
    #[test]
    fn zero_is_black() {
        for mode in MODES.iter().copied() {
            let mut c = Config::default();
            c.mode = mode.into();
            c.brightness = 0;
            assert_eq!(
                Scene::new(c, keys()).unwrap().frame(17., 17., &[(5, 16.9)]),
                [[0; 3]; KEY_COUNT]
            );
        }
    }
    #[test]
    fn per_key_effect_only_changes_its_key() {
        let mut c = Config::default();
        c.mode = "solid".into();
        let plain = Scene::new(c.clone(), keys()).unwrap();
        assert!(!plain.animated());
        c.per_key = vec![None; KEY_COUNT];
        c.per_key[7] = Some(KeyFx {
            mode: "breathe".into(),
            color_a: "#ff0000".into(),
            color_b: "#0000ff".into(),
            speed: 80,
        });
        let mixed = Scene::new(c, keys()).unwrap();
        assert!(mixed.animated());
        let (a, b) = (plain.frame(1., 1., &[]), mixed.frame(1., 1., &[]));
        for i in 0..KEY_COUNT {
            assert_eq!(a[i] == b[i], i != 7);
        }
        assert_ne!(mixed.frame(1., 1., &[])[7], mixed.frame(1., 1.6, &[])[7]);
    }
    #[test]
    fn empty_per_key_is_dropped() {
        let mut c = Config::default();
        c.per_key = vec![None; KEY_COUNT];
        assert!(Scene::new(c, keys()).unwrap().config.per_key.is_empty());
    }
    #[test]
    fn legacy_config_without_per_key_loads() {
        let mut v = serde_json::to_value(Config::default()).unwrap();
        v.as_object_mut().unwrap().remove("perKey");
        let c: Config = serde_json::from_value(v).unwrap();
        assert!(c.per_key.is_empty());
    }
    #[test]
    fn transition_endpoints() {
        let a = [[0; 3]; KEY_COUNT];
        let b = [[255; 3]; KEY_COUNT];
        assert_eq!(blend(&a, &b, 0.), a);
        assert_eq!(blend(&a, &b, 1.), b);
        assert_eq!(blend(&a, &b, 0.5)[0], [128; 3]);
    }
    #[test]
    fn smooth_phase() {
        let s = Scene::new(Config::default(), keys()).unwrap();
        let a = s.frame(1., 1., &[]);
        let b = s.frame(1. + 1. / 120., 1., &[]);
        assert!(a
            .iter()
            .zip(b)
            .all(|(a, b)| a.iter().zip(b).all(|(a, b)| a.abs_diff(b) <= 3)));
    }
}
