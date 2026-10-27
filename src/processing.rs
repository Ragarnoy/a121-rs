use core::ffi::c_void;
use core::ptr::NonNull;

use metadata::ProcessingMetaData;

use crate::config::RadarConfig;
use crate::radar::check_buffer;
use crate::sensor::error::SensorError;
use a121_sys::{
    acc_processing_create, acc_processing_destroy, acc_processing_execute, acc_processing_result_t,
    acc_processing_t,
};

pub mod metadata;

/// Result from radar signal processing.
///
/// Contains status flags and sensor temperature. The raw frame data is not
/// included here as it points into the processing buffer and has complex
/// lifetime requirements. For presence/distance detection, the detectors
/// extract the meaningful information from the frame data.
#[derive(Debug, Clone, Copy, Default)]
pub struct ProcessingResult {
    /// Indication of sensor data being saturated, can cause data corruption.
    /// Lower the receiver gain if this indication is set.
    pub data_saturated: bool,
    /// Indication of a delayed frame.
    /// The frame rate might need to be lowered if this indication is set.
    pub frame_delayed: bool,
    /// Indication of calibration needed.
    /// The sensor calibration needs to be redone if this indication is set.
    pub calibration_needed: bool,
    /// Temperature in sensor during measurement (in degree Celsius).
    /// Note that it has poor absolute accuracy and should only be used
    /// for relative temperature measurements.
    pub temperature: i16,
}

impl ProcessingResult {
    /// Creates a new ProcessingResult with default values.
    pub fn new() -> Self {
        Self::default()
    }
}

pub struct Processing {
    inner: NonNull<acc_processing_t>,
    metadata: ProcessingMetaData,
    /// Size of the buffer the SDK reads a frame from
    buffer_size: usize,
}

impl Processing {
    /// Creates the processing for `config`.
    ///
    /// # Errors
    ///
    /// Returns [`SensorError::ProcessingFailed`] when the SDK cannot create it.
    pub fn new(config: &RadarConfig) -> Result<Self, SensorError> {
        let buffer_size = config
            .config_buffer_size()
            .map_err(|_| SensorError::ProcessingFailed)? as usize;
        let mut metadata = ProcessingMetaData::new();
        let ptr = unsafe { acc_processing_create(config.ptr(), metadata.mut_ptr()) };
        let inner = NonNull::new(ptr).ok_or(SensorError::ProcessingFailed)?;
        Ok(Self {
            inner,
            metadata,
            buffer_size,
        })
    }

    /// Size of the buffer a frame is read into and processed from.
    pub fn buffer_size(&self) -> usize {
        self.buffer_size
    }

    pub fn metadata(&self) -> &ProcessingMetaData {
        &self.metadata
    }

    /// Execute processing on the buffer and return the result.
    ///
    /// Note: The raw frame data is written into `buffer` by the SDK.
    /// This method only returns the status flags and temperature.
    ///
    /// # Errors
    ///
    /// Returns [`SensorError::BufferTooSmall`] when `buffer` is shorter than the configuration's
    /// buffer size, and [`SensorError::BufferMisaligned`] when it is not aligned for any built-in
    /// type.
    pub fn execute(&mut self, buffer: &mut [u8]) -> Result<ProcessingResult, SensorError> {
        use core::mem::MaybeUninit;

        check_buffer(buffer, self.buffer_size)?;
        let mut ffi_result = MaybeUninit::<acc_processing_result_t>::uninit();
        unsafe {
            acc_processing_execute(
                self.inner.as_ptr(),
                buffer.as_mut_ptr() as *mut c_void,
                ffi_result.as_mut_ptr(),
            );
            Ok(ProcessingResult::from(ffi_result.assume_init()))
        }
    }
}

impl Drop for Processing {
    fn drop(&mut self) {
        // NonNull guarantees non-null pointer
        unsafe { acc_processing_destroy(self.inner.as_ptr()) }
    }
}

impl From<acc_processing_result_t> for ProcessingResult {
    fn from(result: acc_processing_result_t) -> Self {
        Self {
            data_saturated: result.data_saturated,
            frame_delayed: result.frame_delayed,
            calibration_needed: result.calibration_needed,
            temperature: result.temperature,
            // Note: result.frame pointer is intentionally not stored
            // as it points into the buffer and becomes invalid
        }
    }
}
