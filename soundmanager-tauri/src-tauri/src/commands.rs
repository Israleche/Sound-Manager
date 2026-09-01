//! IPC command surface. Thin layer: validate → call core services → map errors.

use sound_manager_core as core;
use sound_manager_core::domain::{
    scheme_meta::SchemeMeta,
    settings::Settings,
    sound_event::{event_by_name, ALL_EVENTS},
    translation,
};
use serde::Serialize;

fn err_to_string(e: core::CoreError) -> String {
    format!("{}: {}", e.kind(), e)
}

// ---------------------------------------------------------------- DTOs

#[derive(Serialize)]
pub struct AppInfo {
    pub version: &'static str,
    pub language: String,
    pub windows_friendly: String,
    pub is_admin: bool,
    pub media_dir: String,
}

#[derive(Serialize)]
pub struct SoundEventDto {
    pub internal_name: String,
    pub display_name: String,
    pub description: String,
    pub disabled: bool,
    pub has_file: bool,
    pub file_name: Option<String>,
}

// ---------------------------------------------------------------- Info

#[tauri::command]
pub fn get_app_info(settings: tauri::State<'_, std::sync::Mutex<Settings>>) -> Result<AppInfo, String> {
    Ok(AppInfo {
        version: env!("CARGO_PKG_VERSION"),
        language: settings.lock().unwrap().language.clone(),
        windows_friendly: core::platform::version::info().friendly_name,
        is_admin: core::platform::fs_admin::is_admin(),
        media_dir: core::domain::scheme_meta::media_dir().to_string_lossy().into_owned(),
    })
}

#[tauri::command]
pub fn get_sound_events(settings: tauri::State<'_, std::sync::Mutex<Settings>>) -> Result<Vec<SoundEventDto>, String> {
    let s = settings.lock().unwrap();
    let media = core::domain::scheme_meta::media_dir();
    Ok(ALL_EVENTS
        .iter()
        .map(|ev| {
            let path = ev.file_path(&media);
            SoundEventDto {
                internal_name: ev.internal_name.to_string(),
                display_name: translation::get_in(&s.language, &format!("event_{}_name", ev.internal_name.to_lowercase())),
                description: translation::get_in(&s.language, &format!("event_{}_desc", ev.internal_name.to_lowercase())),
                disabled: s.is_event_disabled(ev.internal_name),
                has_file: path.is_file(),
                file_name: if path.is_file() { Some(ev.file_name()) } else { None },
            }
        })
        .collect())
}

// ---------------------------------------------------------------- Schemes

#[derive(Serialize)]
pub struct SchemeDto {
    pub internal_name: String,
    pub display_name: String,
}

#[tauri::command]
pub fn get_active_scheme() -> Result<Option<SchemeDto>, String> {
    Ok(core::platform::registry_scheme::get_active_scheme()
        .map_err(|e| err_to_string(e.into()))?
        .map(|s| SchemeDto { internal_name: s.internal_name, display_name: s.display_name }))
}

#[tauri::command]
pub fn get_scheme_list() -> Result<Vec<SchemeDto>, String> {
    Ok(core::platform::registry_scheme::get_scheme_list()
        .map_err(|e| err_to_string(e.into()))?
        .into_iter()
        .map(|s| SchemeDto { internal_name: s.internal_name, display_name: s.display_name })
        .collect())
}

#[tauri::command]
pub fn apply_scheme(internal_name: String, missing_use_default: bool) -> Result<(), String> {
    core::platform::registry_scheme::apply(&internal_name, missing_use_default).map_err(|e| err_to_string(e.into()))
}

/// First-run or manual setup: create SoundManager scheme, copy current/default
/// sounds into the media dir, and apply the SoundManager scheme.
#[tauri::command]
pub fn setup_scheme_manager(
    force_reset: bool,
    settings: tauri::State<'_, std::sync::Mutex<Settings>>,
) -> Result<(), String> {
    let media = core::domain::scheme_meta::media_dir();
    let s = settings.lock().unwrap().clone();
    let active = core::platform::registry_scheme::get_active_scheme().map_err(|e| err_to_string(e.into()))?;

    if force_reset {
        SchemeMeta::reset_all();
        if let Some(active) = &active {
            if active.internal_name != ".Default" {
                let mut meta = SchemeMeta::load();
                meta.name = active.display_name.clone();
                let _ = meta.save();
            }
        }
    }

    core::platform::registry_scheme::setup(&s, &media, false).map_err(|e| err_to_string(e.into()))?;

    if force_reset || !media.is_dir() {
        std::fs::create_dir_all(&media).map_err(|e| err_to_string(e.into()))?;
        let source = active.as_ref().map(|a| a.internal_name.as_str());
        for ev in ALL_EVENTS {
            core::platform::registry_scheme::copy_default(ev, &media, source).map_err(|e| err_to_string(e.into()))?;
        }
    }

    core::platform::registry_scheme::apply(
        core::platform::registry_scheme::SCHEME_MANAGER,
        s.missing_sound_use_default,
    )
    .map_err(|e| err_to_string(e.into()))
}

// ---------------------------------------------------------------- Meta

#[tauri::command]
pub fn get_scheme_meta() -> Result<SchemeMeta, String> {
    Ok(SchemeMeta::load())
}

#[tauri::command]
pub fn set_scheme_meta(meta: SchemeMeta) -> Result<(), String> {
    meta.save().map_err(|e| err_to_string(e.into()))
}

// ---------------------------------------------------------------- Sounds

#[tauri::command]
pub fn update_sound_file(event_internal: String, source_path: String) -> Result<(), String> {
    let ev = event_by_name(&event_internal).ok_or("unknown event")?;
    let media = core::domain::scheme_meta::media_dir();
    let source = std::path::PathBuf::from(&source_path);
    if !source.is_file() {
        return Err(format!("file not found: {source_path}"));
    }
    let dest = ev.file_path(&media);
    std::fs::create_dir_all(&media).map_err(|e| err_to_string(e.into()))?;

    if core::audio::convert::is_wav(&source) {
        std::fs::copy(&source, &dest).map_err(|e| err_to_string(e.into()))?;
    } else {
        let wav = core::audio::convert::to_wav(&source).map_err(|e| err_to_string(e.into()))?;
        std::fs::write(&dest, wav).map_err(|e| err_to_string(e.into()))?;
    }

    // Refresh registry so the scheme points at the (possibly new) file.
    let s = Settings::load_or_default();
    core::platform::registry_scheme::setup(&s, &media, false).map_err(|e| err_to_string(e.into()))?;

    // Vista+ : also patch imageres when updating the startup sound.
    if ev.event_type == Some(core::domain::sound_event::EventType::Startup)
        && s.patch_startup_sound
        && core::platform::imageres::is_patching_possible()
        && core::platform::fs_admin::is_admin()
    {
        core::platform::imageres::patch(Some(&dest)).map_err(|e| err_to_string(e.into()))?;
    }
    Ok(())
}

#[tauri::command]
pub fn remove_sound_file(event_internal: String) -> Result<(), String> {
    let ev = event_by_name(&event_internal).ok_or("unknown event")?;
    core::platform::registry_scheme::remove(ev, &core::domain::scheme_meta::media_dir())
        .map_err(|e| err_to_string(e.into()))
}

#[tauri::command]
pub fn set_event_disabled(
    event_internal: String,
    disabled: bool,
    settings: tauri::State<'_, std::sync::Mutex<Settings>>,
) -> Result<(), String> {
    {
        let mut s = settings.lock().unwrap();
        s.set_event_disabled(&event_internal, disabled).map_err(|e| err_to_string(e.into()))?;
    }
    let s = settings.lock().unwrap().clone();
    let media = core::domain::scheme_meta::media_dir();
    core::platform::registry_scheme::setup(&s, &media, false).map_err(|e| err_to_string(e.into()))?;
    core::platform::registry_scheme::apply(
        core::platform::registry_scheme::SCHEME_MANAGER,
        s.missing_sound_use_default,
    )
    .map_err(|e| err_to_string(e.into()))
}

#[tauri::command]
pub fn play_sound_event(event_internal: String) -> Result<(), String> {
    let ev = event_by_name(&event_internal).ok_or("unknown event")?;
    let media = core::domain::scheme_meta::media_dir();
    let path = ev.file_path(&media);
    if !path.is_file() {
        return Err(format!("no sound file for {event_internal}"));
    }
    core::audio::play::play_any(&path, false).map_err(|e| err_to_string(e.into()))
}

// ---------------------------------------------------------------- Archives

#[tauri::command]
pub fn import_archive(zip_path: String, settings: tauri::State<'_, std::sync::Mutex<Settings>>) -> Result<SchemeMeta, String> {
    let mut path = std::path::PathBuf::from(&zip_path);
    let s = settings.lock().unwrap().clone();

    if core::archive::soundpack::is_proprietary(&path) {
        let media = core::domain::scheme_meta::media_dir();
        let out = if s.convert_proprietary_files {
            path.with_extension(core::archive::ths::FILE_EXTENSION)
        } else {
            std::env::temp_dir().join("soundmanager-converted.ths")
        };
        core::archive::soundpack::convert(&path, &out).map_err(|e| err_to_string(e.into()))?;
        path = out;
    }

    let media = core::domain::scheme_meta::media_dir();
    let meta = core::archive::ths::import(&path, &media).map_err(|e| err_to_string(e.into()))?;
    let s = settings.lock().unwrap().clone();
    core::platform::registry_scheme::setup(&s, &media, false).map_err(|e| err_to_string(e.into()))?;
    core::platform::registry_scheme::apply(
        core::platform::registry_scheme::SCHEME_MANAGER,
        s.missing_sound_use_default,
    )
    .map_err(|e| err_to_string(e.into()))?;
    Ok(meta)
}

#[tauri::command]
pub fn export_archive(destination: String) -> Result<(), String> {
    core::archive::ths::export(
        std::path::Path::new(&destination),
        &core::domain::scheme_meta::media_dir(),
    )
    .map_err(|e| err_to_string(e.into()))
}

#[tauri::command]
pub fn convert_soundpack(input: String, output: String) -> Result<Option<String>, String> {
    core::archive::soundpack::convert(std::path::Path::new(&input), std::path::Path::new(&output))
        .map_err(|e| err_to_string(e.into()))
}

// ---------------------------------------------------------------- Settings

#[tauri::command]
pub fn get_settings(settings: tauri::State<'_, std::sync::Mutex<Settings>>) -> Result<Settings, String> {
    Ok(settings.lock().unwrap().clone())
}

#[tauri::command]
pub fn save_settings(new_settings: Settings, settings: tauri::State<'_, std::sync::Mutex<Settings>>) -> Result<(), String> {
    new_settings.save().map_err(|e| err_to_string(e.into()))?;
    *settings.lock().unwrap() = new_settings;
    Ok(())
}

// ---------------------------------------------------------------- Startup patch

#[derive(Serialize)]
pub struct PatchingFlags {
    pub possible: bool,
    pub required: bool,
    pub not_recommended: bool,
    pub is_admin: bool,
}

#[tauri::command]
pub fn get_patching_flags() -> Result<PatchingFlags, String> {
    Ok(PatchingFlags {
        possible: core::platform::imageres::is_patching_possible(),
        required: core::platform::imageres::is_patching_required(),
        not_recommended: core::platform::imageres::is_patching_not_recommended(),
        is_admin: core::platform::fs_admin::is_admin(),
    })
}

#[tauri::command]
pub fn patch_startup_sound(enabled: bool) -> Result<(), String> {
    if enabled {
        let media = core::domain::scheme_meta::media_dir();
        let startup = sound_manager_core::domain::sound_event::event_by_type(
            sound_manager_core::domain::sound_event::EventType::Startup,
        );
        let path = startup.file_path(&media);
        core::platform::imageres::patch(if path.is_file() { Some(&path) } else { None })
            .map_err(|e| err_to_string(e.into()))
    } else {
        core::platform::imageres::restore().map_err(|e| err_to_string(e.into()))
    }
}

#[tauri::command]
pub fn restore_startup_sound() -> Result<(), String> {
    core::platform::imageres::restore().map_err(|e| err_to_string(e.into()))
}

// ---------------------------------------------------------------- i18n / misc

#[derive(Serialize)]
pub struct LocaleDto {
    pub key: String,
    pub entries: std::collections::HashMap<String, String>,
}

#[tauri::command]
pub fn get_locale(key: String) -> Result<LocaleDto, String> {
    sound_manager_core::domain::translation::load_locale(&key)
        .map(|l| LocaleDto { key: l.key, entries: l.entries })
        .map_err(|e| err_to_string(e.into()))
}

#[tauri::command]
pub fn quit_app(app: tauri::AppHandle) -> Result<(), String> {
    app.exit(0);
    Ok(())
}
