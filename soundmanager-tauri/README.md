# Sound Manager (Tauri edition)

Modern rewrite of [ORelio/Sound-Manager](https://github.com/ORelio/Sound-Manager) (v3.5.0) with **Tauri v2 + TypeScript**, replacing the legacy .NET Framework 4.0 / WinForms UI. Licensed under CDDL-1.0 (inherited; upstream credit preserved).

**Status:** v4.0.0-dev — scaffold + core domain, platform, archive, audio, IPC, and frontend views implemented. See `docs/PLAN.md`.

## What works

- Full sound-scheme management over `HKCU\AppEvents\Schemes` (setup, apply, list, reset)
- 32 sound events (same table/registry keys as upstream v3.5.0)
- `.ths` import/export (zip format, byte-compatible, legacy XP filenames supported)
- `.soundpack` import (AES/3DES decrypt-only, fair-use interop like upstream)
- Metadata (name/author/about + 100×100 PNG thumbnail) in `Scheme.ini`/`Scheme.png`
- Settings INI (same keys as upstream) + EN/FR locales
- Audio: WAV sniff, non-WAV→PCM WAV transcode (Symphonia), 30 s guard, PlaySound preview
- Startup-sound patching of `imageres.dll` (UpdateResource path, admin-gated)

## Dev quickstart

Prereqs: Node ≥ 20, Rust (MSVC toolchain + VS Build Tools C++ workload), WebView2.

```powershell
cd soundmanager-tauri
npm install
npm run tauri:dev      # dev window
npm run tauri:build    # msi + nsis
```

## Layout

```
soundmanager-tauri/
├── src/                  # TypeScript frontend (Vite)
├── src-tauri/            # Tauri app: commands, lib, main
└── sidecars/bg-sound-player/
    ├── core/             # shared crate: domain, platform, archive, audio
    └── src/              # bg-sound-player sidecar binary
```

See `docs/PLAN.md` for the full architecture and migration notes.
