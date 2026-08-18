//! QMI8658, Zahlen und Map 1:1 aus der C-Firmware `imu.c`.

use crate::config::{
    GRAVITY_GAIN, GRAVITY_LP_HZ, GRAVITY_MPS2, PX_PER_METER, SHAKE_GAIN,
};
use crate::sim::Forces;
use core::sync::atomic::{AtomicU32, Ordering};
use embassy_time::{Duration, Instant};
use esp_hal::i2c::master::I2c;
use esp_hal::Blocking;
use libm::{expf, sqrtf};
use log::{info, warn};

const ADDR_HIGH: u8 = 0x6B;
const ADDR_LOW: u8 = 0x6A;
const REG_WHO: u8 = 0x00;
const REG_CTRL1: u8 = 0x02;
const REG_CTRL2: u8 = 0x03;
const REG_CTRL3: u8 = 0x04;
const REG_CTRL7: u8 = 0x08;
const REG_STATUS0: u8 = 0x2E;
const REG_AX_L: u8 = 0x35;
const REG_RESET: u8 = 0x60;
const RESET_CMD: u8 = 0xB0;
const CTRL1_VALUE: u8 = 0x60;
const ONE_G: f32 = 9.807;
const ACCEL_LSB: f32 = 4096.0;
const GYRO_LSB: f32 = 64.0;
const DEG_TO_RAD: f32 = 0.01745329252;
const TWO_PI: f32 = 6.28318530718;

static RAW: [AtomicU32; 3] = [AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0)];
static SPEED_CAP: AtomicU32 = AtomicU32::new(0);

fn store_raw(v: [f32; 3]) {
    for i in 0..3 {
        RAW[i].store(v[i].to_bits(), Ordering::Relaxed);
    }
}

pub fn raw_accel() -> [f32; 3] {
    [
        f32::from_bits(RAW[0].load(Ordering::Relaxed)),
        f32::from_bits(RAW[1].load(Ordering::Relaxed)),
        f32::from_bits(RAW[2].load(Ordering::Relaxed)),
    ]
}

pub fn speed_cap() -> f32 {
    f32::from_bits(SPEED_CAP.load(Ordering::Relaxed))
}

fn map_x(x: f32, _y: f32, _z: f32) -> f32 {
    -x
}
fn map_y(_x: f32, y: f32, _z: f32) -> f32 {
    -y
}
fn map_z(_x: f32, _y: f32, z: f32) -> f32 {
    z
}

pub struct Imu {
    addr: u8,
    ready: bool,
    lp: [f32; 3],
    lp_primed: bool,
    prev_omega: [f32; 3],
    down: [f32; 3],
    prev_down: [f32; 3],
    act_lp: f32,
}

impl Imu {
    pub fn new() -> Self {
        Self {
            addr: ADDR_HIGH,
            ready: false,
            lp: [0.0; 3],
            lp_primed: false,
            prev_omega: [0.0; 3],
            down: [0.0, 1.0, 0.0],
            prev_down: [0.0, 1.0, 0.0],
            act_lp: 0.0,
        }
    }

    pub fn down(&self) -> [f32; 3] {
        self.down
    }

    pub fn init(&mut self, i2c: &mut I2c<'static, Blocking>) -> bool {
        let mut addr = ADDR_HIGH;
        if !Self::probe(i2c, addr) {
            addr = ADDR_LOW;
            if !Self::probe(i2c, addr) {
                warn!("QMI8658 not found on I2C");
                return false;
            }
        }
        self.addr = addr;
        if !self.write(i2c, REG_RESET, RESET_CMD) {
            warn!("QMI8658 reset failed");
            return false;
        }
        let wait = Instant::now();
        while wait.elapsed() < Duration::from_millis(20) {}

        if !self.write(i2c, REG_CTRL1, CTRL1_VALUE) {
            return false;
        }
        // 8g | 250 Hz
        if !self.write(i2c, REG_CTRL2, (0x02 << 4) | 0x05) {
            return false;
        }
        // 512 dps | 250 Hz
        if !self.write(i2c, REG_CTRL3, (0x04 << 4) | 0x05) {
            return false;
        }
        if !self.write(i2c, REG_CTRL7, 0x01 | 0x02) {
            return false;
        }
        info!("QMI8658 ready at 0x{addr:02x}");
        self.ready = true;
        true
    }

    fn probe(i2c: &mut I2c<'static, Blocking>, addr: u8) -> bool {
        let mut who = [0u8];
        i2c.write_read(addr, &[REG_WHO], &mut who).is_ok()
    }

    fn write(&self, i2c: &mut I2c<'static, Blocking>, reg: u8, val: u8) -> bool {
        i2c.write(self.addr, &[reg, val]).is_ok()
    }

    fn read(&self, i2c: &mut I2c<'static, Blocking>, reg: u8, buf: &mut [u8]) -> bool {
        i2c.write_read(self.addr, &[reg], buf).is_ok()
    }

    pub fn read_forces(&mut self, i2c: &mut I2c<'static, Blocking>, dt: f32, out: &mut Forces) -> bool {
        if !self.ready {
            return false;
        }
        let mut status = [0u8];
        if !self.read(i2c, REG_STATUS0, &mut status) || (status[0] & 0x03) == 0 {
            return false;
        }
        let mut buf = [0u8; 12];
        if !self.read(i2c, REG_AX_L, &mut buf) {
            return false;
        }
        let raw_ax = i16::from_le_bytes([buf[0], buf[1]]) as f32;
        let raw_ay = i16::from_le_bytes([buf[2], buf[3]]) as f32;
        let raw_az = i16::from_le_bytes([buf[4], buf[5]]) as f32;
        let raw_gx = i16::from_le_bytes([buf[6], buf[7]]) as f32;
        let raw_gy = i16::from_le_bytes([buf[8], buf[9]]) as f32;
        let raw_gz = i16::from_le_bytes([buf[10], buf[11]]) as f32;

        let ax0 = (raw_ax * ONE_G) / ACCEL_LSB;
        let ay0 = (raw_ay * ONE_G) / ACCEL_LSB;
        let az0 = (raw_az * ONE_G) / ACCEL_LSB;
        store_raw([ax0, ay0, az0]);

        let ax = map_x(ax0, ay0, az0);
        let ay = map_y(ax0, ay0, az0);
        let az = map_z(ax0, ay0, az0);
        let gx = map_x(raw_gx / GYRO_LSB, raw_gy / GYRO_LSB, raw_gz / GYRO_LSB) * DEG_TO_RAD;
        let gy = map_y(raw_gx / GYRO_LSB, raw_gy / GYRO_LSB, raw_gz / GYRO_LSB) * DEG_TO_RAD;
        let gz = map_z(raw_gx / GYRO_LSB, raw_gy / GYRO_LSB, raw_gz / GYRO_LSB) * DEG_TO_RAD;

        if !self.lp_primed {
            self.lp = [ax, ay, az];
            self.lp_primed = true;
        } else {
            let k = 1.0 - expf(-TWO_PI * GRAVITY_LP_HZ * dt);
            self.lp[0] += k * (ax - self.lp[0]);
            self.lp[1] += k * (ay - self.lp[1]);
            self.lp[2] += k * (az - self.lp[2]);
        }

        let mut dx = -self.lp[0];
        let mut dy = -self.lp[1];
        let mut dz = -self.lp[2];
        let mag = sqrtf(dx * dx + dy * dy + dz * dz);
        if mag > 0.5 {
            dx /= mag;
            dy /= mag;
            dz /= mag;
        } else {
            dx = 0.0;
            dy = 0.0;
            dz = 1.0;
        }
        self.down = [dx, dy, dz];

        let g_px = GRAVITY_GAIN * GRAVITY_MPS2 * PX_PER_METER;
        let shake_px = SHAKE_GAIN * PX_PER_METER;
        let sx = ax - self.lp[0];
        let sy = ay - self.lp[1];
        let sz = az - self.lp[2];
        out.gravity[0] = dx * g_px - sx * shake_px;
        out.gravity[1] = dy * g_px - sy * shake_px;
        out.gravity[2] = dz * g_px - sz * shake_px;
        let shake = sqrtf(sx * sx + sy * sy + sz * sz);
        let ddown = sqrtf(
            (dx - self.prev_down[0]) * (dx - self.prev_down[0])
                + (dy - self.prev_down[1]) * (dy - self.prev_down[1])
                + (dz - self.prev_down[2]) * (dz - self.prev_down[2]),
        );
        self.prev_down = [dx, dy, dz];
        let spin = sqrtf(gx * gx + gy * gy + gz * gz);
        let shake_n = if shake > 0.35 { shake } else { 0.0 };
        let ddown_n = if ddown > 0.02 { ddown } else { 0.0 };
        let spin_n = if spin > 0.45 { spin } else { 0.0 };
        let activity = shake_n + 10.0 * ddown_n + 0.9 * spin_n;
        let k = if activity > self.act_lp { 0.55 } else { 0.10 };
        self.act_lp += k * (activity - self.act_lp);
        let cap = 240.0 + 2600.0 * self.act_lp;
        out.speed_cap = cap;
        SPEED_CAP.store(cap.to_bits(), Ordering::Relaxed);
        // Gyro off: leftover omega/alpha rührt die Dose dauernd um (Spritzer nach oben).
        // Kippen/Schütteln bleiben über Accel. C-Rotation später, wenn ruhig.
        let _ = (gx, gy, gz);
        out.omega = [0.0, 0.0, 0.0];
        out.alpha = [0.0, 0.0, 0.0];
        self.prev_omega = [0.0, 0.0, 0.0];
        true
    }
}
