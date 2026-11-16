#![allow(dead_code)]
#[path = "../effects.rs"]
mod effects;
#[path = "../protocol.rs"]
mod protocol;
fn main() {
    let keys = protocol::keys();
    let mut cases = Vec::new();
    for mode in effects::MODES {
        for layout in 0..4 {
            for (index, phase) in [0.37, 2.19, 17.3].into_iter().enumerate() {
                let mut c = effects::Config::default();
                c.mode = mode.to_string();
                c.layout = layout;
                c.angle = [-180, 37, 180][index];
                c.scale = [1, 50, 100][index];
                let scene = effects::Scene::new(c.clone(), &keys).unwrap();
                let frame = scene.frame(phase);
                cases.push(serde_json::json!({"config":c,"phase":phase,"frame":frame.as_slice()}));
            }
        }
    }
    println!("{}", serde_json::json!({"keys":keys,"cases":cases}));
}
