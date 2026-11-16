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
static CAPS: AtomicBool = AtomicBool::new(false);
static CAPS_HELD: AtomicBool = AtomicBool::new(false);
static HOOK_READY: AtomicBool = AtomicBool::new(false);
pub fn caps() -> bool {
    CAPS.load(Ordering::Relaxed)
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
            INPUT.event(
                event.vkCode,
                matches!(w as u32, WM_KEYDOWN | WM_SYSKEYDOWN),
                input_ms(),
            );
        }
        if event.vkCode == VK_CAPITAL as u32 {
            if w as u32 == WM_KEYDOWN || w as u32 == WM_SYSKEYDOWN {
                if !CAPS_HELD.swap(true, Ordering::Relaxed) {
                    CAPS.store((GetKeyState(VK_CAPITAL as i32) & 1) == 0, Ordering::Relaxed);
                }
            } else if w as u32 == WM_KEYUP || w as u32 == WM_SYSKEYUP {
                CAPS_HELD.store(false, Ordering::Relaxed);
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
        CAPS.store(GetKeyState(VK_CAPITAL as i32) & 1 != 0, Ordering::Relaxed);
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
            return Err(format!("Không mở được Startup: {r}"));
        }
        let result = if enabled {
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            let text = format!("\"{}\"", exe.display());
            if text.len() > 259 {
                RegCloseKey(key);
                return Err("Đường dẫn app quá dài cho Startup".into());
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
            Err(format!("Không đổi được Startup: {result}"))
        }
    }
}
