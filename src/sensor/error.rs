#[derive(Debug, Copy, Clone, defmt::Format)]
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
}
