#![no_std]
#![no_main]

extern crate alloc;

use alloc::vec;
use core::cell::RefCell;

use defmt::{info, trace, warn};
use embassy_executor::Spawner;
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::gpio::{Input, Level, Output, Pull, Speed};
use embassy_stm32::spi::Spi;
use embassy_time::{Delay, Instant};
use embedded_hal_bus::spi::ExclusiveDevice;

use a121_rs::detector::distance::config::RadarDistanceConfig;
use a121_rs::detector::distance::RadarDistanceDetector;
use a121_rs::radar;
use a121_rs::radar::Radar;
use radar::rss_version;
use xe125_nightly::adapter::SpiAdapter;
use xe125_nightly::*;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(xm125_clock_config());

    let enable = Output::new(p.PB12, Level::Low, Speed::VeryHigh); // ENABLE on PB12
    let cs_pin = Output::new(p.PB0, Level::High, Speed::VeryHigh);
    let input = Input::new(p.PB3, Pull::Up);
    let interrupt = ExtiInput::new(input, p.EXTI3); // INTERRUPT on PB3 used as 'ready' signal
    info!("GPIO initialized.");

    let spi = Spi::new(
        p.SPI1,
        p.PA5, // SCK
        p.PA7, // MOSI
        p.PA6, // MISO
        p.DMA2_CH3,
        p.DMA2_CH2,
        xm125_spi_config(),
    );
    let exclusive_device = ExclusiveDevice::new(spi, cs_pin, Delay);

    unsafe { SPI_DEVICE = Some(RefCell::new(SpiAdapter::new(exclusive_device))) };
    let spi_mut_ref = unsafe { SPI_DEVICE.as_mut().unwrap() };

    info!("RSS Version: {}", rss_version());

    let mut radar = Radar::new(1, spi_mut_ref.get_mut(), interrupt, enable, Delay).await;
    info!("Radar enabled.");
    let mut calibration = radar.calibrate().await.unwrap();
    info!("Calibration complete.");
    let mut radar = radar.prepare_sensor(&mut calibration).unwrap();

    let mut dist_config = RadarDistanceConfig::balanced();
    dist_config.set_interval(4.0..=5.5);
    let mut distance = RadarDistanceDetector::with_config(&mut radar, dist_config);
    let mut buffer = vec![0u8; distance.get_distance_buffer_size()];
    let mut static_cal_result = vec![0u8; distance.get_static_result_buffer_size()];
    trace!("Calibrating detector...");
    let mut dynamic_cal_result = distance
        .calibrate_detector(&calibration, &mut buffer, &mut static_cal_result)
        .await
        .unwrap();

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
                    let calibration = distance.calibrate().await.unwrap();
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
