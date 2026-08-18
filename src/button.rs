//! PWR über TCA9554 0x20, nur Bit 4 lesen. Andere Pins nicht umbiegen.

use embassy_time::Instant;
use esp_hal::i2c::master::I2c;
use esp_hal::Blocking;
use log::info;

const ADDR: u8 = 0x20;
const REG_INPUT: u8 = 0x00;
const REG_CONFIG: u8 = 0x03;
const PWR_BIT: u8 = 1 << 4;
const DEBOUNCE: u8 = 2;
const SHORT_MAX_MS: u64 = 1500;

pub struct Button {
    stable: bool,
    agree: u8,
    press_start: Instant,
    pending: bool,
}

impl Button {
    pub fn new() -> Self {
        Self {
            stable: false,
            agree: 0,
            press_start: Instant::from_ticks(0),
            pending: false,
        }
    }

    pub fn init(&mut self, i2c: &mut I2c<'static, Blocking>) -> bool {
        let mut config = [0xFFu8];
        if i2c.write_read(ADDR, &[REG_CONFIG], &mut config).is_err() {
            return false;
        }
        if (config[0] & PWR_BIT) == 0 {
            let next = config[0] | PWR_BIT;
            if i2c.write(ADDR, &[REG_CONFIG, next]).is_err() {
                return false;
            }
        }
        let mut input = [0u8];
        let _ = i2c.write_read(ADDR, &[REG_INPUT], &mut input);
        info!("PWR button ready on EXIO4 (config 0x{:02x}, input 0x{:02x})", config[0], input[0]);
        true
    }

    pub fn take_short_press(&mut self, i2c: &mut I2c<'static, Blocking>) -> bool {
        let mut input = [0u8];
        if i2c.write_read(ADDR, &[REG_INPUT], &mut input).is_err() {
            return false;
        }
        let pressed = (input[0] & PWR_BIT) != 0;
        if pressed == self.stable {
            self.agree = 0;
        } else {
            self.agree += 1;
            if self.agree >= DEBOUNCE {
                self.agree = 0;
                self.stable = pressed;
                if pressed {
                    self.press_start = Instant::now();
                } else {
                    let held = self.press_start.elapsed().as_millis();
                    if held <= SHORT_MAX_MS {
                        self.pending = true;
                    }
                }
            }
        }
        if self.pending {
            self.pending = false;
            true
        } else {
            false
        }
    }
}
