#![no_std]
#![no_main]

use a121_rs::detector::distance::RadarDistanceDetector;
use a121_rs::radar::Radar;
use a121_rs::AlignedBuffer;
use embassy_executor::Spawner;
use embassy_time::{Delay, Duration, Instant};
use embedded_hal_bus::spi::{ExclusiveDevice, NoDelay};
use esp_backtrace as _;
use esp_hal::{
    clock::CpuClock,
    gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull},
    spi::{
        master::{Config as SpiConfig, Spi},
        Mode,
    },
    time::Rate,
    timer::timg::TimerGroup,
    Blocking,
};
use esp_println as _;
use static_cell::StaticCell;

// The ESP-IDF second stage bootloader requires an application descriptor
esp_bootloader_esp_idf::esp_app_desc!();

type SpiDevice = ExclusiveDevice<Spi<'static, Blocking>, Output<'static>, NoDelay>;

// `Radar::new` keeps the SPI device for the lifetime of the program
static SPI_DEVICE: StaticCell<SpiDevice> = StaticCell::new();

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    let peripherals = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::max()));

    // The Acconeer SDK allocates its buffers through the global allocator
    esp_alloc::heap_allocator!(size: 32 * 1024);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, peripherals.FROM_CPU_INTR0);

    defmt::info!(
        "A121 library version: {}",
        a121_rs::radar::version::rss_version()
    );

    let radar_en = Output::new(peripherals.GPIO33, Level::Low, OutputConfig::default());
    let radar_int = Input::new(
        peripherals.GPIO34,
        InputConfig::default().with_pull(Pull::Down),
    );

    let spi_bus = Spi::new(
        peripherals.SPI2,
        SpiConfig::default()
            .with_frequency(Rate::from_mhz(40))
            .with_mode(Mode::_0),
    )
    .unwrap()
    .with_sck(peripherals.GPIO12)
    .with_miso(peripherals.GPIO13)
    .with_mosi(peripherals.GPIO11);
    let cs = Output::new(peripherals.GPIO10, Level::High, OutputConfig::default());
    let spi_device = SPI_DEVICE.init(ExclusiveDevice::new_no_delay(spi_bus, cs).unwrap());

    spawner.spawn(init(spi_device, radar_int, radar_en).unwrap());
}

/// Runs the radar: calibration, then the measure/process loop
#[embassy_executor::task]
async fn init(
    spi_device: &'static mut SpiDevice,
    radar_int: Input<'static>,
    radar_en: Output<'static>,
) {
    let mut radar = Radar::new(1, spi_device, radar_int, radar_en, Delay)
        .await
        .unwrap();

    defmt::info!("Radar enabled.");
    defmt::info!("Starting calibration...");
    let mut calibration = radar.calibrate().await.unwrap();
    radar.prepare_sensor(&mut calibration).unwrap();
    defmt::info!("Radar calibrated and prepared.");

    let mut distance = RadarDistanceDetector::new(&mut radar).unwrap();
    let mut buffer = AlignedBuffer::new(distance.get_distance_buffer_size().unwrap());
    let mut static_cal_result =
        AlignedBuffer::new(distance.get_static_result_buffer_size().unwrap());
    defmt::info!("Starting detector calibration...");
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
                    defmt::info!(
                        "{} Distances found:\n{:?}",
                        res.num_distances(),
                        res.distances()
                    );
                }
                if res.calibration_needed() {
                    defmt::info!("Calibration needed.");
                    calibration = distance.calibrate().await.unwrap();
                    dynamic_cal_result = distance
                        .update_calibration(&calibration, &mut buffer)
                        .await
                        .unwrap();
                }
            }
            Err(_) => defmt::error!("Failed to process data."),
        }

        if Instant::now() - last_print >= Duration::from_secs(1) {
            defmt::info!(
                "[Measurement frames]:[Frames with at least 1 distance]:[Total distances] per second: \n {}:{}:{}",
                frames,
                measurements,
                distances
            );
            frames = 0;
            measurements = 0;
            distances = 0;
            last_print = Instant::now();
        }
    }
}
