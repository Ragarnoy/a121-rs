#[derive(Debug, Copy, Clone)]
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
