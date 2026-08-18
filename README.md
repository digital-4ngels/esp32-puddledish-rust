# Puddle Dish — Rust-Port 1.75-B

Puddle Dish in Rust (`esp-hal` 1.1 + Embassy) auf Waveshare **ESP32-S3-Touch-AMOLED-1.75-B** (rund 466×466, CO5300).

Nicht das 1.8er. C-Port (`../esp32-puddledish-c3/firmware`) bleibt Feel-SoT. `orb-rust` bleibt unberührt. Original-Clone: `../johannes-fluidbox/repo/`.

## Schicht

1. Panel: Kreis, Kreuz + Ring — ok
2. Tote Perlen — ok
3. Solver — ok (C-sim, TIME 0.048 still, spritzt noch)
4. **jetzt** — IMU + PWR, TIME 0.068

## Build / Flash

PowerShell: zuerst `C:\Users\d4\export-esp.ps1` (Xtensa). `;` nicht `&&`.

```
cd C:\Users\d4\Code\ESP32\esp32-puddledish-rust
cargo build --release
espflash flash --port COM3 --chip esp32s3 --monitor target\xtensa-esp32s3-none-elf\release\fluidbox
```

USB-Serial ist **jtag-serial** (VID 303A), nicht UART0. COM3, MAC zuletzt `28:84:85:57:40:8c`.

## Pins

```
PCLK 38  CS 12  RST 39
D0–D3 = 4,5,6,7
QSPI 40 MHz, gap x=6
I2C SDA 15 / SCL 14   (ab Schicht 4)
PWR TCA9554 0x20 Bit 4 nur lesen
BOOT GPIO0 unberührt
```

Streifen 22 Zeilen, letzter 4 (466 = 21×22+4). RGB565 byte-swapped.
