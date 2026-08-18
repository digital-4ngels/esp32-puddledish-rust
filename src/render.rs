//! Renderer 1:1 aus der C-Firmware `render.c`.

use crate::config::{
    BAND_COUNT, BAND_ROWS, BOX_D, BOX_H, BOX_W, DEPTH_DIM_MIN, DEPTH_LEVELS, DISC_MAX_R,
    HIGHLIGHT_ENABLE, HIGHLIGHT_LIFT, LCD_H_RES, LCD_V_RES, PARTICLE_MAX, PARTICLE_RADIUS_PX,
    PROJ_FOCAL, SCREEN_ROTATE_90_CW, SPEED_COLOR_GAMMA, SPEED_COLOR_MAX, SPEED_LEVELS,
};
use crate::display::Display;
use crate::sim::{ParticleView, Sim};
use libm::{powf, sqrtf};

const RAMP_STOPS: usize = 4;
const STOP_T: [f32; RAMP_STOPS] = [0.00, 0.45, 0.78, 1.00];
const STOP_C: [[i32; 3]; RAMP_STOPS] = [
    [10, 45, 165],
    [40, 125, 235],
    [150, 205, 250],
    [255, 255, 255],
];

fn swap16(v: u16) -> u16 {
    v.rotate_left(8)
}

fn rgb565(mut r: i32, mut g: i32, mut b: i32) -> u16 {
    r = r.clamp(0, 255);
    g = g.clamp(0, 255);
    b = b.clamp(0, 255);
    (((r & 0xF8) << 8) | ((g & 0xFC) << 3) | (b >> 3)) as u16
}

fn speed_color(t: f32) -> (i32, i32, i32) {
    if t <= 0.0 {
        return (STOP_C[0][0], STOP_C[0][1], STOP_C[0][2]);
    }
    if t >= 1.0 {
        let last = STOP_C[RAMP_STOPS - 1];
        return (last[0], last[1], last[2]);
    }
    for s in 0..RAMP_STOPS - 1 {
        if t <= STOP_T[s + 1] {
            let f = (t - STOP_T[s]) / (STOP_T[s + 1] - STOP_T[s]);
            let r = STOP_C[s][0] as f32 + f * (STOP_C[s + 1][0] - STOP_C[s][0]) as f32;
            let g = STOP_C[s][1] as f32 + f * (STOP_C[s + 1][1] - STOP_C[s][1]) as f32;
            let b = STOP_C[s][2] as f32 + f * (STOP_C[s + 1][2] - STOP_C[s][2]) as f32;
            return (r as i32, g as i32, b as i32);
        }
    }
    let last = STOP_C[RAMP_STOPS - 1];
    (last[0], last[1], last[2])
}

fn project(x: f32, y: f32, z: f32) -> (i32, i32, f32) {
    let s = PROJ_FOCAL / (PROJ_FOCAL + z);
    let px = (BOX_W * 0.5) + (x - BOX_W * 0.5) * s;
    let py = (BOX_H * 0.5) + (y - BOX_H * 0.5) * s;
    if SCREEN_ROTATE_90_CW {
        ((BOX_W - py + 0.5) as i32, (px + 0.5) as i32, s)
    } else {
        ((px + 0.5) as i32, (py + 0.5) as i32, s)
    }
}

pub struct Render {
    color_lut: [u16; DEPTH_LEVELS * SPEED_LEVELS],
    highlight_lut: [u16; DEPTH_LEVELS * SPEED_LEVELS],
    disc_span: [[u8; 2 * DISC_MAX_R + 1]; DISC_MAX_R + 1],
    band_used: [bool; BAND_COUNT],
    band_used_prev: [bool; BAND_COUNT],
    snapshot: [ParticleView; PARTICLE_MAX],
    sx: [i16; PARTICLE_MAX],
    sy: [i16; PARTICLE_MAX],
    sr: [u8; PARTICLE_MAX],
    sc: [u16; PARTICLE_MAX],
    sh: [u16; PARTICLE_MAX],
}

impl Render {
    pub const fn empty() -> Self {
        Self {
            color_lut: [0; DEPTH_LEVELS * SPEED_LEVELS],
            highlight_lut: [0; DEPTH_LEVELS * SPEED_LEVELS],
            disc_span: [[0; 2 * DISC_MAX_R + 1]; DISC_MAX_R + 1],
            band_used: [false; BAND_COUNT],
            band_used_prev: [false; BAND_COUNT],
            snapshot: [ParticleView::ZERO; PARTICLE_MAX],
            sx: [0; PARTICLE_MAX],
            sy: [0; PARTICLE_MAX],
            sr: [0; PARTICLE_MAX],
            sc: [0; PARTICLE_MAX],
            sh: [0; PARTICLE_MAX],
        }
    }

    pub fn init(&mut self) {
        self.build_color_lut();
        self.build_disc_spans();
        self.band_used_prev.fill(true);
    }

    fn build_color_lut(&mut self) {
        for d in 0..DEPTH_LEVELS {
            let depth_t = if DEPTH_LEVELS > 1 {
                d as f32 / (DEPTH_LEVELS - 1) as f32
            } else {
                0.0
            };
            let dim = DEPTH_DIM_MIN + (1.0 - DEPTH_DIM_MIN) * (1.0 - depth_t);

            for s in 0..SPEED_LEVELS {
                let linear = if SPEED_LEVELS > 1 {
                    s as f32 / (SPEED_LEVELS - 1) as f32
                } else {
                    0.0
                };
                let speed_t = powf(linear, SPEED_COLOR_GAMMA);
                let (r, g, b) = speed_color(speed_t);
                let idx = d * SPEED_LEVELS + s;
                self.color_lut[idx] = swap16(rgb565(
                    (r as f32 * dim) as i32,
                    (g as f32 * dim) as i32,
                    (b as f32 * dim) as i32,
                ));
                let lift = HIGHLIGHT_LIFT;
                let hr = ((r as f32 + (255.0 - r as f32) * lift) * dim) as i32;
                let hg = ((g as f32 + (255.0 - g as f32) * lift) * dim) as i32;
                let hb = ((b as f32 + (255.0 - b as f32) * lift) * dim) as i32;
                self.highlight_lut[idx] = swap16(rgb565(hr, hg, hb));
            }
        }
    }

    fn build_disc_spans(&mut self) {
        for r in 0..=DISC_MAX_R {
            let rr = r as i32;
            for dy in -rr..=rr {
                let w = sqrtf((rr * rr - dy * dy) as f32);
                self.disc_span[r][(dy + rr) as usize] = (w + 0.5) as u8;
            }
        }
    }

    fn project_all(&mut self, n: usize) {
        let speed_scale = (SPEED_LEVELS - 1) as f32 / SPEED_COLOR_MAX;
        let depth_scale = (DEPTH_LEVELS - 1) as f32 / BOX_D;
        self.band_used.fill(false);

        for i in 0..n {
            let p = self.snapshot[i];
            let (sx, sy, scale) = project(p.x, p.y, p.z);
            self.sx[i] = sx as i16;
            self.sy[i] = sy as i16;

            let mut r = (PARTICLE_RADIUS_PX * scale + 0.5) as i32;
            if r < 1 {
                r = 1;
            }
            if r > DISC_MAX_R as i32 {
                r = DISC_MAX_R as i32;
            }
            self.sr[i] = r as u8;

            let mut sl = (p.speed * speed_scale) as i32;
            sl = sl.clamp(0, SPEED_LEVELS as i32 - 1);
            let mut dl = (p.z * depth_scale + 0.5) as i32;
            dl = dl.clamp(0, DEPTH_LEVELS as i32 - 1);
            let lut = (dl as usize) * SPEED_LEVELS + sl as usize;
            self.sc[i] = self.color_lut[lut];
            self.sh[i] = self.highlight_lut[lut];

            let top = sy - r;
            let bot = sy + r;
            if bot < 0 || top >= LCD_V_RES as i32 {
                continue;
            }
            let b0 = if top < 0 { 0 } else { top as usize / BAND_ROWS };
            let b1 = if bot >= LCD_V_RES as i32 {
                BAND_COUNT - 1
            } else {
                bot as usize / BAND_ROWS
            };
            for b in b0..=b1 {
                self.band_used[b] = true;
            }
        }
    }

    fn draw_disc(
        buf: &mut [u16],
        band_y0: i32,
        rows: i32,
        cx: i32,
        cy: i32,
        mut r: i32,
        color: u16,
        spans: &[[u8; 2 * DISC_MAX_R + 1]; DISC_MAX_R + 1],
    ) {
        if r < 1 {
            r = 1;
        }
        if r > DISC_MAX_R as i32 {
            r = DISC_MAX_R as i32;
        }
        let span = &spans[r as usize];
        let mut dy0 = -r;
        let mut dy1 = r;
        if cy + dy0 < band_y0 {
            dy0 = band_y0 - cy;
        }
        if cy + dy1 >= band_y0 + rows {
            dy1 = band_y0 + rows - 1 - cy;
        }
        let width = LCD_H_RES as i32;
        for dy in dy0..=dy1 {
            let hw = span[(dy + r) as usize] as i32;
            let mut x0 = cx - hw;
            let mut x1 = cx + hw;
            if x0 < 0 {
                x0 = 0;
            }
            if x1 >= width {
                x1 = width - 1;
            }
            if x0 > x1 {
                continue;
            }
            let row = (cy + dy - band_y0) as usize * LCD_H_RES as usize;
            buf[row + x0 as usize..=row + x1 as usize].fill(color);
        }
    }

    pub fn frame(&mut self, sim: &Sim, lcd: &mut Display<'_>) {
        let n = sim.snapshot(&mut self.snapshot);
        self.project_all(n);

        let mut buf_i = 0usize;
        for b in 0..BAND_COUNT {
            if !self.band_used[b] && !self.band_used_prev[b] {
                continue;
            }
            let band_y0 = (b * BAND_ROWS) as i32;
            let mut rows = BAND_ROWS as i32;
            if band_y0 + rows > LCD_V_RES as i32 {
                rows = LCD_V_RES as i32 - band_y0;
            }
            let band_y1 = band_y0 + rows;
            let slice = &mut lcd.band_pixels(buf_i)[0..LCD_H_RES as usize * rows as usize];
            slice.fill(0);

            if self.band_used[b] {
                for i in (0..n).rev() {
                    let r = self.sr[i] as i32;
                    let cy = self.sy[i] as i32;
                    if cy + r < band_y0 || cy - r >= band_y1 {
                        continue;
                    }
                    let sx = self.sx[i] as i32;
                    Self::draw_disc(
                        slice,
                        band_y0,
                        rows,
                        sx,
                        cy,
                        r,
                        self.sc[i],
                        &self.disc_span,
                    );
                    if HIGHLIGHT_ENABLE && r >= 3 {
                        Self::draw_disc(
                            slice,
                            band_y0,
                            rows,
                            sx - r / 3,
                            cy - r / 3,
                            r / 2,
                            self.sh[i],
                            &self.disc_span,
                        );
                    }
                }
            }

            lcd.start_flush(band_y0 as u16, rows as u16, buf_i);
            buf_i ^= 1;
        }

        lcd.finish_frame();
        self.band_used_prev.copy_from_slice(&self.band_used);
    }
}
