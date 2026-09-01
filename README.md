# Istar Sound Manager

Fork/refactor of [ORelio/Sound-Manager](https://github.com/ORelio/Sound-Manager) v3.5.0 (CDDL-1.0).

| Folder | What |
|---|---|
| `soundmanager-tauri/` | **New app**: Tauri v2 + TypeScript UI, Rust core (see its README + `docs/PLAN.md`) |
| `legacy/` | Original C# / .NET 4.0 WinForms source (v3.5.0, tag preserved) |
| `docs/` | PLAN.md (architecture & roadmap), ARCHITECTURE notes |

## Quickstart (new app)

```powershell
cd soundmanager-tauri
npm install
npm run tauri:dev
```

Requires: Node ≥ 20, Rust (MSVC toolchain + VS Build Tools C++), WebView2.

## Compatibility guarantees

- `.ths` / `.soundpack` formats, `%APPDATA%\SoundManager\Media` layout, HKCU scheme naming, Settings.ini keys and EN/FR strings are preserved from v3.5.0.

## License

CDDL-1.0 (inherited). Upstream credit: ORelio. Modifications: Israleche (2026).
