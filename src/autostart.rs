//! HKCU Run autostart for Windows.

use std::path::Path;

use anyhow::{Context, Result};
use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};
use winreg::RegKey;

use crate::RUN_VALUE;

const RUN_KEY_PATH: &str = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run";

pub fn is_enabled() -> Result<bool> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu
        .open_subkey_with_flags(RUN_KEY_PATH, KEY_READ)
        .context("failed to open Run key")?;
    Ok(key.get_value::<String, _>(RUN_VALUE).is_ok())
}

pub fn get_startup_command() -> Result<Option<String>> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu
        .open_subkey_with_flags(RUN_KEY_PATH, KEY_READ)
        .context("failed to open Run key")?;
    match key.get_value::<String, _>(RUN_VALUE) {
        Ok(value) => {
            let text = value.trim().to_string();
            if text.is_empty() {
                Ok(None)
            } else {
                Ok(Some(text))
            }
        }
        Err(_) => Ok(None),
    }
}

pub fn set_enabled(exe_path: &Path, enabled: bool) -> Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu
        .create_subkey(RUN_KEY_PATH)
        .context("failed to create/open Run key")?;

    if enabled {
        let quoted = format!("\"{}\"", exe_path.display());
        key.set_value(RUN_VALUE, &quoted)
            .context("failed writing Run key value")?;
    } else {
        let _ = key.delete_value(RUN_VALUE);
    }
    Ok(())
}
