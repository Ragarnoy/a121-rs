#[derive(Debug)]
/// Custom errors for radar configuration operations.
pub enum ConfigError {
    /// Error indicating invalid hardware accelerated average samples setting.
    Hwaas,
    /// Error indicating invalid continuous sweep mode setting.
    ContinuousSweepMode,
    /// Error indicating invalid sweep rate setting.
    SweepRate,
    NumSubsweep,
}
