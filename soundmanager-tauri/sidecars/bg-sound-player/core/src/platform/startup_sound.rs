//! System startup sound enabled flag — HKLM BootAnimation / EditionOverrides.
//! Port of SystemStartupSound.cs (requires admin for writes).

use crate::errors::{CoreError, CoreResult};

#[cfg(windows)]
const BOOT_ANIM_KEY: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\LogonUI\BootAnimation";
#[cfg(windows)]
const EDITION_OVERRIDES_KEY: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\EditionOverrides";

#[cfg(windows)]
pub fn get_enabled() -> CoreResult<bool> {
    use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ};
    use winreg::RegKey;
    let hk = RegKey::predef(HKEY_LOCAL_MACHINE);
    let is11 = crate::platform::version::is_11();
    let (key_path, value_name) = if is11 {
        (EDITION_OVERRIDES_KEY, "UserSetting_DisableStartupSound")
    } else {
        (BOOT_ANIM_KEY, "DisableStartupSound")
    };
    let default_disabled = {
        let v = crate::platform::version::info();
        v.major == 6 && (v.minor == 2 || v.minor == 3) || (v.major == 10 && v.build < 22000)
    };
    let val: Option<u32> = hk.open_subkey(key_path).ok().and_then(|k| k.get_value(value_name).ok());
    Ok(match val {
        Some(v) => v == 0,
        None => !default_disabled,
    })
}

#[cfg(not(windows))]
pub fn get_enabled() -> CoreResult<bool> { Ok(true) }

#[cfg(windows)]
pub fn set_enabled(enabled: bool) -> CoreResult<()> {
    use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_WRITE};
    use winreg::RegKey;
    if !crate::platform::version::is_at_least_vista() {
        return Ok(());
    }
    let is11 = crate::platform::version::is_11();
    let (key_path, value_name) = if is11 {
        (EDITION_OVERRIDES_KEY, "UserSetting_DisableStartupSound")
    } else {
        (BOOT_ANIM_KEY, "DisableStartupSound")
    };
    let hk = RegKey::predef(HKEY_LOCAL_MACHINE);
    let (k, _) = hk.create_subkey_with_flags(key_path, KEY_WRITE)
        .map_err(|e| CoreError::Registry(e.to_string()))?;
    k.set_value(value_name, &(if enabled { 0u32 } else { 1u32 }))
        .map_err(|e| CoreError::Registry(e.to_string()))?;
    Ok(())
}

#[cfg(not(windows))]
pub fn set_enabled(_enabled: bool) -> CoreResult<()> { Ok(()) }
