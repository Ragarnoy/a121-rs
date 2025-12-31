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

/// Type alias for the signal quality
pub type SignalQuality = f32;

/// Configuration for the radar presence detection.
pub struct PresenceConfig {
    /// Non-null pointer to the inner presence detector configuration.
    pub inner: NonNull<acc_detector_presence_config>,
}

impl Drop for PresenceConfig {
    fn drop(&mut self) {
        // NonNull guarantees non-null pointer
        unsafe { acc_detector_presence_config_destroy(self.inner.as_ptr()) }
    }
}

impl Default for PresenceConfig {
    fn default() -> Self {
        let ptr = unsafe { acc_detector_presence_config_create() };
        Self {
            inner: NonNull::new(ptr).expect("Failed to create presence config"),
        }
    }
}

impl PresenceConfig {
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
    pub fn set_profile(&mut self, profile: RadarProfile) {
        unsafe { acc_detector_presence_config_profile_set(self.inner.as_ptr(), profile as u32) }
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
}
