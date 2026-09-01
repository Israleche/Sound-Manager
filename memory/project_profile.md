# Sound Manager (Istar fork) — Project Profile

- Repo: `S:\GitHub\Istar-SoundManager` = fork `Israleche/Sound-Manager` ← `ORelio/Sound-Manager` v3.5.0 (CDDL-1.0)
- `legacy/` = C# .NET 4.0 WinForms original (no tocar; referencia)
- `soundmanager-tauri/` = app nueva: Tauri v2 + TS (Vite) + Rust core
- Core crate: `soundmanager-tauri/sidecars/bg-sound-player/core` (domain/platform/archive/audio), compartida por app + sidecar bin `bg-sound-player`
- Data: `%APPDATA%\SoundManager\{Media,SoundManager.ini}` — formatos idénticos a v3.5.0 (.ths zip, Scheme.ini/png, claves Settings.ini)
- Registro: `HKCU\AppEvents\Schemes` (Names/Apps/.Current), scheme interno "SoundManager"
- Máquina: **Windows 11 ARM64** (aarch64-pc-windows-msvc); MSVC BuildTools 2022 en `C:\BuildTools`; Rust 1.98 en `%USERPROFILE%\.cargo`
- Build: `npm run tauri:build` → MSI+NSIS en `src-tauri/target/release/bundle/`; tests: `cargo test` en core (9/9)
- Push con PAT del usuario embebido en remote origin
