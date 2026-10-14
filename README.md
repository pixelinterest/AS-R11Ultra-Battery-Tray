# R11 Ultra Battery Tracker

[![Windows](https://img.shields.io/badge/Windows-10%20%7C%2011-0078D4?logo=windows&logoColor=white)](https://github.com/pixelinterest/AS-R11Ultra-Battery-Tray)
[![Release](https://img.shields.io/github/v/release/pixelinterest/AS-R11Ultra-Battery-Tray?color=28a745)](https://github.com/pixelinterest/AS-R11Ultra-Battery-Tray/releases/latest)
[![Downloads](https://img.shields.io/github/downloads/pixelinterest/AS-R11Ultra-Battery-Tray/total?color=7952b3)](https://github.com/pixelinterest/AS-R11Ultra-Battery-Tray/releases)
[![Rust](https://img.shields.io/badge/Rust-stable-orange?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](./LICENSE)

Windows system tray battery monitor for the **Attack Shark R11 Ultra**.

**[Download latest release](https://github.com/pixelinterest/AS-R11Ultra-Battery-Tray/releases/latest)**

![Tray icon and tooltip](assets/tray.png)

---

## Features

- Colored battery percent in the tray (green / orange / red; blue while charging)
- Same font size for 1–3 digit readings
- Runs in the background with no console window
- **Start with Windows** toggle (no admin required). Opening a newer build clears a stale startup entry — turn the option on again if you want it.
- Polls every 30s; keeps the last reading up to 5 minutes on a missed poll
- Supports 2.4 GHz dongle and USB-C charging
- Single native exe

## Supported modes

| Mode | VID | PID |
|------|-----|-----|
| 2.4 GHz dongle | `0x3554` | `0xFB44` |
| USB-C wired | `0x3554` | `0xFB43` |

Bluetooth is not supported.

---

## Install

1. Download **R11UltraBattery-1.0.2.zip** from [Releases](https://github.com/pixelinterest/AS-R11Ultra-Battery-Tray/releases/latest)
2. Extract and run `R11UltraBattery.exe`

## Build from source

Requires [Rust](https://rustup.rs/) and MSVC on Windows.

```powershell
cargo build --release
```

Output: `target\release\R11UltraBattery.exe`

## CLI

```powershell
R11UltraBattery.exe --once   # print battery to stdout
R11UltraBattery.exe --json   # one-line JSON reading
R11UltraBattery.exe --diag   # HID diagnostic dump
```

Tray colors: edit constants in [`src/icon.rs`](src/icon.rs). Protocol: [`protocol.md`](protocol.md).

## Linux and KDE Plasma

The same binary reads the mouse on Linux and a Plasma 6 panel widget shows the percent. Bluetooth is still unsupported.

Build dependencies: Rust, `pkg-config`, and `libudev`.

```bash
cargo build --release
install -Dm755 target/release/R11UltraBattery ~/.local/bin/R11UltraBattery
mkdir -p ~/.local/share/plasma/plasmoids
cp -a plasmoid/com.pixelinterest.r11ultra.battery ~/.local/share/plasma/plasmoids/
```

HID access needs a seat udev rule (sudo). Copy [`udev/60-r11ultra-battery.rules`](udev/60-r11ultra-battery.rules) under that same name so it runs before systemd applies device access, then reload rules and replug the dongle or cable:

```bash
sudo install -Dm644 udev/60-r11ultra-battery.rules /etc/udev/rules.d/60-r11ultra-battery.rules
sudo udevadm control --reload-rules
sudo udevadm trigger --subsystem-match=hidraw
```

Restart Plasma (`systemctl --user restart plasma-plasmashell`) so it sees the new widget, then right-click the panel and choose **Add Widgets…** → **R11 Ultra Battery**. The widget runs `~/.local/bin/R11UltraBattery --json` every 30 seconds. Check a reading with `R11UltraBattery --once`.
