#![allow(dead_code)]
#[path = "../effects.rs"]
mod effects;
#[path = "../protocol.rs"]
mod protocol;
fn main() {
    let keys = protocol::keys();
    let mut cases = Vec::new();
    let hits = [(20usize, 16.9), (41, 16.2), (57, 2.0), (3, 0.1), (20, 0.3)];
    for mode in effects::MODES {
        for layout in 0..4 {
            for (index, phase) in [0.37, 2.19, 17.3].into_iter().enumerate() {
                let mut c = effects::Config::default();
                c.mode = mode.to_string();
                c.layout = layout;
                c.angle = [-180, 37, 180][index];
                c.scale = [1, 50, 100][index];
                if layout == 1 {
                    // Per-key effects: every seventh key runs a different mode at its own speed.
                    c.per_key = vec![None; protocol::KEY_COUNT];
                    let others: Vec<_> = effects::MODES.iter().filter(|m| **m != "paint").collect();
                    for i in (0..protocol::KEY_COUNT).step_by(7) {
                        c.per_key[i] = Some(effects::KeyFx {
                            mode: others[(i + cases.len()) % others.len()].to_string(),
                            color_a: "#ff7a3c".into(),
                            color_b: "#3c7aff".into(),
                            speed: (10 + i) as u8,
                        });
                    }
                }
                let clock = phase * 1.3;
                let scene = effects::Scene::new(c.clone(), keys).unwrap();
                let frame = scene.frame(phase, clock, &hits);
                cases.push(serde_json::json!({"config":scene.config,"phase":phase,"clock":clock,"hits":hits,"frame":frame.as_slice()}));
            }
        }
    }
    println!("{}", serde_json::json!({"keys":keys,"cases":cases}));
}
