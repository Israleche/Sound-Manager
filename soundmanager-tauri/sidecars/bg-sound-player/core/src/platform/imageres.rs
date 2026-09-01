//! Startup sound patching of C:\Windows\System32\imageres.dll.
//! Port of ImageresPatcher.cs + NativeResource.cs using BeginUpdateResource /
//! UpdateResource / EndUpdateResource (Win32; PE resource edits are cross-arch safe).

use crate::errors::{CoreError, CoreResult};
use std::path::{Path, PathBuf};

const WAVE_LOCALE: u16 = 1033;
/// Vista: WAVE resource id 5051; Win7+: 5080
fn wave_resource_number() -> u32 {
    if crate::platform::version::is_vista() { 5051 } else { 5080 }
}

fn system32_dir() -> PathBuf {
    let windir = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string());
    PathBuf::from(windir).join("System32")
}

pub fn imageres_path() -> PathBuf { system32_dir().join("imageres.dll") }
pub fn imageres_bak_path() -> PathBuf { imageres_path().with_extension("dll.bak") }
pub fn imageres_old_path() -> PathBuf { imageres_path().with_extension("dll.old") }

/// Empty-but-valid WAV header (upstream emptyWavFile bytes).
const EMPTY_WAV: &[u8] = &[
    0x52, 0x49, 0x46, 0x46, 0x3e, 0x00, 0x00, 0x00, 0x57, 0x41, 0x56, 0x45, 0x66, 0x6d, 0x74, 0x20,
    0x12, 0x00, 0x00, 0x00, 0x01, 0x00, 0x02, 0x00, 0x44, 0xac, 0x00, 0x00, 0x10, 0xb1, 0x02, 0x00,
    0x04, 0x00, 0x10, 0x00, 0x00, 0x00, 0x66, 0x61, 0x63, 0x74, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x64, 0x61, 0x74, 0x61, 0x00, 0x00, 0x00, 0x00, 0x4c, 0x49, 0x53, 0x54, 0x04, 0x00,
    0x00, 0x00, 0x49, 0x4e, 0x46, 0x4f,
];

pub fn is_patching_possible() -> bool {
    crate::platform::version::is_at_least_vista()
}

pub fn is_patching_required() -> bool {
    crate::platform::version::is_vista() || crate::platform::version::is_7()
}

pub fn is_patching_not_recommended() -> bool {
    crate::platform::version::is_at_least_10()
}

fn require_admin() -> CoreResult<()> {
    if crate::platform::fs_admin::is_admin() {
        Ok(())
    } else {
        Err(CoreError::Permission("admin required to patch imageres.dll".into()))
    }
}

/// Take ownership grants and create imageres.dll.bak if missing.
pub fn backup() -> CoreResult<()> {
    if !is_patching_possible() {
        return Err(CoreError::Patching("not possible on this Windows version".into()));
    }
    let imageres = imageres_path();
    if !imageres.is_file() {
        return Err(CoreError::Patching("imageres.dll not found".into()));
    }
    require_admin()?;
    let bak = imageres_bak_path();
    if bak.is_file() {
        crate::platform::fs_admin::grant_all(&bak)?;
    } else {
        crate::platform::fs_admin::grant_all(&system32_dir())?;
        crate::platform::fs_admin::grant_all(&imageres)?;
        std::fs::rename(&imageres, &bak)?;
        std::fs::copy(&bak, &imageres)?;
    }
    Ok(())
}

/// Restore imageres.dll from backup.
pub fn restore() -> CoreResult<()> {
    if !is_patching_possible() {
        return Err(CoreError::Patching("not possible on this Windows version".into()));
    }
    let bak = imageres_bak_path();
    if !bak.is_file() {
        return Ok(()); // nothing to restore
    }
    require_admin()?;
    let imageres = imageres_path();
    crate::platform::fs_admin::grant_all(&system32_dir())?;
    crate::platform::fs_admin::grant_all(&bak)?;
    crate::platform::fs_admin::grant_all(&imageres)?;
    let old = imageres_old_path();
    if old.exists() {
        std::fs::remove_file(&old).ok();
    }
    std::fs::rename(&imageres, &old).ok(); // in-use DLL may refuse; tolerated upstream
    if imageres.exists() {
        std::fs::remove_file(&imageres).ok();
    }
    std::fs::rename(&bak, &imageres)?;
    Ok(())
}

/// Patch the embedded WAVE resource with the given PCM WAV file (or silence if None).
pub fn patch(replacement_wav: Option<&Path>) -> CoreResult<()> {
    if !is_patching_possible() {
        return Err(CoreError::Patching("not possible on this Windows version".into()));
    }
    if !imageres_path().is_file() && !imageres_bak_path().is_file() {
        return Err(CoreError::Patching("imageres.dll not found".into()));
    }
    require_admin()?;
    if !imageres_bak_path().is_file() {
        backup()?;
    }
    let imageres = imageres_path();
    let old = imageres_old_path();
    if old.exists() {
        std::fs::remove_file(&old).ok();
    }
    std::fs::rename(&imageres, &old).ok();

    std::fs::copy(imageres_bak_path(), &imageres)?;

    let data: Vec<u8> = match replacement_wav {
        Some(p) if p.is_file() => std::fs::read(p)?,
        _ => EMPTY_WAV.to_vec(),
    };

    let success = replace_wave_resource(&imageres, &data);
    if !success {
        std::fs::copy(imageres_bak_path(), &imageres)?;
        return Err(CoreError::Patching("UpdateResource failed".into()));
    }
    Ok(())
}

/// Extract the default startup sound from imageres(.bak).dll to outputFile.
pub fn extract_default(output_file: &Path) -> CoreResult<bool> {
    if !is_patching_possible() {
        return Err(CoreError::Patching("not possible on this Windows version".into()));
    }
    let bak = imageres_bak_path();
    let source = if bak.is_file() { bak } else { imageres_path() };
    if !source.is_file() {
        return Err(CoreError::Patching("imageres.dll not found".into()));
    }
    extract_wave_resource(&source, wave_resource_number(), WAVE_LOCALE, output_file)
}

// ---------------------------------------------------------------------
// Win32 resource API (windows crate 0.58: resource fns live in
// Win32::System::LibraryLoader)
// ---------------------------------------------------------------------

#[cfg(windows)]
fn make_int_resource(id: u32) -> PCWSTR {
    // MAKEINTRESOURCEW semantics: numeric id encoded in the low word.
    unsafe { PCWSTR((id as usize) as *const u16) }
}

#[cfg(windows)]
use windows::core::PCWSTR;

#[cfg(windows)]
fn replace_wave_resource(dll: &Path, data: &[u8]) -> bool {
    use windows::Win32::System::LibraryLoader::{
        BeginUpdateResourceW, EndUpdateResourceW, UpdateResourceW,
    };

    let wide: Vec<u16> = dll.as_os_str().to_string_lossy().encode_utf16().chain([0]).collect();
    unsafe {
        let Ok(handle) = BeginUpdateResourceW(PCWSTR(wide.as_ptr()), false) else {
            log::error!("BeginUpdateResourceW failed");
            return false;
        };
        let wave_type: Vec<u16> = "WAVE\0".encode_utf16().collect();
        let update = UpdateResourceW(
            handle,
            PCWSTR(wave_type.as_ptr()),
            make_int_resource(wave_resource_number()),
            WAVE_LOCALE,
            Some(data.as_ptr() as *const _),
            data.len() as u32,
        );
        if let Err(e) = update {
            log::error!("UpdateResourceW failed: {e}");
            let _ = EndUpdateResourceW(handle, true);
            return false;
        }
        if let Err(e) = EndUpdateResourceW(handle, false) {
            log::error!("EndUpdateResourceW failed: {e}");
            return false;
        }
        true
    }
}

#[cfg(windows)]
fn extract_wave_resource(dll: &Path, resource_id: u32, locale: u16, out: &Path) -> CoreResult<bool> {
    use windows::Win32::Foundation::HMODULE;
    use windows::Win32::System::LibraryLoader::{
        FindResourceExW, GetModuleHandleW, LoadLibraryExW, LoadResource, LockResource,
        SizeofResource, LOAD_LIBRARY_FLAGS,
    };

    let wide: Vec<u16> = dll.as_os_str().to_string_lossy().encode_utf16().chain([0]).collect();
    unsafe {
        // Load as a data file: resources readable, no code executed.
        let hmod: HMODULE = GetModuleHandleW(PCWSTR(wide.as_ptr())).or_else(|_| {
            LoadLibraryExW(
                PCWSTR(wide.as_ptr()),
                None,
                LOAD_LIBRARY_FLAGS(0x00000002), // LOAD_LIBRARY_AS_DATAFILE
            )
        }).map_err(|e| CoreError::Patching(format!("load imageres: {e}")))?;

        let wave_type: Vec<u16> = "WAVE\0".encode_utf16().collect();
        let found = FindResourceExW(
            hmod,
            PCWSTR(wave_type.as_ptr()),
            make_int_resource(resource_id),
            locale,
        );
        let found = match found {
            h if !h.is_invalid() => h,
            _ => return Ok(false),
        };
        let loaded = LoadResource(hmod, found).map_err(|e| CoreError::Patching(e.to_string()))?;
        let ptr = LockResource(loaded);
        if ptr.is_null() {
            return Ok(false);
        }
        let size = SizeofResource(hmod, found);
        let bytes = std::slice::from_raw_parts(ptr as *const u8, size as usize);
        std::fs::write(out, bytes)?;
        Ok(true)
    }
}

#[cfg(not(windows))]
fn replace_wave_resource(_dll: &Path, _data: &[u8]) -> bool { false }
#[cfg(not(windows))]
fn extract_wave_resource(_dll: &Path, _id: u32, _locale: u16, _out: &Path) -> CoreResult<bool> { Ok(false) }
