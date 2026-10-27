use crate::config::error::ConfigError;

/// Hardware accelerated average samples
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hwaas(u16);

impl Hwaas {
    /// Creates a new HWAAS value. Must be between 1 and 511 inclusive.
    pub fn new(value: u16) -> Result<Self, ConfigError> {
        Self::try_from(value)
    }

    /// Wraps a value read back from the SDK, which only stores 1..=511.
    pub(crate) fn from_sdk(value: u16) -> Self {
        Self(value.clamp(1, 511))
    }
}

impl TryFrom<u16> for Hwaas {
    type Error = ConfigError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        // The SDK requires 1..=511
        if (1..=511).contains(&value) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_matches_sdk() {
        assert!(Hwaas::new(0).is_err());
        assert_eq!(u16::from(Hwaas::new(1).unwrap()), 1);
        assert_eq!(u16::from(Hwaas::new(511).unwrap()), 511);
        assert!(Hwaas::new(512).is_err());
        assert_eq!(u16::from(Hwaas::from_sdk(0)), 1);
        assert_eq!(u16::from(Hwaas::from_sdk(600)), 511);
    }
}
