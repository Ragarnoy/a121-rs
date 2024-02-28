#[derive(Debug, PartialEq)]
/// Frame rate options for the radar configuration.
pub enum FrameRate {
    /// No limit on the frame rate.
    Unlimited,
    /// Frame rate limited to a specific value.
    Limited(f32),
}

impl FrameRate {
    /// Returns the frame rate in Hz.
    pub fn value(&self) -> f32 {
        match self {
            FrameRate::Unlimited => 0.0,
            FrameRate::Limited(value) => *value,
        }
    }

    /// Returns true if the frame rate is limited to a specific value.
    pub fn is_limited(&self) -> bool {
        match self {
            FrameRate::Unlimited => false,
            FrameRate::Limited(_) => true,
        }
    }

    /// Returns true if the frame rate is unlimited.
    pub fn is_unlimited(&self) -> bool {
        !self.is_limited()
    }
}
