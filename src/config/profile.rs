use a121_sys::acc_config_profile_t_ACC_CONFIG_PROFILE_1;

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
/// Radar profiles indicating different settings for the sensor's RX and TX paths.
pub enum RadarProfile {
    /// Profile 1
    AccProfile1 = acc_config_profile_t_ACC_CONFIG_PROFILE_1 as isize,
    /// Profile 2
    AccProfile2,
    /// Profile 3
    AccProfile3,
    /// Profile 4
    AccProfile4,
    /// Profile 5
    AccProfile5,
}

impl RadarProfile {
    /// Converts from FFI value, returning `None` for invalid values.
    pub const fn from_ffi(value: u32) -> Option<Self> {
        match value {
            1 => Some(RadarProfile::AccProfile1),
            2 => Some(RadarProfile::AccProfile2),
            3 => Some(RadarProfile::AccProfile3),
            4 => Some(RadarProfile::AccProfile4),
            5 => Some(RadarProfile::AccProfile5),
            _ => None,
        }
    }
}

impl From<u32> for RadarProfile {
    /// Converts from FFI value, defaulting to Profile1 for invalid values.
    fn from(value: u32) -> Self {
        Self::from_ffi(value).unwrap_or(RadarProfile::AccProfile1)
    }
}
