//! Single-instance mutex and stale startup cleanup.

use std::path::PathBuf;

use tracing::info;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, GetLastError};
use windows::Win32::System::Threading::CreateMutexW;

use crate::autostart;

const MUTEX_NAME: &str = "Local\\R11UltraBattery_SingleInstance";
const ERROR_ALREADY_EXISTS: u32 = 183;

pub fn launch_command() -> Result<String, std::io::Error> {
    let exe = std::env::current_exe()?;
    if !exe.is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "executable missing",
        ));
    }
    Ok(format!("\"{}\"", exe.display()))
}

pub fn clear_stale_startup() -> bool {
    let registered = match autostart::get_startup_command() {
        Ok(Some(cmd)) => cmd,
        _ => return false,
    };
    let current = match launch_command() {
        Ok(cmd) => cmd,
        Err(_) => return false,
    };
    if commands_match(&registered, &current) {
        return false;
    }
    if autostart::set_enabled(
        &std::env::current_exe().unwrap_or_else(|_| PathBuf::from(".")),
        false,
    )
    .is_ok()
    {
        info!("Cleared stale Start with Windows entry (was: {registered})");
        return true;
    }
    false
}

fn commands_match(registered: &str, current: &str) -> bool {
    registered.trim().eq_ignore_ascii_case(current.trim())
}

pub fn acquire_single_instance() -> bool {
    let wide: Vec<u16> = MUTEX_NAME.encode_utf16().chain([0]).collect();
    unsafe {
        match CreateMutexW(None, false, PCWSTR(wide.as_ptr())) {
            Ok(handle) => {
                if GetLastError().0 == ERROR_ALREADY_EXISTS {
                    let _ = CloseHandle(handle);
                    return false;
                }
                // Leak the handle for process lifetime — same as typical single-instance guards.
                let _ = handle;
                true
            }
            Err(_) => true,
        }
    }
}

pub fn release_single_instance() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launch_command_is_quoted() {
        let cmd = launch_command().expect("current exe");
        assert!(cmd.starts_with('"'));
        assert!(cmd.ends_with('"'));
        assert!(!cmd.contains('&'));
        assert!(!cmd.contains('|'));
        assert!(!cmd.contains(';'));
    }

    #[test]
    fn commands_match_case_insensitive() {
        assert!(commands_match(
            r#""C:\Apps\R11UltraBattery.exe""#,
            r#""c:\apps\r11ultrabattery.exe""#
        ));
    }
}
