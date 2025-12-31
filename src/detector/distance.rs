pub mod config;
pub mod results;

use core::ffi::c_void;
use core::ptr::NonNull;

use embedded_hal::digital::OutputPin;
use embedded_hal_async::delay::DelayNs;
use embedded_hal_async::digital::Wait;

use a121_sys::*;

use crate::detector::distance::config::RadarDistanceConfig;
use crate::detector::distance::results::DistanceSizes;
use crate::radar::{Radar, RadarState};
use crate::sensor::calibration::CalibrationResult;
use crate::sensor::error::{ProcessDataError, SensorError};
use results::{DistanceResult, DynamicResult};

struct InnerRadarDistanceDetector {
    inner: NonNull<acc_detector_distance_handle>,
}

impl InnerRadarDistanceDetector {
    fn new(config: &RadarDistanceConfig) -> Self {
        let ptr = unsafe { acc_detector_distance_create(config.inner.as_ptr()) };
        Self {
            inner: NonNull::new(ptr).expect("Failed to create distance detector"),
        }
    }

    fn inner(&self) -> *const acc_detector_distance_handle {
        self.inner.as_ptr()
    }

    fn inner_mut(&mut self) -> *mut acc_detector_distance_handle {
        self.inner.as_ptr()
    }
}

impl Drop for InnerRadarDistanceDetector {
    fn drop(&mut self) {
        // NonNull guarantees non-null pointer
        unsafe { acc_detector_distance_destroy(self.inner.as_ptr()) }
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
    pub radar: &'radar mut Radar<SINT, ENABLE, DLY>,
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
    /// Returns an error if the radar is not in Ready state.
    pub fn new(radar: &'radar mut Radar<SINT, ENABLE, DLY>) -> Result<Self, SensorError> {
        if radar.state() != RadarState::Ready {
            return Err(SensorError::NotReady);
        }
        let config = RadarDistanceConfig::default();
        let inner = InnerRadarDistanceDetector::new(&config);
        #[cfg(feature = "defmt")]
        defmt::trace!("{:?}", DistanceSizes::new(&inner));
        Ok(Self {
            radar,
            inner,
            config,
        })
    }

    /// Constructs a new radar distance detector with the provided configuration.
    /// Returns an error if the radar is not in Ready state.
    pub fn with_config(
        radar: &'radar mut Radar<SINT, ENABLE, DLY>,
        config: RadarDistanceConfig,
    ) -> Result<Self, SensorError> {
        if radar.state() != RadarState::Ready {
            return Err(SensorError::NotReady);
        }
        let inner = InnerRadarDistanceDetector::new(&config);
        #[cfg(feature = "defmt")]
        defmt::trace!("{:?}", DistanceSizes::new(&inner));
        Ok(Self {
            radar,
            inner,
            config,
        })
    }

    /// Performs calibration of the radar distance detector.
    ///
    /// This method automatically validates that the provided buffers are large enough
    /// and returns an error if they are not. For the unchecked version that skips
    /// validation, see [`calibrate_detector_unchecked`](Self::calibrate_detector_unchecked).
    ///
    /// # Errors
    ///
    /// Returns [`SensorError::BufferTooSmall`] if either buffer is too small.
    /// Use [`get_distance_buffer_size`](Self::get_distance_buffer_size) and
    /// [`get_static_result_buffer_size`](Self::get_static_result_buffer_size)
    /// to determine required sizes, or use compile-time calculation functions
    /// from the [`memory`](crate::memory) module.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use a121_rs::detector::distance::RadarDistanceDetector;
    /// # async fn example(mut detector: RadarDistanceDetector<'_, impl embedded_hal_async::digital::Wait,
    /// #                             impl embedded_hal::digital::OutputPin,
    /// #                             impl embedded_hal_async::delay::DelayNs>,
    /// #                  calibration: a121_rs::sensor::calibration::CalibrationResult) {
    /// // Safe - automatically checks buffer sizes
    /// let mut buffer = vec![0u8; detector.get_distance_buffer_size()];
    /// let mut static_cal = vec![0u8; detector.get_static_result_buffer_size()];
    ///
    /// match detector.calibrate_detector(&calibration, &mut buffer, &mut static_cal).await {
    ///     Ok(result) => println!("Calibration successful"),
    ///     Err(e) => eprintln!("Calibration failed: {:?}", e),
    /// }
    /// # }
    /// ```
    pub async fn calibrate_detector(
        &mut self,
        sensor_cal_result: &CalibrationResult,
        buffer: &mut [u8],
        detector_cal_result_static: &mut [u8],
    ) -> Result<DynamicResult, SensorError> {
        let mut calibration_complete: bool = false;
        let mut detector_cal_result_dynamic = DynamicResult::default();
        let distances = DistanceSizes::new(&self.inner);

        // Automatic buffer size validation
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

    /// Performs calibration without buffer size checks.
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    /// - `buffer.len() >= self.get_distance_buffer_size()`
    /// - `detector_cal_result_static.len() >= self.get_static_result_buffer_size()`
    ///
    /// Violating these requirements may cause undefined behavior including:
    /// - Buffer overflows
    /// - Memory corruption
    /// - Segmentation faults
    ///
    /// # When to Use
    ///
    /// Only use this method when:
    /// - You have verified buffer sizes at compile time
    /// - Performance is critical and you want to avoid runtime checks
    /// - You are using const-sized arrays that are guaranteed to be large enough
    ///
    /// For most cases, use the safe [`calibrate_detector`](Self::calibrate_detector) instead.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use a121_rs::detector::distance::RadarDistanceDetector;
    /// # use a121_rs::memory::calc_distance_external_heap;
    /// # async fn example(mut detector: RadarDistanceDetector<'_, impl embedded_hal_async::digital::Wait,
    /// #                             impl embedded_hal::digital::OutputPin,
    /// #                             impl embedded_hal_async::delay::DelayNs>,
    /// #                  calibration: a121_rs::sensor::calibration::CalibrationResult) {
    /// // Using compile-time sized buffers
    /// const BUFFER_SIZE: usize = calc_distance_external_heap(100, 1, 16);
    /// const CAL_SIZE: usize = 4096; // Known to be large enough
    ///
    /// let mut buffer = [0u8; BUFFER_SIZE];
    /// let mut static_cal = [0u8; CAL_SIZE];
    ///
    /// // SAFETY: Buffers are sized correctly at compile time
    /// unsafe {
    ///     detector.calibrate_detector_unchecked(&calibration, &mut buffer, &mut static_cal)
    ///         .await
    ///         .unwrap();
    /// }
    /// # }
    /// ```
    pub async unsafe fn calibrate_detector_unchecked(
        &mut self,
        sensor_cal_result: &CalibrationResult,
        buffer: &mut [u8],
        detector_cal_result_static: &mut [u8],
    ) -> Result<DynamicResult, SensorError> {
        let mut calibration_complete: bool = false;
        let mut detector_cal_result_dynamic = DynamicResult::default();

        // NO BUFFER SIZE CHECKS - caller must ensure buffers are large enough

        loop {
            let calibration_attempt = acc_detector_distance_calibrate(
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

            if !calibration_attempt {
                return Err(SensorError::CalibrationFailed);
            }

            if calibration_complete {
                break;
            }

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

    /// Estimates memory requirements for this distance detector configuration.
    ///
    /// This method provides a conservative estimate of memory requirements based on
    /// the radar configuration. It can be called before allocating buffers to ensure
    /// sufficient memory is available.
    ///
    /// # Returns
    ///
    /// A `MemoryRequirements` struct containing:
    /// - `external_heap`: Memory for data buffers (bytes)
    /// - `rss_heap`: Memory for RSS internal use (bytes)
    /// - `total`: Total memory required (bytes)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use a121_rs::radar::Radar;
    /// # use a121_rs::detector::distance::RadarDistanceDetector;
    /// # fn example(radar: &mut Radar<impl embedded_hal_async::digital::Wait,
    /// #                             impl embedded_hal::digital::OutputPin,
    /// #                             impl embedded_hal_async::delay::DelayNs>) {
    /// let mut detector = RadarDistanceDetector::new(radar).unwrap();
    /// let mem = detector.estimate_memory_requirements();
    /// println!("Total memory needed: {} bytes", mem.total);
    /// println!("External heap: {} bytes", mem.external_heap);
    /// println!("RSS heap: {} bytes", mem.rss_heap);
    /// # }
    /// ```
    pub fn estimate_memory_requirements(&self) -> crate::memory::MemoryRequirements {
        use crate::memory::DistanceMemoryCalculator;
        let calc = DistanceMemoryCalculator::new(&self.radar.config);
        calc.memory_requirements()
    }

    /// Updates the calibration dynamically based on new sensor data.
    ///
    /// This function is intended to be used when a recalibration is necessary due to
    /// changes in the operating environment. Buffer size is automatically validated.
    ///
    /// # Errors
    ///
    /// Returns [`SensorError::BufferTooSmall`] if buffer is too small.
    ///
    /// For the unchecked version, see [`update_calibration_unchecked`](Self::update_calibration_unchecked).
    pub async fn update_calibration(
        &mut self,
        sensor_cal_result: &CalibrationResult,
        buffer: &mut [u8],
    ) -> Result<DynamicResult, SensorError> {
        let distances = DistanceSizes::new(&self.inner);

        // Automatic buffer size validation
        if buffer.len() < distances.buffer_size {
            return Err(SensorError::BufferTooSmall);
        }

        unsafe {
            self.update_calibration_unchecked(sensor_cal_result, buffer)
                .await
        }
    }

    /// Updates calibration without buffer size checks.
    ///
    /// # Safety
    ///
    /// The caller must ensure `buffer.len() >= self.get_distance_buffer_size()`.
    pub async unsafe fn update_calibration_unchecked(
        &mut self,
        sensor_cal_result: &CalibrationResult,
        buffer: &mut [u8],
    ) -> Result<DynamicResult, SensorError> {
        let mut calibration_complete: bool = false;
        let mut detector_cal_result_dynamic = DynamicResult::default();
        let calibration_attempt: bool;

        calibration_attempt = acc_detector_distance_update_calibration(
            self.radar.inner_sensor(),
            self.inner.inner_mut(),
            sensor_cal_result.ptr(),
            buffer.as_mut_ptr() as *mut c_void,
            buffer.len() as u32,
            &mut detector_cal_result_dynamic.inner as *mut acc_detector_cal_result_dynamic_t,
            &mut calibration_complete as *mut bool,
        );

        if calibration_attempt {
            while !calibration_complete {
                // Wait for the interrupt to occur asynchronously
                self.radar
                    .interrupt
                    .wait_for_high()
                    .await
                    .expect("Failed to wait for interrupt");
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

            Ok(detector_cal_result_dynamic)
        } else {
            Err(SensorError::CalibrationFailed)
        }
    }

    /// Prepares the detector for a measurement operation.
    ///
    /// This function must be called before performing a distance measurement to configure
    /// the detector properly. Buffer size is automatically validated.
    ///
    /// # Errors
    ///
    /// Returns [`SensorError::BufferTooSmall`] if buffer is too small.
    ///
    /// For the unchecked version, see [`prepare_detector_unchecked`](Self::prepare_detector_unchecked).
    pub fn prepare_detector(
        &mut self,
        sensor_cal_result: &CalibrationResult,
        buffer: &mut [u8],
    ) -> Result<(), SensorError> {
        let distances = DistanceSizes::new(&self.inner);

        // Automatic buffer size validation
        if buffer.len() < distances.buffer_size {
            return Err(SensorError::BufferTooSmall);
        }

        unsafe { self.prepare_detector_unchecked(sensor_cal_result, buffer) }
    }

    /// Prepares the detector without buffer size checks.
    ///
    /// # Safety
    ///
    /// The caller must ensure `buffer.len() >= self.get_distance_buffer_size()`.
    pub unsafe fn prepare_detector_unchecked(
        &mut self,
        sensor_cal_result: &CalibrationResult,
        buffer: &mut [u8],
    ) -> Result<(), SensorError> {
        if acc_detector_distance_prepare(
            self.inner.inner(),
            self.config.inner.as_ptr(),
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
    /// This function analyzes the raw data collected during a measurement operation,
    /// extracting distance information. Buffer sizes are automatically validated.
    ///
    /// # Errors
    ///
    /// Returns [`ProcessDataError::BufferTooSmall`] if buffers are too small.
    ///
    /// For the unchecked version, see [`process_data_unchecked`](Self::process_data_unchecked).
    pub fn process_data(
        &mut self,
        buffer: &mut [u8],
        detector_cal_result_static: &mut [u8],
        detector_cal_result_dynamic: &mut DynamicResult,
    ) -> Result<DistanceResult<'_>, ProcessDataError> {
        let distances = DistanceSizes::new(&self.inner);

        // Automatic buffer size validation
        if buffer.len() < distances.buffer_size
            || detector_cal_result_static.len() < distances.detector_cal_result_static_size
        {
            return Err(ProcessDataError::BufferTooSmall);
        }

        unsafe {
            self.process_data_unchecked(
                buffer,
                detector_cal_result_static,
                detector_cal_result_dynamic,
            )
        }
    }

    /// Processes data without buffer size checks.
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    /// - `buffer.len() >= self.get_distance_buffer_size()`
    /// - `detector_cal_result_static.len() >= self.get_static_result_buffer_size()`
    pub unsafe fn process_data_unchecked(
        &mut self,
        buffer: &mut [u8],
        detector_cal_result_static: &mut [u8],
        detector_cal_result_dynamic: &mut DynamicResult,
    ) -> Result<DistanceResult<'_>, ProcessDataError> {
        let mut result_available: bool = false;
        let mut distance_result = DistanceResult::new(&self.radar.config);
        let mut distance_result_ptr: acc_detector_distance_result_t = distance_result.inner();

        let process_attempt: bool = acc_detector_distance_process(
            self.inner.inner_mut(),
            buffer.as_mut_ptr() as *mut c_void,
            detector_cal_result_static.as_mut_ptr(),
            &mut detector_cal_result_dynamic.inner as *mut acc_detector_cal_result_dynamic_t,
            &mut result_available as *mut bool,
            &mut distance_result_ptr as *mut acc_detector_distance_result_t,
        );
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
