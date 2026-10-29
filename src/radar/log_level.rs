use a121_sys::{
    acc_log_level_t, acc_log_level_t_ACC_LOG_LEVEL_DEBUG, acc_log_level_t_ACC_LOG_LEVEL_ERROR,
    acc_log_level_t_ACC_LOG_LEVEL_INFO, acc_log_level_t_ACC_LOG_LEVEL_VERBOSE,
    acc_log_level_t_ACC_LOG_LEVEL_WARNING, acc_rss_set_log_level,
};

use crate::hal::ensure_registered;
use crate::sensor::error::SensorError;

/// Severity of the SDK's log messages, from the least to the most verbose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum LogLevel {
    /// Errors only
    Error,
    /// Errors and warnings
    Warning,
    /// Errors, warnings and information
    Info,
    /// Also verbose messages
    Verbose,
    /// Every message, including debug messages
    Debug,
}

impl From<LogLevel> for acc_log_level_t {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Error => acc_log_level_t_ACC_LOG_LEVEL_ERROR,
            LogLevel::Warning => acc_log_level_t_ACC_LOG_LEVEL_WARNING,
            LogLevel::Info => acc_log_level_t_ACC_LOG_LEVEL_INFO,
            LogLevel::Verbose => acc_log_level_t_ACC_LOG_LEVEL_VERBOSE,
            LogLevel::Debug => acc_log_level_t_ACC_LOG_LEVEL_DEBUG,
        }
    }
}

/// Sets the most verbose level of SDK messages passed to the logger. Messages only reach a log
/// with the `defmt` feature.
///
/// # Errors
///
/// Returns [`SensorError::InitFailed`] if the HAL could not be registered with the SDK, which the
/// log level needs.
pub fn set_sdk_log_level(level: LogLevel) -> Result<(), SensorError> {
    ensure_registered()?;
    unsafe { acc_rss_set_log_level(level.into()) };
    Ok(())
}

#[cfg(all(test, feature = "stub_library"))]
mod tests {
    use super::*;

    #[test]
    fn log_level_registers_the_hal() {
        assert_eq!(set_sdk_log_level(LogLevel::Warning), Ok(()));
    }
}
