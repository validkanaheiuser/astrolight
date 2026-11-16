use crate::protocol::{Frame, Key, KEY_COUNT};
use serde::{Deserialize, Serialize};
pub const MODES: &[&str] = &[
    "paint",
    "solid",
    "gradient",
    "wave",
    "breathe",
    "rainbow",
    "aurora",
    "spiral",
    "ripple",
    "meteor",
    "stars",
    "rain",
    "fire",
    "checker",
    "beacon",
    "sunset",
    "plasma",
    "nebula",
    "ocean",
    "comets",
    "fireflies",
    "chevron",
    "diamonds",
    "helix",
    "ribbon",
    "scanline",
    "heartbeat",
    "pinwheel",
];
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
        }
    }
}
pub fn rgb(s: &str) -> Result<[f64; 3], String> {
    if s.len() != 7 || !s.starts_with('#') || !s.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit) {
        return Err("Màu phải có dạng #RRGGBB".into());
    }
    Ok([
        u8::from_str_radix(&s[1..3], 16).unwrap() as f64,
        u8::from_str_radix(&s[3..5], 16).unwrap() as f64,
        u8::from_str_radix(&s[5..7], 16).unwrap() as f64,
    ])
}
#[derive(Clone)]
pub struct Scene {
    pub config: Config,
    a: [f64; 3],
    b: [f64; 3],
    paint: [[f64; 3]; KEY_COUNT],
    u: [f64; KEY_COUNT],
    v: [f64; KEY_COUNT],
    radius: [f64; KEY_COUNT],
    polar: [f64; KEY_COUNT],
    seed: [f64; KEY_COUNT],
    rain_seed: [f64; KEY_COUNT],
}
impl Scene {
    pub fn new(mut c: Config, keys: &[Key]) -> Result<Self, String> {
        // Legacy profiles requested 120 FPS; migrate to the stable 30 FPS default.
        if c.fps == 120 {
            c.fps = 30;
        }
        if !MODES.contains(&c.mode.as_str())
            || c.scale < 1
            || c.scale > 100
            || !(-180..=180).contains(&c.angle)
            || c.brightness > 100
            || !(1..=100).contains(&c.speed)
            || c.layout > 3
            || ![15, 30, 60].contains(&c.fps)
            || c.colors.len() != KEY_COUNT
        {
            return Err("Cấu hình ánh sáng không hợp lệ".into());
        }
        let a = rgb(&c.color_a)?;
        let b = rgb(&c.color_b)?;
        let mut paint = [[0.; 3]; KEY_COUNT];
        let mut u = [0.; KEY_COUNT];
        let mut v = u;
        let mut radius = u;
        let mut polar = u;
        let mut seed = u;
        let mut rain_seed = u;
        let angle = c.angle as f64 * std::f64::consts::PI / 180.;
        let (sn, cs) = angle.sin_cos();
        for (i, k) in keys.iter().enumerate() {
            paint[i] = rgb(&c.colors[i])?;
            let p = &k.layouts[c.layout];
            let x = (p.x + p.w / 2.) / 760.;
            let y = (p.y + p.h / 2.) / 270.;
            u[i] = (x - 0.5) * cs + (y - 0.5) * sn + 0.5;
            v[i] = -(x - 0.5) * sn + (y - 0.5) * cs + 0.5;
            radius[i] = ((x - 0.5).powi(2) + ((y - 0.5) * 0.6).powi(2)).sqrt();
            polar[i] = ((y - 0.5) * 0.6).atan2(x - 0.5) / std::f64::consts::TAU;
            let a = (i as f64 * 12.9898).sin() * 43758.5453;
            seed[i] = a - a.floor();
            let b = ((u[i] * 15.).floor() * 12.9898).sin() * 43758.5453;
            rain_seed[i] = b - b.floor();
        }
        Ok(Self {
            config: c,
            a,
            b,
            paint,
            u,
            v,
            radius,
            polar,
            seed,
            rain_seed,
        })
    }
    pub fn animated(&self) -> bool {
        !["paint", "gradient", "solid"].contains(&self.config.mode.as_str())
    }
    pub fn frame(&self, phase: f64) -> Frame {
        let mut out = [[0u8; 3]; KEY_COUNT];
        let brightness = self.config.brightness as f64 / 100.;
        for (i, p) in out.iter_mut().enumerate() {
            let u = self.u[i];
            let v = self.v[i];
            let r = self.radius[i];
            let ang = self.polar[i];
            let scale = 0.5 + self.config.scale as f64 / 33.3333333333;
            let mode = self.config.mode.as_str();
            let mut t = 0.;
            let mut gain = 1.;
            let mut hue = None;
            match mode {
                "solid" => {}
                "gradient" => t = u.clamp(0., 1.),
                "wave" => t = ((u * scale * std::f64::consts::TAU - phase * 2.).sin() + 1.) * 0.5,
                "breathe" => gain = 0.06 + 0.94 * ((phase * 2.).sin() + 1.) * 0.5,
                "rainbow" => hue = Some(u * scale * 0.5 - phase * 0.12),
                "aurora" => {
                    t = ((u * 5. + (v * 3. + phase).sin() * 1.2 - phase).sin() + 1.) * 0.5;
                    gain = 0.2 + 0.8 * ((v * 4. - phase * 1.3 + u * 2.).sin() + 1.) * 0.5;
                }
                "spiral" => hue = Some(ang + r * scale - phase * 0.18),
                "ripple" => {
                    let wave = (r * scale * 15. - phase * 4.).sin() * 0.5 + 0.5;
                    t = r.min(1.);
                    gain = 0.08 + 0.92 * wave.powi(3);
                }
                "meteor" => {
                    let head = (phase * 0.22).rem_euclid(1.5) - 0.25;
                    let d = head - u;
                    gain = if d >= 0. {
                        (-d * (6. + scale * 3.)).exp()
                    } else {
                        (-d * d * 800.).exp()
                    };
                    t = v;
                }
                "stars" => {
                    let seed = self.seed[i];
                    gain = 0.02
                        + 0.98
                            * ((phase * (0.6 + seed) + seed * 25.).sin() * 0.5 + 0.5)
                                .powf(8. - scale);
                    t = seed;
                }
                "rain" => {
                    let seed = self.rain_seed[i];
                    let head = (phase * (0.3 + seed * 0.35) + seed * 3.).rem_euclid(1.7) - 0.3;
                    let d = head - v;
                    gain = if d >= 0. {
                        (-d * 8.).exp()
                    } else {
                        (-d * d * 300.).exp()
                    };
                    t = seed;
                }
                "fire" => {
                    let noise = (u * 12. + phase * 2.5).sin() * (u * 7. - phase * 3.1).cos();
                    t = (v * 0.6 + noise * 0.25 + 0.2).clamp(0., 1.);
                    gain = (0.35 + v * 0.6 + noise * 0.2).clamp(0., 1.);
                }
                "checker" => {
                    t = ((u * scale * 10. + phase).sin() * (v * scale * 6. - phase).sin() + 1.)
                        * 0.5;
                }
                "beacon" => {
                    t = ((ang * std::f64::consts::TAU - phase * 2.).cos() + 1.) * 0.5;
                    gain = 0.08 + 0.92 * t.powi(6);
                }
                "sunset" => {
                    t = ((u + v * 0.3) * scale + phase * 0.15).sin() * 0.5 + 0.5;
                }

                "plasma" => {
                    t = 0.5
                        + ((u * scale * 7. + phase).sin()
                            + (v * scale * 7. - phase * 0.8).sin()
                            + (r * scale * 12. - phase * 1.2).sin())
                            / 6.;
                }
                "nebula" => {
                    let cloud =
                        (u * scale * 3. + (v * 5. - phase * 0.25).sin() + phase * 0.35).sin();
                    t = cloud * 0.5 + 0.5;
                    gain = 0.25 + 0.75 * ((u * 4. - v * 3. + phase * 0.4).sin() * 0.5 + 0.5);
                }
                "ocean" => {
                    let wave = (u * scale * 5. - phase * 1.3 + (v * 6. + phase * 0.6).sin()).sin();
                    t = wave * 0.5 + 0.5;
                    gain = 0.35 + 0.65 * ((v * 8. - phase + u * 2.).sin() * 0.5 + 0.5);
                }
                "comets" => {
                    let d = (u + v * 0.28 - phase * 0.18).rem_euclid(0.48);
                    gain = (-d * (10. + scale * 4.)).exp();
                    t = (v * 3. + phase * 0.25).sin() * 0.5 + 0.5;
                }
                "fireflies" => {
                    let seed = self.seed[i];
                    t = seed;
                    gain = 0.015
                        + 0.985
                            * ((phase * (0.35 + seed * 0.3) + seed * 30.).sin() * 0.5 + 0.5)
                                .powi(12)
                            * ((u * 5. - phase * 0.4).sin() * 0.3 + 0.7);
                }
                "chevron" => {
                    t = ((u + (v - 0.5).abs() * 0.8) * scale * 8. - phase * 2.).sin() * 0.5 + 0.5;
                    gain = 0.15 + 0.85 * t;
                }
                "diamonds" => {
                    let d = (u - 0.5).abs() + (v - 0.5).abs() * 0.6;
                    t = (d * scale * 12. - phase * 2.).sin() * 0.5 + 0.5;
                    gain = 0.1 + 0.9 * t.powi(2);
                }
                "helix" => {
                    let center = (u * scale * 6. - phase * 1.5).sin() * 0.3;
                    let a = (-(v - 0.5 - center).powi(2) * 100.).exp();
                    let b = (-(v - 0.5 + center).powi(2) * 100.).exp();
                    t = b / (a + b + 1e-9);
                    gain = (a + b).min(1.);
                }
                "ribbon" => {
                    let center = 0.5 + (u * scale * 3. - phase * 0.7).sin() * 0.25;
                    gain = 0.05 + 0.95 * (-(v - center).powi(2) * (12. + scale * 8.)).exp();
                    t = (u * 3. + phase * 0.4).sin() * 0.5 + 0.5;
                }
                "scanline" => {
                    let head = (phase * 0.9).sin() * 0.5 + 0.5;
                    gain = 0.02 + 0.98 * (-(u - head).powi(2) * (45. + scale * 30.)).exp();
                    t = v.clamp(0., 1.);
                }
                "heartbeat" => {
                    let beat = (phase * 0.55).rem_euclid(1.);
                    gain = 0.04
                        + 0.96
                            * ((-(beat - 0.2).powi(2) * 220.).exp()
                                + 0.65 * (-(beat - 0.42).powi(2) * 300.).exp())
                            .min(1.);
                    t = r.min(1.);
                }
                "pinwheel" => {
                    hue = Some(ang * (2. + scale) + phase * 0.15 + r * 0.6);
                }
                _ => {}
            }
            for j in 0..3 {
                let value = if mode == "paint" {
                    self.paint[i][j]
                } else if let Some(h) = hue {
                    let k = ([0., 8., 4.][j] + h.rem_euclid(1.) * 12.) % 12.;
                    255. * (1. - (k - 3.).min(9. - k).min(1.).max(0.))
                } else {
                    (self.a[j] * (1. - t) + self.b[j] * t) * gain
                };
                p[j] = (value * brightness).round().clamp(0., 255.) as u8;
            }
        }
        out
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
    #[test]
    fn migrates_legacy_rate() {
        let mut c = Config::default();
        c.fps = 120;
        assert_eq!(Scene::new(c, &keys()).unwrap().config.fps, 30);
    }
    #[test]
    fn all_animations_move() {
        for mode in MODES {
            let mut c = Config::default();
            c.mode = mode.to_string();
            let scene = Scene::new(c, &keys()).unwrap();
            if scene.animated() {
                assert_ne!(scene.frame(0.37), scene.frame(2.19), "{}", mode);
            }
        }
    }
    #[test]
    fn rejects_bad_config() {
        let mut c = Config::default();
        c.colors[0] = "no".into();
        assert!(Scene::new(c, &keys()).is_err());
        let mut c = Config::default();
        c.fps = 1000;
        assert!(Scene::new(c, &keys()).is_err());
    }
    #[test]
    fn zero_is_black() {
        for mode in MODES.iter().copied() {
            let mut c = Config::default();
            c.mode = mode.into();
            c.brightness = 0;
            assert_eq!(
                Scene::new(c, &keys()).unwrap().frame(17.),
                [[0; 3]; KEY_COUNT]
            );
        }
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
        let s = Scene::new(Config::default(), &keys()).unwrap();
        let a = s.frame(1.);
        let b = s.frame(1. + 1. / 120.);
        assert!(a
            .iter()
            .zip(b)
            .all(|(a, b)| a.iter().zip(b).all(|(a, b)| a.abs_diff(b) <= 3)));
    }
}
