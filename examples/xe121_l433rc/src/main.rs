#![no_std]
#![no_main]

use core::cell::RefCell;

use a121_rs::detector::distance::RadarDistanceDetector;
use a121_rs::radar;
use a121_rs::radar::Radar;
use defmt::{info, warn};
use embassy_executor::Spawner;
use embassy_stm32::dma::NoDma;
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::gpio::{Input, Level, Output, Pull, Speed};
use embassy_stm32::peripherals::{PA11, SPI2};
use embassy_stm32::rcc::{
    ClockSrc, LsConfig, Pll, PllMul, PllPDiv, PllPreDiv, PllQDiv, PllRDiv, PllSource,
};
use embassy_stm32::spi::{Config, Spi};
use embassy_stm32::time::Hertz;
use embassy_time::{Delay, Duration, Timer};
use embedded_hal_bus::spi::ExclusiveDevice;
use num::complex::Complex32;
use radar::rss_version;
use talc::{ClaimOnOom, Span, Talc, Talck};
use {defmt_rtt as _, panic_probe as _};

use crate::adapter::SpiAdapter;

mod adapter;
pub mod io;

static mut ARENA: [u8; 10000] = [0; 10000];

#[global_allocator]
static ALLOCATOR: Talck<spin::Mutex<()>, ClaimOnOom> = Talc::new(unsafe {
    // if we're in a hosted environment, the Rust runtime may allocate before
    // main() is called, so we need to initialize the arena automatically
    ClaimOnOom::new(Span::from_const_array(core::ptr::addr_of!(ARENA)))
})
.lock();

type SpiDeviceMutex =
    ExclusiveDevice<Spi<'static, SPI2, NoDma, NoDma>, Output<'static, PA11>, Delay>;
static mut SPI_DEVICE: Option<RefCell<SpiAdapter<SpiDeviceMutex>>> = None;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Starting");
    let p = embassy_stm32::init(xm125_clock_config());

    let enable = Output::new(p.PC2, Level::Low, Speed::VeryHigh);
    let cs_pin = Output::new(p.PA11, Level::High, Speed::VeryHigh);
    let _sel0 = Output::new(p.PC3, Level::Low, Speed::VeryHigh);
    let _sel1 = Output::new(p.PA1, Level::Low, Speed::VeryHigh);
    let _sel2 = Output::new(p.PA0, Level::Low, Speed::VeryHigh);
    let input = Input::new(p.PB6, Pull::Up);
    let interrupt = ExtiInput::new(input, p.EXTI6);
    info!("GPIO initialized");

    let button = Input::new(p.PC13, Pull::Down);
    let button = ExtiInput::new(button, p.EXTI13);

    let spi = Spi::new(
        p.SPI2,
        p.PB13, // SCK
        p.PB15, // MOSI
        p.PB14, // MISO
        NoDma,
        NoDma,
        xm125_spi_config(),
    );
    let exclusive_device = ExclusiveDevice::new(spi, cs_pin, Delay);
    info!("SPI initialized");

    unsafe { SPI_DEVICE = Some(RefCell::new(SpiAdapter::new(exclusive_device))) };
    let spi_mut_ref = unsafe { SPI_DEVICE.as_mut().unwrap() };

    info!("RSS Version: {}", rss_version());

    let mut radar = Radar::new(1, spi_mut_ref.get_mut(), interrupt, enable, Delay).await;
    info!("Radar enabled");
    let mut buffer = [0u8; 2560];
    let mut calibration = loop {
        buffer.fill(0);
        if let Ok(calibration) = radar.calibrate().await {
            if let Ok(()) = calibration.validate_calibration() {
                info!("Calibration is valid");
                break calibration;
            } else {
                warn!("Calibration is invalid");
                warn!("Calibration result: {:?}", calibration);
            }
        } else {
            warn!("Calibration failed");
        }
        Timer::after(Duration::from_millis(1)).await;
    };
    info!("Calibration complete!");
    let mut radar = radar.prepare_sensor(&mut calibration).unwrap();
    let mut distance = RadarDistanceDetector::new(&mut radar);
    let mut buffer = [0u8; 2560 * 3];
    let mut static_call_result = [0u8; 2560];
    let mut dynamic_call_result = distance
        .calibrate_detector(&calibration, &mut buffer, &mut static_call_result)
        .await
        .unwrap();

    spawner.spawn(io::button_task(button)).unwrap();

    loop {
        Timer::after(Duration::from_millis(200)).await;
        'inner: loop {
            distance
                .prepare_detector(&calibration, &mut buffer)
                .unwrap();
            distance.measure().await.unwrap();

            if let Ok(res) = distance.process_data(
                &mut buffer,
                &mut static_call_result,
                &mut dynamic_call_result,
            ) {
                if res.num_distances() > 0 {
                    info!(
                        "{} Distances found:\n{:?}",
                        res.num_distances(),
                        res.distances()
                    );
                }
                if res.calibration_needed() {
                    info!("Calibration needed");
                    break 'inner;
                }
            } else {
                warn!("Failed to process data");
            }
        }
        let calibration = distance.calibrate().await.unwrap();
        dynamic_call_result = distance
            .update_calibration(&calibration, &mut buffer)
            .await
            .unwrap();
    }
}

fn xm125_spi_config() -> Config {
    let mut spi_config = Config::default();
    spi_config.frequency = Hertz(1_000_000);
    spi_config
}

fn xm125_clock_config() -> embassy_stm32::Config {
    let mut config = embassy_stm32::Config::default();
    config.rcc.hsi = true;
    config.rcc.hse = None;
    config.rcc.msi = None;
    config.rcc.mux = ClockSrc::PLL1_R;
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

#[no_mangle]
pub extern "C" fn __hardfp_cosf(f: f32) -> f32 {
    libm::cosf(f)
}

#[no_mangle]
pub extern "C" fn __hardfp_sinf(f: f32) -> f32 {
    libm::sinf(f)
}

#[no_mangle]
pub extern "C" fn __hardfp_roundf(f: f32) -> f32 {
    libm::roundf(f)
}

#[no_mangle]
pub extern "C" fn __hardfp_sqrtf(f: f32) -> f32 {
    libm::sqrtf(f)
}

#[no_mangle]
pub extern "C" fn __hardfp_powf(f: f32, g: f32) -> f32 {
    libm::powf(f, g)
}

#[no_mangle]
pub extern "C" fn __hardfp_cexpf(f: Complex32) -> Complex32 {
    f.exp()
}

#[no_mangle]
pub extern "C" fn __hardfp_cabsf(f: f32) -> f32 {
    libm::fabsf(f)
}

#[no_mangle]
pub extern "C" fn __hardfp_atanf(f: f32) -> f32 {
    libm::atanf(f)
}

#[no_mangle]
pub extern "C" fn __hardfp_floorf(f: f32) -> f32 {
    libm::floorf(f)
}

#[no_mangle]
pub extern "C" fn __hardfp_log10f(f: f32) -> f32 {
    libm::log10f(f)
}

#[no_mangle]
pub extern "C" fn __hardfp_exp2f(f: f32) -> f32 {
    libm::exp2f(f)
}
