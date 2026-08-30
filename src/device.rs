//! HID discovery and Nordic52 battery transactions for the R11 Ultra.

use std::ffi::CString;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use hidapi::{DeviceInfo, HidApi, HidDevice};

use crate::protocol::{
    self, parse_battery_response, battery_request, BatteryData, REPORT_LEN, TRANSACTION_DELAY,
};

#[derive(Debug, Clone)]
pub struct HidCollection {
    pub path: String,
    pub vendor_id: u16,
    pub product_id: u16,
    pub product_string: String,
    pub manufacturer_string: String,
    pub usage_page: u16,
    pub usage: u16,
    pub interface_number: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatteryReading {
    pub percent: u8,
    pub wired: bool,
    pub charging: bool,
    pub voltage_mv: Option<u16>,
    pub vendor_id: u16,
    pub product_id: u16,
    pub product_string: String,
}

impl BatteryReading {
    pub fn state_label(&self) -> &'static str {
        if self.charging {
            "charging"
        } else if self.wired && self.percent >= 100 {
            "full"
        } else if self.wired {
            "wired"
        } else {
            "discharging"
        }
    }

    pub fn from_parts(parsed: BatteryData, collection: &HidCollection) -> Self {
        Self {
            percent: parsed.percent,
            wired: parsed.wired,
            charging: parsed.charging,
            voltage_mv: parsed.voltage_mv,
            vendor_id: collection.vendor_id,
            product_id: collection.product_id,
            product_string: collection.product_string.clone(),
        }
    }
}

fn info_to_collection(info: &DeviceInfo) -> HidCollection {
    HidCollection {
        path: info.path().to_string_lossy().into_owned(),
        vendor_id: info.vendor_id(),
        product_id: info.product_id(),
        product_string: info.product_string().unwrap_or_default().to_string(),
        manufacturer_string: info.manufacturer_string().unwrap_or_default().to_string(),
        usage_page: info.usage_page(),
        usage: info.usage(),
        interface_number: info.interface_number(),
    }
}

pub fn enumerate_all(api: &HidApi) -> Vec<HidCollection> {
    api.device_list().map(info_to_collection).collect()
}

pub fn enumerate_compx(api: &HidApi) -> Vec<HidCollection> {
    api.device_list()
        .filter(|info| info.vendor_id() == protocol::VID)
        .map(info_to_collection)
        .collect()
}

pub fn battery_collections(api: &HidApi) -> Vec<HidCollection> {
    let mut matched: Vec<HidCollection> = enumerate_compx(api)
        .into_iter()
        .filter(|c| c.usage_page == protocol::USAGE_PAGE && c.usage == protocol::USAGE)
        .collect();

    matched.sort_by_key(|c| {
        if c.product_id == protocol::PID_WIRELESS {
            (0, c.product_id)
        } else if c.product_id == protocol::PID_WIRED {
            (1, c.product_id)
        } else {
            (2, c.product_id)
        }
    });
    matched
}

fn drain_device(device: &HidDevice) {
    let mut buf = [0u8; 64];
    for _ in 0..8 {
        match device.read_timeout(&mut buf, 1) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
    }
}

fn transact(path: &str, report: &[u8], read_length: usize, delay: Duration) -> Option<Vec<u8>> {
    let c_path = CString::new(path).ok()?;
    let api = HidApi::new().ok()?;
    let device = api.open_path(c_path.as_c_str()).ok()?;
    transact_on_device(&device, report, read_length, delay)
}

fn transact_on_device(
    device: &HidDevice,
    report: &[u8],
    read_length: usize,
    delay: Duration,
) -> Option<Vec<u8>> {
    drain_device(device);
    let report_id = report.first().copied()?;

    for attempt in 0..3 {
        if device.write(report).ok()? < 1 {
            tracing::warn!("HID write failed (attempt {})", attempt + 1);
            thread::sleep(delay);
            continue;
        }
        thread::sleep(delay);
        let deadline = Instant::now() + Duration::from_secs(1);
        while Instant::now() < deadline {
            let mut buf = vec![0u8; read_length.max(64)];
            match device.read_timeout(&mut buf, 100) {
                Ok(0) => continue,
                Ok(n) => {
                    if buf.first().copied() == Some(report_id) {
                        buf.truncate(n.max(read_length).min(buf.len()));
                        if buf.len() >= read_length {
                            buf.truncate(read_length);
                        }
                        return Some(buf);
                    }
                }
                Err(err) => {
                    tracing::warn!("HID read error: {err}");
                    break;
                }
            }
        }
        tracing::warn!("HID read timeout (attempt {})", attempt + 1);
    }
    None
}

pub fn read_battery(api: &HidApi, collection: Option<&HidCollection>) -> Option<BatteryReading> {
    let targets: Vec<HidCollection> = match collection {
        Some(col) => vec![col.clone()],
        None => battery_collections(api),
    };
    let request = battery_request();

    for col in targets {
        let parsed = transact(&col.path, request, REPORT_LEN, TRANSACTION_DELAY)
            .and_then(|data| parse_battery_response(&data));
        if let Some(parsed) = parsed {
            return Some(BatteryReading::from_parts(parsed, &col));
        }
    }
    None
}

pub fn poll_once() -> PollResult {
    let mut result = PollResult::default();
    let api = match HidApi::new() {
        Ok(api) => api,
        Err(err) => {
            result.errors.push(format!("hidapi init failed: {err}"));
            return result;
        }
    };

    if let Some(reading) = read_battery(&api, None) {
        result.reading = Some(reading);
    } else if battery_collections(&api).is_empty() {
        result.errors.push("No Compx R11 Ultra battery interface found".into());
    } else {
        result.errors.push("Device found but battery query failed".into());
    }
    result
}

#[derive(Debug, Default)]
pub struct PollResult {
    pub reading: Option<BatteryReading>,
    pub errors: Vec<String>,
}

pub fn run_diag() -> Result<()> {
    let api = HidApi::new().context("failed initializing hidapi")?;
    println!("=== R11 Ultra Battery HID diagnostic ===\n");

    let compx = enumerate_compx(&api);
    if compx.is_empty() {
        println!("No HID devices with VID 0x{:04X} found.", protocol::VID);
        return Ok(());
    }

    for (idx, col) in compx.iter().enumerate() {
        println!(
            "[{}] PID=0x{:04X} usage_page=0x{:04X} usage=0x{:04X} iface={} product={:?} path={}",
            idx + 1,
            col.product_id,
            col.usage_page,
            col.usage,
            col.interface_number,
            col.product_string,
            col.path
        );

        if col.usage_page == protocol::USAGE_PAGE && col.usage == protocol::USAGE {
            if let Some(reading) = read_battery(&api, Some(col)) {
                println!(
                    "  battery: {}% ({}) voltage={:?} mV",
                    reading.percent,
                    reading.state_label(),
                    reading.voltage_mv
                );
            } else if let Some(raw) =
                transact(&col.path, battery_request(), REPORT_LEN, TRANSACTION_DELAY)
            {
                let hex: Vec<String> = raw.iter().map(|b| format!("{b:02X}")).collect();
                println!("  raw response: {}", hex.join(" "));
            } else {
                println!("  battery query failed");
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_prefers_wireless() {
        let wireless = HidCollection {
            path: "a".into(),
            vendor_id: protocol::VID,
            product_id: protocol::PID_WIRELESS,
            product_string: String::new(),
            manufacturer_string: String::new(),
            usage_page: protocol::USAGE_PAGE,
            usage: protocol::USAGE,
            interface_number: 0,
        };
        let wired = HidCollection {
            product_id: protocol::PID_WIRED,
            ..wireless.clone()
        };
        let mut list = vec![wired.clone(), wireless.clone()];
        list.sort_by_key(|c| {
            if c.product_id == protocol::PID_WIRELESS {
                (0, c.product_id)
            } else if c.product_id == protocol::PID_WIRED {
                (1, c.product_id)
            } else {
                (2, c.product_id)
            }
        });
        assert_eq!(list[0].product_id, protocol::PID_WIRELESS);
    }
}
