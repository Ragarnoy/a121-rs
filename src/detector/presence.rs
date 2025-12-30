pub mod config;
pub mod results;

use crate::detector::presence::config::PresenceConfig;
use crate::detector::presence::results::{PresenceMetadata, PresenceResult, ProcessDataError};
use crate::radar::{Radar, RadarState};
use crate::sensor::calibration::CalibrationResult;
use crate::sensor::error::SensorError;
use a121_sys::*;
use core::ffi::c_void;
use embedded_hal::digital::OutputPin;
use embedded_hal_async::delay::DelayNs;
use embedded_hal_async::digital::Wait;

struct InnerPresenceDetector {
    presence_metadata: PresenceMetadata,
    inner: *mut acc_detector_presence_handle,
}

impl InnerPresenceDetector {
    fn new(config: &PresenceConfig) -> Self {
        let mut presence_metadata = PresenceMetadata::default();
        Self {
            inner: unsafe {
                acc_detector_presence_create(config.inner, presence_metadata.mut_ptr())
            },
            presence_metadata,
        }
    }

    pub fn presence_metadata(&self) -> &PresenceMetadata {
        &self.presence_metadata
    }

    fn inner(&self) -> *const acc_detector_presence_handle {
        self.inner
    }

    fn inner_mut(&mut self) -> *mut acc_detector_presence_handle {
        self.inner
    }
}

impl Drop for InnerPresenceDetector {
    fn drop(&mut self) {
        debug_assert!(!self.inner.is_null(), "Detector handle is null");
        unsafe { acc_detector_presence_destroy(self.inner) }
    }
}

pub struct PresenceDetector<'radar, SINT, ENABLE, DLY>
where
    SINT: Wait,
    ENABLE: OutputPin,
    DLY: DelayNs,
{
    pub radar: &'radar mut Radar<SINT, ENABLE, DLY>,
    inner: InnerPresenceDetector,
    pub config: PresenceConfig,
}

impl<'radar, SINT, ENABLE, DLY> PresenceDetector<'radar, SINT, ENABLE, DLY>
where
    SINT: Wait,
    ENABLE: OutputPin,
    DLY: DelayNs,
{
    /// Creates a new presence detector with default configuration.
    /// Returns an error if the radar is not in Ready state.
    pub fn new(radar: &'radar mut Radar<SINT, ENABLE, DLY>) -> Result<Self, SensorError> {
        if radar.state() != RadarState::Ready {
            return Err(SensorError::NotReady);
        }
        let config = PresenceConfig::default();
        let inner = InnerPresenceDetector::new(&config);
        Ok(Self {
            radar,
            inner,
            config,
        })
    }

    /// Creates a new presence detector with the specified configuration.
    /// Returns an error if the radar is not in Ready state.
    pub fn with_config(
        radar: &'radar mut Radar<SINT, ENABLE, DLY>,
        config: PresenceConfig,
    ) -> Result<Self, SensorError> {
        if radar.state() != RadarState::Ready {
            return Err(SensorError::NotReady);
        }
        let inner = InnerPresenceDetector::new(&config);
        Ok(Self {
            radar,
            inner,
            config,
        })
    }

    pub fn presence_metadata(&self) -> &PresenceMetadata {
        self.inner.presence_metadata()
    }

    /// Prepares the presence detector for measurements.
    ///
    /// Buffer size is automatically validated. For the unchecked version,
    /// see [`prepare_detector_unchecked`](Self::prepare_detector_unchecked).
    ///
    /// # Errors
    ///
    /// Returns [`SensorError::BufferTooSmall`] if buffer is too small.
    pub async fn prepare_detector(
        &mut self,
        sensor_cal_result: &CalibrationResult,
        buffer: &mut [u8],
    ) -> Result<(), SensorError> {
        let buffer_size = self.get_buffer_size();

        // Automatic buffer size validation
        if buffer.len() < buffer_size {
            return Err(SensorError::BufferTooSmall);
        }

        unsafe { self.prepare_detector_unchecked(sensor_cal_result, buffer).await }
    }

    /// Prepares the detector without buffer size checks.
    ///
    /// # Safety
    ///
    /// The caller must ensure `buffer.len() >= self.get_buffer_size()`.
    pub async unsafe fn prepare_detector_unchecked(
        &mut self,
        sensor_cal_result: &CalibrationResult,
        buffer: &mut [u8],
    ) -> Result<(), SensorError> {
        let prepare_success = acc_detector_presence_prepare(
            self.inner.inner_mut(),
            self.config.inner,
            self.radar.inner_sensor(),
            sensor_cal_result.ptr(),
            buffer.as_mut_ptr() as *mut c_void,
            buffer.len() as u32,
        );

        if prepare_success {
            Ok(())
        } else {
            Err(SensorError::PrepareFailed)
        }
    }

    pub fn get_buffer_size(&self) -> usize {
        let mut buffer_size: u32 = 0;
        unsafe {
            acc_detector_presence_get_buffer_size(self.inner.inner(), &mut buffer_size as *mut u32);
        }
        buffer_size as usize
    }

    /// Estimates memory requirements for this presence detector configuration.
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
    /// # use a121_rs::detector::presence::PresenceDetector;
    /// # fn example(radar: &mut Radar<impl embedded_hal_async::digital::Wait,
    /// #                             impl embedded_hal::digital::OutputPin,
    /// #                             impl embedded_hal_async::delay::DelayNs>) {
    /// let mut detector = PresenceDetector::new(radar).unwrap();
    /// let mem = detector.estimate_memory_requirements();
    /// println!("Total memory needed: {} bytes", mem.total);
    /// println!("External heap: {} bytes", mem.external_heap);
    /// println!("RSS heap: {} bytes", mem.rss_heap);
    /// # }
    /// ```
    pub fn estimate_memory_requirements(&self) -> crate::memory::MemoryRequirements {
        use crate::memory::PresenceMemoryCalculator;
        let calc = PresenceMemoryCalculator::new(&self.radar.config);
        calc.memory_requirements()
    }

    /// Detects presence with automatic buffer size validation.
    ///
    /// For the unchecked version, see [`detect_presence_unchecked`](Self::detect_presence_unchecked).
    ///
    /// # Errors
    ///
    /// Returns [`ProcessDataError::BufferTooSmall`] if buffer is too small.
    pub async fn detect_presence(
        &'_ mut self,
        buffer: &mut [u8],
    ) -> Result<PresenceResult<'_>, ProcessDataError> {
        let buffer_size = self.get_buffer_size();

        // Automatic buffer size validation
        if buffer.len() < buffer_size {
            return Err(ProcessDataError::BufferTooSmall);
        }

        unsafe { self.detect_presence_unchecked(buffer).await }
    }

    /// Detects presence without buffer size checks.
    ///
    /// # Safety
    ///
    /// The caller must ensure `buffer.len() >= self.get_buffer_size()`.
    pub async unsafe fn detect_presence_unchecked(
        &'_ mut self,
        buffer: &mut [u8],
    ) -> Result<PresenceResult<'_>, ProcessDataError> {
        let mut result = PresenceResult::new();
        let detection_success = acc_detector_presence_process(
            self.inner.inner_mut(),
            buffer.as_mut_ptr() as *mut c_void,
            &mut result.inner() as *mut acc_detector_presence_result_t,
        );

        if detection_success {
            Ok(result)
        } else {
            Err(ProcessDataError::ProcessingFailed)
        }
    }
}
