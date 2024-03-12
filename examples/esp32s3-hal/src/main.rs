#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

extern crate alloc;

use a121_rs::detector::distance::RadarDistanceDetector;
use a121_rs::radar::Radar;
use alloc::vec;
use embassy_executor::{task, Spawner};
use embassy_time::{Delay, Instant};
use embedded_alloc::Heap;
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_backtrace as _;
use esp_hal::{
    clock::ClockControl,
    embassy::{self},
    gpio::{self, IO},
    peripherals::Peripherals,
    prelude::*,
    spi::{master::Spi, SpiMode},
    timer::TimerGroup,
};
use esp_println::println;
mod spi_adapter;

extern crate tinyrlibc;

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
async fn init(_spawner: Spawner) {
    let p = Peripherals::take();
    let system = p.SYSTEM.split();

    let clocks = ClockControl::max(system.clock_control).freeze();
    let timg0 = TimerGroup::new(p.TIMG0, &clocks);
    embassy::init(&clocks, timg0);

    let io = IO::new(p.GPIO, p.IO_MUX);

    println!("{}", a121_rs::radar::rss_version());

    let gpio_r_en = io.pins.gpio33.into_push_pull_output();
    let gpio_r_int = io.pins.gpio34.into_pull_down_input();

    let sclk = io.pins.gpio12;
    let miso = io.pins.gpio13;
    let mosi = io.pins.gpio11;
    let cs = io.pins.gpio10;

    let spi_bus = Spi::new(p.SPI2, 40u32.MHz(), SpiMode::Mode0, &clocks);
    let spi_bus = spi_bus.with_pins(Some(sclk), Some(mosi), Some(miso), gpio::NO_PIN);
    let spi_device = ExclusiveDevice::new_no_delay(spi_bus, cs.into_push_pull_output());
    let spi_device = spi_adapter::SpiAdapter::new(spi_device);
    let spi_device = static_cell::make_static!(spi_device);

    //gpio_r_en.set_high().unwrap();
    //Timer::after(Duration::from_millis(5)).await;
    let mut radar = Radar::new(1, spi_device, gpio_r_int, gpio_r_en, Delay).await;
    println!("Radar enabled.");
    println!("Starting calibration...");
    let mut calibration = radar.calibrate().await.unwrap();
    let mut radar = radar.prepare_sensor(&mut calibration).unwrap();
    println!("Radar calibrated and prepared.");
    let mut distance = RadarDistanceDetector::new(&mut radar);
    let mut buffer = vec![0u8; distance.get_distance_buffer_size()];
    let mut static_cal_result = vec![0u8; distance.get_static_result_buffer_size()];
    println!("Starting detector calibration...");
    let mut dynamic_cal_result = distance
        .calibrate_detector(&calibration, &mut buffer, &mut static_cal_result)
        .await
        .unwrap();

    let mut frames = 0;
    let mut measurements = 0;
    let mut distances = 0;
    let mut last_print = Instant::now();

    loop {
        distance
            .prepare_detector(&calibration, &mut buffer)
            .unwrap();
        distance.measure(&mut buffer).await.unwrap();

        match distance.process_data(&mut buffer, &mut static_cal_result, &mut dynamic_cal_result) {
            Ok(res) => {
                frames += 1;
                if res.num_distances() > 0 {
                    measurements += 1;
                    distances += res.num_distances();
                    println!(
                        "{} Distances found:\n{:?}",
                        res.num_distances(),
                        res.distances()
                    );
                }
                if res.calibration_needed() {
                    println!("Calibration needed.");
                    let calibration = distance.calibrate().await.unwrap();
                    dynamic_cal_result = distance
                        .update_calibration(&calibration, &mut buffer)
                        .await
                        .unwrap();
                }
            }
            Err(_) => println!("Failed to process data."),
        }

        if Instant::now() - last_print >= embassy_time::Duration::from_secs(1) {
            println!(
                "[Measurement frames]:[Frames with at least 1 distance]:[Total distances] per second: \n {}:{}:{}",
                frames, measurements, distances
            );
            frames = 0;
            measurements = 0;
            distances = 0;
            last_print = Instant::now();
        }
    }
}
