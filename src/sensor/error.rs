#[derive(Debug, Copy, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SensorError {
    CalibrationFailed,
    PrepareFailed,
    MeasurementError,
    ReadError,
    HibernationOffFailed,
    HibernationOnFailed,
    CalibrationInvalid,
    NotReady,
    CalibrationInfo,
    ResultNotAvailable,
    ProcessingFailed,
    BufferTooSmall,
    InitFailed,
}

impl core::error::Error for SensorError {}

impl core::fmt::Display for SensorError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::CalibrationFailed => write!(f, "calibration failed"),
            Self::PrepareFailed => write!(f, "sensor preparation failed"),
            Self::MeasurementError => write!(f, "measurement error"),
            Self::ReadError => write!(f, "read error"),
            Self::HibernationOffFailed => write!(f, "failed to exit hibernation"),
            Self::HibernationOnFailed => write!(f, "failed to enter hibernation"),
            Self::CalibrationInvalid => write!(f, "calibration invalid"),
            Self::NotReady => write!(f, "sensor not ready"),
            Self::CalibrationInfo => write!(f, "calibration info error"),
            Self::ResultNotAvailable => write!(f, "result not available"),
            Self::ProcessingFailed => write!(f, "processing failed"),
            Self::BufferTooSmall => write!(f, "buffer too small"),
            Self::InitFailed => write!(f, "initialization failed"),
        }
    }
}

/// Enumerates possible errors that can occur during the processing of radar data.
#[derive(Debug, Copy, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ProcessDataError {
    /// Calibration is needed before processing
    CalibrationNeeded,
    /// The processing failed
    ProcessingFailed,
    /// The result is not available
    Unavailable,
    /// One or more buffers are too small
    BufferTooSmall,
}

impl core::error::Error for ProcessDataError {}

impl core::fmt::Display for ProcessDataError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::CalibrationNeeded => write!(f, "calibration needed"),
            Self::ProcessingFailed => write!(f, "processing failed"),
            Self::Unavailable => write!(f, "result unavailable"),
            Self::BufferTooSmall => write!(f, "buffer too small"),
        }
    }
}
