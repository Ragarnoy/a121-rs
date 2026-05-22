//! A121 bring-up on STM32WBA65RI (Embassy).
//!
//! SPI1: SCK PB4, MOSI PA15, MISO PB3, CS PA8, enable PA9.

#![no_std]
#![no_main]

use a121_rs::bringup::{self, Error as BringupError};
use a121_rs::hal::AccHalImpl;
use a121_rs::radar::rss_version;
use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Input, Level, Output, Pull, Speed};
use embassy_stm32::mode::Blocking;
use embassy_stm32::spi::{self, Config as SpiConfig, Spi};
use embassy_stm32::time::Hertz;
use embassy_time::{Duration, Timer};
use embedded_hal::spi::{Operation, SpiDevice};
use embedded_hal_bus::spi::ExclusiveDevice;
use linked_list_allocator::LockedHeap;
use {defmt_rtt as _, panic_probe as _};

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

type SpiBus = ExclusiveDevice<
    Spi<'static, Blocking>,
    Output<'static>,
    embassy_time::Delay,
>;

static mut SPI_BUS: Option<SpiBus> = None;

fn spi_transfer(buffer: &mut [u8]) -> Result<(), BringupError> {
    let bus = unsafe {
        (*core::ptr::addr_of_mut!(SPI_BUS))
            .as_mut()
            .expect("SPI not initialized")
    };
    bus.transaction(&mut [Operation::TransferInPlace(buffer)])
        .map_err(|_| BringupError::Spi)
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    const HEAP_SIZE: usize = 32 * 1024;
    static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];
    unsafe { ALLOCATOR.lock().init(HEAP.as_mut_ptr(), HEAP_SIZE) };

    let p = embassy_stm32::init(Default::default());

    info!("A121 STM32WBA65RI bring-up");
    info!("RSS version: {}", rss_version());

    let mut enable = Output::new(p.PA9, Level::Low, Speed::High);
    let cs = Output::new(p.PA8, Level::High, Speed::High);
    let _int_pin = Input::new(p.PB10, Pull::Up);

    let mut spi_cfg = SpiConfig::default();
    spi_cfg.frequency = Hertz(32_000_000);
    spi_cfg.mode = spi::MODE_0;

    let spi = Spi::new_blocking(p.SPI1, p.PB4, p.PA15, p.PB3, spi_cfg);

    unsafe {
        SPI_BUS = Some(ExclusiveDevice::new(spi, cs, embassy_time::Delay).unwrap());
    }

    enable.set_high();
    Timer::after(Duration::from_millis(2)).await;

    let hal = AccHalImpl::new_with_transfer(spi_transfer, 65535);
    hal.register().expect("HAL register");

    match bringup::read_asic_id(&mut bringup::FnSpiTransfer(spi_transfer)) {
        Ok(id) => info!("ASIC ID: 0x{:04x}", id),
        Err(e) => error!("ASIC ID read failed: {:?}", e),
    }

    loop {
        Timer::after_secs(2).await;
        info!("heartbeat");
    }
}
