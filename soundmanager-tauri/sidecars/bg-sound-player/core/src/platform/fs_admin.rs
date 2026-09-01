//! Filesystem admin helpers: IsAdmin + ACL grants. Port of FileSystemAdmin.cs.
//! grant_all uses takeown/icacls (reliable, avoids raw ACL FFI).

use crate::errors::CoreResult;
use std::path::Path;
use std::process::Command;

/// Whether the current process token belongs to the Administrators group.
#[cfg(windows)]
pub fn is_admin() -> bool {
    use windows::Win32::UI::Shell::IsUserAnAdmin;
    unsafe { IsUserAnAdmin().as_bool() }
}

#[cfg(not(windows))]
pub fn is_admin() -> bool {
    false
}

/// Run a command, returning trimmed stdout.
fn run(cmd: &mut Command) -> CoreResult<String> {
    let out = cmd.output()?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        Err(crate::errors::CoreError::Permission(format!(
            "{} failed: {}",
            cmd.get_program().to_string_lossy(),
            String::from_utf8_lossy(&out.stderr).trim()
        )))
    }
}

/// Grant full access to Administrators on a path (upstream FileSystemAdmin.GrantAll).
/// Uses takeown (ownership → Administrators) + icacls (grant Administrators:F).
#[cfg(windows)]
pub fn grant_all(path: &Path) -> CoreResult<()> {
    let p = path.to_string_lossy().to_string();
    let is_dir = path.is_dir();
    run(Command::new("takeown").args(if is_dir {
        vec!["/F", &p, "/R", "/D", "Y"]
    } else {
        vec!["/F", &p]
    }))?;
    let mut icacls = Command::new("icacls");
    icacls.arg(&p).args(["/grant", "*S-1-5-32-544:F"]); // Administrators
    if is_dir {
        icacls.args(["/T", "/C"]);
    }
    run(&mut icacls)?;
    Ok(())
}

#[cfg(not(windows))]
pub fn grant_all(_path: &Path) -> CoreResult<()> {
    Ok(())
}
