//! Tauri app bootstrap: registers plugins and the IPC command surface.

mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_app_info,
            commands::get_sound_events,
            commands::get_active_scheme,
            commands::get_scheme_list,
            commands::apply_scheme,
            commands::setup_scheme_manager,
            commands::get_scheme_meta,
            commands::set_scheme_meta,
            commands::update_sound_file,
            commands::remove_sound_file,
            commands::set_event_disabled,
            commands::play_sound_event,
            commands::import_archive,
            commands::export_archive,
            commands::convert_soundpack,
            commands::get_settings,
            commands::save_settings,
            commands::get_patching_flags,
            commands::patch_startup_sound,
            commands::restore_startup_sound,
            commands::get_locale,
            commands::quit_app,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
