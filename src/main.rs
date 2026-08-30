// Build as a GUI-subsystem app so launching the tray doesn't pop a console window.
// CLI modes (--once/--diag) re-attach to the parent console below.
#![windows_subsystem = "windows"]

use as_r11_ultra_battery_tray::app;

#[cfg(windows)]
fn attach_parent_console() {
    use windows::Win32::System::Console::AttachConsole;
    const ATTACH_PARENT_PROCESS: u32 = 0xFFFF_FFFF;
    unsafe {
        let _ = AttachConsole(ATTACH_PARENT_PROCESS);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    #[cfg(windows)]
    if args.iter().any(|a| a == "--once" || a == "--diag") {
        attach_parent_console();
    }

    let result = if args.iter().any(|a| a == "--diag") {
        app::run_diag_mode()
    } else if args.iter().any(|a| a == "--once") {
        app::run_once()
    } else {
        app::run_tray()
    };

    if let Err(err) = result {
        eprintln!("R11 Ultra Battery error: {err:#}");
        std::process::exit(1);
    }
}
