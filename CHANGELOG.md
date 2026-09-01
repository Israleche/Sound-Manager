# Changelog

All notable changes to this fork of ORelio/Sound-Manager.

## [4.0.0-dev] - 2026-08-31

### Added
- New Tauri v2 + TypeScript application replacing the WinForms UI (`soundmanager-tauri/`).
- `sound-manager-core` Rust crate: domain (SoundEvent table, Settings INI, SchemeMeta, translations), platform (HKCU registry scheme ops, WindowsVersion, imageres patcher, fs admin), archive (.ths zip I/O, .soundpack AES/3DES decrypt), audio (Symphonia transcode + PlaySound).
- Typed IPC surface (~30 commands) with kind-discriminated error mapping.
- Frontend: sidebar navigation, scheme editor (event grid, play/browse/reset/enable toggles), metadata editor with 100×100 thumbnail, settings view (incl. imageres patch toggle), about view; EN/FR locales; a11y focus rings + list-view mode.
- `bg-sound-player` sidecar (headless Rust binary, console-ctrl-handler based port of BgSoundPlayer; windowed ShutdownBlockReason port pending).
- Docs: PLAN.md (architecture + phased delivery), updated README.

### Changed
- Default branch `master` now hosts both legacy C# source (`soundmanager-src/`, v3.5.0 tag preserved) and the new Tauri app.

### Preserved (compatibility)
- `.ths`/`.soundpack` file formats, `%APPDATA%\SoundManager\Media` layout, HKCU scheme naming, Settings.ini keys, EN/FR strings.

### Pending (roadmap per docs/PLAN.md)
- Windowed sidecar (ShutdownBlockReasonCreate/Destroy + Task Scheduler 2.0)
- GitHub scheme downloader view (DownloadSchemes port)
- MSI/NSIS packaging pipeline + icons
- CI (cargo check/clippy + tsc on push)
