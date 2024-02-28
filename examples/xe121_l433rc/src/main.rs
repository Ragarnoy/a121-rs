#![no_std]
#![no_main]

use core::cell::RefCell;

use a121_rs::config::profile::RadarProfile::AccProfile5;
use a121_rs::radar;
use a121_rs::radar::Radar;
use a121_rs::sensor::data::RadarData;
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
use radar::rss_version;
use {defmt_rtt as _, panic_probe as _};

use crate::adapter::SpiAdapter;

mod adapter;

type SpiDeviceMutex =
    ExclusiveDevice<Spi<'static, SPI2, NoDma, NoDma>, Output<'static, PA11>, Delay>;
static mut SPI_DEVICE: Option<RefCell<SpiAdapter<SpiDeviceMutex>>> = None;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Starting");
    let p = embassy_stm32::init(xm125_clock_config());

    let mut enable = Output::new(p.PC2, Level::Low, Speed::VeryHigh); // ENABLE on PB12
    let cs_pin = Output::new(p.PA11, Level::High, Speed::VeryHigh);
    let _sel0 = Output::new(p.PC3, Level::Low, Speed::VeryHigh);
    let _sel1 = Output::new(p.PA1, Level::Low, Speed::VeryHigh);
    let _sel2 = Output::new(p.PA0, Level::Low, Speed::VeryHigh);
    let input = Input::new(p.PB6, Pull::Up);
    let interrupt = ExtiInput::new(input, p.EXTI6);
    info!("GPIO initialized");

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

    enable.set_high();
    Timer::after(Duration::from_millis(2)).await;

    info!("RSS Version: {}", rss_version());

    let mut radar = Radar::new(1, spi_mut_ref.get_mut(), interrupt).enable();
    radar.config.set_profile(AccProfile5);
    info!("Radar enabled");
    Timer::after(Duration::from_millis(3)).await;
    let mut buffer = [0u8; 2560];
    loop {
        buffer.fill(0);
        if let Ok(mut calibration) = radar.sensor.calibrate(&mut buffer).await {
            if let Ok(()) = calibration.validate_calibration() {
                info!("Calibration is valid");
                radar
                    .sensor
                    .prepare(&radar.config, &mut calibration, &mut buffer)
                    .unwrap();
                break;
            } else {
                warn!("Calibration is invalid");
                warn!("Calibration result: {:?}", calibration);
                enable.set_low();
            }
        } else {
            warn!("Calibration failed");
        }
        Timer::after(Duration::from_millis(1)).await;
    }
    info!("Calibration complete!");
    let mut sensor = radar.sensor.set_ready().unwrap();

    loop {
        Timer::after(Duration::from_secs(1)).await;
        let mut data = RadarData::new();
        sensor.measure().await.unwrap();
        sensor.read(&mut data).await.unwrap();
        info!("Data: {:?}", data);
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
