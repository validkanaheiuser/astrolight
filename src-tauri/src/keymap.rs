use crate::protocol::{self, KEY_COUNT};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct Binding {
    pub kind: String,
    pub code: u8,
    pub mods: u8,
    pub macro_id: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Step {
    pub code: u8,
    pub down: bool,
    pub delay: u16,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Macro {
    pub name: String,
    pub repeat: u16,
    pub steps: Vec<Step>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Keymap {
    pub layers: Vec<Vec<Binding>>,
    pub fn_mode: u8,
    pub macros: Vec<Macro>,
}
impl Default for Keymap {
    fn default() -> Self {
        Self {
            layers: vec![
                vec![
                    Binding {
                        kind: "default".into(),
                        ..Default::default()
                    };
                    KEY_COUNT
                ];
                3
            ],
            fn_mode: 0,
            macros: vec![],
        }
    }
}
impl Keymap {
    pub fn validate(&self) -> Result<(), String> {
        if self.layers.len() != 3
            || self.layers.iter().any(|l| l.len() != KEY_COUNT)
            || self.fn_mode > 1
            || self.macros.len() > 10
        {
            return Err("E:keymap_shape".into());
        }
        let codes: Vec<serde_json::Value> =
            serde_json::from_str(include_str!("../../ui/keycodes.json")).unwrap();
        let valid_code = |c: u8| codes.iter().any(|v| v["code"].as_u64() == Some(c as u64));
        for m in &self.macros {
            if m.name.trim().is_empty()
                || m.name.chars().count() > 60
                || m.repeat == 0
                || m.repeat > 1000
                || m.steps.len() > 84
            {
                return Err("E:macro_limits".into());
            }
            let mut held = std::collections::HashSet::new();
            for s in &m.steps {
                if s.delay < 20
                    || s.delay > 32767
                    || !((4..=115).contains(&s.code) || (224..=231).contains(&s.code))
                {
                    return Err("E:macro_step".into());
                }
                if s.down {
                    if !held.insert(s.code) {
                        return Err("E:macro_repeat_press".into());
                    }
                } else if !held.remove(&s.code) {
                    return Err("E:macro_release".into());
                }
            }
            if !held.is_empty() {
                return Err("E:macro_unbalanced".into());
            }
        }
        for b in self.layers.iter().flatten() {
            match b.kind.as_str() {
                "default" => {}
                "key" if valid_code(b.code) => {}
                "combo" if (4..=115).contains(&b.code) && b.mods <= 15 => {}
                "macro"
                    if b.macro_id < self.macros.len()
                        && !self.macros[b.macro_id].steps.is_empty() => {}
                _ => return Err("E:bad_binding".into()),
            }
        }
        Ok(())
    }
    pub fn report(&self) -> Result<[u8; 620], String> {
        self.validate()?;
        let defaults: Vec<u8> =
            serde_json::from_str(include_str!("../../ui/default-keymap.json")).unwrap();
        let keys = protocol::keys();
        let mut out = [0; 620];
        out[..3].copy_from_slice(&[7, 3, self.fn_mode + 1]);
        for (layer, bindings) in self.layers.iter().enumerate() {
            for (i, b) in bindings.iter().enumerate() {
                let pos = 8 + layer * 204 + keys[i].matrix * 2;
                let (flags, code) = match b.kind.as_str() {
                    "default" => (0, defaults[keys[i].matrix]),
                    "macro" => (128, 240 + b.macro_id as u8),
                    "combo" => (128 + b.mods, b.code),
                    _ => (128, b.code),
                };
                out[pos] = flags;
                out[pos + 1] = code;
            }
        }
        Ok(out)
    }
    pub fn macro_reports(&self) -> Result<Vec<[u8; 620]>, String> {
        self.validate()?;
        let mut out = vec![];
        for (slot, m) in self.macros.iter().enumerate() {
            let mut b = [0; 620];
            b[..4].copy_from_slice(&[7, 5, 1, slot as u8]);
            b[8..10].copy_from_slice(&m.repeat.to_be_bytes());
            for (i, s) in m.steps.iter().enumerate() {
                let d = s.delay | if s.down { 0x8000 } else { 0 };
                b[10 + i * 3..12 + i * 3].copy_from_slice(&d.to_be_bytes());
                b[12 + i * 3] = s.code;
            }
            out.push(b)
        }
        Ok(out)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn matches_vendor_reports_byte_for_byte() {
        let mut k = Keymap::default();
        k.fn_mode = 1;
        k.layers[0][0] = Binding {
            kind: "combo".into(),
            code: 4,
            mods: 1,
            macro_id: 0,
        };
        k.layers[1][1] = Binding {
            kind: "macro".into(),
            code: 0,
            mods: 0,
            macro_id: 0,
        };
        k.layers[2][2] = Binding {
            kind: "key".into(),
            code: 163,
            mods: 0,
            macro_id: 0,
        };
        k.macros.push(Macro {
            name: "A".into(),
            repeat: 2,
            steps: vec![
                Step {
                    code: 4,
                    down: true,
                    delay: 20,
                },
                Step {
                    code: 4,
                    down: false,
                    delay: 50,
                },
            ],
        });
        assert_eq!(
            &k.report().unwrap(),
            include_bytes!("../fixtures/vendor-keymap.bin")
        );
        assert_eq!(
            &k.macro_reports().unwrap()[0],
            include_bytes!("../fixtures/vendor-macro.bin")
        );
    }
    #[test]
    fn mapping_boundaries() {
        let k = Keymap::default();
        let r = k.report().unwrap();
        assert_eq!(&r[..3], &[7, 3, 1]);
        assert_eq!(&r[8 + 51 * 2..10 + 51 * 2], &[0, 57]);
        assert_eq!(r.len(), 620);
    }
    #[test]
    fn macro_bounds_and_release() {
        let mut k = Keymap::default();
        k.macros.push(Macro {
            name: "A".into(),
            repeat: 1,
            steps: vec![Step {
                code: 4,
                down: true,
                delay: 20,
            }],
        });
        assert!(k.validate().is_err());
        k.macros[0].steps.push(Step {
            code: 4,
            down: false,
            delay: 50,
        });
        let r = k.macro_reports().unwrap();
        assert_eq!(&r[0][8..16], &[0, 1, 128, 20, 4, 0, 50, 4]);
        k.macros[0].steps = vec![
            Step {
                code: 4,
                down: true,
                delay: 20
            };
            85
        ];
        assert!(k.validate().is_err());
    }
}
