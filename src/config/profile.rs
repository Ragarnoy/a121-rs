#[derive(Debug, PartialEq)]
/// Radar profiles indicating different settings for the sensor's RX and TX paths.
pub enum RadarProfile {
    /// Profile 1
    AccProfile1 = 1,
    /// Profile 2
    AccProfile2,
    /// Profile 3
    AccProfile3,
    /// Profile 4
    AccProfile4,
    /// Profile 5
    AccProfile5,
}

impl From<u32> for RadarProfile {
    fn from(value: u32) -> Self {
        match value {
            1 => RadarProfile::AccProfile1,
            2 => RadarProfile::AccProfile2,
            3 => RadarProfile::AccProfile3,
            4 => RadarProfile::AccProfile4,
            5 => RadarProfile::AccProfile5,
            _ => panic!("Invalid radar profile: {}", value),
        }
    }
}
