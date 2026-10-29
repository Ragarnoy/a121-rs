//! Presence Detection Module
//!
//! Provides an API for presence detection using radar technology. This module includes
//! functionality to create and configure presence detectors, prepare for
//! measurements, detect presence, and more.
//!
//! For a detailed description of the algorithm and its parameters, see the Acconeer documentation.

#![warn(missing_docs)]

use core::ops::RangeInclusive;
use core::ptr::NonNull;

use a121_sys::*;

use crate::config::profile::RadarProfile;
use crate::config::{Hwaas, RadarIdleState};
use crate::sensor::error::SensorError;

/// Type alias for the signal quality
pub type SignalQuality = f32;

/// Configuration for the radar presence detection.
pub struct PresenceConfig {
    /// Non-null pointer to the inner presence detector configuration.
    pub(super) inner: NonNull<acc_detector_presence_config>,
}

impl Drop for PresenceConfig {
    fn drop(&mut self) {
        // NonNull guarantees non-null pointer
        unsafe { acc_detector_presence_config_destroy(self.inner.as_ptr()) }
    }
}

impl PresenceConfig {
    /// Creates a presence configuration with the SDK defaults.
    ///
    /// # Errors
    ///
    /// Returns [`SensorError::InitFailed`] if the SDK could not allocate it.
    pub fn new() -> Result<Self, SensorError> {
        let ptr = unsafe { acc_detector_presence_config_create() };
        let inner = NonNull::new(ptr).ok_or(SensorError::InitFailed)?;
        Ok(Self { inner })
    }

    /// Sets the measurement range in meters.
    pub fn set_range(&mut self, range: RangeInclusive<f32>) {
        unsafe {
            acc_detector_presence_config_start_set(self.inner.as_ptr(), *range.start());
            acc_detector_presence_config_end_set(self.inner.as_ptr(), *range.end());
        }
    }

    /// Sets the step length based on profile or manually.
    ///
    /// Pass `Some(length)` for manual step length, or `None` for automatic.
    pub fn set_step_length(&mut self, step_length: Option<u16>) {
        match step_length {
            Some(length) => unsafe {
                acc_detector_presence_config_step_length_set(self.inner.as_ptr(), length);
                acc_detector_presence_config_auto_step_length_set(self.inner.as_ptr(), false);
            },
            None => unsafe {
                acc_detector_presence_config_auto_step_length_set(self.inner.as_ptr(), true)
            },
        }
    }

    /// Sets the sensor ID.
    pub fn set_sensor(&mut self, sensor_id: u32) {
        unsafe { acc_detector_presence_config_sensor_set(self.inner.as_ptr(), sensor_id) }
    }

    /// Enables or disables automatic profile selection.
    pub fn set_auto_profile(&mut self, enable: bool) {
        unsafe { acc_detector_presence_config_auto_profile_set(self.inner.as_ptr(), enable) }
    }

    /// Sets the profile for presence detection.
    ///
    /// The SDK only uses the profile when automatic profile selection is disabled,
    /// so this also disables it.
    pub fn set_profile(&mut self, profile: RadarProfile) {
        unsafe {
            acc_detector_presence_config_auto_profile_set(self.inner.as_ptr(), false);
            acc_detector_presence_config_profile_set(self.inner.as_ptr(), profile as u32)
        }
    }

    /// Configures frame rate for presence detection.
    pub fn set_frame_rate(&mut self, frame_rate: f32) {
        unsafe { acc_detector_presence_config_frame_rate_set(self.inner.as_ptr(), frame_rate) }
    }

    /// Enables or disables filter reset on prepare.
    pub fn set_reset_filters_on_prepare(&mut self, enable: bool) {
        unsafe {
            acc_detector_presence_config_reset_filters_on_prepare_set(self.inner.as_ptr(), enable)
        }
    }

    /// Configures detection thresholds for fast and slow movements.
    ///
    /// # Arguments
    /// * `intra` - Threshold for intra-frame (fast) movement detection
    /// * `inter` - Threshold for inter-frame (slow) movement detection
    pub fn set_detection_thresholds(&mut self, intra: f32, inter: f32) {
        unsafe {
            acc_detector_presence_config_intra_detection_threshold_set(self.inner.as_ptr(), intra);
            acc_detector_presence_config_inter_detection_threshold_set(self.inner.as_ptr(), inter);
        }
    }

    /// Enables or disables intra-frame and inter-frame detection.
    ///
    /// # Arguments
    /// * `intra_enable` - Enable intra-frame (fast movement) detection
    /// * `inter_enable` - Enable inter-frame (slow movement) detection
    pub fn set_detection_enabled(&mut self, intra_enable: bool, inter_enable: bool) {
        unsafe {
            acc_detector_presence_config_intra_detection_set(self.inner.as_ptr(), intra_enable);
            acc_detector_presence_config_inter_detection_set(self.inner.as_ptr(), inter_enable);
        }
    }

    /// Returns the measurement range in meters.
    pub fn range(&self) -> RangeInclusive<f32> {
        unsafe {
            acc_detector_presence_config_start_get(self.inner.as_ptr())
                ..=acc_detector_presence_config_end_get(self.inner.as_ptr())
        }
    }

    /// Returns the manual step length, or `None` when the step length is chosen automatically.
    pub fn step_length(&self) -> Option<u16> {
        unsafe {
            if acc_detector_presence_config_auto_step_length_get(self.inner.as_ptr()) {
                None
            } else {
                Some(acc_detector_presence_config_step_length_get(
                    self.inner.as_ptr(),
                ))
            }
        }
    }

    /// Returns the sensor ID.
    pub fn sensor(&self) -> u32 {
        unsafe { acc_detector_presence_config_sensor_get(self.inner.as_ptr()) }
    }

    /// Returns whether the profile is selected automatically.
    pub fn auto_profile(&self) -> bool {
        unsafe { acc_detector_presence_config_auto_profile_get(self.inner.as_ptr()) }
    }

    /// Returns the profile, which the SDK only uses when automatic profile selection is disabled.
    pub fn profile(&self) -> RadarProfile {
        unsafe { acc_detector_presence_config_profile_get(self.inner.as_ptr()) }.into()
    }

    /// Returns the frame rate in Hz.
    pub fn frame_rate(&self) -> f32 {
        unsafe { acc_detector_presence_config_frame_rate_get(self.inner.as_ptr()) }
    }

    /// Sets whether the application drives the frame rate by calling measure at its own pace,
    /// instead of the sensor measuring at the configured frame rate.
    ///
    /// The frame rate must still be set, since the filters depend on it.
    pub fn set_frame_rate_app_driven(&mut self, enable: bool) {
        unsafe {
            acc_detector_presence_config_frame_rate_app_driven_set(self.inner.as_ptr(), enable)
        }
    }

    /// Returns whether the application drives the frame rate.
    pub fn frame_rate_app_driven(&self) -> bool {
        unsafe { acc_detector_presence_config_frame_rate_app_driven_get(self.inner.as_ptr()) }
    }

    /// Returns whether the filters are reset on prepare.
    pub fn reset_filters_on_prepare(&self) -> bool {
        unsafe { acc_detector_presence_config_reset_filters_on_prepare_get(self.inner.as_ptr()) }
    }

    /// Returns the intra-frame and inter-frame detection thresholds, as `(intra, inter)`.
    pub fn detection_thresholds(&self) -> (f32, f32) {
        unsafe {
            (
                acc_detector_presence_config_intra_detection_threshold_get(self.inner.as_ptr()),
                acc_detector_presence_config_inter_detection_threshold_get(self.inner.as_ptr()),
            )
        }
    }

    /// Returns whether intra-frame and inter-frame detection are enabled, as `(intra, inter)`.
    pub fn detection_enabled(&self) -> (bool, bool) {
        unsafe {
            (
                acc_detector_presence_config_intra_detection_get(self.inner.as_ptr()),
                acc_detector_presence_config_inter_detection_get(self.inner.as_ptr()),
            )
        }
    }

    /// Sets the state the sensor idles in between frames.
    pub fn set_inter_frame_idle_state(&mut self, idle_state: RadarIdleState) {
        unsafe {
            acc_detector_presence_config_inter_frame_idle_state_set(
                self.inner.as_ptr(),
                idle_state as u32,
            )
        }
    }

    /// Returns the state the sensor idles in between frames.
    pub fn inter_frame_idle_state(&self) -> RadarIdleState {
        let value =
            unsafe { acc_detector_presence_config_inter_frame_idle_state_get(self.inner.as_ptr()) };
        RadarIdleState::from_ffi(value).unwrap_or(RadarIdleState::DeepSleep)
    }

    /// Sets the hardware accelerated average samples (HWAAS). Automatic subsweeps override it;
    /// see [`set_automatic_subsweeps`](Self::set_automatic_subsweeps).
    pub fn set_hwaas(&mut self, hwaas: Hwaas) {
        unsafe { acc_detector_presence_config_hwaas_set(self.inner.as_ptr(), hwaas.into()) }
    }

    /// Returns the hardware accelerated average samples (HWAAS).
    pub fn hwaas(&self) -> Hwaas {
        Hwaas::from_sdk(unsafe { acc_detector_presence_config_hwaas_get(self.inner.as_ptr()) })
    }

    /// Sets the number of sweeps in each frame.
    pub fn set_sweeps_per_frame(&mut self, sweeps_per_frame: u16) {
        unsafe {
            acc_detector_presence_config_sweeps_per_frame_set(self.inner.as_ptr(), sweeps_per_frame)
        }
    }

    /// Returns the number of sweeps in each frame.
    pub fn sweeps_per_frame(&self) -> u16 {
        unsafe { acc_detector_presence_config_sweeps_per_frame_get(self.inner.as_ptr()) }
    }

    /// Sets the number of seconds the inter-frame presence score needs to decrease before it
    /// declines faster, from 0 to 30. 0 disables the timeout.
    pub fn set_inter_frame_presence_timeout(&mut self, seconds: u16) {
        unsafe {
            acc_detector_presence_config_inter_frame_presence_timeout_set(
                self.inner.as_ptr(),
                seconds,
            )
        }
    }

    /// Returns the inter-frame presence timeout in seconds.
    pub fn inter_frame_presence_timeout(&self) -> u16 {
        unsafe {
            acc_detector_presence_config_inter_frame_presence_timeout_get(self.inner.as_ptr())
        }
    }

    /// Sets the time constant in seconds of the low pass filter for the inter-frame deviation
    /// between the fast and slow filters.
    pub fn set_inter_frame_deviation_time_const(&mut self, time_const: f32) {
        unsafe {
            acc_detector_presence_config_inter_frame_deviation_time_const_set(
                self.inner.as_ptr(),
                time_const,
            )
        }
    }

    /// Returns the time constant in seconds of the inter-frame deviation filter.
    pub fn inter_frame_deviation_time_const(&self) -> f32 {
        unsafe {
            acc_detector_presence_config_inter_frame_deviation_time_const_get(self.inner.as_ptr())
        }
    }

    /// Sets the cutoff frequency in Hz of the fast inter-frame low pass filter. Above half the
    /// frame rate, no filtering is applied.
    pub fn set_inter_frame_fast_cutoff(&mut self, cutoff: f32) {
        unsafe {
            acc_detector_presence_config_inter_frame_fast_cutoff_set(self.inner.as_ptr(), cutoff)
        }
    }

    /// Returns the cutoff frequency in Hz of the fast inter-frame filter.
    pub fn inter_frame_fast_cutoff(&self) -> f32 {
        unsafe { acc_detector_presence_config_inter_frame_fast_cutoff_get(self.inner.as_ptr()) }
    }

    /// Sets the cutoff frequency in Hz of the slow inter-frame low pass filter.
    pub fn set_inter_frame_slow_cutoff(&mut self, cutoff: f32) {
        unsafe {
            acc_detector_presence_config_inter_frame_slow_cutoff_set(self.inner.as_ptr(), cutoff)
        }
    }

    /// Returns the cutoff frequency in Hz of the slow inter-frame filter.
    pub fn inter_frame_slow_cutoff(&self) -> f32 {
        unsafe { acc_detector_presence_config_inter_frame_slow_cutoff_get(self.inner.as_ptr()) }
    }

    /// Sets the time constant in seconds of the depthwise intra-frame filter.
    pub fn set_intra_frame_time_const(&mut self, time_const: f32) {
        unsafe {
            acc_detector_presence_config_intra_frame_time_const_set(self.inner.as_ptr(), time_const)
        }
    }

    /// Returns the time constant in seconds of the depthwise intra-frame filter.
    pub fn intra_frame_time_const(&self) -> f32 {
        unsafe { acc_detector_presence_config_intra_frame_time_const_get(self.inner.as_ptr()) }
    }

    /// Sets the time constant in seconds of the intra-frame output filter.
    pub fn set_intra_output_time_const(&mut self, time_const: f32) {
        unsafe {
            acc_detector_presence_config_intra_output_time_const_set(
                self.inner.as_ptr(),
                time_const,
            )
        }
    }

    /// Returns the time constant in seconds of the intra-frame output filter.
    pub fn intra_output_time_const(&self) -> f32 {
        unsafe { acc_detector_presence_config_intra_output_time_const_get(self.inner.as_ptr()) }
    }

    /// Sets the time constant in seconds of the inter-frame output filter.
    pub fn set_inter_output_time_const(&mut self, time_const: f32) {
        unsafe {
            acc_detector_presence_config_inter_output_time_const_set(
                self.inner.as_ptr(),
                time_const,
            )
        }
    }

    /// Returns the time constant in seconds of the inter-frame output filter.
    pub fn inter_output_time_const(&self) -> f32 {
        unsafe { acc_detector_presence_config_inter_output_time_const_get(self.inner.as_ptr()) }
    }

    /// Sets whether the detector uses subsweeps chosen automatically for the range. They override
    /// other settings, such as HWAAS and the step length.
    pub fn set_automatic_subsweeps(&mut self, enable: bool) {
        unsafe { acc_detector_presence_config_automatic_subsweeps_set(self.inner.as_ptr(), enable) }
    }

    /// Returns whether the detector picks its subsweeps automatically.
    pub fn automatic_subsweeps(&self) -> bool {
        unsafe { acc_detector_presence_config_automatic_subsweeps_get(self.inner.as_ptr()) }
    }

    /// Sets the signal quality the automatic subsweeps aim for. Only used with
    /// [automatic subsweeps](Self::set_automatic_subsweeps).
    pub fn set_signal_quality(&mut self, signal_quality: SignalQuality) {
        unsafe {
            acc_detector_presence_config_signal_quality_set(self.inner.as_ptr(), signal_quality)
        }
    }

    /// Returns the signal quality the automatic subsweeps aim for.
    pub fn signal_quality(&self) -> SignalQuality {
        unsafe { acc_detector_presence_config_signal_quality_get(self.inner.as_ptr()) }
    }

    /// Prints the configuration through the SDK logger.
    pub fn log(&self) {
        unsafe { acc_detector_presence_config_log(self.inner.as_ptr()) }
    }
}

#[cfg(all(test, feature = "stub_library"))]
mod tests {
    use super::*;

    #[test]
    fn create_failure_is_an_error() {
        // The stub acc_detector_presence_config_create returns NULL
        assert!(matches!(
            PresenceConfig::new(),
            Err(SensorError::InitFailed)
        ));
    }
}
