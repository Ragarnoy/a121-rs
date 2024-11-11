use a121_sys::acc_version_get_hex;
use core::fmt::Display;

/// Radar Sensor Software Version
#[derive(Debug)]
pub struct RssVersion {
    version: u32,
}

#[cfg(feature = "defmt")]
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

/// Get the RSS version of the sensor
pub fn rss_version() -> RssVersion {
    let version = unsafe { acc_version_get_hex() };
    RssVersion::new(version)
}
