//! Compx / Nordic 52840 battery HID protocol constants and parsing.

use std::time::Duration;

pub const VID: u16 = 0x3554;
pub const PID_WIRELESS: u16 = 0xFB44;
pub const PID_WIRED: u16 = 0xFB43;

pub const USAGE_PAGE: u16 = 0xFF02;
pub const USAGE: u16 = 0x0002;

pub const REPORT_ID: u8 = 0x08;
pub const SUBCOMMAND_BATTERY: u8 = 0x04;
pub const REPORT_LEN: usize = 17;
pub const CHECKSUM_MOD: u8 = 0x55;

pub const PERCENT_OFFSET: usize = 6;
pub const WIRED_FLAG_OFFSET: usize = 7;
pub const VOLTAGE_MSB_OFFSET: usize = 8;
pub const VOLTAGE_LSB_OFFSET: usize = 9;

pub const POLL_INTERVAL_SEC: u64 = 30;
pub const STALE_READING_SEC: u64 = 300;

pub const POLL_INTERVAL: Duration = Duration::from_secs(POLL_INTERVAL_SEC);
pub const STALE_READING: Duration = Duration::from_secs(STALE_READING_SEC);
pub const TRANSACTION_DELAY: Duration = Duration::from_millis(100);

/// Fixed request: report id 0x08, subcmd 0x04, checksum 0x49 (bytes sum to 0x55).
const BATTERY_REQUEST: [u8; REPORT_LEN] = {
    let mut buf = [0u8; REPORT_LEN];
    buf[0] = REPORT_ID;
    buf[1] = SUBCOMMAND_BATTERY;
    buf[16] = 0x49;
    buf
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatteryData {
    pub percent: u8,
    pub wired: bool,
    pub charging: bool,
    pub voltage_mv: Option<u16>,
}

pub fn battery_request() -> &'static [u8; REPORT_LEN] {
    &BATTERY_REQUEST
}

pub fn checksum_ok(data: &[u8]) -> bool {
    if data.len() < REPORT_LEN {
        return false;
    }
    data[..REPORT_LEN].iter().map(|b| u16::from(*b)).sum::<u16>() % 256
        == u16::from(CHECKSUM_MOD)
}

pub fn parse_battery_response(data: &[u8]) -> Option<BatteryData> {
    if data.len() < REPORT_LEN || !checksum_ok(data) {
        return None;
    }
    if data[0] != REPORT_ID || data[1] != SUBCOMMAND_BATTERY {
        return None;
    }
    let percent = data[PERCENT_OFFSET];
    if percent > 100 {
        return None;
    }
    let wired = data[WIRED_FLAG_OFFSET] != 0;
    let voltage_mv =
        u16::from(data[VOLTAGE_MSB_OFFSET]) << 8 | u16::from(data[VOLTAGE_LSB_OFFSET]);
    let voltage_mv = if voltage_mv == 0 { None } else { Some(voltage_mv) };
    Some(BatteryData {
        percent,
        wired,
        charging: wired && percent < 100,
        voltage_mv,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(percent: u8, wired: u8, voltage_mv: u16, bad_checksum: bool) -> Vec<u8> {
        let mut buf = vec![0u8; REPORT_LEN];
        buf[0] = REPORT_ID;
        buf[1] = SUBCOMMAND_BATTERY;
        buf[PERCENT_OFFSET] = percent;
        buf[WIRED_FLAG_OFFSET] = wired;
        buf[VOLTAGE_MSB_OFFSET] = ((voltage_mv >> 8) & 0xFF) as u8;
        buf[VOLTAGE_LSB_OFFSET] = (voltage_mv & 0xFF) as u8;
        let sum = buf[..16].iter().map(|b| u16::from(*b)).sum::<u16>() % 256;
        buf[16] = ((256 + u16::from(CHECKSUM_MOD) - sum) % 256) as u8;
        if bad_checksum {
            buf[16] = buf[16].wrapping_add(1);
        }
        buf
    }

    #[test]
    fn request_checksum() {
        let req = battery_request();
        assert_eq!(req.len(), REPORT_LEN);
        assert!(checksum_ok(req));
        assert_eq!(req, battery_request());
    }

    #[test]
    fn parse_wireless_full() {
        let data = parse_battery_response(&frame(100, 0, 4193, false)).unwrap();
        assert_eq!(data.percent, 100);
        assert!(!data.wired);
        assert!(!data.charging);
        assert_eq!(data.voltage_mv, Some(4193));
    }

    #[test]
    fn parse_charging() {
        let data = parse_battery_response(&frame(95, 1, 4100, false)).unwrap();
        assert_eq!(data.percent, 95);
        assert!(data.wired);
        assert!(data.charging);
    }

    #[test]
    fn parse_wired_full_not_charging() {
        let data = parse_battery_response(&frame(100, 1, 0, false)).unwrap();
        assert!(!data.charging);
    }

    #[test]
    fn rejects_bad_checksum() {
        assert!(parse_battery_response(&frame(80, 0, 0, true)).is_none());
    }

    #[test]
    fn rejects_bad_percent() {
        let mut buf = frame(50, 0, 0, false);
        buf[PERCENT_OFFSET] = 150;
        buf[16] = ((256 + u16::from(CHECKSUM_MOD)
            - (buf[..16].iter().map(|b| u16::from(*b)).sum::<u16>() % 256))
            % 256) as u8;
        assert!(parse_battery_response(&buf).is_none());
    }

    #[test]
    fn rejects_wrong_report() {
        let mut buf = frame(50, 0, 0, false);
        buf[0] = 0x09;
        buf[16] = ((256 + u16::from(CHECKSUM_MOD)
            - (buf[..16].iter().map(|b| u16::from(*b)).sum::<u16>() % 256))
            % 256) as u8;
        assert!(parse_battery_response(&buf).is_none());
    }

    #[test]
    fn rejects_short() {
        assert!(parse_battery_response(&[1, 2, 3]).is_none());
    }
}
