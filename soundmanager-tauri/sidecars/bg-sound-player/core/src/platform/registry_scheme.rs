//! Registry access for sound schemes: HKCU\AppEvents\Schemes.
//! Port of SoundScheme.cs + the parts of ShellFileType.cs the core needs.

use crate::domain::settings::Settings;
use crate::domain::sound_event::{ALL_EVENTS, EventType, SoundEvent};
use crate::errors::{CoreError, CoreResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const REG_SCHEMES: &str = r"AppEvents\Schemes";
pub const REG_NAMES: &str = r"AppEvents\Schemes\Names\";
pub const REG_APPS: &str = r"AppEvents\Schemes\Apps\";
pub const SCHEME_MANAGER: &str = "SoundManager";
pub const SCHEME_DEFAULT: &str = ".Default";
pub const SCHEME_CURRENT: &str = ".Current";
const DISABLED_PLACEHOLDER: &str = "<DISABLED>";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemeInfo {
    pub internal_name: String,
    pub display_name: String,
}

fn open_hkcu() -> CoreResult<winreg::RegKey> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};
    use winreg::RegKey;
    let hklm = RegKey::predef(HKEY_CURRENT_USER);
    Ok(hklm.open_subkey_with_flags("", KEY_READ | KEY_WRITE)
        .map_err(|e| CoreError::Registry(e.to_string()))?)
}

fn reg_string(path: &str) -> CoreResult<Option<String>> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    match hkcu.open_subkey(path) {
        Ok(key) => Ok(key.get_value::<String, _>("").ok()),
        Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(CoreError::Registry(e.to_string())),
    }
}

fn set_reg_string(path: &str, value: &str) -> CoreResult<()> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_WRITE};
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu
        .create_subkey(path)
        .map_err(|e| CoreError::Registry(format!("{path}: {e}")))?;
    key.set_value("", &value.to_string())
        .map_err(|e| CoreError::Registry(format!("{path}: {e}")))
}

fn delete_reg_key(path: &str) -> CoreResult<bool> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    // Delete only the leaf: open parent, delete child
    let Some(idx) = path.rfind('\\') else {
        return Err(CoreError::Registry(format!("bad key path: {path}")));
    };
    let (parent, child) = (&path[..idx], &path[idx + 1..]);
    match hkcu.open_subkey_with_flags(parent, KEY_READ) {
        Ok(p) => match p.delete_subkey(child) {
            Ok(()) => Ok(true),
            Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(CoreError::Registry(format!("{path}: {e}"))),
        },
        Err(e) => Err(CoreError::Registry(format!("{parent}: {e}"))),
    }
}

fn subkeys_of(path: &str) -> CoreResult<Vec<String>> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    match hkcu.open_subkey(path) {
        Ok(key) => Ok(key.enum_keys().filter_map(|r| r.ok()).collect()),
        Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(CoreError::Registry(e.to_string())),
    }
}

/// The active scheme display+internal name, or None.
pub fn get_active_scheme() -> CoreResult<Option<SchemeInfo>> {
    let Some(internal) = reg_string(REG_SCHEMES)? else { return Ok(None) };
    Ok(get_scheme_list()?.into_iter().find(|s| s.internal_name == internal))
}

pub fn already_setup() -> CoreResult<bool> {
    Ok(reg_string(&format!("{REG_NAMES}{SCHEME_MANAGER}"))?.is_some())
}

/// List all schemes in HKCU\AppEvents\Schemes\Names.
pub fn get_scheme_list() -> CoreResult<Vec<SchemeInfo>> {
    let mut out = Vec::new();
    for internal in subkeys_of(REG_NAMES)? {
        if internal.is_empty() {
            continue;
        }
        let display = reg_string(&format!("{REG_NAMES}{internal}"))?
            .filter(|d| !d.trim().is_empty() && !d.starts_with('@'))
            .unwrap_or_else(|| internal.clone());
        out.push(SchemeInfo { internal_name: internal, display_name: display });
    }
    Ok(out)
}

/// Create/refresh the SoundManager scheme registry entries.
pub fn setup(settings: &Settings, media_dir: &Path, form_main_open: bool) -> CoreResult<()> {
    set_reg_string(&format!("{REG_NAMES}{SCHEME_MANAGER}"), "Sound Manager")?;
    for ev in ALL_EVENTS {
        // Temporarily disable Select while the main form is open (upstream behavior).
        let temp_disable = ev.event_type == Some(EventType::Select) && form_main_open;
        let path_str = ev.file_path(media_dir).to_string_lossy().into_owned();
        let value: &str = if settings.is_event_disabled(ev.internal_name) || temp_disable {
            DISABLED_PLACEHOLDER
        } else {
            &path_str
        };
        for reg_key in ev.registry_keys {
            set_reg_string(&format!("{REG_APPS}{reg_key}\\{SCHEME_MANAGER}"), value)?;
        }
    }
    Ok(())
}

/// Read the file path for an event from the currently applied scheme (.Current).
pub fn get_current_file(ev: &SoundEvent) -> Option<PathBuf> {
    for reg_key in ev.registry_keys {
        if let Ok(Some(p)) = reg_string(&format!("{REG_APPS}{reg_key}\\{SCHEME_CURRENT}")) {
            let expanded = expand_env(&p);
            let path = PathBuf::from(expanded);
            if path.is_file() {
                return Some(path);
            }
        }
    }
    None
}

fn expand_env(s: &str) -> String {
    // %SystemRoot%, %SystemDrive%, ... limited expansion via std::env
    let mut out = s.to_string();
    for (k, v) in std::env::vars() {
        let pat = format!("%{k}%");
        if out.contains(&pat) {
            out = out.replace(&pat, &v);
        }
    }
    out
}

/// Copy the default (or given source scheme) sound for an event into the media dir.
pub fn copy_default(ev: &SoundEvent, media_dir: &Path, source_scheme: Option<&str>) -> CoreResult<bool> {
    let source = source_scheme.unwrap_or(SCHEME_DEFAULT);
    let mut found = false;
    for reg_key in ev.registry_keys {
        let path = format!("{REG_APPS}{reg_key}\\{source}");
        if let Ok(Some(p)) = reg_string(&path) {
            let expanded = expand_env(&p);
            if Path::new(&expanded).is_file() {
                let dest = ev.file_path(media_dir);
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::copy(&expanded, &dest)?;
                found = true;
            }
        }
    }
    if !found {
        remove(ev, media_dir)?;
    }
    Ok(found)
}

/// Remove the sound file for an event from the SoundManager scheme.
pub fn remove(ev: &SoundEvent, media_dir: &Path) -> CoreResult<()> {
    let dest = ev.file_path(media_dir);
    if dest.exists() {
        std::fs::remove_file(&dest)?;
    }
    // Registry entry of SoundManager scheme for this event also cleared
    for reg_key in ev.registry_keys {
        set_reg_string(&format!("{REG_APPS}{reg_key}\\{SCHEME_MANAGER}"), DISABLED_PLACEHOLDER).ok();
    }
    Ok(())
}

/// Apply a scheme: for every registry event under Apps, write .Current from the scheme.
pub fn apply(scheme_internal: &str, missing_use_default: bool) -> CoreResult<()> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};
    use winreg::RegKey;
    // Validate scheme exists
    if reg_string(&format!("{REG_NAMES}{scheme_internal}"))?.is_none() {
        return Err(CoreError::Scheme(format!("scheme '{scheme_internal}' does not exist")));
    }
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let apps = hkcu
        .open_subkey_with_flags(REG_APPS, KEY_READ)
        .map_err(|e| CoreError::Registry(e.to_string()))?;
    for app_name in apps.enum_keys().filter_map(|r| r.ok()) {
        let app = match apps.open_subkey_with_flags(&app_name, KEY_READ) {
            Ok(a) => a,
            Err(_) => continue,
        };
        for sound_name in app.enum_keys().filter_map(|r| r.ok()) {
            let sound_path = format!("{REG_APPS}{app_name}\\{sound_name}");
            let sound = match hkcu.open_subkey_with_flags(&sound_path, KEY_READ | KEY_WRITE) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let scheme_sound: Option<String> = sound
                .open_subkey(scheme_internal)
                .ok()
                .and_then(|k| k.get_value("").ok());
            let mut sound_path_value = scheme_sound.clone();
            if missing_use_default {
                let missing = sound_path_value
                    .as_deref()
                    .map(|p| !Path::new(&expand_env(p)).is_file())
                    .unwrap_or(true);
                let is_disabled = sound_path_value.as_deref() == Some(DISABLED_PLACEHOLDER);
                if missing && !is_disabled {
                    if let Ok(def) = sound.open_subkey(SCHEME_DEFAULT) {
                        sound_path_value = def.get_value("").ok();
                    }
                }
            }
            if let Some(p) = &sound_path_value {
                if !Path::new(&expand_env(p)).is_file() {
                    sound_path_value = None;
                }
            }
            let (current, _) = sound
                .create_subkey(SCHEME_CURRENT)
                .map_err(|e| CoreError::Registry(format!("{sound_path}\\{SCHEME_CURRENT}: {e}")))?;
            current.set_value("", &sound_path_value.unwrap_or_default())?;
        }
    }
    set_reg_string(REG_SCHEMES, scheme_internal)?;
    Ok(())
}

/// Remove the SoundManager scheme from the registry entirely.
pub fn uninstall() -> CoreResult<()> {
    if let Ok(Some(active)) = get_active_scheme() {
        if active.internal_name == SCHEME_MANAGER {
            apply(SCHEME_DEFAULT, true)?;
        }
    }
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let apps = hkcu.open_subkey_with_flags(REG_APPS, KEY_READ)
        .map_err(|e| CoreError::Registry(e.to_string()))?;
    for app_name in apps.enum_keys().filter_map(|r| r.ok()) {
        let app_path = format!("{REG_APPS}{app_name}");
        if let Ok(app) = hkcu.open_subkey_with_flags(&app_path, KEY_READ) {
            for sound_name in app.enum_keys().filter_map(|r| r.ok()) {
                let _ = delete_reg_key(&format!("{app_path}\\{sound_name}\\{SCHEME_MANAGER}"));
            }
        }
    }
    delete_reg_key(&format!("{REG_NAMES}{SCHEME_MANAGER}"))?;
    Ok(())
}
