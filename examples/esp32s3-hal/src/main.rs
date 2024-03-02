#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use a121_rs::config::profile::RadarProfile::AccProfile5;
use a121_rs::radar::{self, Radar};
use embassy_executor::{task, Spawner};
use embassy_time::{Delay, Duration, Timer};
use embedded_alloc::Heap;
use embedded_hal_bus::spi::ExclusiveDevice;
use esp32s3_hal::{
    clock::ClockControl,
    embassy::{self},
    gpio::{self, IO},
    peripherals::Peripherals,
    prelude::*,
    spi::{master::Spi, SpiMode},
    timer::TimerGroup,
};
use esp_backtrace as _;
use esp_println::println;
use tinyrlibc as _;

mod spi_adapter;

static COUNT: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
defmt::timestamp!(
    "{=u32:us}",
    COUNT.fetch_add(1, core::sync::atomic::Ordering::Relaxed)
);

#[global_allocator]
static HEAP: Heap = Heap::empty();

#[main]
async fn main(spawner: Spawner) {
    // Initialize the allocator BEFORE you use it
    {
        use core::mem::MaybeUninit;
        const HEAP_SIZE: usize = 1024 * 32;
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe { HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE) }
    }
    spawner.spawn(init(spawner)).ok();
}

#[task]
async fn init(spawner: Spawner) {
    let p = Peripherals::take();
    let system = p.SYSTEM.split();

    let clocks = ClockControl::max(system.clock_control).freeze();
    let timg0 = TimerGroup::new(p.TIMG0, &clocks);
    embassy::init(&clocks, timg0);

    let io = IO::new(p.GPIO, p.IO_MUX);

    println!("Hello world!");

    println!("{}", a121_rs::radar::rss_version());

    let mut gpio_r_en = io.pins.gpio33.into_push_pull_output();
    let gpio_r_int = io.pins.gpio34.into_pull_down_input();

    let sclk = io.pins.gpio12;
    let miso = io.pins.gpio13;
    let mosi = io.pins.gpio11;
    let cs = io.pins.gpio10;

    let spi_bus = Spi::new(p.SPI2, 1000u32.kHz(), SpiMode::Mode0, &clocks);
    let spi_bus = spi_bus.with_pins(Some(sclk), Some(mosi), Some(miso), gpio::NO_PIN);
    let spi_device = ExclusiveDevice::new_no_delay(spi_bus, cs.into_push_pull_output());
    let spi_device = spi_adapter::SpiAdapter::new(spi_device);
    let spi_device = static_cell::make_static!(spi_device);

    gpio_r_en.set_high().unwrap();
    Timer::after(Duration::from_millis(5)).await;
    let mut radar = Radar::new(1, spi_device, gpio_r_int, gpio_r_en, Delay).await;
    println!("Radar enabled.");
    let mut buffer = [0u8; 2560];
    println!("Starting calibration.");

    let mut calibration = loop {
        buffer.fill(0);
        if let Ok(mut calibration) = radar.calibrate().await {
            if let Ok(()) = calibration.validate_calibration() {
                println!("Calibration is valid");
                break calibration;
            } else {
                println!("Calibration is invalid");
                println!("Calibration result: {:?}", calibration);
                //gpio_r_int.set_low();
            }
        } else {
            println!("Calibration failed");
        }
        Timer::after(Duration::from_millis(1)).await;
    };
    println!("Calibration complete!");
    let mut radar = radar.prepare_sensor(&mut calibration).unwrap();

    loop {
        Timer::after(Duration::from_millis(500)).await;
        let data = radar.measure().await.unwrap();
        println!("Data: {:?}", data);
    }
}
