//! C-Solver (`native/sim.c`) 1:1 aus der Firmware, `-O2 -ffast-math`.

use core::sync::atomic::{AtomicU32, Ordering};

use crate::config::PARTICLE_MAX;
use esp_sync::RawMutex;
use log::info;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ParticleView {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub speed: f32,
}

impl ParticleView {
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        speed: 0.0,
    };
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Forces {
    pub gravity: [f32; 3],
    pub omega: [f32; 3],
    pub alpha: [f32; 3],
    pub speed_cap: f32,
}

#[derive(Clone, Copy)]
pub struct Stats {
    pub mean_density: f32,
    pub rest_density: f32,
    pub mean_speed: f32,
    pub max_speed: f32,
    pub pairs: u32,
    pub front_hits: i32,
    pub back_hits: i32,
    pub us_grid: u32,
    pub us_density: u32,
    pub us_relax: u32,
}

#[repr(C)]
struct CStats {
    mean_density: f32,
    rest_density: f32,
    mean_speed: f32,
    max_speed: f32,
    us_grid: i32,
    us_density: i32,
    us_relax: i32,
    front_density: f32,
    back_density: f32,
    front_speed: f32,
    back_speed: f32,
    front_push: f32,
    back_push: f32,
    front_count: i32,
    back_count: i32,
    front_hits: i32,
    back_hits: i32,
    clamped: i32,
    pairs: i32,
}

unsafe extern "C" {
    fn sim_init();
    fn sim_reset();
    fn sim_step(dt_real: f32, forces: *const Forces);
    fn sim_snapshot(out: *mut ParticleView, max: i32) -> i32;
    fn sim_stats(out: *mut CStats);
    fn sim_set_down(x: f32, y: f32, z: f32);
}

pub struct Sim {
    _lock: RawMutex,
    steps: AtomicU32,
}

unsafe impl Sync for Sim {}

impl Sim {
    pub const fn empty() -> Self {
        Self {
            _lock: RawMutex::new(),
            steps: AtomicU32::new(0),
        }
    }

    pub fn init(&self) {
        unsafe {
            sim_set_down(0.0, 1.0, 0.0);
            sim_init();
        }
        let st = self.stats();
        info!(
            "C solver rest density {:.3}",
            st.rest_density
        );
    }

    pub fn reset(&self, down: [f32; 3]) {
        unsafe {
            sim_set_down(down[0], down[1], down[2]);
            sim_reset();
        }
    }

    pub fn step(&self, dt_real: f32, forces: &Forces) {
        unsafe {
            sim_step(dt_real, forces);
        }
        self.steps.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self, out: &mut [ParticleView]) -> usize {
        let n = out.len().min(PARTICLE_MAX) as i32;
        unsafe { sim_snapshot(out.as_mut_ptr(), n) as usize }
    }

    pub fn stats(&self) -> Stats {
        let mut c = CStats {
            mean_density: 0.0,
            rest_density: 0.0,
            mean_speed: 0.0,
            max_speed: 0.0,
            us_grid: 0,
            us_density: 0,
            us_relax: 0,
            front_density: 0.0,
            back_density: 0.0,
            front_speed: 0.0,
            back_speed: 0.0,
            front_push: 0.0,
            back_push: 0.0,
            front_count: 0,
            back_count: 0,
            front_hits: 0,
            back_hits: 0,
            clamped: 0,
            pairs: 0,
        };
        unsafe {
            sim_stats(&mut c);
        }
        Stats {
            mean_density: c.mean_density,
            rest_density: c.rest_density,
            mean_speed: c.mean_speed,
            max_speed: c.max_speed,
            pairs: c.pairs as u32,
            front_hits: c.front_hits,
            back_hits: c.back_hits,
            us_grid: c.us_grid as u32,
            us_density: c.us_density as u32,
            us_relax: c.us_relax as u32,
        }
    }

    pub fn steps(&self) -> u32 {
        self.steps.load(Ordering::Relaxed)
    }
}
