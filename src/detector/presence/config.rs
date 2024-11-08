//! Presence Detection Module
//!
//! Provides an API for presence detection using radar technology. This module includes
//! functionality to create and configure presence detectors, prepare for
//! measurements, detect presence, and more.
//!
//! For a detailed description of the algorithm and its parameters, see the Acconeer documentation.

#![warn(missing_docs)]

use crate::config::profile::RadarProfile;
use a121_sys::*;
use core::ops::RangeInclusive;

/// Type alias for the signal quality
pub type SignalQuality = f32;

/// Configuration for the radar presence detection.
pub struct PresenceConfig {
    /// Pointer to the inner presence detector configuration.
    pub inner: *mut acc_detector_presence_config,
}

impl Drop for PresenceConfig {
    fn drop(&mut self) {
        debug_assert!(
            !self.inner.is_null(),
            "Presence detector configuration is null"
        );
        unsafe { acc_detector_presence_config_destroy(self.inner) }
    }
}

impl Default for PresenceConfig {
    fn default() -> Self {
        Self {
            inner: unsafe { acc_detector_presence_config_create() },
        }
    }
}

impl PresenceConfig {
    /// Sets the measurement range in meters.
    pub fn set_range(&mut self, range: RangeInclusive<f32>) {
        unsafe {
            acc_detector_presence_config_start_set(self.inner, *range.start());
            acc_detector_presence_config_end_set(self.inner, *range.end());
        }
    }

    /// Sets the step length based on profile or manually.
    pub fn set_step_length(&mut self, step_length: Option<u16>) {
        match step_length {
            Some(length) => unsafe {
                acc_detector_presence_config_step_length_set(self.inner, length);
                acc_detector_presence_config_auto_step_length_set(self.inner, false);
            },
            None => unsafe { acc_detector_presence_config_auto_step_length_set(self.inner, true) },
        }
    }

    /// Sets the sensor ID.
    pub fn sensor_set(&mut self, sensor_id: u32) {
        unsafe { acc_detector_presence_config_sensor_set(self.inner, sensor_id) }
    }

    /// Enables or disables automatic profile selection.
    pub fn auto_profile_set(&mut self, enable: bool) {
        unsafe { acc_detector_presence_config_auto_profile_set(self.inner, enable) }
    }

    /// Sets the profile for presence detection.
    pub fn profile_set(&mut self, profile: RadarProfile) {
        unsafe { acc_detector_presence_config_profile_set(self.inner, profile as u32) }
    }

    /// Configures frame rate for presence detection.
    pub fn frame_rate_set(&mut self, frame_rate: f32) {
        unsafe { acc_detector_presence_config_frame_rate_set(self.inner, frame_rate) }
    }

    /// Enables or disables filter reset on prepare.
    pub fn reset_filters_on_prepare_set(&mut self, enable: bool) {
        unsafe { acc_detector_presence_config_reset_filters_on_prepare_set(self.inner, enable) }
    }

    /// Configures detection thresholds for fast and slow movements.
    pub fn detection_thresholds_set(&mut self, intra: f32, inter: f32) {
        unsafe {
            acc_detector_presence_config_intra_detection_threshold_set(self.inner, intra);
            acc_detector_presence_config_inter_detection_threshold_set(self.inner, inter);
        }
    }

    /// Enables or disables intra-frame and inter-frame detection.
    pub fn detection_enable(&mut self, intra_enable: bool, inter_enable: bool) {
        unsafe {
            acc_detector_presence_config_intra_detection_set(self.inner, intra_enable);
            acc_detector_presence_config_inter_detection_set(self.inner, inter_enable);
        }
    }
}
