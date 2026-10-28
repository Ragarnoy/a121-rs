#![no_std]
#![no_main]

extern crate alloc;

use a121_rs::detector::distance::RadarDistanceDetector;
use a121_rs::memory::sdk_heap_stats;
use a121_rs::radar::version::rss_version;
use a121_rs::radar::Radar;
use a121_rs::AlignedBuffer;
use defmt::{info, warn};
use embassy_executor::Spawner;
use embassy_stm32::exti::{self, ExtiInput};
use embassy_stm32::gpio::{Level, Output, Pull, Speed};
use embassy_stm32::mode::Blocking;
use embassy_stm32::rcc::{
    LsConfig, Pll, PllMul, PllPDiv, PllPreDiv, PllQDiv, PllRDiv, PllSource, Sysclk,
};
use embassy_stm32::spi::mode::Master;
use embassy_stm32::spi::{Config, Spi};
use embassy_stm32::time::Hertz;
use embassy_stm32::{bind_interrupts, interrupt};
use embassy_time::{Delay, Duration, Timer};
use embedded_hal_bus::spi::ExclusiveDevice;
use static_cell::StaticCell;
use talc::source::Claim;
use talc::TalcLock;
use {defmt_rtt as _, panic_probe as _};

// Heap for the SDK and the detector buffers
const HEAP_SIZE: usize = 32 * 1024;

#[global_allocator]
static ALLOCATOR: TalcLock<spin::Mutex<()>, Claim> = TalcLock::new(unsafe {
    static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];
    Claim::array(&raw mut HEAP)
});

type RadarSpi = ExclusiveDevice<Spi<'static, Blocking, Master>, Output<'static>, Delay>;

bind_interrupts!(struct Irqs {
    EXTI9_5 => exti::InterruptHandler<interrupt::typelevel::EXTI9_5>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Starting");
    let p = embassy_stm32::init(clock_config());

    let enable = Output::new(p.PC2, Level::Low, Speed::VeryHigh);
    let cs_pin = Output::new(p.PA11, Level::High, Speed::VeryHigh);
    // Sensor select lines of the XE121: all low selects sensor 1
    let _sel0 = Output::new(p.PC3, Level::Low, Speed::VeryHigh);
    let _sel1 = Output::new(p.PA1, Level::Low, Speed::VeryHigh);
    let _sel2 = Output::new(p.PA0, Level::Low, Speed::VeryHigh);
    let interrupt = ExtiInput::new(p.PB6, p.EXTI6, Pull::Up, Irqs);
    info!("GPIO initialized");

    let spi = Spi::new_blocking(
        p.SPI2,
        p.PB13, // SCK
        p.PB15, // MOSI
        p.PB14, // MISO
        spi_config(),
    );
    static SPI_DEVICE: StaticCell<RadarSpi> = StaticCell::new();
    let spi_device =
        SPI_DEVICE.init(ExclusiveDevice::new(spi, cs_pin, Delay).expect("SPI device init failed"));
    info!("SPI initialized");

    info!("RSS Version: {}", rss_version());

    let mut radar = Radar::new(1, spi_device, interrupt, enable, Delay)
        .await
        .unwrap();
    info!("Radar enabled");
    let mut calibration = loop {
        match radar.calibrate().await {
            Ok(calibration) if calibration.validate_calibration().is_ok() => break calibration,
            Ok(calibration) => warn!("Calibration is invalid: {:?}", calibration),
            Err(e) => warn!("Calibration failed: {:?}", e),
        }
        Timer::after(Duration::from_millis(1)).await;
    };
    info!("Calibration complete");
    radar.prepare_sensor(&mut calibration).unwrap();

    let mut distance = RadarDistanceDetector::new(&mut radar).unwrap();
    // Buffer sizes depend on the detector configuration, so ask the SDK
    let sizes = distance.memory_requirements().unwrap();
    info!("Detector buffers: {}", sizes);
    let mut buffer = AlignedBuffer::new(sizes.buffer);
    let mut static_cal_result = AlignedBuffer::new(sizes.static_calibration);
    let mut dynamic_cal_result = distance
        .calibrate_detector(&calibration, &mut buffer, &mut static_cal_result)
        .await
        .unwrap();
    info!("SDK heap after detector calibration: {}", sdk_heap_stats());

    loop {
        Timer::after(Duration::from_millis(200)).await;
        distance
            .prepare_detector(&calibration, &mut buffer)
            .unwrap();
        distance.measure(&mut buffer).await.unwrap();

        match distance.process_data(&mut buffer, &mut static_cal_result, &mut dynamic_cal_result) {
            Ok(res) => {
                if res.num_distances() > 0 {
                    info!(
                        "{} Distances found:\n{:?}",
                        res.num_distances(),
                        res.distances()
                    );
                }
                if res.calibration_needed() {
                    info!("Calibration needed");
                    calibration = distance.calibrate().await.unwrap();
                    dynamic_cal_result = distance
                        .update_calibration(&calibration, &mut buffer)
                        .await
                        .unwrap();
                }
            }
            Err(_) => warn!("Failed to process data"),
        }
    }
}

fn spi_config() -> Config {
    let mut spi_config = Config::default();
    spi_config.frequency = Hertz(1_000_000);
    spi_config
}

fn clock_config() -> embassy_stm32::Config {
    let mut config = embassy_stm32::Config::default();
    config.rcc.hsi = true;
    config.rcc.hse = None;
    config.rcc.msi = None;
    config.rcc.sys = Sysclk::PLL1_R;
    config.rcc.pll = Some(Pll {
        source: PllSource::HSI,
        prediv: PllPreDiv::DIV1,
        mul: PllMul::MUL10,
        divp: Some(PllPDiv::DIV7),
        divq: Some(PllQDiv::DIV2),
        divr: Some(PllRDiv::DIV2),
    });
    config.rcc.pllsai1 = Some(Pll {
        source: PllSource::HSI,
        prediv: PllPreDiv::DIV1,
        mul: PllMul::MUL8,
        divp: Some(PllPDiv::DIV7),
        divq: Some(PllQDiv::DIV2),
        divr: Some(PllRDiv::DIV2),
    });
    config.rcc.ls = LsConfig::default_lsi();
    config
}
