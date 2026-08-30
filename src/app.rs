use anyhow::Result;

use crate::device::{poll_once, run_diag};
use crate::logging;
use crate::startup;
use crate::tray;

pub fn run_once() -> Result<()> {
    logging::init_logging("info");
    let result = poll_once();
    if let Some(reading) = result.reading {
        print!(
            "Attack Shark R11 Ultra — {}{}\n",
            reading.percent,
            if reading.charging { "% (charging)" } else { "%" }
        );
    } else {
        println!("No battery reading available.");
    }
    for err in result.errors {
        eprintln!("{err}");
    }
    Ok(())
}

pub fn run_tray() -> Result<()> {
    logging::init_logging("info");
    if !startup::acquire_single_instance() {
        tracing::info!("Another instance is already running; exiting");
        return Ok(());
    }
    startup::clear_stale_startup();
    tray::run_tray_app()
}

pub fn run_diag_mode() -> Result<()> {
    logging::init_logging("info");
    run_diag()
}
