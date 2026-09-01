//! Sound playback. On Windows uses PlaySound (WAV only, synchronous option);
//! cross-platform fallback decodes and plays via the default device (TODO rodio).

use crate::errors::{CoreError, CoreResult};
use std::path::Path;

/// Play a WAV file. If `sync` is true, block until playback finishes
/// (upstream PlaySound / PlaySync behavior for the bg sound player).
#[cfg(windows)]
pub fn play_wav(path: &Path, sync: bool) -> CoreResult<()> {
    use windows::core::PCWSTR;
    use windows::Win32::Media::Audio::{PlaySoundW, SND_FLAGS};

    let wide: Vec<u16> = path.as_os_str().to_string_lossy().encode_utf16().chain([0]).collect();
    let flags = if sync {
        SND_FLAGS(0x0000) // SND_SYNC
    } else {
        SND_FLAGS(0x0001) // SND_ASYNC
    };
    unsafe {
        let ok = PlaySoundW(PCWSTR(wide.as_ptr()), None, flags);
        if sync && !ok.as_bool() {
            return Err(CoreError::Audio("PlaySound failed".into()));
        }
        Ok(())
    }
}

#[cfg(not(windows))]
pub fn play_wav(_path: &Path, _sync: bool) -> CoreResult<()> {
    Err(CoreError::Audio("playback unsupported on this platform".into()))
}

/// Play any file: non-WAV inputs are transcoded to a temp WAV first.
pub fn play_any(path: &Path, sync: bool) -> CoreResult<()> {
    if crate::audio::convert::is_wav(path) {
        return play_wav(path, sync);
    }
    let wav = crate::audio::convert::to_wav(path)?;
    let tmp = std::env::temp_dir().join("soundmanager-preview.wav");
    std::fs::write(&tmp, wav)?;
    let r = play_wav(&tmp, sync);
    let _ = std::fs::remove_file(&tmp);
    r
}
