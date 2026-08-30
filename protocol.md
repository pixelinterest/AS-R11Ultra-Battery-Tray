# R11 Ultra HID battery protocol

Shared Compx / Nordic 52840 protocol (same as ATK / VXE / Zaopin “nordic52”).

## Device IDs

| Mode | VID | PID | Notes |
|------|-----|-----|-------|
| Wireless (dongle) | `0x3554` | `0xFB44` | ATTACK SHARK Mouse (Compx) |
| Wired (USB-C) | `0x3554` | `0xFB43` | R11Ultra (Compx) |

Both modes share Compx VID `0x3554`. Battery collection: usage page `0xFF02`, usage `0x0002`.

Verified wireless and wired: Nordic52 query returns percent, charging flag, and voltage.

## Battery query

17-byte interrupt report, report ID `0x08`, sub-command `0x04`.

Request (hex):

```
08 04 00 00 00 00 00 00 00 00 00 00 00 00 00 00 49
```

All 17 bytes sum to `0x55` mod 256 (checksum in last byte).

Response layout:

| Offset | Meaning |
|--------|---------|
| 0 | Report ID `0x08` |
| 1 | Sub-command `0x04` |
| 6 | Battery percent (0–100) |
| 7 | Wired / charging flag (`0` wireless, `1` cable) |
| 8–9 | Cell voltage mV (big-endian, optional) |
| 16 | Checksum (all 17 bytes sum to `0x55` mod 256) |

Parsers reject frames that fail the checksum, report id/sub-command, or percent range checks.
