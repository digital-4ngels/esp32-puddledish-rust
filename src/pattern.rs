//! Layer-1 Kreuz + Ring. Zahlen 1:1 aus der C-Firmware `pattern.c`.
//! Pixelmitten von 0..465 sitzen auf 232.5. Radius 232.5 = eingeschriebener Kreis.

use crate::display::LCD_H;
use libm::hypotf;

const CX: f32 = 232.5;
const CY: f32 = 232.5;
const R_MAX: f32 = 232.5;
const RING_MID: f32 = R_MAX - 2.0;
const RING_HALF: f32 = 1.6;
const CROSS_LIMIT: f32 = RING_MID + RING_HALF;

fn rgb565(r: i32, g: i32, b: i32) -> u16 {
    let c = (((r & 0xF8) << 8) | ((g & 0xFC) << 3) | (b >> 3)) as u16;
    c.rotate_left(8)
}

pub fn draw_band(band: &mut [u16], y0: i32, rows: i32) {
    let white = rgb565(255, 255, 255);
    band.fill(0);

    for row in 0..rows {
        let y = y0 + row;
        let dst_off = row as usize * LCD_H as usize;
        let fy = y as f32 + 0.5;

        for x in 0..LCD_H as i32 {
            let fx = x as f32 + 0.5;
            let r = hypotf(fx - CX, fy - CY);
            if r > R_MAX {
                continue;
            }

            let on_cross = (libm::fabsf(fx - CX) < 1.5 || libm::fabsf(fy - CY) < 1.5)
                && r <= CROSS_LIMIT;
            let on_ring = libm::fabsf(r - RING_MID) <= RING_HALF;
            if on_cross || on_ring {
                band[dst_off + x as usize] = white;
            }
        }
    }
}
