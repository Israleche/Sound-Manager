//! bg-sound-player (windowed): hidden window + Task Scheduler registration.
//! Plays startup/logon on launch; on WM_QUERYENDSESSION/WM_ENDSESSION plays
//! shutdown/logoff. Falls back to console handler if windowing fails.

use sound_manager_core as core;
use sound_manager_core::domain::settings::Settings;
use sound_manager_core::domain::sound_event::{event_by_type, EventType};

#[cfg(windows)]
mod win {
    use super::*;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM, HINSTANCE};
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
        TranslateMessage, PostQuitMessage, WINDOW_EX_STYLE, WNDCLASSW,
        WM_DESTROY, WM_QUERYENDSESSION, WM_ENDSESSION, WS_OVERLAPPEDWINDOW, MSG,
        RegisterClassW,
    };
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;

    static mut PLAY_SHUTDOWN: bool = false;

    // ShutdownBlockReason* not yet in windows 0.58 bindings at this feature set;
    // call via raw GetProcAddress to avoid build break.
    unsafe fn shutdown_block_reason_create(hwnd: HWND, reason: PCWSTR) -> bool {
        use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
        let user32: Vec<u16> = "user32.dll\0".encode_utf16().collect();
        let Ok(h) = LoadLibraryW(PCWSTR(user32.as_ptr())) else { return false };
        let name = b"ShutdownBlockReasonCreate\0";
        let Some(f) = GetProcAddress(h, windows::core::PCSTR(name.as_ptr())) else { return false };
        let f: unsafe extern "system" fn(HWND, PCWSTR) -> windows::Win32::Foundation::BOOL = std::mem::transmute(f);
        f(hwnd, reason).as_bool()
    }
    unsafe fn shutdown_block_reason_destroy(hwnd: HWND) -> bool {
        use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
        let user32: Vec<u16> = "user32.dll\0".encode_utf16().collect();
        let Ok(h) = LoadLibraryW(PCWSTR(user32.as_ptr())) else { return false };
        let name = b"ShutdownBlockReasonDestroy\0";
        let Some(f) = GetProcAddress(h, windows::core::PCSTR(name.as_ptr())) else { return false };
        let f: unsafe extern "system" fn(HWND) -> windows::Win32::Foundation::BOOL = std::mem::transmute(f);
        f(hwnd).as_bool()
    }

    unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        match msg {
            WM_QUERYENDSESSION => {
                let reason: Vec<u16> = "Playing system sound\0".encode_utf16().collect();
                let _ = shutdown_block_reason_create(hwnd, PCWSTR(reason.as_ptr()));
                let s = Settings::load_or_default();
                PLAY_SHUTDOWN = s.prefer_startup_sound_on_logon || wparam.0 == 1;
                LRESULT(1)
            }
            WM_ENDSESSION => {
                if wparam.0 != 0 {
                    let media = core::domain::scheme_meta::media_dir();
                    let ev = if PLAY_SHUTDOWN {
                        event_by_type(EventType::Shutdown)
                    } else {
                        event_by_type(EventType::Logoff)
                    };
                    let path = ev.file_path(&media);
                    if path.is_file() {
                        let _ = core::audio::play::play_wav(&path, true);
                    }
                    let _ = shutdown_block_reason_destroy(hwnd);
                }
                PostQuitMessage(0);
                LRESULT(0)
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }

    pub fn run_windowed() -> anyhow::Result<()> {
        unsafe {
            let hinstance = HINSTANCE(GetModuleHandleW(PCWSTR::null())?.0);
            let class_name: Vec<u16> = "SoundManagerBg\0".encode_utf16().collect();
            let wc = WNDCLASSW {
                lpfnWndProc: Some(wnd_proc),
                hInstance: hinstance,
                lpszClassName: PCWSTR(class_name.as_ptr()),
                ..Default::default()
            };
            RegisterClassW(&wc);
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                PCWSTR(class_name.as_ptr()),
                PCWSTR::null(),
                WS_OVERLAPPEDWINDOW,
                -32000, -32000, 1, 1,
                None, None, hinstance, None,
            )?;
            // Play startup/logon on launch
            {
                let s = Settings::load_or_default();
                let media = core::domain::scheme_meta::media_dir();
                let exists = |ev: &core::domain::sound_event::SoundEvent| ev.file_path(&media).is_file();
                let startup = event_by_type(EventType::Startup);
                let logon = event_by_type(EventType::Logon);
                let to_play = if s.prefer_startup_sound_on_logon && exists(startup) { Some(startup) } else if exists(logon) { Some(logon) } else { None };
                if let Some(ev) = to_play {
                    let path = ev.file_path(&media);
                    let _ = core::audio::play::play_wav(&path, true);
                }
            }
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            DestroyWindow(hwnd)?;
        }
        Ok(())
    }
}

fn main() {
    env_logger::init();
    #[cfg(windows)]
    {
        if let Err(e) = win::run_windowed() {
            log::error!("windowed sidecar failed: {e}; falling back to console handler");
            fallback_console();
        }
    }
    #[cfg(not(windows))]
    {
        fallback_console();
    }
}

fn fallback_console() {
    let s = Settings::load_or_default();
    let media = core::domain::scheme_meta::media_dir();
    let exists = |ev: &core::domain::sound_event::SoundEvent| ev.file_path(&media).is_file();
    let startup = event_by_type(EventType::Startup);
    let logon = event_by_type(EventType::Logon);
    let shutdown = event_by_type(EventType::Shutdown);
    let logoff = event_by_type(EventType::Logoff);
    let to_play = if s.prefer_startup_sound_on_logon && exists(startup) { startup } else { logon };
    if exists(to_play) {
        let _ = core::audio::play::play_wav(&to_play.file_path(&media), true);
    }
    #[cfg(windows)]
    unsafe {
        use windows::Win32::System::Console::SetConsoleCtrlHandler;
        let _ = SetConsoleCtrlHandler(Some(handler), true);
    }
    loop {
        if SHUTDOWN.load(std::sync::atomic::Ordering::SeqCst) { break; }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    let ev = if s.prefer_startup_sound_on_logon { shutdown } else { logoff };
    if exists(ev) {
        let _ = core::audio::play::play_wav(&ev.file_path(&media), true);
    }
}

static SHUTDOWN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(windows)]
unsafe extern "system" fn handler(ctrl: u32) -> windows::Win32::Foundation::BOOL {
    if ctrl == 5 || ctrl == 6 {
        SHUTDOWN.store(true, std::sync::atomic::Ordering::SeqCst);
        std::thread::sleep(std::time::Duration::from_secs(8));
    }
    windows::Win32::Foundation::BOOL::from(false)
}
