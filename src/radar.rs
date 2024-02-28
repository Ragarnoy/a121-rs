use core::fmt::Display;

use embedded_hal::spi::{ErrorKind as SpiErrorKind, SpiDevice};
use embedded_hal_async::digital::Wait;

use crate::config::RadarConfig;
use crate::hal::AccHalImpl;
use crate::processing::Processing;
use crate::rss_bindings::acc_version_get_hex;
use crate::sensor::{Disabled, Enabled, Sensor};

pub struct Radar<STATE, SINT>
where
    SINT: Wait,
{
    id: u32,
    pub config: RadarConfig,
    pub sensor: Sensor<STATE, SINT>,
    pub processing: Processing,
    _hal: AccHalImpl,
}

/// Radar Sensor Software Version
/// 0xMMMMmmPP where M is major, m is minor and P is patch
#[derive(Debug)]
pub struct RssVersion {
    version: u32,
}

impl defmt::Format for RssVersion {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "{}.{}.{}", self.major(), self.minor(), self.patch())
    }
}

impl Display for RssVersion {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}.{}.{}", self.major(), self.minor(), self.patch())
    }
}

impl RssVersion {
    pub fn new(version: u32) -> Self {
        Self { version }
    }

    pub fn major(&self) -> u16 {
        ((self.version & 0xFFFF0000) >> 16) as u16
    }

    pub fn minor(&self) -> u8 {
        ((self.version & 0x0000FF00) >> 8) as u8
    }

    pub fn patch(&self) -> u8 {
        (self.version & 0x000000FF) as u8
    }
}

impl<SINT> Radar<Disabled, SINT>
where
    SINT: Wait,
{
    pub fn new<SPI>(id: u32, spi: &'static mut SPI, interrupt: SINT) -> Self
    where
        SPI: SpiDevice<u8, Error = SpiErrorKind> + Send + 'static,
    {
        let hal = AccHalImpl::new(spi);
        hal.register();
        let config = RadarConfig::default();
        let sensor = Sensor::new(id, interrupt).expect("Failed to create sensor");
        let processing = Processing::new(&config);
        Self {
            id,
            config,
            sensor,
            processing,
            _hal: hal,
        }
    }

    pub fn enable(self) -> Radar<Enabled, SINT> {
        let sensor = self.sensor.enable();
        Radar {
            id: self.id,
            config: self.config,
            sensor,
            processing: self.processing,
            _hal: self._hal,
        }
    }
}

impl<SINT, T> Radar<T, SINT>
where
    SINT: Wait,
{
    pub fn id(&self) -> u32 {
        self.id
    }
}

pub fn rss_version() -> RssVersion {
    let version = unsafe { acc_version_get_hex() };
    RssVersion::new(version)
}
