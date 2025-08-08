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
