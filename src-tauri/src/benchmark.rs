#![allow(dead_code)]
mod effects;
mod keymap;
mod protocol;
mod timing;
mod transport;
use std::time::{Duration, Instant};
fn main() {
    if !std::env::args().any(|a| a == "--write-rgb") {
        eprintln!("Explicit hardware test: hid-benchmark --write-rgb [--seconds 3]");
        return;
    }
    let args: Vec<_> = std::env::args().collect();
    let seconds = args
        .iter()
        .position(|a| a == "--seconds")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(3)
        .clamp(1, 30);
    let k = protocol::keys();
    let device = transport::Keyboard::connect().expect("connect Lucky65");
    let clock = timing::Clock::new();
    let scene = effects::Scene::new(effects::Config::default(), &k).unwrap();
    for fps in [15, 30] {
        let start = Instant::now();
        let mut deadline = start;
        let mut samples = Vec::new();
        let mut intervals = Vec::new();
        let mut last = start;
        let mut late = 0;
        while start.elapsed() < Duration::from_secs(seconds) {
            let t = Instant::now();
            intervals.push(t.duration_since(last).as_secs_f64() * 1000.);
            last = t;
            device
                .send_frame(&scene.frame(start.elapsed().as_secs_f64() * 0.7), &k, 0)
                .expect("write RGB");
            samples.push(t.elapsed().as_secs_f64() * 1000.);
            let period = Duration::from_secs_f64(1. / fps as f64);
            let finished = Instant::now();
            if finished > deadline + period {
                late += 1;
            }
            deadline = timing::next_frame(deadline, finished, period);
            clock.until(deadline);
        }
        let elapsed = start.elapsed().as_secs_f64();
        samples.sort_by(f64::total_cmp);
        intervals.remove(0);
        intervals.sort_by(f64::total_cmp);
        println!(
            "{}",
            serde_json::json!({"targetFps":fps,"actualFps":samples.len()as f64/elapsed,"frames":samples.len(),"sendMedianMs":samples[samples.len()/2],"sendP95Ms":samples[samples.len()*95/100],"intervalP95Ms":intervals[intervals.len()*95/100],"deadlineMisses":late})
        );
    }
}
