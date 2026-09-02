//! File association for .ths / .soundpack — port of ShellFileType file type registration.
//! Uses HKCU\Software\Classes (per-user, no admin) + SHChangeNotify.

use crate::errors::CoreResult;

pub const THS_EXT: &str = "ths";
pub const SOUNDPACK_EXT: &str = "soundpack";

#[cfg(windows)]
fn prog_id(ext: &str) -> String {
    format!("SoundManager.{}", ext)
}

#[cfg(windows)]
pub fn is_associated(ext: &str) -> CoreResult<bool> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let ext_key = format!(r"Software\Classes\.{}", ext);
    let prog = prog_id(ext);
    let cur = hkcu.open_subkey(&ext_key)
        .and_then(|k| k.get_value::<String, _>(""))
        .unwrap_or_default();
    if cur != prog {
        return Ok(false);
    }
    // Check open command
    let cmd_key = format!(r"Software\Classes\{}\shell\open\command", prog);
    let exe = std::env::current_exe().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
    let expected = format!("\"{}\" \"%1\"", exe);
    let cmd = hkcu.open_subkey(&cmd_key)
        .and_then(|k| k.get_value::<String, _>(""))
        .unwrap_or_default();
    Ok(cmd == expected)
}

#[cfg(not(windows))]
pub fn is_associated(_ext: &str) -> CoreResult<bool> { Ok(false) }

#[cfg(windows)]
pub fn set_associated(ext: &str, associated: bool) -> CoreResult<()> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_WRITE};
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let prog = prog_id(ext);
    if associated {
        let exe = std::env::current_exe().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
        let ext_key = format!(r"Software\Classes\.{}", ext);
        let (k, _) = hkcu.create_subkey(&ext_key)?;
        k.set_value("", &prog)?;
        let (k2, _) = hkcu.create_subkey(format!(r"Software\Classes\{}\shell\open\command", prog))?;
        k2.set_value("", &format!("\"{}\" \"%1\"", exe))?;
        let (k3, _) = hkcu.create_subkey(format!(r"Software\Classes\{}", prog))?;
        k3.set_value("", &format!("Sound Manager {} file", ext))?;
        notify_shell();
    } else {
        let _ = hkcu.delete_subkey_all(format!(r"Software\Classes\.{}", ext));
        let _ = hkcu.delete_subkey_all(format!(r"Software\Classes\{}", prog));
        notify_shell();
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn set_associated(_ext: &str, _associated: bool) -> CoreResult<()> { Ok(()) }

#[cfg(windows)]
fn notify_shell() {
    use windows::Win32::UI::Shell::{SHChangeNotify, SHCNE_ID, SHCNF_FLAGS};
    unsafe { SHChangeNotify(SHCNE_ID(0x08000000), SHCNF_FLAGS(0x0000), None, None); }
}

pub fn is_all_associated() -> CoreResult<bool> {
    Ok(is_associated(THS_EXT)? && is_associated(SOUNDPACK_EXT)?)
}
pub fn set_all_associated(associated: bool) -> CoreResult<()> {
    set_associated(THS_EXT, associated)?;
    set_associated(SOUNDPACK_EXT, associated)?;
    Ok(())
}
