pub mod config;
pub mod results;

use crate::detector::distance::config::RadarDistanceConfig;
use crate::detector::distance::results::{DistanceSizes, ProcessDataError};
use crate::radar::{Radar, Ready};
use crate::rss_bindings::*;
use crate::sensor::calibration::CalibrationResult;
use crate::sensor::data::RadarData;
use crate::sensor::error::SensorError;
use core::ffi::c_void;
use defmt::trace;
use embedded_hal::digital::OutputPin;
use embedded_hal_async::delay::DelayNs;
use embedded_hal_async::digital::Wait;
use results::{DistanceResult, DynamicResult};

struct InnerRadarDistanceDetector {
    inner: *mut acc_detector_distance_handle,
}

impl InnerRadarDistanceDetector {
    pub fn new(config: &RadarDistanceConfig) -> Self {
        Self {
            inner: unsafe { acc_detector_distance_create(config.inner) },
        }
    }

    pub fn inner(&self) -> *const acc_detector_distance_handle {
        self.inner
    }

    pub fn inner_mut(&mut self) -> *mut acc_detector_distance_handle {
        self.inner
    }
}

impl Drop for InnerRadarDistanceDetector {
    fn drop(&mut self) {
        unsafe { acc_detector_distance_destroy(self.inner) }
    }
}

pub struct RadarDistanceDetector<'radar, SINT, ENABLE, DLY>
where
    SINT: Wait,
    ENABLE: OutputPin,
    DLY: DelayNs,
{
    pub radar: &'radar mut Radar<Ready, SINT, ENABLE, DLY>,
    inner: InnerRadarDistanceDetector,
    pub config: RadarDistanceConfig,
}

impl<'radar, SINT, ENABLE, DLY> RadarDistanceDetector<'radar, SINT, ENABLE, DLY>
where
    SINT: Wait,
    ENABLE: OutputPin,
    DLY: DelayNs,
{
    pub fn new(radar: &'radar mut Radar<Ready, SINT, ENABLE, DLY>) -> Self {
        let config = RadarDistanceConfig::default();
        let inner = InnerRadarDistanceDetector::new(&config);
        trace!("{:?}", DistanceSizes::new(&inner));
        Self {
            radar,
            inner,
            config,
        }
    }

    pub async fn calibrate_detector(
        &mut self,
        sensor_cal_result: &CalibrationResult,
        buffer: &mut [u8],
        detector_cal_result_static: &mut [u8],
    ) -> Result<DynamicResult, SensorError> {
        let mut calibration_complete: bool = false;
        let mut detector_cal_result_dynamic = DynamicResult::default();
        let calibration_attempt: bool;

        unsafe {
            calibration_attempt = acc_detector_distance_calibrate(
                self.radar.inner_sensor(),
                self.inner.inner_mut(),
                sensor_cal_result.ptr(),
                buffer.as_mut_ptr() as *mut c_void,
                buffer.len() as u32,
                detector_cal_result_static.as_mut_ptr(),
                detector_cal_result_static.len() as u32,
                &mut detector_cal_result_dynamic.inner as *mut acc_detector_cal_result_dynamic_t,
                &mut calibration_complete as *mut bool,
            );
        }

        if calibration_attempt {
            while !calibration_complete {
                // Wait for the interrupt to occur asynchronously
                self.radar
                    .interrupt
                    .wait_for_high()
                    .await
                    .expect("Failed to wait for interrupt");
                unsafe {
                    acc_detector_distance_calibrate(
                        self.radar.inner_sensor(),
                        self.inner.inner_mut(),
                        sensor_cal_result.ptr(),
                        buffer.as_mut_ptr() as *mut c_void,
                        buffer.len() as u32,
                        detector_cal_result_static.as_mut_ptr(),
                        detector_cal_result_static.len() as u32,
                        &mut detector_cal_result_dynamic.inner
                            as *mut acc_detector_cal_result_dynamic_t,
                        &mut calibration_complete as *mut bool,
                    );
                }
            }

            Ok(detector_cal_result_dynamic)
        } else {
            Err(SensorError::CalibrationFailed)
        }
    }

    pub async fn update_calibration(
        &mut self,
        sensor_cal_result: &CalibrationResult,
        buffer: &mut [u8],
    ) -> Result<DynamicResult, SensorError> {
        let mut calibration_complete: bool = false;
        let mut detector_cal_result_dynamic = DynamicResult::default();
        let calibration_attempt: bool;

        unsafe {
            calibration_attempt = acc_detector_distance_update_calibration(
                self.radar.inner_sensor(),
                self.inner.inner_mut(),
                sensor_cal_result.ptr(),
                buffer.as_mut_ptr() as *mut c_void,
                buffer.len() as u32,
                &mut detector_cal_result_dynamic.inner as *mut acc_detector_cal_result_dynamic_t,
                &mut calibration_complete as *mut bool,
            );
        }

        if calibration_attempt {
            while !calibration_complete {
                // Wait for the interrupt to occur asynchronously
                self.radar
                    .interrupt
                    .wait_for_high()
                    .await
                    .expect("Failed to wait for interrupt");
                unsafe {
                    acc_detector_distance_update_calibration(
                        self.radar.inner_sensor(),
                        self.inner.inner_mut(),
                        sensor_cal_result.ptr(),
                        buffer.as_mut_ptr() as *mut c_void,
                        buffer.len() as u32,
                        &mut detector_cal_result_dynamic.inner
                            as *mut acc_detector_cal_result_dynamic_t,
                        &mut calibration_complete as *mut bool,
                    );
                }
            }

            Ok(detector_cal_result_dynamic)
        } else {
            Err(SensorError::CalibrationFailed)
        }
    }

    pub fn prepare_detector(
        &mut self,
        sensor_cal_result: &CalibrationResult,
        buffer: &mut [u8],
    ) -> Result<(), SensorError> {
        unsafe {
            if acc_detector_distance_prepare(
                self.inner.inner(),
                self.config.inner,
                self.radar.inner_sensor(),
                sensor_cal_result.ptr(),
                buffer.as_mut_ptr() as *mut c_void,
                buffer.len() as u32,
            ) {
                Ok(())
            } else {
                Err(SensorError::PrepareFailed)
            }
        }
    }

    pub async fn measure(&mut self) -> Result<RadarData, SensorError> {
        self.radar.measure().await
    }

    pub async fn calibrate(&mut self) -> Result<CalibrationResult, SensorError> {
        self.radar.calibrate().await
    }

    pub fn process_data(
        &mut self,
        buffer: &mut [u8],
        detector_cal_result_static: &mut [u8],
        detector_cal_result_dynamic: &mut DynamicResult,
    ) -> Result<DistanceResult<'_>, ProcessDataError> {
        let mut result_available: bool = false;
        let mut distance_result = DistanceResult::new(&self.radar.config);
        let mut distance_result_ptr: acc_detector_distance_result_t = distance_result.inner();

        let process_attempt: bool = unsafe {
            acc_detector_distance_process(
                self.inner.inner_mut(),
                buffer.as_mut_ptr() as *mut c_void,
                detector_cal_result_static.as_mut_ptr(),
                &mut detector_cal_result_dynamic.inner as *mut acc_detector_cal_result_dynamic_t,
                &mut result_available as *mut bool,
                &mut distance_result_ptr as *mut acc_detector_distance_result_t,
            )
        };
        distance_result.update_from_detector_result(distance_result_ptr);

        if process_attempt {
            if result_available {
                Ok(distance_result)
            } else {
                Err(ProcessDataError::Unavailable)
            }
        } else {
            Err(ProcessDataError::ProcessingFailed)
        }
    }

    pub fn print_status(&mut self) {
        self.radar.check_status()
    }
}
