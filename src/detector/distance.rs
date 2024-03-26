pub mod config;
pub mod results;

use crate::detector::distance::config::RadarDistanceConfig;
use crate::detector::distance::results::{DistanceSizes, ProcessDataError};
use crate::radar::{Radar, Ready};
use crate::rss_bindings::*;
use crate::sensor::calibration::CalibrationResult;
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
    fn new(config: &RadarDistanceConfig) -> Self {
        Self {
            inner: unsafe { acc_detector_distance_create(config.inner) },
        }
    }

    fn inner(&self) -> *const acc_detector_distance_handle {
        self.inner
    }

    fn inner_mut(&mut self) -> *mut acc_detector_distance_handle {
        self.inner
    }
}

impl Drop for InnerRadarDistanceDetector {
    fn drop(&mut self) {
        unsafe { acc_detector_distance_destroy(self.inner) }
    }
}

/// The main structure representing the radar distance detector.
pub struct RadarDistanceDetector<'radar, SINT, ENABLE, DLY>
where
    SINT: Wait,
    ENABLE: OutputPin,
    DLY: DelayNs,
{
    /// Reference to the radar system, configured and ready for operation.
    pub radar: &'radar mut Radar<Ready, SINT, ENABLE, DLY>,
    inner: InnerRadarDistanceDetector,
    /// Configuration for the radar distance detection.
    pub config: RadarDistanceConfig,
}

impl<'radar, SINT, ENABLE, DLY> RadarDistanceDetector<'radar, SINT, ENABLE, DLY>
where
    SINT: Wait,
    ENABLE: OutputPin,
    DLY: DelayNs,
{
    /// Constructs a new radar distance detector with default configuration.
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

    /// Constructs a new radar distance detector with the provided configuration.
    pub fn with_config(radar: &'radar mut Radar<Ready, SINT, ENABLE, DLY>, config: RadarDistanceConfig) -> Self {
        let inner = InnerRadarDistanceDetector::new(&config);
        trace!("{:?}", DistanceSizes::new(&inner));
        Self {
            radar,
            inner,
            config,
        }
    }

    /// Performs calibration of the radar distance detector.
    pub async fn calibrate_detector(
        &mut self,
        sensor_cal_result: &CalibrationResult,
        buffer: &mut [u8],
        detector_cal_result_static: &mut [u8],
    ) -> Result<DynamicResult, SensorError> {
        let mut calibration_complete: bool = false;
        let mut detector_cal_result_dynamic = DynamicResult::default();
        let distances = DistanceSizes::new(&self.inner);

        // Check buffer sizes before attempting calibration
        if buffer.len() < distances.buffer_size
            || detector_cal_result_static.len() < distances.detector_cal_result_static_size
        {
            return Err(SensorError::BufferTooSmall);
        }

        loop {
            let calibration_attempt = unsafe {
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
                )
            };

            // Check if the calibration attempt was successful
            if !calibration_attempt {
                return Err(SensorError::CalibrationFailed);
            }

            // Break the loop if calibration is complete
            if calibration_complete {
                break;
            }

            // Wait for the interrupt signal asynchronously
            self.radar
                .interrupt
                .wait_for_high()
                .await
                .expect("Failed to wait for interrupt");
        }

        Ok(detector_cal_result_dynamic)
    }

    /// Returns the size of the buffer needed for static calibration results.
    pub fn get_static_result_buffer_size(&self) -> usize {
        DistanceSizes::new(&self.inner).detector_cal_result_static_size
    }

    /// Returns the size of the buffer needed for distance detection.
    pub fn get_distance_buffer_size(&self) -> usize {
        DistanceSizes::new(&self.inner).buffer_size
    }

    /// Updates the calibration dynamically based on new sensor data.
    /// This function is intended to be used when a recalibration is necessary due to changes in the operating environment.
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

    /// Prepares the detector for a measurement operation.
    ///
    /// This function must be called before performing a distance measurement to configure the detector properly.
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

    /// Performs a distance measurement operation asynchronously.
    ///
    /// This function initiates a measurement operation, returning the results asynchronously.
    pub async fn measure(&mut self, data: &mut [u8]) -> Result<(), SensorError> {
        self.radar.measure(data).await
    }

    /// Calibrates the associated radar asynchronously.
    ///
    /// This function performs a calibration operation on the radar, necessary for accurate distance measurements.
    pub async fn calibrate(&mut self) -> Result<CalibrationResult, SensorError> {
        self.radar.calibrate().await
    }

    /// Processes the data collected from a distance measurement operation.
    ///
    /// This function analyzes the raw data collected during a measurement operation, extracting distance information.
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

    /// Prints the status of the radar distance detector.
    pub fn print_status(&mut self) {
        self.radar.check_status()
    }
}
