# Sound-Manager Modernization Plan

**Project:** `Israleche/Sound-Manager` (fork of `ORelio/Sound-Manager` v3.5.0)
**Goal:** Replace the legacy WinForms/.NET Framework 4.0 UI with a modern Tauri v2 + TypeScript stack. Refactor domain code for maintainability and future implementations.
**License:** CDDL-1.0 (preserved; mandatory credit to ORelio per upstream)
**Target platforms:** Windows 7 SP1 → Windows 11 (matches upstream minimum)

---

## 1. Why Tauri v2

| Concern | Old (.NET 4.0 / WinForms) | New (Tauri v2 + TS) |
|---|---|---|
| Installer | ~6–10 MB, requires .NET 4.0 preinstalled or shipped | ~5–12 MB MSI/NSIS, no runtime, uses Edge WebView2 (already on Win10/11) |
| Memory idle | ~80–150 MB (WinForms + .NET CLR) | ~40–80 MB (WebView + Rust core) |
| Dev loop | Visual Studio 2010 Express (archived) | `pnpm tauri dev` / `cargo check` |
| Modern UI | GDI+ buttons, ~Win7 look | Web stack: any modern CSS/UI lib (Tailwind, shadcn, etc.) |
| Native ops | C# P/Invoke + WMI + COM | Rust + `windows` crate + `tauri-plugin-*` |
| Code reuse | .NET Framework only | Frontend reusable in web variants (e.g. docs site) |

Tauri v2 is stable as of v2.0 (Oct 2024) with regular releases (v2.10.x in 2026). The Rust ↔ JS bridge is via `tauri::command` macros exposed as `invoke()`. Native Win32 access is via the `windows` crate (microsoft/windows-rs).

---

## 2. Architecture

```
soundmanager-tauri/
├── src-tauri/                       # Rust backend (binary + library)
│   ├── Cargo.toml
│   ├── tauri.conf.json              # Window/bundle/identifier config
│   ├── build.rs                     # tauri-build glue
│   ├── icons/                       # app + tray icons (ico, png multi-size)
│   ├── binaries/                    # sidecar (bg-sound-player.exe) for production
│   └── src/
│       ├── main.rs                  # entrypoint, sets up tauri::Builder
│       ├── lib.rs                   # tauri commands + plugin registration
│       ├── domain/                  # pure logic, no I/O
│       │   ├── mod.rs
│       │   ├── sound_event.rs       # static table of 32 events (Startup, Logon, …)
│       │   ├── sound_scheme.rs      # active scheme queries, apply()
│       │   ├── scheme_meta.rs       # name/author/about/thumbnail + SchemeInfo.ini
│       │   ├── settings.rs          # settings.ini Load/Save
│       │   └── translation.rs       # i18n loader (eng, fra) with fallback
│       ├── platform/                # Windows-specific I/O
│       │   ├── mod.rs
│       │   ├── registry.rs          # winreg wrappers (HKCU\..)
│       │   ├── version.rs           # WindowsVersion (registry + WMI for Win11)
│       │   ├── fs_admin.rs          # FileSystemAdmin (ownership/ACLs)
│       │   ├── imageres.rs          # BeginUpdateResource/UpdateResource/EndUpdateResource
│       │   ├── native_resource.rs   # Replace/Extract WAVE resources
│       │   ├── task_scheduler.rs    # COM TaskScheduler 2.0 wrapper
│       │   └── window_manager.rs    # GetActiveWindowExeName + Desktop focus
│       ├── archive/                 # .ths + .soundpack
│       │   ├── mod.rs
│       │   ├── ths.rs               # zip read/write + SchemeMeta serialization
│       │   ├── themepack.rs         # .theme (Windows theme) import
│       │   └── soundpack.rs         # AES/3DES decryption → ths
│       ├── audio/                   # playback helpers
│       │   ├── mod.rs
│       │   ├── convert.rs           # non-WAV → PCM WAV (Symphonia/FFmpeg-next)
│       │   └── play.rs              # rodio or winapi PlaySound (Sync)
│       ├── services/                # use-case orchestrators (bridge domain + platform)
│       │   ├── mod.rs
│       │   ├── scheme_service.rs    # Setup/Apply/CopyDefault/Update/Remove
│       │   ├── archive_service.rs   # Import/Export
│       │   ├── settings_service.rs  # Load/Save
│       │   ├── installer_service.rs # First-run setup, AssocFiles, register bg player
│       │   └── downloader_service.rs# GitHub releases + assets
│       ├── commands.rs              # #[tauri::command] surface for the frontend
│       └── errors.rs                # thiserror-based unified error type
├── src/                             # TypeScript frontend
│   ├── index.html
│   ├── main.ts                      # bootstrap, router, i18n init
│   ├── styles.css                   # base design tokens (dark/light, spacing)
│   ├── app/                         # framework-agnostic components (web-components or signals)
│   │   ├── router.ts
│   │   ├── store.ts                 # reactive state (signals from @preact/signals-core)
│   │   ├── ipc.ts                   # typed wrappers over invoke()
│   │   └── i18n.ts                  # loadLocale("eng"|"fra")
│   ├── views/
│   │   ├── scheme/                  # sound scheme editor (cards per event)
│   │   ├── settings/                # settings + imageres patch toggle
│   │   ├── downloader/              # GitHub scheme browser
│   │   └── about/                   # credits, license
│   ├── components/                  # design-system primitives
│   │   ├── Button.ts
│   │   ├── Card.ts
│   │   ├── Dialog.ts
│   │   ├── Slider.ts
│   │   └── IconButton.ts
│   └── locales/
│       ├── eng.json
│       └── fra.json
├── sidecars/
│   └── bg-sound-player/             # tiny Rust binary compiled separately
│       ├── Cargo.toml               # uses same modules as src-tauri via path dep
│       └── src/main.rs              # hidden window, ShutdownBlockReason, task
├── package.json
├── tsconfig.json
├── vite.config.ts
├── .gitignore
├── README.md                        # rewritten for new stack
├── CHANGELOG.md                     # v4.0.0 entry
└── docs/
    ├── ARCHITECTURE.md
    ├── MIGRATION.md                 # what changed vs v3.5.0
    └── API.md                       # Tauri command surface
```

**Workspace wiring:** `soundmanager-tauri/` holds `src-tauri/`, `sidecars/bg-sound-player/`, and the frontend root (`src/`, `package.json`). `src-tauri/Cargo.toml` declares both itself and the sidecar as members of an internal workspace so they share domain code.

---

## 3. Domain model (port from C#)

| C# class | Rust location | Notes |
|---|---|---|
| `SoundEvent` (static table) | `domain/sound_event.rs` | Same 32 events, same registry keys, same `EventType` enum |
| `SoundScheme` | `domain/sound_scheme.rs` + `platform/registry.rs` | Pure logic stays in `domain`, registry I/O in `platform` |
| `SchemeMeta` | `domain/scheme_meta.rs` | Replace `Image` (GDI+) with `image` crate for thumbnail resize to 100x100 PNG |
| `Settings` | `domain/settings.rs` | INI parser rewritten in Rust (no `SharpTools` external dep) |
| `Translations` | `domain/translation.rs` | JSON locales replacing `Lang/*.ini`; keys identical to upstream strings |
| `SoundArchive` | `archive/ths.rs` | `zip` crate replaces `Ionic.Zip`; identical on-disk format |
| `SoundArchiveThemepack` | `archive/themepack.rs` | Same logic |
| `SoundArchiveProprietary` | `archive/soundpack.rs` | `aes` + `des` crates reproduce AES-CBC + 3DES decryption (note: only the read/decrypt path; we do not write `.soundpack`) |
| `ImageresPatcher` | `platform/imageres.rs` + `platform/native_resource.rs` | Direct `windows` crate calls to `BeginUpdateResourceW`, `UpdateResourceW`, `EndUpdateResource` |
| `WindowsVersion` | `platform/version.rs` | `winreg` for `CurrentVersion`, `wmi` crate for `Win32_OperatingSystem.Caption` to detect Win11 |
| `FileSystemAdmin` | `platform/fs_admin.rs` | `windows` crate for `SetNamedSecurityInfoW` |
| `BgSoundPlayer` | `sidecars/bg-sound-player/src/main.rs` | Headless Tauri/webview-free Rust binary using `windows` crate for `ShutdownBlockReasonCreate/Destroy`, Task Scheduler 2.0 COM, `PlaySound` |
| `WindowManager` | `platform/window_manager.rs` | `GetForegroundWindow` + `GetWindowThreadProcessId` |
| `ShellFileType` | `platform/registry.rs` (extracted) | HKCR associations + `SHChangeNotify` |
| `DownloadSchemes` | `services/downloader_service.rs` | `reqwest` to call GitHub API; same JSON manifest format |
| `FormMain` (UI) | **deleted** | Replaced by frontend |
| `Privilege` (elevation) | `platform/elevation.rs` | New: `ShellExecuteW` with `"runas"` verb; only when needed (e.g. imageres patch) |

---

## 4. Tauri command surface (IPC)

Commands invoked from the frontend via `invoke<T>(name, args)`:

| Command | Args | Returns | Purpose |
|---|---|---|---|
| `get_app_info` | – | `{ version, lang, windows_friendly, nt_version, is_admin, running_in_portable_mode }` | Boot info |
| `get_sound_events` | – | `SoundEventDto[]` | 32 events with display name, desc, enabled, currentFile |
| `get_active_scheme` | – | `{ internal, display }` | Which scheme is currently applied |
| `get_scheme_list` | – | `SchemeDto[]` | All schemes in registry |
| `apply_scheme` | `{ internal: string, missing_use_default: bool }` | – | Apply scheme (current/.default/soundmanager) |
| `setup_scheme_manager` | `{ force_reset: bool, offer_import: bool }` | – | First-run / `--setup` |
| `get_scheme_meta` | – | `{ name, author, about, thumbnailB64 }` | Current scheme metadata |
| `set_scheme_meta` | `{ name, author, about, thumbnailBase64? }` | – | Update + persist |
| `update_sound_file` | `{ eventInternal, sourcePath }` | – | Replace a single sound (transcode if needed) |
| `remove_sound_file` | `{ eventInternal }` | – | Reset event to scheme default |
| `set_event_disabled` | `{ eventInternal, disabled }` | – | Toggle per-event disable |
| `play_sound_event` | `{ eventInternal }` | – | Test-play a single event |
| `import_archive` | – (uses dialog plugin) | `{ imported, scheme }` | Pick .ths or .soundpack, import |
| `export_archive` | `{ destinationPath? }` | – | Save current scheme to .ths |
| `get_settings` | – | `SettingsDto` | Read settings.ini |
| `save_settings` | `{ settings: SettingsDto }` | – | Persist |
| `patch_startup_sound` | `{ enabled }` | `{ success, error? }` | Toggle imageres patch (may elevate) |
| `is_patching_possible` | – | `{ possible, required, notRecommended }` | Capability flags |
| `set_bg_sound_player` | `{ enabled }` | – | Register/unregister scheduled task |
| `get_bg_sound_player` | – | `{ registered, requiredForWindowsVersion }` | Status |
| `list_downloaded_schemes` | – | `DownloadedScheme[]` | Local schemes folder |
| `list_github_schemes` | – | `GithubScheme[]` | Fetch from ORelio/Sound-Manager-Schemes |
| `download_scheme` | `{ name }` | – | Download to local folder |
| `pick_file` / `pick_save_file` | `{ filters, defaultPath? }` | `{ path }` | Dialog plugin |
| `show_in_folder` | `{ path }` | – | Reveal in Explorer (opener plugin) |
| `get_locale` | – | `{ key, entries }` | Load `eng.json` or `fra.json` |
| `open_external` | `{ url }` | – | Browser/external links |
| `quit` | – | – | Clean shutdown |

All commands return `Result<T, AppError>` where `AppError` is serialized with a `kind` discriminant (`Registry`, `Iores`, `Wav`, `Zip`, `Patching`, `Permission`, `Scheme`, `Other`) for typed error handling in the UI.

---

## 5. UI plan

**Stack:** Vite + TypeScript + lit-html or @preact/signals (no React to keep bundle tiny). Tailwind for utility CSS. Vanilla `fetch` to `invoke()`; no state-management library beyond signals.

**Layout (single window, sidebar nav):**
1. **Scheme view (default)** — hero card with scheme thumbnail + name/author/about, grid of 32 event cards. Each card has icon, name, current-file badge, play/browse/reset buttons, "disabled" switch.
2. **Settings** — toggles for imageres patch, bg sound player, missing-sound fallback, list-view (a11y). Language switch (eng/fra).
3. **Downloader** — list of available schemes from GitHub with thumbnail + size + install button. Local tab.
4. **About** — credits, license (CDDL-1.0), upstream link.

**Visual language:** dark-first palette, Inter or system font, 8-pt grid, soft elevation, focus rings for a11y, 200ms motion. Matches the modernised feel of the upstream `Images/logo-en.svg`.

---

## 6. Backward compatibility

- `.ths` and `.soundpack` file formats are preserved byte-for-byte (zip + SchemeInfo.ini scheme).
- `AppData\Roaming\SoundManager\Media\` location is preserved so existing schemes from v3.5.0 keep working.
- HKCU registry tree under `AppEvents\Schemes\Names\SoundManager` is preserved.
- `Lang/eng.ini`/`fra.ini` keys are mapped 1:1 into `eng.json`/`fra.json` (no string changes for the eng/fra speakers).
- `imageres.dll` patching logic is bit-for-bit equivalent.
- Settings.ini keys are preserved (`PatchStartupSound`, `UseDefaultOnMissingSound`, etc.).

CDDL-1.0: every file preserves the upstream copyright header and notes that the original is by ORelio. The fork adds "Modifications by Israleche (2026)".

---

## 7. Phased delivery (current iteration: Phase 1-3, smoke-ready)

| Phase | Scope | Status |
|---|---|---|
| 1 | Workspace scaffold, Cargo, tauri.conf, frontend skeleton, i18n stub, hello-world command | doing now |
| 2 | Domain modules: SoundEvent, Settings, SchemeMeta, Translations (pure) | todo |
| 3 | Platform: registry, version, fs_admin, imageres, native_resource, task_scheduler | todo |
| 4 | Services: scheme_service, archive_service (ths), settings_service | todo |
| 5 | Commands surface + frontend Scheme view (event list + import/export) | todo |
| 6 | Settings view, imageres toggle, elevation flow | todo |
| 7 | Bg-sound-player sidecar + Task Scheduler wiring | todo |
| 8 | Downloader (GitHub API) | todo |
| 9 | i18n, a11y pass, polish | todo |
| 10 | Build, smoke test, commit, push to `Israleche/Sound-Manager` | todo |

Each phase ends with `cargo check`, `cargo test` (where applicable), and a manual smoke check. Frontend and Rust changes are committed together so the fork history reads coherently.

---

## 8. Constraints & risks

- **Rust toolchain** must be installed (rustup, MSVC Build Tools on Windows). Disk ~3 GB.
- **WebView2 Runtime** is required on Windows 7/8; bundled installer adds ~5 MB.
- **Imageres patch** still requires admin elevation (UAC). Tauri cannot avoid that.
- **Task Scheduler 2.0** requires the `taskschd.dll` COM interface; bind via `windows` crate.
- **SharpTools** is gone — re-implement INI parsing with a small `ini` crate (or hand-rolled, it's 60 LOC).
- **First boot of the new app** on a system that had v3.5.0 will see the existing SoundManager scheme and reuse its media folder; no migration script needed because the on-disk format is unchanged.

---

## 9. Verification (per phase)

| Level | Check |
|---|---|
| V1 (syntax) | `cargo check --manifest-path src-tauri/Cargo.toml`; `tsc --noEmit` |
| V2 (lint) | `cargo clippy -- -D warnings`; `eslint .` |
| V3 (unit tests) | `cargo test` for domain & platform modules (registry mock, imageres fake, archive round-trip) |
| V4 (typecheck) | `tsc --strict` |
| V5 (build) | `pnpm tauri build` (release MSI + NSIS) |
| V6 (smoke) | `pnpm tauri dev`; click through scheme/settings/downloader views; import a sample .ths; toggle settings; verify HKCU registry tree is identical to what C# version wrote |

---

## 10. Out of scope (this iteration)

- Linux/macOS support (Tauri can target them, but upstream is Windows-only — keeping focus)
- Auto-update channel (can be added later via `tauri-plugin-updater`)
- Telemetry/diagnostics (none planned)
- Replacing the .NET CLI helpers (no CLI in the new design — UI-driven only)
