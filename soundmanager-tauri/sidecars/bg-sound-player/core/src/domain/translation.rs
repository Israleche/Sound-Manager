//! Translation loader. Replaces Translations.cs + Lang/*.ini with embedded
//! JSON locale files (eng, fra). Keys are 1:1 with the upstream Lang/*.ini.

use crate::errors::{CoreError, CoreResult};
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use serde::Serialize;
use std::collections::HashMap;

/// Upstream Lang/*.ini keys compiled into the binary.
/// Values are copied verbatim from Lang/eng.ini (see locales/eng.json for the full set;
/// critical keys duplicated here for the sidecar which has no filesystem access).
static BUILTIN: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    HashMap::from([
        ("app_name", "Sound Manager"),
        ("app_desc", "Manage and share Windows sound schemes !"),
        ("default_scheme_name", "Custom scheme"),
        ("default_scheme_author", "Unknown author"),
        ("default_scheme_about", "No description"),
        ("playing_shutdown_sound", "Playing shutdown sound"),
        ("playing_logoff_sound", "Playing logoff sound"),
        ("scheme_file_desc", "Sound Scheme File"),
        ("scheme_file_proprietary_desc", "Proprietary sound scheme file"),
        ("button_open", "Open"),
        ("button_import", "Import"),
        ("button_export", "Export"),
        ("button_reset", "Reset"),
        ("button_exit", "Exit"),
        ("button_ok", "OK"),
        ("button_cancel", "Cancel"),
        ("sound_file_too_long", "The sound file is too long. The maximum duration is 30 seconds."),
        ("startup_patch_not_admin", "Administrator privileges are required to patch the startup sound."),
        ("startup_patch_no_imageres_dll", "imageres.dll was not found in the system directory."),
        ("startup_patch_not_possible", "Startup sound patching is not possible on this Windows version."),
    ])
});

#[derive(Debug, Clone, Serialize)]
pub struct Locale {
    pub key: String,
    pub entries: HashMap<String, String>,
}

static CURRENT: Lazy<RwLock<String>> = Lazy::new(|| RwLock::new("eng".to_string()));

pub fn set_language(key: &str) {
    *CURRENT.write() = key.to_string();
}

pub fn current_language() -> String {
    CURRENT.read().clone()
}

/// Translate a key in the current language. Falls back to builtin, then the key itself.
pub fn get(key: &str) -> String {
    get_in(&current_language(), key)
}

pub fn get_in(lang: &str, key: &str) -> String {
    if let Ok(text) = std::fs::read_to_string(locale_path(lang)) {
        if let Ok(entries) = serde_json::from_str::<HashMap<String, String>>(&text) {
            if let Some(v) = entries.get(key) {
                return v.clone();
            }
        }
    }
    // Fallback chain: requested lang -> builtin english
    BUILTIN
        .get(key)
        .map(|s| s.to_string())
        .unwrap_or_else(|| key.to_string())
}

pub fn locale_path(lang: &str) -> std::path::PathBuf {
    // locales are bundled next to the exe in resources/lang/<lang>.json
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_default();
    // Resource layout: <exe>/lang/<lang>.json or <exe>/resources/lang/<lang>.json (msi)
    for base in [exe_dir.clone(), exe_dir.join("resources")] {
        let p = base.join("lang").join(format!("{lang}.json"));
        if p.exists() {
            return p;
        }
    }
    exe_dir.join("lang").join(format!("{lang}.json"))
}

/// Load a full locale map (eng|fra) for the frontend.
pub fn load_locale(lang: &str) -> CoreResult<Locale> {
    let path = locale_path(lang);
    if path.exists() {
        let text = std::fs::read_to_string(&path)?;
        let entries: HashMap<String, String> = serde_json::from_str(&text)
            .map_err(|e| CoreError::MissingTranslation(format!("{lang}: {e}")))?;
        return Ok(Locale { key: lang.to_string(), entries });
    }
    // No file: serve the builtin map
    let entries: HashMap<String, String> =
        BUILTIN.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    Ok(Locale { key: lang.to_string(), entries })
}
