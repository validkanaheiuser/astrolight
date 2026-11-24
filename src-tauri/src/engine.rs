use crate::{
    effects::{blend, Config, Scene},
    protocol::{self, Frame, KEY_COUNT},
    timing::Clock,
    transport::Keyboard,
};
use serde::Serialize;
use std::{
    sync::{
        mpsc::{self, Receiver, SyncSender},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};
use tauri::Emitter;
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub active_profile: String,
    pub foreground: String,
    pub intro: bool,
    pub caps_lock: bool,
    pub caps_monitor: bool,
    pub input_paused: bool,
    pub effective: Config,
    pub connected: bool,
    pub connecting: bool,
    pub running: bool,
    pub auto_connect: bool,
    pub frames: u64,
    pub actual_fps: f64,
    pub send_ms: f64,
    pub missed_deadlines: u64,
    pub phase: f64,
    /// Animation seconds; per-key and reactive effects run on this.
    pub clock: f64,
    pub error: String,
}
impl Default for Status {
    fn default() -> Self {
        Self {
            active_profile: String::new(),
            foreground: String::new(),
            intro: false,
            caps_lock: false,
            caps_monitor: false,
            input_paused: false,
            effective: Config::default(),
            connected: false,
            connecting: true,
            running: false,
            auto_connect: true,
            frames: 0,
            actual_fps: 0.,
            send_ms: 0.,
            missed_deadlines: 0,
            phase: 0.,
            clock: 0.,
            error: String::new(),
        }
    }
}
pub enum Action {
    Connect,
    Disconnect,
    Start,
    Stop,
    Static,
    Effect(u8, u8, u8),
    Keymap(crate::keymap::Keymap),
    Wake,
    Quit,
}
pub struct Command {
    pub action: Action,
    pub reply: Option<mpsc::Sender<Result<Status, String>>>,
}
pub struct Engine {
    pub latest: Arc<Mutex<(u64, Scene)>>,
    pub status: Arc<Mutex<Status>>,
    pub settings: Arc<Mutex<(u64, crate::settings::Settings)>>,
    tx: SyncSender<Command>,
}
impl Engine {
    pub fn new(config: Config, settings: crate::settings::Settings, app: tauri::AppHandle) -> Self {
        let scene = Scene::new(config, protocol::keys()).expect("validated initial config");
        let latest = Arc::new(Mutex::new((0, scene)));
        let status = Arc::new(Mutex::new(Status::default()));
        let (tx, rx) = mpsc::sync_channel(16);
        let options = Arc::new(Mutex::new((0, settings)));
        let opt = options.clone();
        let l = latest.clone();
        let s = status.clone();
        thread::Builder::new()
            .name("astrolight-hid".into())
            .spawn(move || run(rx, l, s, opt, app))
            .expect("RGB worker");
        Self {
            latest,
            status,
            tx,
            settings: options,
        }
    }
    pub fn set_settings(&self, value: crate::settings::Settings) -> Result<(), String> {
        value.validate()?;
        let mut v = self.settings.lock().unwrap();
        v.0 += 1;
        v.1 = value;
        Ok(())
    }
    pub fn update(&self, config: Config) -> Result<(), String> {
        let scene = Scene::new(config, protocol::keys())?;
        let mut latest = self.latest.lock().unwrap();
        latest.0 += 1;
        latest.1 = scene;
        drop(latest);
        let _ = self.tx.try_send(Command {
            action: Action::Wake,
            reply: None,
        });
        Ok(())
    }
    pub fn request(&self, action: Action) -> Result<Receiver<Result<Status, String>>, String> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .try_send(Command {
                action,
                reply: Some(tx),
            })
            .map_err(|_| "E:busy".to_owned())?;
        Ok(rx)
    }
    pub fn shutdown(&self) {
        let _ = self.tx.try_send(Command {
            action: Action::Quit,
            reply: None,
        });
    }
    pub fn config(&self) -> Config {
        self.latest.lock().unwrap().1.config.clone()
    }
}
/// Set while the window is minimized: nobody is looking, so routine status ticks stay
/// in-process instead of waking the webview four times a second.
pub static MINIMIZED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
fn publish(status: &Status, shared: &Mutex<Status>, app: &tauri::AppHandle) {
    *shared.lock().unwrap() = status.clone();
    let _ = app.emit("device:status", status);
}
fn tick(status: &Status, shared: &Mutex<Status>, app: &tauri::AppHandle) {
    if MINIMIZED.load(std::sync::atomic::Ordering::Relaxed) {
        *shared.lock().unwrap() = status.clone();
    } else {
        publish(status, shared, app);
    }
}
/// Which key indices light up for each virtual key in the given layout.
fn press_targets(keys: &[protocol::Key], layout: usize) -> Vec<Vec<usize>> {
    (0..256)
        .map(|vk| {
            crate::system::vk_key_ids(vk)
                .iter()
                .filter_map(|id| keys.iter().position(|k| k.id == *id))
                .filter(|&i| keys[i].layouts[layout].w > 0.)
                .collect()
        })
        .collect()
}
fn run(
    rx: Receiver<Command>,
    latest: Arc<Mutex<(u64, Scene)>>,
    shared: Arc<Mutex<Status>>,
    options: Arc<Mutex<(u64, crate::settings::Settings)>>,
    app: tauri::AppHandle,
) {
    let keys = protocol::keys();
    let clock = Clock::new();
    let mut device: Option<Keyboard> = None;
    let mut status = Status::default();
    let (mut revision, mut scene) = latest.lock().unwrap().clone();
    let mut next = Instant::now();
    let mut last_tick = next;
    let mut last_emit = next;
    let mut next_connect = next;
    let mut next_probe = next;
    let mut meter = next;
    let mut meter_frames = 0u64;
    let mut meter_send = 0.;
    let mut last_frame: Frame = [[0; 3]; KEY_COUNT];
    let mut from = last_frame;
    let mut transition = next - Duration::from_secs(1);
    let mut dirty = true;
    let (mut option_revision, mut settings) = options.lock().unwrap().clone();
    let mut resume = settings.auto_play;
    let mut first_connection = true;
    let mut scene_version = (u64::MAX, u64::MAX, usize::MAX, false);
    let mut stop_after_transition = false;
    let mut intro_started: Option<Instant> = None;
    let mut paused_at: Option<Instant> = None;
    let mut next_foreground = Instant::now();
    let mut caps_on = crate::effects::rgb(&settings.caps_on).unwrap();
    let mut caps_off = crate::effects::rgb(&settings.caps_off).unwrap();
    let mut base = scene.config.clone();
    status.effective = base.clone();
    let mut hits: Vec<(usize, f64)> = Vec::with_capacity(16);
    let mut seen_presses = [0u32; 256];
    let mut targets = press_targets(keys, scene.config.layout);
    loop {
        let command = if status.running {
            rx.try_recv().ok()
        } else {
            match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(c) => Some(c),
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Err(_) => None,
            }
        };
        if let Some(command) = command {
            let mut failure = None;
            let mut quit = false;
            match command.action {
                Action::Quit => quit = true,
                Action::Keymap(map) => {
                    status.error.clear();
                    match &device {
                        Some(d) => {
                            if let Err(e) = d.apply_keymap(&map) {
                                failure = Some(format!("E:keymap_write:{e}"));
                            }
                        }
                        None => failure = Some("E:not_connected".into()),
                    }
                    next = Instant::now();
                    last_tick = next;
                    meter = next;
                    meter_frames = 0;
                    meter_send = 0.;
                }
                Action::Wake => {}
                Action::Disconnect => {
                    intro_started = None;
                    stop_after_transition = false;
                    status.intro = false;
                    status.running = false;
                    resume = false;
                    status.connected = false;
                    status.connecting = false;
                    status.auto_connect = false;
                    device = None;
                    status.error.clear();
                }
                Action::Connect => {
                    status.auto_connect = true;
                    status.connecting = device.is_none();
                    status.error.clear();
                    next_connect = Instant::now();
                }
                Action::Start => {
                    if device.is_some() {
                        status.running = true;
                        resume = true;
                        dirty = true;
                        status.error.clear();
                        next = Instant::now();
                        last_tick = next;
                        meter = next;
                        meter_frames = 0;
                        meter_send = 0.;
                    } else {
                        failure = Some("E:not_connected".into());
                    }
                }
                Action::Stop => {
                    intro_started = None;
                    stop_after_transition = false;
                    status.intro = false;
                    status.running = false;
                    resume = false;
                    status.actual_fps = 0.;
                }
                Action::Static => {
                    intro_started = None;
                    stop_after_transition = false;
                    status.intro = false;
                    status.running = false;
                    resume = false;
                    let config = latest.lock().unwrap().1.clone();
                    let frame = config.frame(status.phase, status.clock, &hits);
                    match &device {
                        Some(d) => {
                            if let Err(e) = d.static_frame(&frame, keys) {
                                failure = Some(e);
                                device = None;
                                status.connected = false;
                            } else {
                                last_frame = frame;
                            }
                        }
                        None => failure = Some("E:not_connected".into()),
                    }
                }
                Action::Effect(id, b, s) => {
                    intro_started = None;
                    stop_after_transition = false;
                    status.intro = false;
                    status.running = false;
                    resume = false;
                    match &device {
                        Some(d) => {
                            if let Err(e) = d.effect(id, b, s) {
                                failure = Some(e);
                                device = None;
                                status.connected = false;
                            }
                        }
                        None => failure = Some("E:not_connected".into()),
                    }
                }
            }
            if let Some(e) = &failure {
                status.error = e.clone();
            }
            publish(&status, &shared, &app);
            if let Some(reply) = command.reply {
                let _ = reply.send(failure.map_or_else(|| Ok(status.clone()), Err));
            }
            if quit {
                break;
            }
        }
        let now = Instant::now();
        if device.is_none() && status.auto_connect && now >= next_connect {
            status.connecting = true;
            publish(&status, &shared, &app);
            match Keyboard::connect() {
                Ok(d) => {
                    device = Some(d);
                    status.connected = true;
                    status.error.clear();
                    status.running = resume;
                    if first_connection {
                        first_connection = false;
                        if settings.intro {
                            intro_started = Some(Instant::now());
                            status.running = true;
                            status.intro = true;
                        }
                    }
                    meter = Instant::now();
                    meter_frames = 0;
                    meter_send = 0.;
                    dirty = true;
                    next = Instant::now();
                    last_tick = next;
                    next_probe = next + Duration::from_secs(3);
                }
                Err(e) => {
                    status.connected = false;
                    status.error = e;
                }
            }
            status.connecting = false;
            next_connect = Instant::now() + Duration::from_secs(2);
            publish(&status, &shared, &app);
        }
        if device.is_some() && !status.running && now >= next_probe {
            next_probe = now + Duration::from_secs(3);
            if !Keyboard::present() {
                device = None;
                status.connected = false;
                status.error = "E:unplugged".into();
                publish(&status, &shared, &app);
            }
        }
        {
            let current = latest.lock().unwrap();
            if current.0 != revision {
                revision = current.0;
                base = current.1.config.clone();
            }
            let opts = options.lock().unwrap();
            if opts.0 != option_revision {
                option_revision = opts.0;
                settings = opts.1.clone();
                caps_on = crate::effects::rgb(&settings.caps_on).unwrap();
                caps_off = crate::effects::rgb(&settings.caps_off).unwrap();
                dirty = true;
                next_foreground = Instant::now();
            }
        }
        if now >= next_foreground {
            next_foreground = now + Duration::from_millis(500);
            if settings.auto_profiles {
                status.foreground = crate::system::foreground_exe().unwrap_or_default();
            } else {
                status.foreground.clear();
            }
        }
        let rule_index = settings.matching_index(&status.foreground);
        let rule = rule_index.map(|i| &settings.rules[i]);
        let wanted = rule.map(|r| &r.config).unwrap_or(&base);
        let label = rule.map(|r| r.name.as_str()).unwrap_or("");
        if let Some(start) = intro_started {
            if paused_at
                .unwrap_or_else(Instant::now)
                .saturating_duration_since(start)
                .as_secs_f64()
                < settings.intro_seconds
            {
                status.intro = true;
            } else {
                intro_started = None;
                status.intro = false;
                status.running = true;
                stop_after_transition = !resume;
                from = last_frame;
                transition = Instant::now();
                dirty = true;
            }
        }
        if status.active_profile != label {
            status.active_profile = label.to_owned();
        }
        let version = (
            revision,
            option_revision,
            rule_index.unwrap_or(usize::MAX),
            status.intro,
        );
        if scene_version != version {
            scene_version = version;
            let mut config = wanted.clone();
            if status.intro {
                config.mode = settings.intro_mode.clone();
                config.per_key.clear();
            }
            scene = Scene::new(config, keys).unwrap();
            targets = press_targets(keys, scene.config.layout);
            from = last_frame;
            transition = Instant::now();
            dirty = true;
            status.effective = scene.config.clone();
        }
        // Fresh presses feed reactive effects. The clock stands still while input priority
        // holds the LEDs, so a burst of typing plays back together once the stream resumes.
        for (vk, seen) in seen_presses.iter_mut().enumerate() {
            let count = crate::system::press_count(vk);
            if count != *seen {
                *seen = count;
                if scene.reactive() && status.running {
                    for &key in &targets[vk] {
                        if hits.len() == 16 {
                            hits.remove(0);
                        }
                        hits.push((key, status.clock));
                    }
                }
            }
        }
        if !scene.reactive() && !hits.is_empty() {
            hits.clear();
        }
        let caps = crate::system::caps();
        if caps != status.caps_lock {
            status.caps_lock = caps;
            if settings.caps_enabled {
                dirty = true;
            }
        }
        status.caps_monitor = crate::system::caps_ready();
        let pause = status.running
            && settings.typing_priority
            && (!status.caps_monitor || crate::system::typing_active());
        if pause != status.input_paused {
            status.input_paused = pause;
            let now = Instant::now();
            if pause {
                paused_at = Some(now);
                status.actual_fps = 0.;
            } else {
                if let Some(start) = paused_at.take() {
                    if let Some(intro) = intro_started.as_mut() {
                        *intro += now - start;
                    }
                }
                from = last_frame;
                transition = now;
                dirty = true;
            }
            next = now;
            last_tick = now;
            meter = now;
            meter_frames = 0;
            meter_send = 0.;
            publish(&status, &shared, &app);
            last_emit = now;
        }
        if status.input_paused {
            // Complete an in-flight frame, then leave USB quiet for held keys and key-up.
            last_tick = Instant::now();
            clock.sleep(Duration::from_millis(8));
        } else if status.running {
            let now = Instant::now();
            let dt = now.duration_since(last_tick).as_secs_f64().min(0.1);
            last_tick = now;
            status.phase += dt * scene.config.speed as f64 / 50.;
            status.clock += dt;
            hits.retain(|&(_, at)| status.clock - at < 30.);
            let in_transition = transition.elapsed() < Duration::from_millis(180);
            if scene.animated() || dirty || in_transition {
                let target = scene.frame(status.phase, status.clock, &hits);
                let mut frame = if in_transition {
                    blend(&from, &target, transition.elapsed().as_secs_f64() / 0.18)
                } else {
                    target
                };
                if settings.caps_enabled && (status.caps_lock || !settings.caps_off_base) {
                    let color = if status.caps_lock { caps_on } else { caps_off };
                    let level = scene.config.brightness as f64 / 100.;
                    if let Some(i) = keys.iter().position(|k| k.id == "caps") {
                        frame[i] = color.map(|c| (c * level).round() as u8);
                    }
                }
                let before = Instant::now();
                // An unchanged frame is already on the LEDs: keep the USB bus quiet.
                let result = if frame == last_frame && !dirty {
                    Ok(false)
                } else {
                    device
                        .as_ref()
                        .unwrap()
                        .send_frame(&frame, keys, 0)
                        .map(|()| true)
                };
                match result {
                    Ok(sent) => {
                        if sent {
                            last_frame = frame;
                            status.frames += 1;
                            meter_frames += 1;
                            meter_send += before.elapsed().as_secs_f64() * 1000.;
                        }
                        dirty = false;
                        if stop_after_transition && !in_transition {
                            status.running = false;
                            stop_after_transition = false;
                        }
                    }
                    Err(e) => {
                        device = None;
                        status.connected = false;
                        status.running = false;
                        status.error = e;
                        next_connect = Instant::now() + Duration::from_secs(2);
                        publish(&status, &shared, &app);
                    }
                }
            }
            if meter.elapsed() >= Duration::from_millis(500) {
                status.actual_fps = meter_frames as f64 / meter.elapsed().as_secs_f64();
                status.send_ms = if meter_frames > 0 {
                    meter_send / meter_frames as f64
                } else {
                    0.
                };
                meter = Instant::now();
                meter_frames = 0;
                meter_send = 0.;
            }
            let period = if scene.animated() || in_transition {
                Duration::from_secs_f64(1. / scene.config.fps as f64)
            } else {
                Duration::from_millis(40)
            };
            let finished = Instant::now();
            if finished > next + period {
                status.missed_deadlines += 1;
            }
            next = crate::timing::next_frame(next, finished, period);
            clock.until(next);
        } else {
            last_tick = Instant::now();
            status.actual_fps = 0.;
        }
        if last_emit.elapsed() >= Duration::from_millis(250) {
            last_emit = Instant::now();
            tick(&status, &shared, &app);
        }
    }
}
