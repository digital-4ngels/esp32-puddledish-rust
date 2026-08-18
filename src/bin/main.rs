#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use embassy_executor::Spawner;
use embassy_time::{Duration, Instant, Timer};
use esp_hal::clock::CpuClock;
use esp_hal::i2c::master::{Config as I2cConfig, I2c};
use esp_hal::system::Stack;
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use fluidbox::button::Button;
use fluidbox::config::{GRAVITY_GAIN, GRAVITY_MPS2, PX_PER_METER};
use fluidbox::display::Display;
use fluidbox::imu::{self, Imu};
use fluidbox::render::Render;
use fluidbox::sim::{Forces, Sim};
use log::{error, info, warn};
use static_cell::StaticCell;

#[panic_handler]
fn panic(panic_info: &core::panic::PanicInfo) -> ! {
    error!("{}", panic_info);
    loop {}
}

extern crate alloc;

esp_bootloader_esp_idf::esp_app_desc!();

const SIM_STACK_SIZE: usize = 8192;

fn default_forces() -> Forces {
    Forces {
        gravity: [0.0, GRAVITY_GAIN * GRAVITY_MPS2 * PX_PER_METER, 0.0],
        omega: [0.0, 0.0, 0.0],
        alpha: [0.0, 0.0, 0.0],
        speed_cap: 0.0,
    }
}

fn sim_loop(sim: &'static Sim, i2c: &'static mut I2c<'static, esp_hal::Blocking>) {
    let mut imu = Imu::new();
    let mut button = Button::new();
    if !imu.init(i2c) {
        warn!("continuing without motion input");
    }
    if !button.init(i2c) {
        warn!("continuing without the reset button");
    }

    let mut forces = default_forces();
    for _ in 0..10 {
        let _ = imu.read_forces(i2c, 0.01, &mut forces);
    }
    sim.reset(imu.down());
    info!("seed reset along imu_down");

    let mut last = Instant::now();
    let mut last_button = last;
    loop {
        let now = Instant::now();
        let mut dt = (now - last).as_micros() as f32 / 1_000_000.0;
        last = now;
        if dt > 0.05 {
            dt = 0.05;
        } else if dt < 1e-4 {
            dt = 1e-4;
        }
        let _ = imu.read_forces(i2c, dt, &mut forces);
        sim.step(dt, &forces);
        let pause = Instant::now();
        while pause.elapsed() < Duration::from_millis(1) {}

        if (now - last_button).as_millis() >= 25 {
            last_button = now;
            if button.take_short_press(i2c) {
                info!("PWR pressed, resetting fluid");
                sim.reset(imu.down());
            }
        }
    }
}

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(_spawner: Spawner) -> ! {
    esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 73744);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_interrupt =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);

    info!("Embassy initialized");
    info!("FluidBox rust layer 4");

    let mut lcd = Display::init(
        peripherals.SPI2,
        peripherals.DMA_CH0,
        peripherals.GPIO38,
        peripherals.GPIO12,
        peripherals.GPIO39,
        peripherals.GPIO4,
        peripherals.GPIO5,
        peripherals.GPIO6,
        peripherals.GPIO7,
    )
    .await;
    info!("466x466 CO5300 ready");

    let i2c = I2c::new(
        peripherals.I2C0,
        I2cConfig::default().with_frequency(Rate::from_khz(400)),
    )
    .expect("i2c")
    .with_sda(peripherals.GPIO15)
    .with_scl(peripherals.GPIO14);
    static I2C: StaticCell<I2c<'static, esp_hal::Blocking>> = StaticCell::new();
    let i2c = I2C.init(i2c);

    static SIM: Sim = Sim::empty();
    SIM.init();
    let sim: &'static Sim = &SIM;

    static RENDER: StaticCell<Render> = StaticCell::new();
    let render = RENDER.init(Render::empty());
    render.init();

    static mut APP_CORE_STACK: Stack<SIM_STACK_SIZE> = Stack::new();

    esp_rtos::start_second_core(
        peripherals.CPU_CTRL,
        sw_interrupt.software_interrupt1,
        unsafe { &mut *core::ptr::addr_of_mut!(APP_CORE_STACK) },
        move || sim_loop(sim, i2c),
    );

    let mut last_stats = Instant::now();
    let mut last_frames = 0u32;
    let mut last_steps = 0u32;
    let mut frames = 0u32;

    loop {
        render.frame(sim, &mut lcd);
        frames += 1;
        Timer::after(Duration::from_millis(1)).await;

        let elapsed = last_stats.elapsed();
        if elapsed >= Duration::from_secs(2) {
            let secs = elapsed.as_millis() as f32 / 1000.0;
            let steps = sim.steps();
            let st = sim.stats();
            let a = imu::raw_accel();
            info!(
                "{:.1} fps | {:.1} steps/s | rho {:.2}/{:.2} | speed avg {:.0} max {:.0} | cap {:.0} | glass {}/{} | accel {:.2} {:.2} {:.2}",
                (frames - last_frames) as f32 / secs,
                (steps - last_steps) as f32 / secs,
                st.mean_density,
                st.rest_density,
                st.mean_speed,
                st.max_speed,
                imu::speed_cap(),
                st.front_hits,
                st.back_hits,
                a[0],
                a[1],
                a[2]
            );
            last_stats = Instant::now();
            last_frames = frames;
            last_steps = steps;
        }
    }
}
