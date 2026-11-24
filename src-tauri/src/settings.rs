use crate::effects::{Config, Scene};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct Settings {
    pub typing_priority: bool,
    pub intro: bool,
    pub intro_mode: String,
    pub intro_seconds: f64,
    pub auto_play: bool,
    pub caps_enabled: bool,
    pub caps_on: String,
    pub caps_off: String,
    pub caps_off_base: bool,
    pub auto_profiles: bool,
    pub rules: Vec<Rule>,
    pub logo_motion: bool,
    /// UI language tag. Unknown tags fall back to English in the interface.
    pub language: String,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            typing_priority: true,
            intro: true,
            intro_mode: "aurora".into(),
            intro_seconds: 2.8,
            auto_play: true,
            caps_enabled: false,
            caps_on: "#ffcf5c".into(),
            caps_off: "#26334c".into(),
            caps_off_base: true,
            auto_profiles: false,
            rules: vec![],
            logo_motion: true,
            language: "en".into(),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Rule {
    pub name: String,
    pub exe: String,
    pub enabled: bool,
    pub config: Config,
}
impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        if !self.intro_seconds.is_finite()
            || !(0.5..=10.).contains(&self.intro_seconds)
            || self.rules.len() > 50
            || self.language.len() > 12
        {
            return Err("E:bad_options".into());
        }
        crate::effects::rgb(&self.caps_on)?;
        crate::effects::rgb(&self.caps_off)?;
        let mut c = Config::default();
        c.mode = self.intro_mode.clone();
        c.per_key.clear();
        Scene::new(c, crate::protocol::keys())?;
        for r in &self.rules {
            if r.name.trim().is_empty() || r.name.chars().count() > 60 || !valid_exe(&r.exe) {
                return Err("E:bad_rule".into());
            }
            Scene::new(r.config.clone(), crate::protocol::keys())?;
        }
        Ok(())
    }
    pub fn matching_index(&self, exe: &str) -> Option<usize> {
        if !self.auto_profiles {
            return None;
        }
        self.rules
            .iter()
            .position(|r| r.enabled && r.exe.eq_ignore_ascii_case(exe))
    }
}
fn valid_exe(s: &str) -> bool {
    !s.trim().is_empty()
        && s.len() <= 200
        && s.to_lowercase().ends_with(".exe")
        && !s.contains(['/', '\\', ':', '*', '?', '\0'])
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_options_enable_input_priority() {
        let old: Settings = serde_json::from_str("{}").unwrap();
        assert!(old.typing_priority);
        assert_eq!(old.language, "en");
        let explicit: Settings = serde_json::from_str(r#"{"typingPriority":false}"#).unwrap();
        assert!(!explicit.typing_priority);
    }
    #[test]
    fn rules_case_and_priority() {
        let mut s = Settings::default();
        s.auto_profiles = true;
        for n in ["First", "Second"] {
            s.rules.push(Rule {
                name: n.into(),
                exe: "GAME.EXE".into(),
                enabled: true,
                config: Config::default(),
            })
        }
        assert_eq!(s.rules[s.matching_index("game.exe").unwrap()].name, "First");
        s.auto_profiles = false;
        assert!(s.matching_index("game.exe").is_none());
    }
    #[test]
    fn invalid_import() {
        let mut s = Settings::default();
        s.intro_seconds = 99.;
        assert!(s.validate().is_err());
    }
}
