#![no_std]
#![no_main]

extern crate alloc;

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::exti::{self, ExtiInput};
use embassy_stm32::gpio::{Level, Output, Pull, Speed};
use embassy_stm32::spi::Spi;
use embassy_stm32::{bind_interrupts, dma, interrupt, peripherals};
use embassy_time::{Delay, Instant};
use embedded_hal_bus::spi::ExclusiveDevice;
use static_cell::StaticCell;

use a121_rs::detector::distance::config::RadarDistanceConfig;
use a121_rs::detector::distance::RadarDistanceDetector;
use a121_rs::memory::sdk_heap_stats;
use a121_rs::radar::version::rss_version;
use a121_rs::radar::Radar;
use a121_rs::AlignedBuffer;
use xe125::*;
use {defmt_rtt as _, panic_probe as _};

// Makes the linker take sinf, cosf and tanf from newlib's libm, linked ahead of compiler_builtins
// by build.rs, see there
unsafe extern "C" {
    fn sinf(x: f32) -> f32;
    fn cosf(x: f32) -> f32;
    fn tanf(x: f32) -> f32;
}
#[used]
static NEWLIB_TRIG: [unsafe extern "C" fn(f32) -> f32; 3] = [sinf, cosf, tanf];

bind_interrupts!(struct Irqs {
    EXTI3 => exti::InterruptHandler<interrupt::typelevel::EXTI3>;
    DMA2_CHANNEL3 => dma::InterruptHandler<peripherals::DMA2_CH3>;
    DMA2_CHANNEL4 => dma::InterruptHandler<peripherals::DMA2_CH4>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(xm125_clock_config());

    let enable = Output::new(p.PB12, Level::Low, Speed::VeryHigh); // ENABLE on PB12
    let cs_pin = Output::new(p.PB0, Level::High, Speed::VeryHigh);
    let interrupt = ExtiInput::new(p.PB3, p.EXTI3, Pull::Up, Irqs); // INTERRUPT on PB3 used as 'ready' signal
    info!("GPIO initialized.");

    let spi = Spi::new(
        p.SPI1,
        p.PA5,      // SCK
        p.PA7,      // MOSI
        p.PA6,      // MISO
        p.DMA2_CH4, // TX DMA for SPI1
        p.DMA2_CH3, // RX DMA for SPI1
        Irqs,
        xm125_spi_config(),
    );
    static SPI_DEVICE: StaticCell<RadarSpi> = StaticCell::new();
    let spi_device =
        SPI_DEVICE.init(ExclusiveDevice::new(spi, cs_pin, Delay).expect("SPI device init failed!"));

    debug!("RSS Version: {}", rss_version());

    let mut radar = Radar::new(1, spi_device, interrupt, enable, Delay)
        .await
        .unwrap();
    info!("Radar enabled.");

    // Check sensor connectivity before calibration
    if !radar.is_connected() {
        defmt::panic!("Sensor is not connected or not responding");
    }
    info!("Sensor connectivity verified.");

    // Check sensor status for debugging
    radar.check_status().unwrap();

    // Validate calibration before using it
    let mut calibration = loop {
        match radar.calibrate().await {
            Ok(calibration) => match calibration.validate_calibration() {
                Ok(()) => {
                    info!("Calibration complete and validated.");
                    break calibration;
                }
                Err(_) => {
                    warn!("Calibration invalid, retrying...");
                    embassy_time::Timer::after(embassy_time::Duration::from_millis(100)).await;
                }
            },
            Err(e) => {
                warn!("Calibration failed: {:?}, retrying...", e);
                embassy_time::Timer::after(embassy_time::Duration::from_millis(100)).await;
            }
        }
    };

    radar.prepare_sensor(&mut calibration).unwrap();

    let mut dist_config = RadarDistanceConfig::balanced().unwrap();
    dist_config.set_interval(0.7..=1.5);
    let mut distance = RadarDistanceDetector::with_config(&mut radar, dist_config).unwrap();
    // Buffer sizes depend on the detector configuration, so ask the SDK
    let sizes = distance.memory_requirements().unwrap();
    let mut buffer = AlignedBuffer::new(sizes.buffer);
    let mut static_cal_result = AlignedBuffer::new(sizes.static_calibration);
    trace!("Calibrating detector...");
    let mut dynamic_cal_result = distance
        .calibrate_detector(&calibration, &mut buffer, &mut static_cal_result)
        .await
        .unwrap();
    info!("SDK heap after detector calibration: {}", sdk_heap_stats());
    let mut heap_logged = false;

    let mut counter = 0;
    let mut counter_d = 0;
    let mut counter_dt = 0;
    let mut last_print = Instant::now();

    loop {
        distance
            .prepare_detector(&calibration, &mut buffer)
            .unwrap();
        distance.measure(&mut buffer).await.unwrap();

        match distance.process_data(&mut buffer, &mut static_cal_result, &mut dynamic_cal_result) {
            Ok(res) => {
                if !heap_logged {
                    // The peak covers sensor and detector calibration and the first frame
                    info!("SDK heap after the first frame: {}", sdk_heap_stats());
                    heap_logged = true;
                }
                counter += 1;
                if res.num_distances() > 0 {
                    counter_d += 1;
                    counter_dt += res.num_distances();
                    info!(
                        "{} Distances found:\n{:?}",
                        res.num_distances(),
                        res.distances()
                    );
                }
                if res.calibration_needed() {
                    info!("Calibration needed.");
                    calibration = distance.calibrate().await.unwrap();
                    dynamic_cal_result = distance
                        .update_calibration(&calibration, &mut buffer)
                        .await
                        .unwrap();
                }
            }
            Err(_) => warn!("Failed to process data."),
        }

        if Instant::now() - last_print >= embassy_time::Duration::from_secs(1) {
            info!(
                "[Measurement frames]:[Frames with at least 1 distance]:[Total distances] per second: \n {}:{}:{}",
                counter, counter_d, counter_dt
            );
            counter = 0;
            counter_d = 0;
            counter_dt = 0;
            last_print = Instant::now();
        }
    }
}
