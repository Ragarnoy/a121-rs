use a121_sys::acc_config_prf_t;

/// Pulse Repetition Frequency (PRF)
///
/// PRF is the frequency at which pulses are sent out from the radar system.
/// The measurement time is approximately inversely proportional to the PRF.
/// The higher the PRF, the shorter the measurement time.
///
/// This parameter sets the Maximum Measurable Distance (MMD) and the
/// Maximum Unambiguous Range (MUR). MMD is the maximum value for the end
/// point (start point + (number of points * step length)). MUR is the maximum
/// distance at which an object can be located to guarantee its reflection
/// corresponds to the most recent transmitted pulse.
///
/// | PRF Setting         | PRF      | MMD  | MUR   |
/// |---------------------|----------|------|-------|
/// | Prf19_5Mhz*         | 19.5 MHz | 3.1m | 7.7m  |
/// | Prf15_6Mhz          | 15.6 MHz | 5.1m | 9.6m  |
/// | Prf13_0Mhz          | 13.0 MHz | 7.0m | 11.5m |
/// | Prf8_7Mhz           | 8.7 MHz  | 12.7m| 17.3m |
/// | Prf6_5Mhz           | 6.5 MHz  | 18.5m| 23.1m |
/// | Prf5_2Mhz           | 5.2 MHz  | 24.3m| 28.8m |
///
/// *19.5MHz is only available for profile 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PulseRepetitionFrequency {
    /// 19.5 MHz (Available only for profile 1)
    Prf19_5Mhz = 0,
    /// 15.6 MHz
    Prf15_6Mhz,
    /// 13.0 MHz
    Prf13_0Mhz,
    /// 8.7 MHz
    Prf8_7Mhz,
    /// 6.5 MHz
    Prf6_5Mhz,
    /// 5.2 MHz
    Prf5_2Mhz,
}

impl TryFrom<acc_config_prf_t> for PulseRepetitionFrequency {
    type Error = ();

    fn try_from(value: acc_config_prf_t) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(PulseRepetitionFrequency::Prf19_5Mhz),
            1 => Ok(PulseRepetitionFrequency::Prf15_6Mhz),
            2 => Ok(PulseRepetitionFrequency::Prf13_0Mhz),
            3 => Ok(PulseRepetitionFrequency::Prf8_7Mhz),
            4 => Ok(PulseRepetitionFrequency::Prf6_5Mhz),
            5 => Ok(PulseRepetitionFrequency::Prf5_2Mhz),
            _ => Err(()),
        }
    }
}

impl PulseRepetitionFrequency {
    /// Converts a value read back from the SDK.
    ///
    /// A value the SDK never stores means a corrupted configuration, mismatched bindings, or a
    /// subsweep index the configuration does not have: it fails a debug assertion, and otherwise
    /// it is logged and replaced by the SDK default of 15.6 MHz. The stub library skips the
    /// assertion, since its getters return 0.
    pub(crate) fn from_sdk(value: acc_config_prf_t) -> Self {
        let prf = Self::try_from(value);
        debug_assert!(
            prf.is_ok() || cfg!(feature = "stub_library"),
            "SDK returned unknown PRF {value}"
        );
        #[cfg(feature = "defmt")]
        if prf.is_err() {
            defmt::warn!("SDK returned unknown PRF {}, using 15.6 MHz", value);
        }
        prf.unwrap_or(Self::Prf15_6Mhz)
    }

    /// Returns the PRF value in Hz.
    pub const fn value(&self) -> u32 {
        match self {
            PulseRepetitionFrequency::Prf19_5Mhz => 19_500_000,
            PulseRepetitionFrequency::Prf15_6Mhz => 15_600_000,
            PulseRepetitionFrequency::Prf13_0Mhz => 13_000_000,
            PulseRepetitionFrequency::Prf8_7Mhz => 8_700_000,
            PulseRepetitionFrequency::Prf6_5Mhz => 6_500_000,
            PulseRepetitionFrequency::Prf5_2Mhz => 5_200_000,
        }
    }

    /// Returns the maximum measurable distance in meters.
    pub const fn max_measurable_distance(&self) -> f32 {
        match self {
            PulseRepetitionFrequency::Prf19_5Mhz => 3.1,
            PulseRepetitionFrequency::Prf15_6Mhz => 5.1,
            PulseRepetitionFrequency::Prf13_0Mhz => 7.0,
            PulseRepetitionFrequency::Prf8_7Mhz => 12.7,
            PulseRepetitionFrequency::Prf6_5Mhz => 18.5,
            PulseRepetitionFrequency::Prf5_2Mhz => 24.3,
        }
    }

    /// Converts from FFI value, returning `None` for invalid values.
    pub const fn from_ffi(value: u32) -> Option<Self> {
        match value {
            0 => Some(PulseRepetitionFrequency::Prf19_5Mhz),
            1 => Some(PulseRepetitionFrequency::Prf15_6Mhz),
            2 => Some(PulseRepetitionFrequency::Prf13_0Mhz),
            3 => Some(PulseRepetitionFrequency::Prf8_7Mhz),
            4 => Some(PulseRepetitionFrequency::Prf6_5Mhz),
            5 => Some(PulseRepetitionFrequency::Prf5_2Mhz),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [PulseRepetitionFrequency; 6] = [
        PulseRepetitionFrequency::Prf19_5Mhz,
        PulseRepetitionFrequency::Prf15_6Mhz,
        PulseRepetitionFrequency::Prf13_0Mhz,
        PulseRepetitionFrequency::Prf8_7Mhz,
        PulseRepetitionFrequency::Prf6_5Mhz,
        PulseRepetitionFrequency::Prf5_2Mhz,
    ];

    #[test]
    fn ffi_round_trip() {
        for prf in ALL {
            let raw = prf as acc_config_prf_t;
            assert_eq!(PulseRepetitionFrequency::try_from(raw), Ok(prf));
            assert_eq!(PulseRepetitionFrequency::from_ffi(raw), Some(prf));
        }
        assert!(PulseRepetitionFrequency::try_from(6).is_err());
    }

    #[test]
    #[cfg_attr(
        all(debug_assertions, not(feature = "stub_library")),
        should_panic(expected = "unknown PRF")
    )]
    fn unknown_sdk_value_falls_back() {
        assert_eq!(
            PulseRepetitionFrequency::from_sdk(6),
            PulseRepetitionFrequency::Prf15_6Mhz
        );
    }

    #[test]
    fn discriminants_match_sdk() {
        assert_eq!(
            PulseRepetitionFrequency::Prf19_5Mhz as acc_config_prf_t,
            a121_sys::acc_config_prf_t_ACC_CONFIG_PRF_19_5_MHZ
        );
        assert_eq!(
            PulseRepetitionFrequency::Prf5_2Mhz as acc_config_prf_t,
            a121_sys::acc_config_prf_t_ACC_CONFIG_PRF_5_2_MHZ
        );
    }
}
