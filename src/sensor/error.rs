#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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
    /// A buffer is not aligned for any built-in type, as the SDK requires
    BufferMisaligned,
    InitFailed,
    /// The sensor enable pin could not be set
    EnablePinFailed,
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
            Self::BufferMisaligned => write!(f, "buffer misaligned"),
            Self::InitFailed => write!(f, "initialization failed"),
            Self::EnablePinFailed => write!(f, "failed to set the sensor enable pin"),
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
    /// A buffer is not aligned for any built-in type, as the SDK requires
    BufferMisaligned,
}

impl core::error::Error for ProcessDataError {}

impl core::fmt::Display for ProcessDataError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::CalibrationNeeded => write!(f, "calibration needed"),
            Self::ProcessingFailed => write!(f, "processing failed"),
            Self::Unavailable => write!(f, "result unavailable"),
            Self::BufferTooSmall => write!(f, "buffer too small"),
            Self::BufferMisaligned => write!(f, "buffer misaligned"),
        }
    }
}

impl From<SensorError> for ProcessDataError {
    /// Maps buffer check errors; any other sensor error is a processing failure.
    fn from(error: SensorError) -> Self {
        match error {
            SensorError::BufferTooSmall => Self::BufferTooSmall,
            SensorError::BufferMisaligned => Self::BufferMisaligned,
            _ => Self::ProcessingFailed,
        }
    }
}
