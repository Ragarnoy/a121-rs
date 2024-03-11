//! Distance Detection Module
//!
//! Provides an API for distance detection using radar technology. This module includes
//! functionality to create and configure distance detectors, calibrate the detector, prepare for
//! measurements, process data, and more.
//!
//! For a detailed description of the algorithm and its parameters, see the Acconeer documentation.

#![warn(missing_docs)]

use crate::config::profile::RadarProfile;
use crate::config::profile::RadarProfile::AccProfile5;
use crate::rss_bindings::*;
use core::ops::RangeInclusive;

/// Type alias for the signal quality
pub type SignalQuality = f32;
/// Type alias for the threshold sensitivity
pub type ThresholdSensitivity = f32;

/// Enum representing the reflector shape
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum ReflectorShape {
    /// Generic reflector shape
    /// This is the default value and represents any non liquid reflector
    #[default]
    Generic = acc_detector_distance_reflector_shape_t_ACC_DETECTOR_DISTANCE_REFLECTOR_SHAPE_GENERIC
        as isize,
    /// Planar reflector shape
    /// This represents a planar reflector, usually water
    Planar = acc_detector_distance_reflector_shape_t_ACC_DETECTOR_DISTANCE_REFLECTOR_SHAPE_PLANAR
        as isize,
}

impl From<u32> for ReflectorShape {
    fn from(value: u32) -> Self {
        match value {
            0 => Self::Generic,
            1 => Self::Planar,
            _ => panic!("Invalid reflector shape"),
        }
    }
}

/// Enum representing the maximum step length
pub enum MaxStepLenght {
    /// Uses the step length based on the profile
    ProfileBased,
    /// Maximum step length in points
    Manual(u16),
}

/// Enum representing the peak sorting method
pub enum PeakSortingMethod {
    /// Closest peak sorting method
    Amplitude =
        acc_detector_distance_peak_sorting_t_ACC_DETECTOR_DISTANCE_PEAK_SORTING_CLOSEST as isize,
    /// Strongest peak sorting method
    Strength =
        acc_detector_distance_peak_sorting_t_ACC_DETECTOR_DISTANCE_PEAK_SORTING_STRONGEST as isize,
}

impl From<u32> for PeakSortingMethod {
    fn from(value: u32) -> Self {
        match value {
            0 => Self::Amplitude,
            1 => Self::Strength,
            _ => panic!("Invalid peak sorting"),
        }
    }
}

/// Enum representing the threshold method
pub enum ThresholdMethod {
    /// Fixed amplitude threshold method
    FixedAmplitude(f32),
    /// Fixed strength threshold method
    FixedStrenght(f32),
    /// Recorded threshold method
    Recorded(u16),
    /// Constant false alarm rate threshold method
    Cfar,
}

/// Configuration for the radar distance detection.
///
/// This struct encapsulates all the parameters and settings for configuring
/// the distance detection functionality of the radar.
pub struct RadarDistanceConfig {
    pub(super) inner: *mut acc_detector_distance_config,
}

impl Drop for RadarDistanceConfig {
    fn drop(&mut self) {
        unsafe { acc_detector_distance_config_destroy(self.inner) }
    }
}

impl Default for RadarDistanceConfig {
    fn default() -> Self {
        Self::balanced()
    }
}

impl RadarDistanceConfig {
    /// Create a new distance detection configuration.
    fn new() -> Self {
        Self {
            inner: unsafe { acc_detector_distance_config_create() },
        }
    }

    /// Create a balanced distance detection configuration.
    pub fn balanced() -> Self {
        let mut config = Self::new();
        config.set_interval(0.2..=3.0);
        config.set_max_step_length(MaxStepLenght::ProfileBased);
        config.set_max_profile(AccProfile5);
        config.set_reflector_shape(ReflectorShape::Generic);
        config.set_peak_sorting_method(PeakSortingMethod::Strength);
        config.set_threshold_method(ThresholdMethod::Cfar);
        config.set_threshold_sensitivity(0.5);
        config.set_signal_quality(15.0);
        config.set_close_range_leakage_cancelation(false);
        config
    }

    /// Sets the sensor ID to be used for detection.
    pub fn sensor_set(&mut self, sensor_id: u32) {
        unsafe { acc_detector_distance_config_sensor_set(self.inner, sensor_id) }
    }

    /// Configures the measurement interval in meters.
    pub fn set_interval(&mut self, range: RangeInclusive<f32>) {
        self.set_start_interval(*range.start());
        self.set_end_interval(*range.end());
    }

    /// Sets the start of the measurement interval in meters.
    pub fn set_start_interval(&mut self, start_interval: f32) {
        unsafe { acc_detector_distance_config_start_set(self.inner, start_interval) }
    }

    /// Returns the start of the measurement interval in meters.
    pub fn start_interval(&self) -> f32 {
        unsafe { acc_detector_distance_config_start_get(self.inner) }
    }

    /// Sets the end of the measurement interval in meters.
    pub fn set_end_interval(&mut self, end_interval: f32) {
        unsafe { acc_detector_distance_config_end_set(self.inner, end_interval) }
    }

    /// Returns the end of the measurement interval in meters.
    pub fn end_interval(&self) -> f32 {
        unsafe { acc_detector_distance_config_end_get(self.inner) }
    }

    /// Sets the maximum step length in points.
    /// Using a manual maximum step length can have a big impact on memory usage and performance.
    pub fn set_max_step_length(&mut self, max_step_length: MaxStepLenght) {
        match max_step_length {
            MaxStepLenght::ProfileBased => unsafe {
                acc_detector_distance_config_max_step_length_set(self.inner, 0)
            },
            MaxStepLenght::Manual(length) => unsafe {
                acc_detector_distance_config_max_step_length_set(self.inner, length)
            },
        }
    }

    /// Returns the maximum step length in points.
    pub fn max_step_length(&self) -> u16 {
        unsafe { acc_detector_distance_config_max_step_length_get(self.inner) }
    }

    /// Enable or disable close range leakage cancellation.
    /// This feature is used to cancel out the leakage from the close range (< 100mm from the sensor).
    pub fn set_close_range_leakage_cancelation(&mut self, enable: bool) {
        unsafe {
            acc_detector_distance_config_close_range_leakage_cancellation_set(self.inner, enable)
        }
    }

    /// Returns the close range leakage cancellation status.
    pub fn close_range_leakage_cancelation(&self) -> bool {
        unsafe { acc_detector_distance_config_close_range_leakage_cancellation_get(self.inner) }
    }

    /// Sets the signal quality in dB.
    pub fn set_signal_quality(&mut self, signal_quality: SignalQuality) {
        unsafe {
            acc_detector_distance_config_signal_quality_set(
                self.inner,
                signal_quality.clamp(-10.0, 35.0),
            )
        }
    }

    /// Returns the signal quality in dB.
    pub fn signal_quality(&self) -> SignalQuality {
        unsafe { acc_detector_distance_config_signal_quality_get(self.inner) }
    }

    /// Sets the maximum profile to use.
    pub fn set_max_profile(&mut self, max_profile: RadarProfile) {
        unsafe { acc_detector_distance_config_max_profile_set(self.inner, max_profile as u32) }
    }

    /// Returns the maximum profile to use.
    pub fn max_profile(&self) -> RadarProfile {
        unsafe { acc_detector_distance_config_max_profile_get(self.inner) }.into()
    }

    /// Sets the threshold method with the given parameters.
    pub fn set_threshold_method(&mut self, method: ThresholdMethod) {
        match method {
            ThresholdMethod::FixedAmplitude(amp) => unsafe {
                acc_detector_distance_config_threshold_method_set(self.inner, acc_detector_distance_threshold_method_t_ACC_DETECTOR_DISTANCE_THRESHOLD_METHOD_FIXED_AMPLITUDE);
                acc_detector_distance_config_fixed_amplitude_threshold_value_set(self.inner, amp)
            },
            ThresholdMethod::FixedStrenght(str) => unsafe {
                acc_detector_distance_config_threshold_method_set(self.inner, acc_detector_distance_threshold_method_t_ACC_DETECTOR_DISTANCE_THRESHOLD_METHOD_FIXED_STRENGTH);
                acc_detector_distance_config_fixed_strength_threshold_value_set(self.inner, str)
            },
            ThresholdMethod::Recorded(num) => unsafe {
                acc_detector_distance_config_threshold_method_set(self.inner, acc_detector_distance_threshold_method_t_ACC_DETECTOR_DISTANCE_THRESHOLD_METHOD_RECORDED);
                acc_detector_distance_config_num_frames_recorded_threshold_set(self.inner, num)
            },
            ThresholdMethod::Cfar => unsafe {
                acc_detector_distance_config_threshold_method_set(self.inner, acc_detector_distance_threshold_method_t_ACC_DETECTOR_DISTANCE_THRESHOLD_METHOD_CFAR);
            },
        }
    }

    /// Returns the threshold method.
    pub fn threshold_method(&self) -> ThresholdMethod {
        let method = unsafe { acc_detector_distance_config_threshold_method_get(self.inner) };
        match method {
            0 => ThresholdMethod::FixedAmplitude(unsafe {
                acc_detector_distance_config_fixed_amplitude_threshold_value_get(self.inner)
            }),
            1 => ThresholdMethod::FixedStrenght(unsafe {
                acc_detector_distance_config_fixed_strength_threshold_value_get(self.inner)
            }),
            2 => ThresholdMethod::Recorded(unsafe {
                acc_detector_distance_config_num_frames_recorded_threshold_get(self.inner)
            }),
            3 => ThresholdMethod::Cfar,
            _ => panic!("Invalid threshold method"),
        }
    }

    /// Sets the threshold sensitivity.
    pub fn set_threshold_sensitivity(&mut self, sensitivity: ThresholdSensitivity) {
        unsafe {
            acc_detector_distance_config_threshold_sensitivity_set(
                self.inner,
                sensitivity.clamp(0.0, 1.0),
            )
        }
    }

    /// Returns the threshold sensitivity.
    pub fn threshold_sensitivity(&self) -> ThresholdSensitivity {
        unsafe { acc_detector_distance_config_threshold_sensitivity_get(self.inner) }
    }

    /// Sets the peak sorting method.
    pub fn set_peak_sorting_method(&mut self, method: PeakSortingMethod) {
        unsafe { acc_detector_distance_config_peak_sorting_set(self.inner, method as u32) }
    }

    /// Returns the peak sorting method.
    pub fn peak_sorting_method(&self) -> PeakSortingMethod {
        unsafe { acc_detector_distance_config_peak_sorting_get(self.inner) }.into()
    }

    /// Sets the reflector shape.
    pub fn set_reflector_shape(&mut self, shape: ReflectorShape) {
        unsafe { acc_detector_distance_config_reflector_shape_set(self.inner, shape as u32) }
    }

    /// Returns the reflector shape.
    pub fn reflector_shape(&self) -> ReflectorShape {
        unsafe { acc_detector_distance_config_reflector_shape_get(self.inner) }.into()
    }
}
