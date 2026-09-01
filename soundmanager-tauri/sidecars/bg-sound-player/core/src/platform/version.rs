//! Windows version detection. Port of WindowsVersion.cs (registry-based; WMI
//! friendly-name lookup replaced with a build-number check for Win11).

use serde::Serialize;

const CURRENT_VERSION_REGKEY: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion";

#[derive(Debug, Clone, Serialize)]
pub struct WindowsVersionInfo {
    pub major: u32,
    pub minor: u32,
    pub build: u32,
    pub friendly_name: String,
}

fn try_get_u32(key: &str) -> Option<u32> {
    use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ};
    use winreg::RegKey;
    let hk = RegKey::predef(HKEY_LOCAL_MACHINE);
    let k = hk.open_subkey(CURRENT_VERSION_REGKEY).ok()?;
    k.get_value::<u32, _>(key).ok()
}

fn try_get_string(key: &str) -> Option<String> {
    use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ};
    use winreg::RegKey;
    let hk = RegKey::predef(HKEY_LOCAL_MACHINE);
    let k = hk.open_subkey(CURRENT_VERSION_REGKEY).ok()?;
    k.get_value::<String, _>(key).ok()
}

pub fn info() -> WindowsVersionInfo {
    let major = try_get_u32("CurrentMajorVersionNumber")
        .or_else(|| try_get_string("CurrentVersion")?.split('.').next()?.parse().ok())
        .unwrap_or(0);
    let minor = try_get_u32("CurrentMinorVersionNumber")
        .or_else(|| try_get_string("CurrentVersion")?.split('.').nth(1)?.parse().ok())
        .unwrap_or(0);
    let build = try_get_u32("CurrentBuildNumber")
        .or_else(|| try_get_string("CurrentBuildNumber")?.parse().ok())
        .unwrap_or(0);
    // Windows 11 = build >= 22000
    let friendly = if build >= 22000 {
        "Windows 11".to_string()
    } else {
        try_get_string("ProductName").unwrap_or_else(|| "Windows".to_string())
    };
    WindowsVersionInfo { major, minor, build, friendly_name: friendly }
}

pub fn is_at_least(major: u32, minor: u32) -> bool {
    let i = info();
    i.major > major || (i.major == major && i.minor >= minor)
}

pub fn is_vista() -> bool { let i = info(); i.major == 6 && i.minor == 0 }
pub fn is_at_least_vista() -> bool { is_at_least(6, 0) }
pub fn is_7() -> bool { let i = info(); i.major == 6 && i.minor == 1 }
pub fn is_at_least_7() -> bool { is_at_least(6, 1) }
pub fn is_at_least_8() -> bool { is_at_least(6, 2) }
pub fn is_at_least_10() -> bool { is_at_least(10, 0) }
pub fn is_11() -> bool { info().build >= 22000 }
