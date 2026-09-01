//! bg-sound-player sidecar: plays startup/logon sound at session start and
//! shutdown/logoff sound at session end (Windows 8+ removed built-in playback).
//! Port of BgSoundPlayer.cs as a headless Rust binary.

use sound_manager_core as core;
use sound_manager_core::domain::settings::Settings;
use sound_manager_core::domain::sound_event::{event_by_type, EventType};

fn main() {
    env_logger::init();
    let settings = Settings::load_or_default();

    let startup = event_by_type(EventType::Startup);
    let shutdown = event_by_type(EventType::Shutdown);
    let logon = event_by_type(EventType::Logon);
    let logoff = event_by_type(EventType::Logoff);

    let media = core::domain::scheme_meta::media_dir();
    let exists = |ev: &core::domain::sound_event::SoundEvent| ev.file_path(&media).is_file();

    // Startup vs Logon: upstream logic — on fresh boot prefer startup sound
    // (unless the system itself plays it because the startup sound is patched).
    let mut to_play: Option<&core::domain::sound_event::SoundEvent> = None;
    if settings.prefer_startup_sound_on_logon && exists(startup) {
        to_play = Some(startup);
    } else if exists(logon) {
        to_play = Some(logon);
    }

    if let Some(ev) = to_play {
        log::info!("playing {}", ev.internal_name);
        let path = ev.file_path(&media);
        let _ = core::audio::play::play_wav(&path, true);
    }

    // Session-end hooks: watch for logoff/shutdown via console ctrl handler.
    // Full port (ShutdownBlockReason + hidden window) arrives with the
    // windowed sidecar; for now keep a minimal blocking wait.
    wait_for_session_end();
    let session_end_shutdown = read_shutdown_flag();
    let ev = if session_end_shutdown || settings.prefer_startup_sound_on_logon {
        shutdown
    } else {
        logoff
    };
    if exists(ev) {
        log::info!("playing {}", ev.internal_name);
        let path = ev.file_path(&media);
        let _ = core::audio::play::play_wav(&path, true);
    }
}

fn wait_for_session_end() {
    #[cfg(windows)]
    unsafe {
        use windows::Win32::System::Console::SetConsoleCtrlHandler;
        let _ = SetConsoleCtrlHandler(Some(handler), true);
    }
    // Block until a signal flips the flag.
    loop {
        if read_shutdown_flag() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
}

static SHUTDOWN_FLAG: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static ENDED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn read_shutdown_flag() -> bool {
    SHUTDOWN_FLAG.load(std::sync::atomic::Ordering::SeqCst)
}

#[cfg(windows)]
unsafe extern "system" fn handler(ctrl: u32) -> windows::Win32::Foundation::BOOL {
    // CTRL_LOGOFF_EVENT = 5, CTRL_SHUTDOWN_EVENT = 6
    if ctrl == 5 || ctrl == 6 {
        SHUTDOWN_FLAG.store(true, std::sync::atomic::Ordering::SeqCst);
        // Give the main thread a moment to play the sound before returning FALSE
        // (returning FALSE lets the default handler proceed with logoff).
        for _ in 0..80 {
            if ENDED.load(std::sync::atomic::Ordering::SeqCst) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
    windows::Win32::Foundation::BOOL::from(false)
}
