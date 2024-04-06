use crate::sensor::error::SensorError;
use a121_sys::{
    acc_cal_info_t, acc_cal_result_t, acc_sensor_get_cal_info, acc_sensor_validate_calibration,
    ACC_CAL_RESULT_DATA_SIZE,
};

pub struct CalibrationInfo {
    inner: acc_cal_info_t,
}

impl CalibrationInfo {
    pub fn temperature(&self) -> i16 {
        self.inner.temperature
    }

    /// Returns a mutable pointer to the inner `acc_cal_info_t` struct.
    /// # Safety
    /// This function is unsafe because it returns a raw pointer.
    pub unsafe fn mut_ptr(&mut self) -> *mut acc_cal_info_t {
        &mut self.inner
    }
}

#[derive(Debug)]
pub struct CalibrationResult {
    inner: acc_cal_result_t,
}

impl defmt::Format for CalibrationResult {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "{:?}", self.inner.data)
    }
}

impl CalibrationResult {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ptr(&self) -> *const acc_cal_result_t {
        &self.inner
    }

    /// Returns a mutable pointer to the inner `acc_cal_result_t` struct.
    /// # Safety
    /// This function is unsafe because it returns a raw pointer.
    pub unsafe fn mut_ptr(&mut self) -> *mut acc_cal_result_t {
        &mut self.inner
    }

    /// Validates a calibration result.
    ///
    /// # Arguments
    /// * `cal_result` - The calibration result to validate.
    ///
    /// # Returns
    /// `Ok(())` if the calibration result is valid, `Err(CalibrationInvalid)` otherwise.
    pub fn validate_calibration(&self) -> Result<(), SensorError> {
        unsafe {
            let result = acc_sensor_validate_calibration(self.ptr());
            if result {
                Ok(())
            } else {
                Err(SensorError::CalibrationInvalid)
            }
        }
    }

    pub fn temperature(&self) -> Result<i16, SensorError> {
        let mut calibration_info = CalibrationInfo::default();
        let res = unsafe { acc_sensor_get_cal_info(self.ptr(), calibration_info.mut_ptr()) };
        if res {
            Ok(calibration_info.temperature())
        } else {
            Err(SensorError::CalibrationInfo)
        }
    }
}

impl From<CalibrationResult> for CalibrationInfo {
    fn from(calibration_result: CalibrationResult) -> Self {
        let mut calibration_info = CalibrationInfo::default();
        unsafe { acc_sensor_get_cal_info(calibration_result.ptr(), calibration_info.mut_ptr()) };
        calibration_info
    }
}

impl Default for CalibrationResult {
    fn default() -> Self {
        let inner = acc_cal_result_t {
            data: [0; ACC_CAL_RESULT_DATA_SIZE as usize],
        };
        Self { inner }
    }
}

impl Default for CalibrationInfo {
    fn default() -> Self {
        let inner = acc_cal_info_t { temperature: 0 };
        Self { inner }
    }
}
