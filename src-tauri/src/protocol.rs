use serde::{Deserialize, Serialize};
pub const VID: u16 = 0x0603;
pub const PID: u16 = 0x0351;
pub const KEY_COUNT: usize = 71;
pub type Frame = [[u8; 3]; KEY_COUNT];
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Key {
    pub id: String,
    pub label: String,
    pub matrix: usize,
    pub layouts: Vec<Rect>,
}
pub fn keys() -> Vec<Key> {
    serde_json::from_str(include_str!("../../ui/keyboard.json")).expect("embedded keyboard map")
}
pub fn reports(frame: &Frame, keys: &[Key]) -> [[u8; 64]; 5] {
    let mut data = [0u8; 310];
    for (i, key) in keys.iter().enumerate() {
        data[key.matrix * 3..key.matrix * 3 + 3].copy_from_slice(&frame[i]);
    }
    let mut result = [[0u8; 64]; 5];
    for (i, p) in result.iter_mut().enumerate() {
        p[0] = 6;
        p[1] = (i + 1) as u8;
        p[2..].copy_from_slice(&data[i * 62..(i + 1) * 62]);
    }
    result
}
pub const DIRECT: [u8; 8] = [8, 6, 0, 0, 0, 0, 0, 0];
pub fn effect(id: u8, brightness: u8, speed: u8, color: u8) -> Result<[u8; 620], String> {
    if id > 19 || brightness > 4 || !(1..=5).contains(&speed) || color > 7 {
        return Err("Tham số firmware không hợp lệ".into());
    }
    let mut b = [0u8; 620];
    b[..3].copy_from_slice(&[7, 2, 1]);
    b[9] = id;
    b[10] = brightness;
    b[11] = speed;
    b[12..32].fill(color);
    Ok(b)
}
pub fn custom(frame: &Frame, keys: &[Key]) -> [u8; 620] {
    let mut b = [0u8; 620];
    b[0] = 7;
    b[1] = 7;
    for (i, k) in keys.iter().enumerate() {
        b[8 + k.matrix * 3..11 + k.matrix * 3].copy_from_slice(&frame[i]);
    }
    b
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn packet_boundaries() {
        let keys = keys();
        let mut frame = [[0; 3]; KEY_COUNT];
        for (i, k) in keys.iter().enumerate() {
            if k.matrix == 41 {
                frame[i] = [71, 82, 93];
            }
        }
        let p = reports(&frame, &keys);
        assert_eq!(p[1][63], 71);
        assert_eq!(&p[2][2..4], &[82, 93]);
        for (i, p) in p.iter().enumerate() {
            assert_eq!(&p[..2], &[6, (i + 1) as u8]);
        }
    }
    #[test]
    fn known_slots() {
        let k = keys();
        let mut f = [[0; 3]; KEY_COUNT];
        f[0] = [1, 2, 3];
        let p = reports(&f, &k);
        assert_eq!(&p[0][53..56], &[1, 2, 3]);
        let b = custom(&f, &k);
        assert_eq!(&b[59..62], &[1, 2, 3]);
        assert_eq!(k.iter().find(|k| k.id == "mspace").unwrap().matrix, 91);
    }
    #[test]
    fn config_validation() {
        assert!(effect(20, 4, 3, 7).is_err());
        assert!(effect(0, 5, 3, 7).is_err());
        let b = effect(2, 3, 4, 7).unwrap();
        assert_eq!(&b[9..13], &[2, 3, 4, 7]);
        assert!(b[32..].iter().all(|&v| v == 0));
    }
}
