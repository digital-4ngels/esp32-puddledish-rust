//! Zahlen 1:1 aus der C-Firmware `config.h`. Nicht „verbessern“.

pub const LCD_H_RES: u16 = 466;
pub const LCD_V_RES: u16 = 466;
pub const LCD_COL_OFFSET: u16 = 6;
pub const BAND_ROWS: usize = 22;
pub const BAND_COUNT: usize = (LCD_V_RES as usize + BAND_ROWS - 1) / BAND_ROWS;

pub const BOX_W: f32 = LCD_H_RES as f32;
pub const BOX_H: f32 = LCD_V_RES as f32;
pub const BOX_D: f32 = 75.0;
pub const BOX_CORNER_R: f32 = BOX_W * 0.5;

pub const PX_PER_METER: f32 = 12677.0;
pub const PX_PER_MM: f32 = PX_PER_METER / 1000.0;
pub const BOX_BACK_FILLET_MM: f32 = 2.0;
pub const BOX_BACK_FILLET: f32 = BOX_BACK_FILLET_MM * PX_PER_MM;
pub const BOX_FRONT_FILLET: f32 = BOX_BACK_FILLET * 0.25;

pub const PARTICLE_MAX: usize = 1000;
pub const PARTICLE_COUNT: usize = 900;
pub const REST_SPACING: f32 = 16.0;

pub const TIME_SCALE: f32 = 0.068;
pub const GRAVITY_LP_HZ: f32 = 1.2;
pub const SHAKE_GAIN: f32 = 3.0;
pub const SIM_DT_MAX: f32 = 0.0022;
pub const GRAVITY_MPS2: f32 = 9.81;
pub const GRAVITY_GAIN: f32 = 1.8;
pub const SMOOTH_RADIUS: f32 = 28.0;
pub const SUBSTEPS: i32 = 1;
pub const K_PRESSURE: f32 = 400_000.0;
pub const K_NEAR_PRESSURE: f32 = 800_000.0;
pub const MAX_DISPLACEMENT: f32 = 4.0;
pub const WALL_JITTER: f32 = 0.0;
pub const VISC_SIGMA: f32 = 45.0;
pub const VISC_BETA: f32 = 0.03;
pub const WALL_RESTITUTION: f32 = 0.0;
pub const WALL_FRICTION: f32 = 0.96;
pub const ROTATION_GAIN: f32 = 1.0;

pub const GRID_CX: i32 = 16;
pub const GRID_CY: i32 = 16;
pub const GRID_CZ: i32 = 2;
pub const GRID_CELLS: usize = (GRID_CX * GRID_CY * GRID_CZ) as usize;
pub const WALL_MARGIN: f32 = 5.0;
pub const PAIR_MAX: usize = 24576;

pub const PROJ_FOCAL: f32 = 100.0;
pub const PARTICLE_RADIUS_PX: f32 = 7.2;
pub const DISC_MAX_R: usize = 10;
pub const HIGHLIGHT_ENABLE: bool = true;
pub const HIGHLIGHT_LIFT: f32 = 0.55;
pub const SPEED_LEVELS: usize = 64;
pub const DEPTH_LEVELS: usize = 16;
pub const SPEED_COLOR_MAX: f32 = 5000.0;
pub const SPEED_COLOR_GAMMA: f32 = 0.55;
pub const DEPTH_DIM_MIN: f32 = 0.32;

pub const SCREEN_ROTATE_90_CW: bool = true;
