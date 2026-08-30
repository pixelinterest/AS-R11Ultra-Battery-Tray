//! Transparent tray icons with colored percent text (pixel glyphs).

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tray_icon::Icon;
use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};
use winreg::RegKey;

use crate::device::BatteryReading;

/// Tray percent glyph scale (32x32 canvas, logitray-style).
pub const TEXT_SIZE: usize = 32;

// Charging percent color as RGB (light/dark taskbar themes).
pub const CHARGING_COLOR_LIGHT: [u8; 3] = [21, 101, 192];
pub const CHARGING_COLOR_DARK: [u8; 3] = [52, 152, 219];

const ICON_CACHE_MAX: usize = 48;
const THEME_TTL: Duration = Duration::from_secs(5);

static ICON_CACHE: Mutex<Option<HashMap<IconKey, Vec<u8>>>> = Mutex::new(None);
static THEME_CACHE: Mutex<Option<(Instant, bool)>> = Mutex::new(None);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct IconKey {
    text: [u8; 4],
    text_len: u8,
    rgba: [u8; 4],
}

impl IconKey {
    fn new(text: &str, rgba: [u8; 4]) -> Self {
        let mut buf = [0u8; 4];
        let bytes = text.as_bytes();
        let len = bytes.len().min(4) as u8;
        buf[..len as usize].copy_from_slice(&bytes[..len as usize]);
        Self {
            text: buf,
            text_len: len,
            rgba,
        }
    }
}

fn is_light_mode() -> bool {
    let now = Instant::now();
    if let Ok(guard) = THEME_CACHE.lock() {
        if let Some((cached_at, light)) = *guard {
            if now.duration_since(cached_at) < THEME_TTL {
                return light;
            }
        }
    }

    let light = read_light_theme().unwrap_or(false);
    if let Ok(mut guard) = THEME_CACHE.lock() {
        *guard = Some((now, light));
    }
    light
}

fn read_light_theme() -> Option<bool> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu
        .open_subkey_with_flags(
            r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize",
            KEY_READ,
        )
        .ok()?;
    let val: u32 = key.get_value("SystemUsesLightTheme").ok()?;
    Some(val == 1)
}

fn text_color(percent: Option<u8>, charging: bool, light: bool) -> [u8; 4] {
    let rgb = match percent {
        None => {
            if light {
                [70, 70, 70]
            } else {
                [170, 170, 170]
            }
        }
        Some(_p) if charging => {
            if light {
                CHARGING_COLOR_LIGHT
            } else {
                CHARGING_COLOR_DARK
            }
        }
        Some(p) if p >= 50 => {
            if light {
                [30, 140, 60]
            } else {
                [46, 204, 113]
            }
        }
        Some(p) if p >= 20 => {
            if light {
                [211, 84, 0]
            } else {
                [230, 126, 34]
            }
        }
        Some(_) => {
            if light {
                [192, 57, 43]
            } else {
                [231, 76, 60]
            }
        }
    };
    [rgb[0], rgb[1], rgb[2], 255]
}

pub fn make_icon(reading: Option<&BatteryReading>) -> Result<Icon, tray_icon::BadIcon> {
    let light = is_light_mode();
    let (text, color) = match reading {
        None => ("??".to_string(), text_color(None, false, light)),
        Some(r) => (
            r.percent.to_string(),
            text_color(Some(r.percent), r.charging, light),
        ),
    };

    let key = IconKey::new(&text, color);
    if let Ok(mut guard) = ICON_CACHE.lock() {
        let cache = guard.get_or_insert_with(HashMap::new);
        if let Some(pixels) = cache.get(&key) {
            return Icon::from_rgba(pixels.clone(), TEXT_SIZE as u32, TEXT_SIZE as u32);
        }
        let pixels = render_pixels(&text, color);
        if cache.len() >= ICON_CACHE_MAX {
            cache.clear();
        }
        cache.insert(key, pixels.clone());
        return Icon::from_rgba(pixels, TEXT_SIZE as u32, TEXT_SIZE as u32);
    }

    let pixels = render_pixels(&text, color);
    Icon::from_rgba(pixels, TEXT_SIZE as u32, TEXT_SIZE as u32)
}

fn render_pixels(text: &str, color: [u8; 4]) -> Vec<u8> {
    let mut pixels = vec![0u8; TEXT_SIZE * TEXT_SIZE * 4];
    draw_number(&mut pixels, text, color);
    pixels
}

fn digit_glyph(d: u8) -> [u8; 5] {
    match d {
        0 => [0b111, 0b101, 0b101, 0b101, 0b111],
        1 => [0b010, 0b110, 0b010, 0b010, 0b111],
        2 => [0b111, 0b001, 0b111, 0b100, 0b111],
        3 => [0b111, 0b001, 0b111, 0b001, 0b111],
        4 => [0b101, 0b101, 0b111, 0b001, 0b001],
        5 => [0b111, 0b100, 0b111, 0b001, 0b111],
        6 => [0b111, 0b100, 0b111, 0b101, 0b111],
        7 => [0b111, 0b001, 0b001, 0b001, 0b001],
        8 => [0b111, 0b101, 0b111, 0b101, 0b111],
        9 => [0b111, 0b101, 0b111, 0b001, 0b111],
        _ => [0; 5],
    }
}

fn draw_number(pixels: &mut [u8], label: &str, color: [u8; 4]) {
    let outline = [25, 25, 25, 235];
    let n = label.len();
    if n == 0 {
        return;
    }

    let units_w = 3 * n + n.saturating_sub(1);
    let scale = ((TEXT_SIZE - 2) / units_w).clamp(1, (TEXT_SIZE - 2) / 5);
    let total_w = units_w * scale;
    let total_h = 5 * scale;
    let x0 = (TEXT_SIZE - total_w) / 2;
    let y0 = (TEXT_SIZE - total_h) / 2;

    let mut mask = vec![false; TEXT_SIZE * TEXT_SIZE];
    for (gi, ch) in label.bytes().enumerate() {
        let glyph = digit_glyph(ch.wrapping_sub(b'0'));
        let gx = x0 + gi * 4 * scale;
        for (row, bits) in glyph.iter().enumerate() {
            for col in 0..3 {
                if bits & (0b100 >> col) != 0 {
                    for dy in 0..scale {
                        for dx in 0..scale {
                            let x = gx + col * scale + dx;
                            let y = y0 + row * scale + dy;
                            if x < TEXT_SIZE && y < TEXT_SIZE {
                                mask[y * TEXT_SIZE + x] = true;
                            }
                        }
                    }
                }
            }
        }
    }

    for y in 0..TEXT_SIZE {
        for x in 0..TEXT_SIZE {
            if mask[y * TEXT_SIZE + x] {
                continue;
            }
            let mut touches = false;
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx >= 0
                        && ny >= 0
                        && (nx as usize) < TEXT_SIZE
                        && (ny as usize) < TEXT_SIZE
                        && mask[ny as usize * TEXT_SIZE + nx as usize]
                    {
                        touches = true;
                    }
                }
            }
            if touches {
                put_pixel(pixels, x, y, outline);
            }
        }
    }

    for y in 0..TEXT_SIZE {
        for x in 0..TEXT_SIZE {
            if mask[y * TEXT_SIZE + x] {
                put_pixel(pixels, x, y, color);
            }
        }
    }
}

fn put_pixel(pixels: &mut [u8], x: usize, y: usize, rgba: [u8; 4]) {
    if x >= TEXT_SIZE || y >= TEXT_SIZE {
        return;
    }
    let idx = (y * TEXT_SIZE + x) * 4;
    pixels[idx..idx + 4].copy_from_slice(&rgba);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_generation_works() {
        assert!(make_icon(None).is_ok());
        let reading = BatteryReading {
            percent: 100,
            wired: false,
            charging: false,
            voltage_mv: None,
            vendor_id: 0x3554,
            product_id: 0xFB44,
            product_string: String::new(),
        };
        assert!(make_icon(Some(&reading)).is_ok());
        let reading95 = BatteryReading {
            percent: 95,
            wired: true,
            charging: true,
            ..reading
        };
        assert!(make_icon(Some(&reading95)).is_ok());
    }

    #[test]
    fn charging_color_constants_in_range() {
        for rgb in [CHARGING_COLOR_LIGHT, CHARGING_COLOR_DARK] {
            assert!(rgb.iter().all(|&c| c <= 255));
        }
    }
}
