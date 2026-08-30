//! Windows system-tray UI (tao + tray-icon + muda).

use std::sync::mpsc;
use std::thread;
use std::time::Instant;

use anyhow::{Context, Result};
use tao::event::Event;
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy};
use tracing::{debug, info, warn};
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use crate::autostart;
use crate::device::{poll_once, BatteryReading};
use crate::icon::make_icon;
use crate::protocol::{POLL_INTERVAL, STALE_READING};
use crate::startup;
use crate::APP_NAME;

#[derive(Debug, Clone)]
enum UserEvent {
    Menu(String),
    Battery(BatteryDisplay),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DisplayKey {
    percent: Option<u8>,
    charging: bool,
    stale: bool,
}

#[derive(Debug, Clone)]
struct BatteryDisplay {
    reading: Option<BatteryReading>,
    stale: bool,
}

struct MenuHandles {
    root: Menu,
    status_item: MenuItem,
    autostart_item: CheckMenuItem,
}

impl MenuHandles {
    fn build(initial_autostart: bool) -> Result<Self> {
        let root = Menu::new();
        let status_item = MenuItem::new("Attack Shark R11 Ultra: …", false, None);
        let refresh_item = MenuItem::with_id("refresh", "Refresh", true, None);
        let autostart_item = CheckMenuItem::with_id(
            "autostart",
            "Start with Windows",
            true,
            initial_autostart,
            None,
        );
        let exit_item = MenuItem::with_id("exit", "Quit", true, None);

        root.append_items(&[
            &status_item,
            &PredefinedMenuItem::separator(),
            &refresh_item,
            &autostart_item,
            &PredefinedMenuItem::separator(),
            &exit_item,
        ])?;

        Ok(Self {
            root,
            status_item,
            autostart_item,
        })
    }
}

pub fn run_tray_app() -> Result<()> {
    let exe_path = std::env::current_exe().context("failed resolving executable path")?;
    startup::clear_stale_startup();

    let autostart_enabled = autostart::is_enabled().unwrap_or(false);

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    MenuEvent::set_event_handler(Some({
        let proxy = proxy.clone();
        move |event: MenuEvent| {
            let _ = proxy.send_event(UserEvent::Menu(event.id.0.clone()));
        }
    }));

    let (cmd_tx, cmd_rx) = mpsc::channel::<WorkerCommand>();
    spawn_poll_worker(proxy.clone(), cmd_rx);

    let menu = MenuHandles::build(autostart_enabled)?;
    let initial_icon = make_icon(None).map_err(|e| anyhow::anyhow!("{e:?}"))?;
    let mut tray = Some(build_tray_icon(&menu.root, initial_icon)?);

    let mut display_key = DisplayKey {
        percent: None,
        charging: false,
        stale: false,
    };

    let _ = cmd_tx.send(WorkerCommand::PollNow);

    event_loop.run(move |event, _target, control_flow| {
        *control_flow = ControlFlow::Wait;

        if let Event::UserEvent(user_event) = event {
            match user_event {
                UserEvent::Menu(id) => {
                    if id == "refresh" {
                        let _ = cmd_tx.send(WorkerCommand::PollNow);
                    } else if id == "exit" {
                        let _ = cmd_tx.send(WorkerCommand::Shutdown);
                        tray.take();
                        *control_flow = ControlFlow::Exit;
                    } else if id == "autostart" {
                        let enabled = menu.autostart_item.is_checked();
                        if let Err(err) = autostart::set_enabled(&exe_path, enabled) {
                            warn!("failed to set autostart: {err:#}");
                        } else {
                            info!("Start with Windows: {}", if enabled { "on" } else { "off" });
                        }
                    }
                }
                UserEvent::Battery(display) => {
                    if let Err(err) = refresh_tray_visuals(
                        &mut tray,
                        display.reading.as_ref(),
                        display.stale,
                        &menu.status_item,
                        &mut display_key,
                    ) {
                        warn!("failed refreshing tray: {err:#}");
                    }
                }
            }
        }
    });

    #[allow(unreachable_code)]
    Ok(())
}

fn build_tray_icon(menu: &Menu, icon: Icon) -> Result<TrayIcon> {
    TrayIconBuilder::new()
        .with_menu(Box::new(menu.clone()))
        .with_tooltip(APP_NAME)
        .with_icon(icon)
        .build()
        .context("failed creating tray icon")
}

fn status_text(reading: Option<&BatteryReading>, stale: bool) -> String {
    match reading {
        None => "Attack Shark R11 Ultra: disconnected".into(),
        Some(r) => {
            let mut label = format!("Attack Shark R11 Ultra: {}%", r.percent);
            if r.charging {
                label.push_str(" (charging)");
            } else if stale {
                label.push_str(" (last reading)");
            }
            label
        }
    }
}

fn display_key_for(reading: Option<&BatteryReading>, stale: bool) -> DisplayKey {
    match reading {
        None => DisplayKey {
            percent: None,
            charging: false,
            stale,
        },
        Some(r) => DisplayKey {
            percent: Some(r.percent),
            charging: r.charging,
            stale,
        },
    }
}

fn refresh_tray_visuals(
    tray: &mut Option<TrayIcon>,
    reading: Option<&BatteryReading>,
    stale: bool,
    status_item: &MenuItem,
    last_key: &mut DisplayKey,
) -> Result<()> {
    let Some(tray) = tray.as_mut() else {
        return Ok(());
    };
    let key = display_key_for(reading, stale);

    if key.percent != last_key.percent || key.charging != last_key.charging {
        let icon = make_icon(reading).map_err(|e| anyhow::anyhow!("{e:?}"))?;
        tray.set_icon(Some(icon))?;
    }

    if key != *last_key {
        let tooltip = status_text(reading, stale);
        tray.set_tooltip(Some(tooltip.clone()))?;
        status_item.set_text(&tooltip);
        *last_key = key;
    }

    Ok(())
}

#[derive(Debug, Clone)]
enum WorkerCommand {
    PollNow,
    Shutdown,
}

fn spawn_poll_worker(proxy: EventLoopProxy<UserEvent>, cmd_rx: mpsc::Receiver<WorkerCommand>) {
    thread::spawn(move || {
        let mut stop = false;
        let mut last_reading: Option<BatteryReading> = None;
        let mut last_reading_at: Option<Instant> = None;

        while !stop {
            let timeout = POLL_INTERVAL;
            match cmd_rx.recv_timeout(timeout) {
                Ok(WorkerCommand::Shutdown) => break,
                Ok(WorkerCommand::PollNow) | Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }

            if cmd_rx.try_recv().ok().is_some() {
                // coalesce rapid refresh requests
            }

            let result = poll_once();
            let now = Instant::now();

            if let Some(reading) = result.reading {
                last_reading = Some(reading.clone());
                last_reading_at = Some(now);
                debug!(
                    "Battery {}% ({}) pid=0x{:04x}",
                    reading.percent,
                    reading.state_label(),
                    reading.product_id
                );
                let _ = proxy.send_event(UserEvent::Battery(BatteryDisplay {
                    reading: Some(reading),
                    stale: false,
                }));
                continue;
            }

            for err in &result.errors {
                debug!("poll note: {err}");
            }

            if let (Some(cached), Some(at)) = (&last_reading, last_reading_at) {
                if now.duration_since(at) <= STALE_READING {
                    debug!(
                        "Poll missed; keeping last reading {}% ({}s old)",
                        cached.percent,
                        now.duration_since(at).as_secs()
                    );
                    let _ = proxy.send_event(UserEvent::Battery(BatteryDisplay {
                        reading: last_reading.clone(),
                        stale: true,
                    }));
                    continue;
                }
            }

            last_reading = None;
            last_reading_at = None;
            debug!("Mouse not found / no battery reply");
            let _ = proxy.send_event(UserEvent::Battery(BatteryDisplay {
                reading: None,
                stale: false,
            }));

            if let Ok(WorkerCommand::Shutdown) = cmd_rx.try_recv() {
                stop = true;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skip_redundant_icon_key() {
        let reading = BatteryReading {
            percent: 100,
            wired: false,
            charging: false,
            voltage_mv: None,
            vendor_id: 0x3554,
            product_id: 0xFB44,
            product_string: String::new(),
        };
        let a = display_key_for(Some(&reading), false);
        let b = display_key_for(Some(&reading), false);
        assert_eq!(a, b);
        let c = display_key_for(Some(&reading), true);
        assert_ne!(a, c);
    }

    #[test]
    fn status_text_formats_stale() {
        let reading = BatteryReading {
            percent: 50,
            wired: false,
            charging: false,
            voltage_mv: None,
            vendor_id: 0x3554,
            product_id: 0xFB44,
            product_string: String::new(),
        };
        assert!(status_text(Some(&reading), true).contains("last reading"));
    }
}
