// Build as a GUI-subsystem app so launching the tray doesn't pop a console window.
// CLI modes (--once/--json/--diag) re-attach to the parent console below.
#![cfg_attr(windows, windows_subsystem = "windows")]

use anyhow::Result;

use as_r11_ultra_battery_tray::app;

#[cfg(windows)]
fn attach_parent_console() {
    use windows::Win32::System::Console::AttachConsole;
    const ATTACH_PARENT_PROCESS: u32 = 0xFFFF_FFFF;
    unsafe {
        let _ = AttachConsole(ATTACH_PARENT_PROCESS);
    }
}

#[cfg(windows)]
fn run_default() -> Result<()> {
    app::run_tray()
}

#[cfg(not(windows))]
fn run_default() -> Result<()> {
    eprintln!("Usage: R11UltraBattery --once | --json | --diag");
    std::process::exit(2);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    #[cfg(windows)]
    if args
        .iter()
        .any(|a| a == "--once" || a == "--json" || a == "--diag")
    {
        attach_parent_console();
    }

    let result = if args.iter().any(|a| a == "--diag") {
        app::run_diag_mode()
    } else if args.iter().any(|a| a == "--json") {
        app::run_json()
    } else if args.iter().any(|a| a == "--once") {
        app::run_once()
    } else {
        run_default()
    };

    if let Err(err) = result {
        eprintln!("R11 Ultra Battery error: {err:#}");
        std::process::exit(1);
    }
}
