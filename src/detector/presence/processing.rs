//! Presence processing without the presence detector.
//!
//! [`PresenceProcessing`] runs the presence algorithm on frames the application measured itself
//! with a [`Radar`](crate::radar::Radar) prepared for a [`RadarConfig`], for example to look for
//! presence in the frames of another use of the sensor. Get the frames with
//! [`Radar::process_frame`](crate::radar::Radar::process_frame).

#![warn(missing_docs)]

use core::ffi::c_void;
use core::ptr::NonNull;

use a121_sys::*;

use crate::config::RadarConfig;
use crate::memory::total_points;
use crate::num::AccComplex;
use crate::radar::check_buffer;
use crate::sensor::error::SensorError;

/// Configuration of the presence processing.
///
/// Set its [frame rate](Self::set_frame_rate) to the frame rate the frames are measured at.
pub struct PresenceProcessingConfig {
    inner: NonNull<acc_detector_presence_processing_config>,
}

impl Drop for PresenceProcessingConfig {
    fn drop(&mut self) {
        // NonNull guarantees non-null pointer
        unsafe { acc_detector_presence_processing_config_destroy(self.inner.as_ptr()) }
    }
}

impl PresenceProcessingConfig {
    /// Creates a presence processing configuration with the SDK defaults.
    ///
    /// # Errors
    ///
    /// Returns [`SensorError::InitFailed`] if the SDK could not allocate it.
    pub fn new() -> Result<Self, SensorError> {
        let ptr = unsafe { acc_detector_presence_processing_config_create() };
        let inner = NonNull::new(ptr).ok_or(SensorError::InitFailed)?;
        Ok(Self { inner })
    }

    /// Sets the frame rate in Hz the frames are measured at. Must be above 0.
    pub fn set_frame_rate(&mut self, frame_rate: f32) {
        unsafe {
            acc_detector_presence_processing_config_frame_rate_set(self.inner.as_ptr(), frame_rate)
        }
    }

    /// Returns the frame rate in Hz.
    pub fn frame_rate(&self) -> f32 {
        unsafe { acc_detector_presence_processing_config_frame_rate_get(self.inner.as_ptr()) }
    }

    /// Sets the number of seconds the inter-frame presence score needs to decrease before it
    /// declines faster, from 0 to 30. 0 disables the timeout.
    pub fn set_inter_frame_presence_timeout(&mut self, seconds: u16) {
        unsafe {
            acc_detector_presence_processing_config_inter_frame_presence_timeout_set(
                self.inner.as_ptr(),
                seconds,
            )
        }
    }

    /// Returns the inter-frame presence timeout in seconds.
    pub fn inter_frame_presence_timeout(&self) -> u16 {
        unsafe {
            acc_detector_presence_processing_config_inter_frame_presence_timeout_get(
                self.inner.as_ptr(),
            )
        }
    }

    /// Enables or disables intra-frame detection, which detects fast movements inside frames.
    pub fn set_intra_detection(&mut self, enable: bool) {
        unsafe {
            acc_detector_presence_processing_config_intra_detection_set(self.inner.as_ptr(), enable)
        }
    }

    /// Returns whether intra-frame detection is enabled.
    pub fn intra_detection(&self) -> bool {
        unsafe { acc_detector_presence_processing_config_intra_detection_get(self.inner.as_ptr()) }
    }

    /// Sets the intra-frame detection threshold.
    pub fn set_intra_detection_threshold(&mut self, threshold: f32) {
        unsafe {
            acc_detector_presence_processing_config_intra_detection_threshold_set(
                self.inner.as_ptr(),
                threshold,
            )
        }
    }

    /// Returns the intra-frame detection threshold.
    pub fn intra_detection_threshold(&self) -> f32 {
        unsafe {
            acc_detector_presence_processing_config_intra_detection_threshold_get(
                self.inner.as_ptr(),
            )
        }
    }

    /// Enables or disables inter-frame detection, which detects slow movements between frames.
    pub fn set_inter_detection(&mut self, enable: bool) {
        unsafe {
            acc_detector_presence_processing_config_inter_detection_set(self.inner.as_ptr(), enable)
        }
    }

    /// Returns whether inter-frame detection is enabled.
    pub fn inter_detection(&self) -> bool {
        unsafe { acc_detector_presence_processing_config_inter_detection_get(self.inner.as_ptr()) }
    }

    /// Sets the inter-frame detection threshold.
    pub fn set_inter_detection_threshold(&mut self, threshold: f32) {
        unsafe {
            acc_detector_presence_processing_config_inter_detection_threshold_set(
                self.inner.as_ptr(),
                threshold,
            )
        }
    }

    /// Returns the inter-frame detection threshold.
    pub fn inter_detection_threshold(&self) -> f32 {
        unsafe {
            acc_detector_presence_processing_config_inter_detection_threshold_get(
                self.inner.as_ptr(),
            )
        }
    }

    /// Sets the time constant in seconds of the low pass filter for the inter-frame deviation
    /// between the fast and slow filters.
    pub fn set_inter_frame_deviation_time_const(&mut self, time_const: f32) {
        unsafe {
            acc_detector_presence_processing_config_inter_frame_deviation_time_const_set(
                self.inner.as_ptr(),
                time_const,
            )
        }
    }

    /// Returns the time constant in seconds of the inter-frame deviation filter.
    pub fn inter_frame_deviation_time_const(&self) -> f32 {
        unsafe {
            acc_detector_presence_processing_config_inter_frame_deviation_time_const_get(
                self.inner.as_ptr(),
            )
        }
    }

    /// Sets the cutoff frequency in Hz of the fast inter-frame low pass filter. Above half the
    /// frame rate, no filtering is applied.
    pub fn set_inter_frame_fast_cutoff(&mut self, cutoff: f32) {
        unsafe {
            acc_detector_presence_processing_config_inter_frame_fast_cutoff_set(
                self.inner.as_ptr(),
                cutoff,
            )
        }
    }

    /// Returns the cutoff frequency in Hz of the fast inter-frame filter.
    pub fn inter_frame_fast_cutoff(&self) -> f32 {
        unsafe {
            acc_detector_presence_processing_config_inter_frame_fast_cutoff_get(self.inner.as_ptr())
        }
    }

    /// Sets the cutoff frequency in Hz of the slow inter-frame low pass filter.
    pub fn set_inter_frame_slow_cutoff(&mut self, cutoff: f32) {
        unsafe {
            acc_detector_presence_processing_config_inter_frame_slow_cutoff_set(
                self.inner.as_ptr(),
                cutoff,
            )
        }
    }

    /// Returns the cutoff frequency in Hz of the slow inter-frame filter.
    pub fn inter_frame_slow_cutoff(&self) -> f32 {
        unsafe {
            acc_detector_presence_processing_config_inter_frame_slow_cutoff_get(self.inner.as_ptr())
        }
    }

    /// Sets the time constant in seconds of the depthwise intra-frame filter.
    pub fn set_intra_frame_time_const(&mut self, time_const: f32) {
        unsafe {
            acc_detector_presence_processing_config_intra_frame_time_const_set(
                self.inner.as_ptr(),
                time_const,
            )
        }
    }

    /// Returns the time constant in seconds of the depthwise intra-frame filter.
    pub fn intra_frame_time_const(&self) -> f32 {
        unsafe {
            acc_detector_presence_processing_config_intra_frame_time_const_get(self.inner.as_ptr())
        }
    }

    /// Sets the time constant in seconds of the intra-frame output filter.
    pub fn set_intra_output_time_const(&mut self, time_const: f32) {
        unsafe {
            acc_detector_presence_processing_config_intra_output_time_const_set(
                self.inner.as_ptr(),
                time_const,
            )
        }
    }

    /// Returns the time constant in seconds of the intra-frame output filter.
    pub fn intra_output_time_const(&self) -> f32 {
        unsafe {
            acc_detector_presence_processing_config_intra_output_time_const_get(self.inner.as_ptr())
        }
    }

    /// Sets the time constant in seconds of the inter-frame output filter.
    pub fn set_inter_output_time_const(&mut self, time_const: f32) {
        unsafe {
            acc_detector_presence_processing_config_inter_output_time_const_set(
                self.inner.as_ptr(),
                time_const,
            )
        }
    }

    /// Returns the time constant in seconds of the inter-frame output filter.
    pub fn inter_output_time_const(&self) -> f32 {
        unsafe {
            acc_detector_presence_processing_config_inter_output_time_const_get(self.inner.as_ptr())
        }
    }
}

/// The presence found in one frame by [`PresenceProcessing::process`].
#[derive(Debug, Clone, Copy)]
pub struct PresenceProcessingResult<'a> {
    /// Whether presence was detected
    pub presence_detected: bool,
    /// Intra-frame presence score (fast movements)
    pub intra_presence_score: f32,
    /// Inter-frame presence score (slow movements)
    pub inter_presence_score: f32,
    /// Estimated distance to the presence in meters
    pub presence_distance: f32,
    /// Intra-frame presence score at each distance point
    pub depthwise_intra_presence_scores: &'a [f32],
    /// Inter-frame presence score at each distance point
    pub depthwise_inter_presence_scores: &'a [f32],
}

/// Presence processing for the frames of one [`RadarConfig`].
pub struct PresenceProcessing {
    inner: NonNull<acc_detector_presence_processing_handle>,
    /// Size of the working memory `process` needs
    buffer_size: usize,
    /// Number of samples in a frame
    frame_len: usize,
}

impl Drop for PresenceProcessing {
    fn drop(&mut self) {
        // NonNull guarantees non-null pointer
        unsafe { acc_detector_presence_processing_destroy(self.inner.as_ptr()) }
    }
}

impl PresenceProcessing {
    /// Creates the presence processing for frames measured with `sensor_config`.
    ///
    /// The processing copies what it needs, so both configurations can be dropped or changed
    /// afterwards. Frames from a changed configuration need a new processing.
    ///
    /// # Errors
    ///
    /// Returns [`SensorError::InitFailed`] if the SDK could not create it, for example for a
    /// configuration it does not support.
    pub fn new(
        config: &PresenceProcessingConfig,
        sensor_config: &RadarConfig,
    ) -> Result<Self, SensorError> {
        let ptr = unsafe {
            acc_detector_presence_processing_create(config.inner.as_ptr(), sensor_config.ptr())
        };
        let inner = NonNull::new(ptr).ok_or(SensorError::InitFailed)?;
        let buffer_size =
            unsafe { acc_detector_presence_processing_get_buffer_size(sensor_config.ptr()) };
        let frame_len = total_points(sensor_config) * sensor_config.sweeps_per_frame() as usize;
        Ok(Self {
            inner,
            buffer_size: buffer_size as usize,
            frame_len,
        })
    }

    /// Size in bytes of the working memory [`process`](Self::process) needs.
    pub fn buffer_size(&self) -> usize {
        self.buffer_size
    }

    /// Clears the processing's filters, for example after a pause in the frames.
    ///
    /// # Errors
    ///
    /// Returns [`SensorError::ProcessingFailed`] if the SDK could not reset them.
    pub fn reset(&mut self) -> Result<(), SensorError> {
        if unsafe { acc_detector_presence_processing_reset(self.inner.as_ptr()) } {
            Ok(())
        } else {
            Err(SensorError::ProcessingFailed)
        }
    }

    /// Looks for presence in `frame`, using `buffer` as working memory.
    ///
    /// `frame` is a frame of the configuration the processing was created for, as returned by
    /// [`Radar::process_frame`](crate::radar::Radar::process_frame).
    ///
    /// # Errors
    ///
    /// Returns [`SensorError::BufferTooSmall`] if `buffer` is shorter than
    /// [`buffer_size`](Self::buffer_size) or `frame` is shorter than a frame, and
    /// [`SensorError::BufferMisaligned`] if `buffer` is not aligned for any built-in type; see
    /// [`AlignedBuffer`](crate::AlignedBuffer). Returns [`SensorError::ProcessingFailed`] if the
    /// SDK could not process the frame.
    pub fn process<'a>(
        &'a mut self,
        buffer: &'a mut [u8],
        frame: &[AccComplex],
    ) -> Result<PresenceProcessingResult<'a>, SensorError> {
        check_buffer(buffer, self.buffer_size)?;
        if frame.len() < self.frame_len {
            return Err(SensorError::BufferTooSmall);
        }
        let mut result =
            core::mem::MaybeUninit::<acc_detector_presence_processing_result_t>::zeroed();
        let processed = unsafe {
            acc_detector_presence_processing_process(
                self.inner.as_ptr(),
                buffer.as_mut_ptr() as *mut c_void,
                frame.as_ptr().cast(),
                result.as_mut_ptr(),
            )
        };
        if !processed {
            return Err(SensorError::ProcessingFailed);
        }
        let result = unsafe { result.assume_init() };
        let len = result.depthwise_presence_scores_length as usize;
        // SAFETY: the scores live in the processing's memory or in `buffer`, both borrowed for
        // `'a`, until the next call
        let scores = |ptr: *mut f32| {
            if ptr.is_null() {
                &[][..]
            } else {
                unsafe { core::slice::from_raw_parts(ptr as *const f32, len) }
            }
        };
        Ok(PresenceProcessingResult {
            presence_detected: result.presence_detected,
            intra_presence_score: result.intra_presence_score,
            inter_presence_score: result.inter_presence_score,
            presence_distance: result.presence_distance,
            depthwise_intra_presence_scores: scores(result.depthwise_intra_presence_scores),
            depthwise_inter_presence_scores: scores(result.depthwise_inter_presence_scores),
        })
    }
}

#[cfg(all(test, feature = "stub_library"))]
mod tests {
    use super::*;

    #[test]
    fn create_failure_is_an_error() {
        // The stub acc_detector_presence_processing_config_create returns NULL
        assert!(matches!(
            PresenceProcessingConfig::new(),
            Err(SensorError::InitFailed)
        ));
    }
}
