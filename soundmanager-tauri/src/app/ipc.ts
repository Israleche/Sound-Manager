/**
 * Typed IPC wrappers over Tauri invoke().
 * Mirrors src-tauri/src/commands.rs.
 */
import { invoke } from "@tauri-apps/api/core";

export interface AppInfo {
  version: string;
  language: string;
  windows_friendly: string;
  is_admin: boolean;
  media_dir: string;
}

export interface SoundEventDto {
  internal_name: string;
  display_name: string;
  description: string;
  disabled: boolean;
  has_file: boolean;
  file_name: string | null;
}

export interface SchemeDto {
  internal_name: string;
  display_name: string;
}

export interface SchemeMeta {
  name: string;
  author: string;
  about: string;
  thumbnail_base64: string;
}

export interface Settings {
  patch_startup_sound: boolean;
  missing_sound_use_default: boolean;
  convert_proprietary_files: boolean;
  prefer_startup_sound_on_logon: boolean;
  disabled_sound_events: string[];
  scheme_items_list_view: boolean;
  language: string;
}

export interface PatchingFlags {
  possible: boolean;
  required: boolean;
  not_recommended: boolean;
  is_admin: boolean;
}

export const ipc = {
  getAppInfo: () => invoke<AppInfo>("get_app_info"),
  getSoundEvents: () => invoke<SoundEventDto[]>("get_sound_events"),
  getActiveScheme: () => invoke<SchemeDto | null>("get_active_scheme"),
  getSchemeList: () => invoke<SchemeDto[]>("get_scheme_list"),
  applyScheme: (internalName: string, missingUseDefault: boolean) =>
    invoke<void>("apply_scheme", { internalName, missingUseDefault }),
  setupSchemeManager: (forceReset: boolean) =>
    invoke<void>("setup_scheme_manager", { forceReset }),
  getSchemeMeta: () => invoke<SchemeMeta>("get_scheme_meta"),
  setSchemeMeta: (meta: SchemeMeta) => invoke<void>("set_scheme_meta", { meta }),
  updateSoundFile: (eventInternal: string, sourcePath: string) =>
    invoke<void>("update_sound_file", { eventInternal, sourcePath }),
  removeSoundFile: (eventInternal: string) =>
    invoke<void>("remove_sound_file", { eventInternal }),
  setEventDisabled: (eventInternal: string, disabled: boolean) =>
    invoke<void>("set_event_disabled", { eventInternal, disabled }),
  playSoundEvent: (eventInternal: string) =>
    invoke<void>("play_sound_event", { eventInternal }),
  importArchive: (zipPath: string) =>
    invoke<SchemeMeta>("import_archive", { zipPath }),
  exportArchive: (destination: string) =>
    invoke<void>("export_archive", { destination }),
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<void>("save_settings", { newSettings: settings }),
  getPatchingFlags: () => invoke<PatchingFlags>("get_patching_flags"),
  patchStartupSound: (enabled: boolean) => invoke<void>("patch_startup_sound", { enabled }),
  restoreStartupSound: () => invoke<void>("restore_startup_sound"),
  getLocale: (key: string) =>
    invoke<{ key: string; entries: Record<string, string> }>("get_locale", { key }),
};
