#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

extern crate alloc;

use core::mem::MaybeUninit;
use esp_backtrace as _;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer, Delay, Instant};
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_hal::{
    clock::ClockControl,
    embassy::{self},
    peripherals::Peripherals,
    prelude::*,
    timer::TimerGroup,
    gpio::{self, IO},
    spi::{master::Spi, SpiMode},
};
mod spi_adapter;
use a121_rs::radar::Radar;

extern crate tinyrlibc; // this provides malloc and free via the global allocator

#[global_allocator]
static ALLOCATOR: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

fn init_heap() {
    const HEAP_SIZE: usize = 32 * 1024;
    static mut HEAP: MaybeUninit<[u8; HEAP_SIZE]> = MaybeUninit::uninit();

    unsafe {
        ALLOCATOR.init(HEAP.as_mut_ptr() as *mut u8, HEAP_SIZE);
    }
}

// #[embassy_executor::task]
// async fn run() {
//     loop {
//         esp_println::println!("Hello world from embassy using esp-hal-async!");
//         Timer::after(Duration::from_millis(1_000)).await;
//     }
// }

#[main]
async fn main(spawner: Spawner) {
    init_heap();
    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();
    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);

    let clocks = ClockControl::max(system.clock_control).freeze();
    let timg0 = TimerGroup::new(peripherals.TIMG0, &clocks);
    embassy::init(&clocks, timg0);

    // setup logger
    // To change the log_level change the env section in .cargo/config.toml
    // or remove it and set ESP_LOGLEVEL manually before running cargo run
    // this requires a clean rebuild because of https://github.com/rust-lang/cargo/issues/10358
    esp_println::logger::init_logger_from_env();

    log::info!("A121 library version: {}", a121_rs::radar::rss_version());

    //spawner.spawn(run()).ok();

    // XE121
    /*
        #define GPIO_SEL0 4
        #define GPIO_SEL1 3
        #define GPIO_SEL2 2
        #define GPIO_ENABLE 0

        #define GPIO_INTERRUPT 23
        #define GPIO_SCLK 19
        #define GPIO_MOSI 21
        #define GPIO_MISO 20
        #define GPIO_CS   22
     */

    // this is only required for the XE121
    let mut sel0 = io.pins.gpio4.into_push_pull_output();
    let mut sel1 = io.pins.gpio3.into_push_pull_output();
    let mut sel2 = io.pins.gpio2.into_push_pull_output();
    sel0.set_low().unwrap();
    sel1.set_low().unwrap();
    sel2.set_low().unwrap();

    let radar_en = io.pins.gpio0.into_push_pull_output();
    let radar_int = io.pins.gpio23.into_pull_down_input();

    let sclk = io.pins.gpio19;
    let miso = io.pins.gpio20;
    let mosi = io.pins.gpio21;
    let cs = io.pins.gpio22;

    let spi_bus = Spi::new(peripherals.SPI2, 1u32.MHz(), SpiMode::Mode0, &clocks);
    let spi_bus = spi_bus.with_pins(Some(sclk), Some(mosi), Some(miso), gpio::NO_PIN);
    let spi_device = ExclusiveDevice::new_no_delay(spi_bus, cs.into_push_pull_output());
    let spi_device = spi_adapter::SpiAdapter::new(spi_device);
    let spi_device = static_cell::make_static!(spi_device);

    let mut radar = Radar::new(1, spi_device, radar_int, radar_en, Delay).await;

    log::info!("Radar enabled.");
    log::info!("Starting calibration...");
    let mut calibration = radar.calibrate().await.unwrap();
    let mut radar = radar.prepare_sensor(&mut calibration).unwrap();
    log::info!("Radar calibrated and prepared.");
}
