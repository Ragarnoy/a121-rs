#[derive(Debug, Copy, Clone, defmt::Format)]
pub enum SensorError {
    FailedCalibration,
    FailedPrepare,
    MeasurementError,
    ReadError,
    FailedHibernate,
    CalibrationInvalid,
    NotReady,
    CalibrationInfo,
}
