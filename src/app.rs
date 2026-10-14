use anyhow::Result;

use crate::device::{poll_once, run_diag, BatteryReading};
use crate::logging;

#[cfg(windows)]
use crate::startup;
#[cfg(windows)]
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

pub fn run_json() -> Result<()> {
    logging::init_logging("info");
    let result = poll_once();
    match result.reading {
        Some(reading) => println!("{}", reading_json(&reading)),
        None => {
            println!(r#"{{"ok":false}}"#);
            for err in result.errors {
                eprintln!("{err}");
            }
        }
    }
    Ok(())
}

fn reading_json(reading: &BatteryReading) -> String {
    let voltage = match reading.voltage_mv {
        Some(mv) => mv.to_string(),
        None => "null".to_string(),
    };
    format!(
        r#"{{"ok":true,"percent":{},"charging":{},"wired":{},"voltage_mv":{}}}"#,
        reading.percent, reading.charging, reading.wired, voltage
    )
}

#[cfg(windows)]
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
