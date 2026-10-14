pub mod app;
pub mod device;
pub mod logging;
pub mod protocol;

#[cfg(windows)]
pub mod autostart;
#[cfg(windows)]
pub mod icon;
#[cfg(windows)]
pub mod startup;
#[cfg(windows)]
pub mod tray;

pub const APP_NAME: &str = "R11 Ultra Battery Tracker";
pub const RUN_VALUE: &str = "R11UltraBattery";
