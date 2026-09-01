//! Application settings persisted to %APPDATA%\SoundManager\SoundManager.ini.
//! Port of Settings.cs + a minimal INI reader (replaces SharpTools.INIFile).

use crate::errors::CoreResult;
use crate::domain::sound_event::ALL_EVENTS;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Patch imageres.dll to customize the startup sound (Vista+).
    pub patch_startup_sound: bool,
    /// Use the default system sound when a sound is missing after loading an archive.
    pub missing_sound_use_default: bool,
    /// Convert proprietary (.soundpack) files to .ths on import, recycling the original.
    pub convert_proprietary_files: bool,
    /// Prefer Startup/Shutdown over Logon/Logoff sounds.
    pub prefer_startup_sound_on_logon: bool,
    /// Internal names of disabled sound events (they will not play).
    pub disabled_sound_events: Vec<String>,
    /// Accessibility: render scheme items as a list.
    pub scheme_items_list_view: bool,
    /// UI language key: "eng" or "fra".
    pub language: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            patch_startup_sound: false,
            missing_sound_use_default: true,
            convert_proprietary_files: false,
            prefer_startup_sound_on_logon: false,
            disabled_sound_events: Vec::new(),
            scheme_items_list_view: false,
            language: "eng".to_string(),
        }
    }
}

pub fn settings_file() -> PathBuf {
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(appdata).join("SoundManager").join("SoundManager.ini")
}

/// Minimal INI parser: [Section] + key=value lines; '#' and ';' comments.
pub fn parse_ini(text: &str) -> Vec<(String, Vec<(String, String)>)> {
    let mut sections: Vec<(String, Vec<(String, String)>)> = Vec::new();
    let mut current = ("default".to_string(), Vec::new());
    for raw in text.lines() {
        let line = raw.split('#').next().unwrap_or("").trim().to_string();
        if line.is_empty() || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') && line.len() > 2 {
            if !current.1.is_empty() || current.0 != "default" {
                sections.push(current);
            }
            current = (line[1..line.len() - 1].to_lowercase(), Vec::new());
        } else if let Some(eq) = line.find('=') {
            let key = line[..eq].trim().to_lowercase();
            let value = line[eq + 1..].trim().to_string();
            current.1.push((key, value));
        }
    }
    sections.push(current);
    sections
}

fn ini_escape(v: &str) -> String {
    // Values we store are bools and comma-joined identifiers: safe as-is.
    v.to_string()
}

impl Settings {
    pub fn load() -> Self {
        Self::load_from(&settings_file()).unwrap_or_default()
    }

    pub fn load_from(path: &Path) -> CoreResult<Self> {
        let text = std::fs::read_to_string(path)?;
        let mut s = Settings::default();
        let only_valid: HashSet<&str> = ALL_EVENTS.iter().map(|e| e.internal_name).collect();
        for (section, entries) in parse_ini(&text) {
            if section != "main" {
                continue;
            }
            for (key, value) in entries {
                match key.as_str() {
                    "win7patch" | "patchstartupsound" => s.patch_startup_sound = value.eq_ignore_ascii_case("true"),
                    "usedefaultonmissingsound" => s.missing_sound_use_default = value.eq_ignore_ascii_case("true"),
                    "convertproprietaryfiles" => s.convert_proprietary_files = value.eq_ignore_ascii_case("true"),
                    "preferstartupsoundonlogon" => s.prefer_startup_sound_on_logon = value.eq_ignore_ascii_case("true"),
                    "disabledsoundevents" => {
                        s.disabled_sound_events = value
                            .split(',')
                            .map(|v| v.trim().to_string())
                            .filter(|v| !v.is_empty() && only_valid.contains(v.as_str()))
                            .collect();
                    }
                    "schemeitemslistview" => s.scheme_items_list_view = value.eq_ignore_ascii_case("true"),
                    "language" => s.language = value,
                    _ => {}
                }
            }
        }
        Ok(s)
    }

    pub fn save(&self) -> CoreResult<()> {
        self.save_to(&settings_file())
    }

    pub fn save_to(&self, path: &Path) -> CoreResult<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut out = String::new();
        out.push_str("; SoundManager Configuration File\r\n");
        out.push_str("; CDDL-1.0 - based on ORelio/Sound-Manager\r\n\r\n");
        out.push_str("[Main]\r\n");
        out.push_str(&format!("PatchStartupSound={}\r\n", ini_escape(&self.patch_startup_sound.to_string())));
        out.push_str(&format!("UseDefaultOnMissingSound={}\r\n", ini_escape(&self.missing_sound_use_default.to_string())));
        out.push_str(&format!("ConvertProprietaryFiles={}\r\n", ini_escape(&self.convert_proprietary_files.to_string())));
        out.push_str(&format!("PreferStartupSoundOnLogon={}\r\n", ini_escape(&self.prefer_startup_sound_on_logon.to_string())));
        out.push_str(&format!("DisabledSoundEvents={}\r\n", ini_escape(&self.disabled_sound_events.join(","))));
        out.push_str(&format!("SchemeItemsListView={}\r\n", ini_escape(&self.scheme_items_list_view.to_string())));
        out.push_str(&format!("Language={}\r\n", ini_escape(&self.language)));
        std::fs::write(path, out)?;
        Ok(())
    }

    pub fn is_event_disabled(&self, internal_name: &str) -> bool {
        self.disabled_sound_events.iter().any(|n| n == internal_name)
    }

    pub fn set_event_disabled(&mut self, internal_name: &str, disabled: bool) -> CoreResult<()> {
        if disabled && !self.is_event_disabled(internal_name) {
            self.disabled_sound_events.push(internal_name.to_string());
        } else if !disabled {
            self.disabled_sound_events.retain(|n| n != internal_name);
        }
        self.save()
    }

    /// Load settings; missing/corrupt file falls back to defaults (load() already unwraps).
    pub fn load_or_default() -> Self {
        Self::load()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ini_roundtrip() {
        let s = Settings { patch_startup_sound: true, language: "fra".into(), ..Default::default() };
        let dir = std::env::temp_dir().join("smcore-test");
        let p = dir.join("settings.ini");
        s.save_to(&p).unwrap();
        let s2 = Settings::load_from(&p).unwrap();
        assert!(s2.patch_startup_sound);
        assert_eq!(s2.language, "fra");
        std::fs::remove_file(&p).ok();
    }

    #[test]
    fn parse_ini_basic() {
        let ini = "[Main]\nPatchStartupSound=true # comment\n; comment line\nFoo=bar\n";
        let secs = parse_ini(ini);
        assert_eq!(secs[0].0, "main");
        assert!(secs[0].1.iter().any(|(k, v)| k == "patchstartupsound" && v == "true"));
    }
}
