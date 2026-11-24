#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod effects;
mod engine;
mod input_guard;
mod keymap;
mod protocol;
mod settings;
mod system;
mod timing;
mod transport;
use effects::{Config, Scene};
use engine::{Action, Engine, Status};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};
use tauri::{Emitter, Manager, State};
#[derive(Clone, Serialize, Deserialize)]
struct Preset {
    name: String,
    config: Config,
}
#[derive(Serialize, Deserialize)]
struct Export {
    format: String,
    version: u8,
    name: String,
    config: Config,
}
struct Store {
    dir: PathBuf,
    lock: Mutex<()>,
}
impl Store {
    fn read(&self) -> Result<Vec<Preset>, String> {
        let p = self.dir.join("presets.json");
        if !p.exists() {
            return Ok(vec![]);
        }
        if fs::metadata(&p).map_err(err)?.len() > 2_000_000 {
            return Err("E:file_too_large".into());
        }
        let list: Vec<Preset> =
            serde_json::from_str(&fs::read_to_string(p).map_err(err)?).map_err(err)?;
        if list.len() > 100 {
            return Err("E:too_many_presets".into());
        }
        for p in &list {
            validate_preset(p)?;
        }
        Ok(list)
    }
    fn write(&self, list: &[Preset]) -> Result<(), String> {
        atomic_write(
            &self.dir.join("presets.json"),
            &serde_json::to_vec_pretty(list).map_err(err)?,
        )
    }
}
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    fs::create_dir_all(path.parent().unwrap()).map_err(err)?;
    let temp = path.with_extension("tmp");
    fs::write(&temp, bytes).map_err(err)?;
    fs::rename(temp, path).map_err(err)
}
fn validate_preset(p: &Preset) -> Result<(), String> {
    if p.name.trim().is_empty() || p.name.chars().count() > 60 {
        return Err("E:preset_name".into());
    }
    Scene::new(p.config.clone(), protocol::keys())?;
    Ok(())
}
#[derive(Serialize)]
struct Initial {
    minimized: bool,
    keys: &'static [protocol::Key],
    config: Config,
    status: Status,
    presets: Vec<Preset>,
    warning: String,
    settings: settings::Settings,
    keymap: keymap::Keymap,
    default_keymap: keymap::Keymap,
    startup: bool,
}
#[tauri::command]
fn init(app: tauri::AppHandle, engine: State<Engine>, store: State<Store>) -> Initial {
    let _guard = store.lock.lock().unwrap();
    let (presets, mut warning) = match store.read() {
        Ok(p) => (p, String::new()),
        Err(e) => (vec![], e),
    };
    let keymap = match read_keymap(&store) {
        Ok(k) => k,
        Err(e) => {
            warning = format!("E:keymap_unreadable:{e}");
            keymap::Keymap::default()
        }
    };
    Initial {
        minimized: app
            .get_webview_window("main")
            .and_then(|w| w.is_minimized().ok())
            .unwrap_or(false),
        keys: protocol::keys(),
        config: engine.config(),
        status: engine.status.lock().unwrap().clone(),
        presets,
        warning,
        settings: engine.settings.lock().unwrap().1.clone(),
        keymap,
        default_keymap: keymap::Keymap::default(),
        startup: system::startup_enabled(),
    }
}
#[tauri::command]
fn update(config: Config, engine: State<Engine>) -> Result<(), String> {
    engine.update(config)
}
#[tauri::command]
async fn control(
    action: String,
    effect: Option<u8>,
    brightness: Option<u8>,
    speed: Option<u8>,
    engine: State<'_, Engine>,
) -> Result<Status, String> {
    let action = match action.as_str() {
        "connect" => Action::Connect,
        "disconnect" => Action::Disconnect,
        "start" => Action::Start,
        "stop" => Action::Stop,
        "static" => Action::Static,
        "effect" => {
            let id = effect.unwrap_or(0);
            let b = brightness.unwrap_or(4);
            let s = speed.unwrap_or(3);
            protocol::effect(id, b, s, 7)?;
            Action::Effect(id, b, s)
        }
        _ => return Err("E:bad_command".into()),
    };
    let rx = engine.request(action)?;
    tauri::async_runtime::spawn_blocking(move || {
        rx.recv_timeout(std::time::Duration::from_secs(5))
            .map_err(|_| "E:hid_timeout".to_owned())?
    })
    .await
    .map_err(err)?
}
#[tauri::command]
fn save_preset(preset: Preset, store: State<Store>) -> Result<Vec<Preset>, String> {
    validate_preset(&preset)?;
    let _guard = store.lock.lock().unwrap();
    let mut items = store.read()?;
    if let Some(i) = items.iter().position(|p| p.name == preset.name) {
        items[i] = preset;
    } else {
        if items.len() >= 100 {
            return Err("E:too_many_presets".into());
        }
        items.push(preset);
    }
    store.write(&items)?;
    Ok(items)
}
#[tauri::command]
fn delete_preset(name: String, store: State<Store>) -> Result<Vec<Preset>, String> {
    let _guard = store.lock.lock().unwrap();
    let mut items = store.read()?;
    items.retain(|p| p.name != name);
    store.write(&items)?;
    Ok(items)
}
#[tauri::command]
async fn export_preset(preset: Preset, title: Option<String>) -> Result<bool, String> {
    validate_preset(&preset)?;
    tauri::async_runtime::spawn_blocking(move || {
        let Some(path) = rfd::FileDialog::new()
            .set_title(title.as_deref().unwrap_or("Export preset"))
            .add_filter("JSON", &["json"])
            .set_file_name("astrolight-preset.json")
            .save_file()
        else {
            return Ok(false);
        };
        let data = Export {
            format: "astrolight".into(),
            version: 2,
            name: preset.name,
            config: preset.config,
        };
        fs::write(path, serde_json::to_vec_pretty(&data).map_err(err)?).map_err(err)?;
        Ok(true)
    })
    .await
    .map_err(err)?
}
#[tauri::command]
async fn import_preset(title: Option<String>) -> Result<Option<Preset>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let Some(path) = rfd::FileDialog::new()
            .set_title(title.as_deref().unwrap_or("Import preset"))
            .add_filter("JSON", &["json"])
            .pick_file()
        else {
            return Ok(None);
        };
        if fs::metadata(&path).map_err(err)?.len() > 100_000 {
            return Err("E:file_too_large".into());
        }
        let data: Export =
            serde_json::from_str(&fs::read_to_string(path).map_err(err)?).map_err(err)?;
        if !((data.format == "astrolight" && data.version == 2)
            || (data.format == "lucky65-studio" && data.version == 1))
        {
            return Err("E:preset_format".into());
        }
        let p = Preset {
            name: data.name,
            config: data.config,
        };
        validate_preset(&p)?;
        Ok(Some(p))
    })
    .await
    .map_err(err)?
}
fn read_keymap(store: &Store) -> Result<keymap::Keymap, String> {
    let path = store.dir.join("keymap.json");
    if !path.exists() {
        return Ok(keymap::Keymap::default());
    }
    if fs::metadata(&path).map_err(err)?.len() > 500_000 {
        return Err("E:file_too_large".into());
    }
    let map: keymap::Keymap = serde_json::from_slice(&fs::read(path).map_err(err)?).map_err(err)?;
    map.validate()?;
    Ok(map)
}
#[tauri::command]
fn save_options(
    options: settings::Settings,
    engine: State<Engine>,
    store: State<Store>,
) -> Result<(), String> {
    options.validate()?;
    let _g = store.lock.lock().unwrap();
    atomic_write(
        &store.dir.join("options.json"),
        &serde_json::to_vec_pretty(&options).map_err(err)?,
    )?;
    engine.set_settings(options)
}
#[tauri::command]
fn save_keymap(map: keymap::Keymap, store: State<Store>) -> Result<(), String> {
    map.validate()?;
    let _g = store.lock.lock().unwrap();
    atomic_write(
        &store.dir.join("keymap.json"),
        &serde_json::to_vec_pretty(&map).map_err(err)?,
    )
}
#[tauri::command]
async fn apply_keymap(
    map: keymap::Keymap,
    engine: State<'_, Engine>,
    store: State<'_, Store>,
) -> Result<Status, String> {
    map.validate()?;
    let rx = engine.request(Action::Keymap(map.clone()))?;
    let result = tauri::async_runtime::spawn_blocking(move || {
        rx.recv_timeout(std::time::Duration::from_secs(15))
            .map_err(|_| {
                "E:keymap_timeout".to_string()
            })?
    })
    .await
    .map_err(err)??;
    let _g = store.lock.lock().unwrap();
    atomic_write(
        &store.dir.join("keymap.json"),
        &serde_json::to_vec_pretty(&map).map_err(err)?,
    )?;
    Ok(result)
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Workspace {
    format: String,
    version: u8,
    config: Config,
    settings: settings::Settings,
    keymap: keymap::Keymap,
    presets: Vec<Preset>,
}
impl Workspace {
    fn validate(&self) -> Result<(), String> {
        if self.format != "astrolight-workspace" || !(3..=4).contains(&self.version) {
            return Err("E:workspace_format".into());
        }
        Scene::new(self.config.clone(), protocol::keys())?;
        self.settings.validate()?;
        self.keymap.validate()?;
        if self.presets.len() > 100 {
            return Err("E:too_many_presets".into());
        }
        for p in &self.presets {
            validate_preset(p)?
        }
        Ok(())
    }
}
#[tauri::command]
async fn export_workspace(bundle: Workspace, title: Option<String>) -> Result<bool, String> {
    bundle.validate()?;
    tauri::async_runtime::spawn_blocking(move || {
        let Some(path) = rfd::FileDialog::new()
            .set_title(title.as_deref().unwrap_or("Export Astrolight backup"))
            .add_filter("JSON", &["json"])
            .set_file_name("astrolight-workspace.json")
            .save_file()
        else {
            return Ok(false);
        };
        fs::write(path, serde_json::to_vec_pretty(&bundle).map_err(err)?).map_err(err)?;
        Ok(true)
    })
    .await
    .map_err(err)?
}
#[tauri::command]
async fn import_workspace(title: Option<String>) -> Result<Option<Workspace>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let Some(path) = rfd::FileDialog::new()
            .set_title(title.as_deref().unwrap_or("Import Astrolight backup"))
            .add_filter("JSON", &["json"])
            .pick_file()
        else {
            return Ok(None);
        };
        if fs::metadata(&path).map_err(err)?.len() > 2_000_000 {
            return Err("E:file_too_large".into());
        }
        let bundle: Workspace =
            serde_json::from_slice(&fs::read(path).map_err(err)?).map_err(err)?;
        bundle.validate()?;
        Ok(Some(bundle))
    })
    .await
    .map_err(err)?
}
#[tauri::command]
fn apply_workspace(
    bundle: Workspace,
    engine: State<Engine>,
    store: State<Store>,
) -> Result<(), String> {
    bundle.validate()?;
    let _g = store.lock.lock().unwrap();
    let items = vec![
        (
            "settings.json",
            serde_json::to_vec_pretty(&bundle.config).map_err(err)?,
        ),
        (
            "options.json",
            serde_json::to_vec_pretty(&bundle.settings).map_err(err)?,
        ),
        (
            "keymap.json",
            serde_json::to_vec_pretty(&bundle.keymap).map_err(err)?,
        ),
        (
            "presets.json",
            serde_json::to_vec_pretty(&bundle.presets).map_err(err)?,
        ),
    ];
    let old: Vec<_> = items
        .iter()
        .map(|(name, _)| fs::read(store.dir.join(name)).ok())
        .collect();
    for (i, (name, data)) in items.iter().enumerate() {
        if let Err(e) = atomic_write(&store.dir.join(name), data) {
            for j in 0..i {
                let path = store.dir.join(items[j].0);
                if let Some(b) = &old[j] {
                    let _ = atomic_write(&path, b);
                } else {
                    let _ = fs::remove_file(path);
                }
            }
            return Err(e);
        }
    }
    engine.set_settings(bundle.settings)?;
    engine.update(bundle.config)?;
    Ok(())
}
#[tauri::command]
fn set_startup(enabled: bool) -> Result<bool, String> {
    system::set_startup(enabled)?;
    Ok(system::startup_enabled())
}
#[tauri::command]
async fn choose_exe(title: Option<String>) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        rfd::FileDialog::new()
            .set_title(title.as_deref().unwrap_or("Choose an app or game"))
            .add_filter("EXE", &["exe"])
            .pick_file()
            .and_then(|p| p.file_name().map(|f| f.to_string_lossy().to_string()))
    })
    .await
    .map_err(err)
}
fn main() {
    system::start_caps_monitor();
    // Keep exactly one RGB owner. A second launch must not race the HID worker.
    let instance_name: Vec<u16> = "Local\\AstrolightLucky65Studio\0".encode_utf16().collect();
    let _instance = unsafe {
        windows_sys::Win32::System::Threading::CreateMutexW(
            std::ptr::null(),
            0,
            instance_name.as_ptr(),
        )
    };
    if !_instance.is_null()
        && unsafe { windows_sys::Win32::Foundation::GetLastError() }
            == windows_sys::Win32::Foundation::ERROR_ALREADY_EXISTS
    {
        return;
    }
    tauri::Builder::default()
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            fs::create_dir_all(&dir)?;
            // First-run migration preserves existing Electron presets, without modifying their source.
            if !dir.join("presets.json").exists() {
                if let Ok(roaming) = std::env::var("APPDATA") {
                    let old = PathBuf::from(roaming).join("lucky65-studio/presets.json");
                    if let Ok(data) = fs::read(old) {
                        if let Ok(items) = serde_json::from_slice::<Vec<Preset>>(&data) {
                            if items.len() <= 100
                                && items.iter().all(|p| validate_preset(p).is_ok())
                            {
                                let _ = atomic_write(
                                    &dir.join("presets.json"),
                                    &serde_json::to_vec(&items)?,
                                );
                            }
                        }
                    }
                }
            }
            let config = fs::read(dir.join("settings.json"))
                .ok()
                .and_then(|b| serde_json::from_slice::<Config>(&b).ok())
                .filter(|c| Scene::new(c.clone(), protocol::keys()).is_ok())
                .unwrap_or_default();
            let options = fs::read(dir.join("options.json"))
                .ok()
                .and_then(|b| serde_json::from_slice::<settings::Settings>(&b).ok())
                .filter(|s| s.validate().is_ok())
                .unwrap_or_default();
            app.manage(Engine::new(config, options, app.handle().clone()));
            app.manage(Store {
                dir,
                lock: Mutex::new(()),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            init,
            update,
            control,
            save_preset,
            delete_preset,
            export_preset,
            import_preset,
            save_options,
            save_keymap,
            apply_keymap,
            export_workspace,
            import_workspace,
            apply_workspace,
            set_startup,
            choose_exe
        ])
        .on_window_event(|window, event| {
            if matches!(
                event,
                tauri::WindowEvent::Resized(_) | tauri::WindowEvent::Focused(_)
            ) {
                let minimized = window.is_minimized().unwrap_or(false);
                engine::MINIMIZED.store(minimized, std::sync::atomic::Ordering::Relaxed);
                let _ = window.emit("window:minimized", minimized);
                if let Some(view) = window.app_handle().get_webview_window("main") {
                    let _ = view.with_webview(move |platform| {
                        use webview2_com::Microsoft::Web::WebView2::Win32::{
                            ICoreWebView2_19, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW,
                            COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL,
                        };
                        use windows_core::Interface;
                        // Optional on older WebView2 runtimes. The RGB worker is independent.
                        unsafe {
                            let _ = platform.controller().SetIsVisible(!minimized);
                            if let Ok(core) = platform.controller().CoreWebView2() {
                                if let Ok(core) = core.cast::<ICoreWebView2_19>() {
                                    let _ = core.SetMemoryUsageTargetLevel(if minimized {
                                        COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW
                                    } else {
                                        COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL
                                    });
                                }
                            }
                        }
                    });
                }
            }
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                let engine = window.state::<Engine>();
                let store = window.state::<Store>();
                if let Ok(data) = serde_json::to_vec(&engine.config()) {
                    let _ = atomic_write(&store.dir.join("settings.json"), &data);
                }
                engine.shutdown();
            }
        })
        .run(tauri::generate_context!())
        .expect("Astrolight failed to start");
}
