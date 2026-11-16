use crate::protocol::{self, Frame, Key, PID, VID};
use hidapi::{HidApi, HidDevice};
pub struct Keyboard {
    config: HidDevice,
    control: HidDevice,
    output: HidDevice,
    wire_clock: crate::timing::Clock,
}
impl Keyboard {
    pub fn apply_keymap(&self, map: &crate::keymap::Keymap) -> Result<(), String> {
        let report = map.report()?;
        let macros = map.macro_reports()?;
        self.config
            .send_feature_report(&report)
            .map_err(|e| e.to_string())?;
        std::thread::sleep(std::time::Duration::from_millis(100));
        for packet in macros {
            self.config
                .send_feature_report(&packet)
                .map_err(|e| e.to_string())?;
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
        Ok(())
    }
    pub fn present() -> bool {
        HidApi::new()
            .map(|api| {
                api.device_list().any(|d| {
                    d.vendor_id() == VID && d.product_id() == PID && d.usage_page() == 0xff02
                })
            })
            .unwrap_or(false)
    }
    pub fn connect() -> Result<Self, String> {
        let api = HidApi::new().map_err(|e| e.to_string())?;
        let target = api
            .device_list()
            .find(|d| {
                d.vendor_id() == VID
                    && d.product_id() == PID
                    && d.usage_page() == 0xff02
                    && d.usage() == 1
            })
            .ok_or("Chưa thấy Lucky65 USB. Kiểm tra cáp và chế độ có dây.")?;
        let group = target
            .path()
            .to_string_lossy()
            .to_lowercase()
            .split("&col")
            .next()
            .unwrap()
            .to_owned();
        let open = |page, usage| -> Result<HidDevice, String> {
            let d = api
                .device_list()
                .find(|d| {
                    d.vendor_id() == VID
                        && d.product_id() == PID
                        && d.usage_page() == page
                        && d.usage() == usage
                        && d.path()
                            .to_string_lossy()
                            .to_lowercase()
                            .starts_with(&group)
                })
                .ok_or("Thiếu HID collection của Lucky65")?;
            d.open_device(&api).map_err(|e| e.to_string())
        };
        Ok(Self {
            config: open(0xff02, 1)?,
            control: open(0xff03, 1)?,
            output: open(0xff55, 0x202)?,
            wire_clock: crate::timing::Clock::new(),
        })
    }
    pub fn send_frame(&self, frame: &Frame, keys: &[Key], gap_us: u64) -> Result<(), String> {
        self.control
            .send_feature_report(&protocol::DIRECT)
            .map_err(|e| e.to_string())?;
        // The firmware arms ONE frame per DIRECT command (hardware test confirmed).
        // Allow mode/control processing before the complete five-packet transaction.
        self.wire_clock.sleep(std::time::Duration::from_millis(2));
        for (index, p) in protocol::reports(frame, keys).iter().enumerate() {
            if index > 0 {
                self.wire_clock
                    .sleep(std::time::Duration::from_micros(gap_us.max(500)));
            }
            let n = self.output.write(p).map_err(|e| e.to_string())?;
            if n != 64 {
                return Err(format!("HID gửi thiếu: {n}/64 byte"));
            }
        }
        Ok(())
    }
    pub fn effect(&self, id: u8, brightness: u8, speed: u8) -> Result<(), String> {
        let b = protocol::effect(id, brightness, speed, 7)?;
        self.config
            .send_feature_report(&b)
            .map_err(|e| e.to_string())?;
        std::thread::sleep(std::time::Duration::from_millis(100));
        Ok(())
    }
    pub fn static_frame(&self, frame: &Frame, keys: &[Key]) -> Result<(), String> {
        self.effect(19, 4, 3)?;
        self.config
            .send_feature_report(&protocol::custom(frame, keys))
            .map_err(|e| e.to_string())?;
        std::thread::sleep(std::time::Duration::from_millis(50));
        Ok(())
    }
}
