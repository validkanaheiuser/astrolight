use std::sync::atomic::{AtomicBool, Ordering};
use windows_sys::Win32::{
    Foundation::*,
    System::{Registry::*, Threading::*},
    UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};
static INPUT: crate::input_guard::InputGuard = crate::input_guard::InputGuard::new();
static INPUT_EPOCH: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
fn input_ms() -> u64 {
    INPUT_EPOCH
        .get_or_init(std::time::Instant::now)
        .elapsed()
        .as_millis() as u64
}
pub fn typing_active() -> bool {
    let now = input_ms();
    INPUT.reconcile(now, |key| unsafe { GetAsyncKeyState(key as i32) < 0 });
    INPUT.busy(now)
}
// Reactive lighting: a counter per virtual key, bumped on each fresh press (not on
// auto-repeat). Counters only; nothing about order or timing is kept.
static PRESSES: [std::sync::atomic::AtomicU32; 256] =
    [const { std::sync::atomic::AtomicU32::new(0) }; 256];
static DOWN: [AtomicBool; 256] = [const { AtomicBool::new(false) }; 256];
pub fn press_count(vk: usize) -> u32 {
    PRESSES[vk & 255].load(Ordering::Relaxed)
}
/// Virtual-key code to `keyboard.json` ids. Space and backslash exist at more than one
/// position depending on the layout; the caller keeps whichever are present.
pub fn vk_key_ids(vk: usize) -> &'static [&'static str] {
    const LETTERS: [&str; 26] = [
        "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p", "q", "r",
        "s", "t", "u", "v", "w", "x", "y", "z",
    ];
    const DIGITS: [&str; 10] = ["n0", "n1", "n2", "n3", "n4", "n5", "n6", "n7", "n8", "n9"];
    match vk {
        0x41..=0x5A => std::slice::from_ref(&LETTERS[vk - 0x41]),
        0x30..=0x39 => std::slice::from_ref(&DIGITS[vk - 0x30]),
        0x1B => &["esc"],
        0xBD => &["minus"],
        0xBB => &["plus"],
        0x08 => &["bksp"],
        0x2D => &["Insert"],
        0x09 => &["tab"],
        0xDB => &["lqu"],
        0xDD => &["rqu"],
        0xDC => &["k29", "k42"],
        0x2E => &["delete"],
        0x14 => &["caps"],
        0xBA => &["sem"],
        0xDE => &["quo"],
        0x0D => &["enter"],
        0x21 => &["pageup"],
        0x22 => &["pagedown"],
        0xA0 | 0x10 => &["lshift"],
        0xA1 => &["rshift"],
        0xBC => &["comma"],
        0xBE => &["dot"],
        0xBF => &["qmark"],
        0x26 => &["up"],
        0x25 => &["left"],
        0x28 => &["down"],
        0x27 => &["right"],
        0xA2 | 0x11 => &["lctrl"],
        0xA3 => &["rctrl"],
        0x5B => &["win"],
        0xA4 | 0x12 => &["lalt"],
        0xA5 => &["ralt"],
        0x20 => &["mspace", "lspace", "rspace"],
        _ => &[],
    }
}
static HOOK_READY: AtomicBool = AtomicBool::new(false);
/// The real Caps Lock toggle, read from Windows each time. Earlier versions predicted it
/// inside the keyboard hook, which drifted for good after one missed or injected event.
pub fn caps() -> bool {
    unsafe { GetKeyState(VK_CAPITAL as i32) & 1 != 0 }
}
pub fn caps_ready() -> bool {
    HOOK_READY.load(Ordering::Relaxed)
}
unsafe extern "system" fn keyboard_hook(code: i32, w: WPARAM, l: LPARAM) -> LRESULT {
    if code >= 0 {
        let event = &*(l as *const KBDLLHOOKSTRUCT);
        if matches!(
            w as u32,
            WM_KEYDOWN | WM_SYSKEYDOWN | WM_KEYUP | WM_SYSKEYUP
        ) {
            let down = matches!(w as u32, WM_KEYDOWN | WM_SYSKEYDOWN);
            INPUT.event(event.vkCode, down, input_ms());
            let vk = (event.vkCode & 255) as usize;
            if DOWN[vk].swap(down, Ordering::Relaxed) != down && down {
                PRESSES[vk].fetch_add(1, Ordering::Relaxed);
            }
        }
    }
    CallNextHookEx(std::ptr::null_mut(), code, w, l)
}
pub fn start_caps_monitor() {
    std::thread::spawn(|| unsafe {
        for key in 8..256 {
            if GetAsyncKeyState(key) < 0 {
                INPUT.event(key as u32, true, input_ms());
            }
        }
        let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook), std::ptr::null_mut(), 0);
        if hook.is_null() {
            return;
        }
        HOOK_READY.store(true, Ordering::Relaxed);
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        UnhookWindowsHookEx(hook);
        HOOK_READY.store(false, Ordering::Relaxed);
    });
}
pub fn foreground_exe() -> Option<String> {
    unsafe {
        let window = GetForegroundWindow();
        if window.is_null() {
            return None;
        }
        let mut pid = 0;
        GetWindowThreadProcessId(window, &mut pid);
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return None;
        }
        let mut buf = [0u16; 32768];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut len);
        CloseHandle(handle);
        if ok == 0 {
            return None;
        }
        let path = String::from_utf16_lossy(&buf[..len as usize]);
        Some(path.rsplit('\\').next()?.to_lowercase())
    }
}
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
const RUN: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
pub fn startup_enabled() -> bool {
    unsafe {
        let mut key = std::ptr::null_mut();
        if RegOpenKeyExW(HKEY_CURRENT_USER, wide(RUN).as_ptr(), 0, KEY_READ, &mut key) != 0 {
            return false;
        }
        let mut buf = [0u16; 1024];
        let mut size = (buf.len() * 2) as u32;
        let mut typ = 0;
        let r = RegQueryValueExW(
            key,
            wide("Astrolight").as_ptr(),
            std::ptr::null(),
            &mut typ,
            buf.as_mut_ptr() as *mut u8,
            &mut size,
        );
        RegCloseKey(key);
        r == 0 && typ == REG_SZ
    }
}
pub fn set_startup(enabled: bool) -> Result<(), String> {
    unsafe {
        let mut key = std::ptr::null_mut();
        let r = RegCreateKeyExW(
            HKEY_CURRENT_USER,
            wide(RUN).as_ptr(),
            0,
            std::ptr::null(),
            0,
            KEY_WRITE,
            std::ptr::null(),
            &mut key,
            std::ptr::null_mut(),
        );
        if r != 0 {
            return Err(format!("E:startup_open:{r}"));
        }
        let result = if enabled {
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            let text = format!("\"{}\"", exe.display());
            if text.len() > 259 {
                RegCloseKey(key);
                return Err("E:startup_path".into());
            }
            let val = wide(&text);
            RegSetValueExW(
                key,
                wide("Astrolight").as_ptr(),
                0,
                REG_SZ,
                val.as_ptr() as *const u8,
                (val.len() * 2) as u32,
            )
        } else {
            RegDeleteValueW(key, wide("Astrolight").as_ptr())
        };
        RegCloseKey(key);
        if result == 0 || (!enabled && result == ERROR_FILE_NOT_FOUND) {
            Ok(())
        } else {
            Err(format!("E:startup_set:{result}"))
        }
    }
}
