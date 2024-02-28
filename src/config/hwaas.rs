use crate::config::error::ConfigError;

/// Hardware accelerated average samples
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hwaas(u16);

impl Hwaas {
    pub fn new(value: u16) -> Self {
        Self(value)
    }
}

impl TryFrom<u16> for Hwaas {
    type Error = ConfigError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        if (0..512).contains(&value) {
            Ok(Self(value))
        } else {
            Err(ConfigError::Hwaas)
        }
    }
}

impl From<Hwaas> for u16 {
    fn from(hwaas: Hwaas) -> Self {
        hwaas.0
    }
}
