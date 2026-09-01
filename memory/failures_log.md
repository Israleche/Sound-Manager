# Failures Log — Sound Manager fork

## Tauri v2
- `plugins.fs.scope` en tauri.conf.json → panic PluginInitialization al boot (scope va en capabilities). Fix: `"plugins": {}`.
- Versión con sufijo `-dev` → WiX/MSI falla ("pre-release must be numeric-only"). Fix: versión numérica `4.0.0`.
- `windows` crate 0.58: BOOL está en `Win32::Foundation` (NO `core`); resource APIs (BeginUpdateResource/FindResourceEx/LoadResource/SizeofResource) en `Win32::System::LibraryLoader` (requiere feature `Win32_System_LibraryLoader`); SetConsoleCtrlHandler handler recibe `u32`.
- `cbc 0.4` no existe → `cbc 0.1` + `cipher 0.4`; `des::TdesEde3` (no `DesEde3`); `decrypt_padded_vec_mut` devuelve Result.
- hound 3.5: `WavWriter::new(writer, WavSpec)` (2 args); SampleWriter16 no tiene `finish()` → header WAV manual (44 bytes).
- `windows::Win32::UI::Shell::IsUserAnAdmin()` devuelve BOOL directo (no Result).

## Entorno
- rustup default host = aarch64 (máquina ARM64); sin MSVC, `link.exe` de coreutils intercepta → instalar BuildTools via `winget install Microsoft.VisualStudio.2022.BuildTools` con override `--installPath C:\BuildTools --add ...VC.Tools.ARM64`.
- `Start-Process` de bootstrapper descargado con Invoke-WebRequest falla ("dañado") — usar winget.
- Variables de entorno NO persisten entre calls de bash (cada call es proceso nuevo).
- PowerShell: evitar `$var-subexpr` dentro de New-Object args; precomputar.
- gh CLI/GITHUB_TOKEN fine-grained: fork/create repo → 403; hace falta PAT del usuario.

## Bugs de código encontrados por tests
- `SchemeMeta::load()` lee dir global → `ths::import()` debe parsear meta del media_dir param (bug pillado por test roundtrip).
