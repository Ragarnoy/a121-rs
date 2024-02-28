use crate::rss_bindings::acc_config_prf_t;

/// Pulse Repetition Frequency (PRF)
///
/// PRF is the frequency at which pulses are sent out from the radar system.
/// The measurement time is approximately proportional to the PRF. The higher
/// the PRF, the shorter the measurement time.
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

impl From<PulseRepetitionFrequency> for u32 {
    fn from(prf: PulseRepetitionFrequency) -> Self {
        prf.value()
    }
}

impl PulseRepetitionFrequency {
    /// Returns the PRF value in Hz.
    pub fn value(&self) -> u32 {
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
    pub fn max_measurable_distance(&self) -> f32 {
        match self {
            PulseRepetitionFrequency::Prf19_5Mhz => 3.1,
            PulseRepetitionFrequency::Prf15_6Mhz => 5.1,
            PulseRepetitionFrequency::Prf13_0Mhz => 7.0,
            PulseRepetitionFrequency::Prf8_7Mhz => 12.7,
            PulseRepetitionFrequency::Prf6_5Mhz => 18.5,
            PulseRepetitionFrequency::Prf5_2Mhz => 24.3,
        }
    }
}
