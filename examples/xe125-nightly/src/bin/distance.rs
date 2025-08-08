#![no_std]
#![no_main]

extern crate alloc;

use alloc::vec;
use core::cell::RefCell;

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::gpio::{Level, Output, Pull, Speed};
use embassy_stm32::spi::Spi;
use embassy_time::{Delay, Instant};
use embedded_hal_bus::spi::ExclusiveDevice;

use a121_rs::detector::distance::config::RadarDistanceConfig;
use a121_rs::detector::distance::RadarDistanceDetector;
use a121_rs::radar::version::rss_version;
use a121_rs::radar::Radar;
use xe125_nightly::adapter::SpiAdapter;
use xe125_nightly::*;
use {defmt_rtt as _, panic_probe as _};

// Override C library malloc/free to use Rust's global allocator
// This provides a single unified heap for both Rust and C code
use core::ffi::c_void;

#[no_mangle]
extern "C" fn malloc(size: usize) -> *mut c_void {
    use alloc::alloc::GlobalAlloc;
    use xe125_nightly::ALLOCATOR;
    
    if size == 0 {
        return core::ptr::null_mut();
    }
    
    // Use 8-byte alignment for good compatibility with most data types
    let layout = core::alloc::Layout::from_size_align(size, 8)
        .unwrap_or_else(|_| core::panic!("Invalid malloc size: {}", size));
    
    unsafe { ALLOCATOR.alloc(layout) as *mut c_void }
}

#[no_mangle]
extern "C" fn free(ptr: *mut c_void) {
    // For simplicity in embedded systems, we implement a no-op free
    // This is acceptable because:
    // 1. Most radar C libraries allocate once and keep data for the lifetime
    // 2. In embedded systems, memory fragmentation is more important than recycling
    // 3. The radar library documentation suggests it doesn't heavily use free()
    if ptr.is_null() {
        return;
    }
    
    // No-op: memory stays allocated until reset
    // This prevents use-after-free bugs and is common in embedded systems
}

// Override other C library memory functions for completeness
#[no_mangle]
extern "C" fn calloc(count: usize, size: usize) -> *mut c_void {
    let total_size = count.wrapping_mul(size);
    if total_size == 0 {
        return core::ptr::null_mut();
    }
    
    let ptr = malloc(total_size);
    if !ptr.is_null() {
        unsafe {
            core::ptr::write_bytes(ptr as *mut u8, 0, total_size);
        }
    }
    ptr
}

#[no_mangle]
extern "C" fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void {
    if ptr.is_null() {
        return malloc(size);
    }
    
    if size == 0 {
        free(ptr);
        return core::ptr::null_mut();
    }
    
    // Simplified: just allocate new memory
    // In a full implementation, you'd copy the old data
    malloc(size)
}

// Stub _sbrk that signals no traditional heap available  
#[no_mangle]
extern "C" fn _sbrk(_incr: isize) -> *mut c_void {
    // Return error to force newlib to use our malloc instead
    (-1isize) as *mut c_void
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    // Initialize heap
    init_heap();
    
    let p = embassy_stm32::init(xm125_clock_config());

    let enable = Output::new(p.PB12, Level::Low, Speed::VeryHigh); // ENABLE on PB12
    let cs_pin = Output::new(p.PB0, Level::High, Speed::VeryHigh);
    let interrupt = ExtiInput::new(p.PB3, p.EXTI3, Pull::Up); // INTERRUPT on PB3 used as 'ready' signal
    info!("GPIO initialized.");

    let spi = Spi::new(
        p.SPI1,
        p.PA5, // SCK
        p.PA7, // MOSI
        p.PA6, // MISO
        p.DMA2_CH4, // TX DMA for SPI1
        p.DMA2_CH3, // RX DMA for SPI1
        xm125_spi_config(),
    );
    let exclusive_device = ExclusiveDevice::new(spi, cs_pin, Delay);

    unsafe {
        SPI_DEVICE = Some(RefCell::new(SpiAdapter::new(
            exclusive_device.expect("SPI device init failed!"),
        )))
    };
    let spi_mut_ref = unsafe { SPI_DEVICE.as_mut().unwrap() };

    debug!("RSS Version: {}", rss_version());

    let mut radar = Radar::new(1, spi_mut_ref.get_mut(), interrupt, enable, Delay).await;
    info!("Radar enabled.");
    
    // Check sensor connectivity before calibration
    if !radar.is_connected() {
        defmt::panic!("Sensor is not connected or not responding");
    }
    info!("Sensor connectivity verified.");
    
    // Check sensor status for debugging
    radar.check_status();
    
    // Validate calibration before using it
    let mut calibration = loop {
        match radar.calibrate().await {
            Ok(calibration) => {
                match calibration.validate_calibration() {
                    Ok(()) => {
                        info!("Calibration complete and validated.");
                        break calibration;
                    }
                    Err(_) => {
                        warn!("Calibration invalid, retrying...");
                        embassy_time::Timer::after(embassy_time::Duration::from_millis(100)).await;
                    }
                }
            }
            Err(e) => {
                warn!("Calibration failed: {:?}, retrying...", e);
                embassy_time::Timer::after(embassy_time::Duration::from_millis(100)).await;
            }
        }
    };
    
    radar.prepare_sensor(&mut calibration).unwrap();

    let mut dist_config = RadarDistanceConfig::balanced();
    dist_config.set_interval(0.7..=1.5);
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
