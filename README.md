# R11 Ultra Battery Tracker

Windows system-tray battery monitor for the **Attack Shark R11 Ultra** mouse.

Single native `R11UltraBattery.exe` — reads battery over Compx / Nordic 52840 HID (wireless dongle `0x3554:0xFB44`, wired USB-C `0x3554:0xFB43`).

## Features

- Live colored **percent in the system tray**
- Prefers wireless dongle, falls back to wired
- Context menu: status, Refresh, Start with Windows, Quit
- 30s poll, 5 min stale cache, single-instance guard
- Log: `%LOCALAPPDATA%\R11UltraBattery\tray.log`

## Install

1. Download **R11UltraBattery-windows.zip** from [Releases](https://github.com/pixelinterest/AS-R11Ultra-Battery-Tray/releases)
2. Extract and run `R11UltraBattery.exe`

## Build

Requires [Rust](https://rustup.rs/) and MSVC on Windows.

```powershell
cargo build --release
```

Output: `target\release\R11UltraBattery.exe`

## CLI

```powershell
R11UltraBattery.exe --once   # print battery to stdout
R11UltraBattery.exe --diag   # HID diagnostic dump
```

Tray colors: edit constants in [`src/icon.rs`](src/icon.rs).

Protocol: [`protocol.md`](protocol.md).

MIT — [LICENSE](LICENSE).
