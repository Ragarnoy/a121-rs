#![warn(missing_docs)]

use core::num::NonZeroU8;
use core::ptr::NonNull;

use num::Zero;

use error::ConfigError;
use error::ConfigError::ContinuousSweepMode;
use frame_rate::FrameRate;
use profile::RadarProfile;

use crate::config::hwaas::Hwaas;
use crate::config::prf::PulseRepetitionFrequency;
use crate::config::subsweep::Subsweep;
use crate::sensor::error::SensorError;
use a121_sys::*;

/// Module for radar configuration errors
mod error;
/// Module for frame rate values
pub mod frame_rate;
/// Module for hardware accelerated average samples (HWAAS) values
mod hwaas;
/// Module for Pulse Repetition Frequency (PRF) values
pub mod prf;
/// Module for radar profiles
pub mod profile;
/// Module for subsweep configuration
pub mod subsweep;

#[derive(Debug, PartialEq, Clone, Copy)]
/// Idle states for the radar sensor between sweeps or frames.
pub enum RadarIdleState {
    /// Deep sleep state for maximum power saving.
    DeepSleep = 0,
    /// Sleep state with reduced power consumption.
    Sleep,
    /// Ready state for quick start of operations.
    Ready,
}

impl RadarIdleState {
    /// Converts from FFI value, returning `None` for invalid values.
    pub const fn from_ffi(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::DeepSleep),
            1 => Some(Self::Sleep),
            2 => Some(Self::Ready),
            _ => None,
        }
    }
}

/// Enum representing different sweep modes for the radar sensor.
pub enum SweepMode {
    /// Continuous sweep mode with specified constraints.
    Continuous {
        /// Sweep rate in Hz, must be >= 0
        sweep_rate: f32,
    },
    /// Non-continuous (or discrete) sweep mode with different settings.
    Discrete {
        /// Frame rate in Hz
        frame_rate: FrameRate,
        /// Number of sweeps per frame
        sweeps_per_frame: u16,
    },
}

#[derive(Debug)]
/// Radar configuration structure to manage the settings of a radar sensor.
pub struct RadarConfig {
    /// Number of subsweeps in the radar configuration.
    num_subsweep: Option<NonZeroU8>,
    /// Non-null pointer to the radar configuration.
    inner: NonNull<acc_config_t>,
}

impl Drop for RadarConfig {
    /// Destroys the radar configuration instance, freeing any allocated resources.
    fn drop(&mut self) {
        // NonNull guarantees non-null pointer
        unsafe { acc_config_destroy(self.inner.as_ptr()) };
    }
}

impl RadarConfig {
    /// Creates a new radar configuration instance.
    ///
    /// # Errors
    ///
    /// Returns `SensorError::InitFailed` if the configuration could not be created.
    pub fn new() -> Result<Self, SensorError> {
        #[cfg(feature = "defmt")]
        defmt::trace!("Creating radar configuration");
        let ptr = unsafe { acc_config_create() };
        let inner = NonNull::new(ptr).ok_or_else(|| {
            #[cfg(feature = "defmt")]
            defmt::error!("Failed to create radar configuration: acc_config_create returned null");
            SensorError::InitFailed
        })?;
        #[cfg(feature = "defmt")]
        defmt::trace!("Radar configuration created successfully");
        Ok(Self {
            inner,
            num_subsweep: None,
        })
    }

    /// Returns a mutable pointer to the internal radar configuration structure
    /// # Safety
    /// This function is unsafe because it returns a raw pointer.
    pub unsafe fn mut_ptr(&mut self) -> *mut acc_config_t {
        self.inner.as_ptr()
    }

    /// Returns a pointer to the internal radar configuration structure
    pub fn ptr(&self) -> *const acc_config_t {
        self.inner.as_ptr()
    }

    /// Sets the sweep mode for the radar sensor
    /// # Arguments
    /// * `sweep_mode` - The sweep mode to set
    pub fn set_sweep_mode(&mut self, sweep_mode: SweepMode) -> Result<(), ConfigError> {
        match sweep_mode {
            SweepMode::Continuous { sweep_rate } => {
                self.set_continuous_sweep_mode(true)?;
                self.set_sweep_rate(sweep_rate)?;
            }
            SweepMode::Discrete {
                frame_rate,
                sweeps_per_frame,
            } => {
                self.set_continuous_sweep_mode(false)?;
                self.set_frame_rate(frame_rate);
                self.set_sweeps_per_frame(sweeps_per_frame);
            }
        }
        Ok(())
    }

    /// Sets the starting point of the sweep.
    ///
    /// # Arguments
    ///
    /// * `start_point` - The starting point of the sweep in millimeters.
    pub fn set_start_point(&mut self, start_point: i32) {
        unsafe { acc_config_start_point_set(self.inner.as_ptr(), start_point) };
    }

    /// Get the starting point of the sweep.
    pub fn start_point(&self) -> i32 {
        unsafe { acc_config_start_point_get(self.inner.as_ptr()) }
    }

    /// Set the number of data points to measure in a sweep.
    ///
    /// # Arguments
    ///
    /// * `num_points` - Number of data points to measure.
    pub fn set_num_points(&mut self, num_points: u16) {
        unsafe { acc_config_num_points_set(self.inner.as_ptr(), num_points) };
    }

    /// Get the number of data points set to measure in a sweep.
    pub fn num_points(&self) -> u16 {
        unsafe { acc_config_num_points_get(self.inner.as_ptr()) }
    }

    /// Set the step length between each data point in a sweep.
    ///
    /// # Arguments
    ///
    /// * `step_length` - The step length.
    pub fn set_step_length(&mut self, step_length: u16) {
        unsafe { acc_config_step_length_set(self.inner.as_ptr(), step_length) };
    }

    /// Get the current step length between each data point in a sweep.
    pub fn step_length(&self) -> u16 {
        unsafe { acc_config_step_length_get(self.inner.as_ptr()) }
    }

    /// Set the radar profile.
    ///
    /// # Arguments
    ///
    /// * `profile` - The radar profile to set.
    pub fn set_profile(&mut self, profile: RadarProfile) {
        unsafe { acc_config_profile_set(self.inner.as_ptr(), profile as u32) };
    }

    /// Get the currently used radar profile
    pub fn profile(&self) -> RadarProfile {
        unsafe { acc_config_profile_get(self.inner.as_ptr()) }.into()
    }

    /// Set the hardware accelerated average samples (HWAAS).
    ///
    /// Each data point can be sampled several times, and the sensor hardware produces an average value of those samples.
    /// The time needed to measure a sweep is roughly proportional to the number of averaged samples.
    /// Decreasing HWAAS can increase the update rate but may lead to lower SNR.
    ///
    /// HWAAS must be between 1 and 511 inclusive.
    ///
    /// # Arguments
    ///
    /// * `hwaas` - Number of hardware accelerated average samples.
    pub fn set_hwaas(&mut self, hwaas: Hwaas) -> Result<(), ConfigError> {
        unsafe { acc_config_hwaas_set(self.inner.as_ptr(), hwaas.into()) };
        Ok(())
    }

    /// Get the hardware accelerated average samples (HWAAS).
    ///
    /// Returns the number of hardware accelerated average samples currently set.
    pub fn hwaas(&self) -> Hwaas {
        unsafe { acc_config_hwaas_get(self.inner.as_ptr()) }
            .try_into()
            .unwrap()
    }

    /// Set the receiver gain setting.
    ///
    /// Must be a value between 0 and 23 inclusive where 23 is the highest gain and 0 the lowest.
    /// Lower gain gives higher SNR. However, too low gain may result in quantization, lowering SNR.
    /// Too high gain may result in saturation, corrupting the data.
    ///
    /// # Arguments
    ///
    /// * `receiver_gain` - Receiver gain setting.
    pub fn receiver_gain_set(&mut self, receiver_gain: u8) {
        unsafe { acc_config_receiver_gain_set(self.inner.as_ptr(), receiver_gain) };
    }

    /// Get the current receiver gain setting.
    ///
    /// Returns the receiver gain setting. The range is between 0 (lowest gain) and 23 (highest gain).
    pub fn receiver_gain(&self) -> u8 {
        unsafe { acc_config_receiver_gain_get(self.inner.as_ptr()) }
    }

    /// Set the number of sweeps captured in each frame (measurement).
    ///
    /// # Arguments
    ///
    /// * `sweeps_per_frame` - Number of sweeps per frame.
    pub fn set_sweeps_per_frame(&mut self, sweeps_per_frame: u16) {
        unsafe { acc_config_sweeps_per_frame_set(self.inner.as_ptr(), sweeps_per_frame) };
    }

    /// Get the number of sweeps captured in each frame (measurement).
    pub fn sweeps_per_frame(&self) -> u16 {
        unsafe { acc_config_sweeps_per_frame_get(self.inner.as_ptr()) }
    }

    /// Set the Pulse Repetition Frequency (PRF)
    ///
    /// See @ref acc_config_prf_t for details.
    ///
    /// # Arguments
    ///
    /// * `prf` - The Pulse Repetition Frequency to use
    pub fn set_prf(&mut self, prf: PulseRepetitionFrequency) {
        unsafe { acc_config_prf_set(self.inner.as_ptr(), prf as acc_config_prf_t) };
    }

    /// Get the Pulse Repetition Frequency
    ///
    /// Returns the currently set Pulse Repetition Frequency.
    pub fn prf(&self) -> PulseRepetitionFrequency {
        unsafe { acc_config_prf_get(self.inner.as_ptr()) }
            .try_into()
            .unwrap()
    }

    /// Enable or disable phase enhancement
    ///
    /// If enabled, the data phase will be enhanced such that coherent distance filtering can be applied.
    ///
    /// # Arguments
    ///
    /// * `enable` - true to enable phase enhancement, false to disable
    pub fn set_phase_enhancement(&mut self, enable: bool) {
        unsafe { acc_config_phase_enhancement_set(self.inner.as_ptr(), enable) };
    }

    /// Check if phase enhancement is enabled
    ///
    /// Returns true if phase enhancement is enabled.
    pub fn is_phase_enhancement_enabled(&self) -> bool {
        unsafe { acc_config_phase_enhancement_get(self.inner.as_ptr()) }
    }

    /// Enable or disable loopback
    ///
    /// Loopback can't be enabled together with profile 2.
    ///
    /// # Arguments
    ///
    /// * `enable` - true to enable loopback, false otherwise
    pub fn set_loopback(&mut self, enable: bool) {
        unsafe { acc_config_enable_loopback_set(self.inner.as_ptr(), enable) };
    }

    /// Get the enable loopback configuration
    ///
    /// Returns true if loopback is enabled.
    pub fn is_loopback_enabled(&self) -> bool {
        unsafe { acc_config_enable_loopback_get(self.inner.as_ptr()) }
    }

    /// Enable or disable double buffering
    ///
    /// If enabled, the sensor buffer will be split in two halves reducing the
    /// maximum number of samples.
    ///
    /// # Arguments
    ///
    /// * `enable` - true to enable double buffering, false otherwise
    pub fn set_double_buffering(&mut self, enable: bool) {
        unsafe { acc_config_double_buffering_set(self.inner.as_ptr(), enable) };
    }

    /// Get the double buffering configuration
    ///
    /// Returns true if double buffering is enabled.
    pub fn is_double_buffering_enabled(&self) -> bool {
        unsafe { acc_config_double_buffering_get(self.inner.as_ptr()) }
    }

    /// Set the frame rate
    ///
    /// Sets the frame rate.
    ///
    /// # Arguments
    ///
    /// * `frame_rate` - Frame rate in Hz. 0 is interpreted as unlimited
    pub fn set_frame_rate(&mut self, frame_rate: FrameRate) {
        match frame_rate {
            FrameRate::Unlimited => unsafe { acc_config_frame_rate_set(self.inner.as_ptr(), 0.0) },
            FrameRate::Limited(rate) => unsafe {
                acc_config_frame_rate_set(self.inner.as_ptr(), rate)
            },
        }
    }

    /// Get the frame rate
    ///
    /// Returns the currently set frame rate in Hz.
    pub fn frame_rate(&self) -> FrameRate {
        let val = unsafe { acc_config_frame_rate_get(self.inner.as_ptr()) };
        if val.is_zero() {
            FrameRate::Unlimited
        } else {
            FrameRate::Limited(val)
        }
    }

    /// Enable or disable the transmitter
    ///
    /// # Arguments
    ///
    /// * `enable` - true to enable the transmitter, false to disable it
    pub fn set_transmitter_enabled(&mut self, enable: bool) {
        unsafe { acc_config_enable_tx_set(self.inner.as_ptr(), enable) };
    }

    /// Get transmitter enable configuration
    ///
    /// Returns true if the transmitter is enabled.
    pub fn is_transmitter_enabled(&self) -> bool {
        unsafe { acc_config_enable_tx_get(self.inner.as_ptr()) }
    }

    /// Set inter frame idle state
    ///
    /// # Arguments
    ///
    /// * `idle_state` - The idle state to use between frames
    pub fn set_inter_frame_idle_state(&mut self, idle_state: RadarIdleState) {
        unsafe { acc_config_inter_frame_idle_state_set(self.inner.as_ptr(), idle_state as u32) };
    }

    /// Get inter frame idle state
    ///
    /// Returns the currently set idle state used between frames.
    /// Defaults to `DeepSleep` if FFI returns an invalid value.
    pub fn inter_frame_idle_state(&self) -> RadarIdleState {
        let val = unsafe { acc_config_inter_frame_idle_state_get(self.inner.as_ptr()) };
        RadarIdleState::from_ffi(val).unwrap_or(RadarIdleState::DeepSleep)
    }

    /// Set inter sweep idle state
    ///
    /// # Arguments
    ///
    /// * `idle_state` - The idle state to use between sweeps within a frame
    pub fn set_inter_sweep_idle_state(&mut self, idle_state: RadarIdleState) {
        unsafe { acc_config_inter_sweep_idle_state_set(self.inner.as_ptr(), idle_state as u32) };
    }

    /// Get inter sweep idle state
    ///
    /// Returns the currently set idle state used between sweeps within a frame.
    /// Defaults to `DeepSleep` if FFI returns an invalid value.
    pub fn inter_sweep_idle_state(&self) -> RadarIdleState {
        let val = unsafe { acc_config_inter_sweep_idle_state_get(self.inner.as_ptr()) };
        RadarIdleState::from_ffi(val).unwrap_or(RadarIdleState::DeepSleep)
    }

    /// Set continuous sweep mode.
    ///
    /// In continuous sweep mode, the timing is identical over all sweeps, not just the sweeps in a frame.
    /// Enabling continuous sweep mode imposes certain constraints:
    /// - Frame rate must be set to unlimited (0.0 Hz).
    /// - Sweep rate must be set (a value greater than 0 Hz).
    /// - Inter-frame and inter-sweep idle states must be the same.
    ///
    /// # Arguments
    ///
    /// * `enabled` - true to enable continuous sweep mode, false to disable it.
    fn set_continuous_sweep_mode(&mut self, enabled: bool) -> Result<(), ConfigError> {
        if enabled {
            if self.frame_rate().is_limited() {
                return Err(ContinuousSweepMode);
            }
            if self.sweep_rate().is_zero() {
                return Err(ContinuousSweepMode);
            }
            if self.inter_frame_idle_state() != self.inter_sweep_idle_state() {
                return Err(ContinuousSweepMode);
            }
        }
        unsafe { acc_config_continuous_sweep_mode_set(self.inner.as_ptr(), enabled) };
        Ok(())
    }

    /// Get continuous sweep mode
    ///
    /// Returns true if continuous sweep mode is enabled.
    pub fn is_continuous_sweep_mode_enabled(&self) -> bool {
        unsafe { acc_config_continuous_sweep_mode_get(self.inner.as_ptr()) }
    }

    /// Set the sweep rate
    ///
    /// # Arguments
    ///
    /// * `sweep_rate` - Sweep rate in Hz. Must be >= 0, 0 is interpreted as max sweep rate
    fn set_sweep_rate(&mut self, sweep_rate: f32) -> Result<(), ConfigError> {
        if sweep_rate.is_zero() || sweep_rate.is_sign_negative() {
            return Err(ConfigError::SweepRate);
        }
        unsafe { acc_config_sweep_rate_set(self.inner.as_ptr(), sweep_rate) };
        Ok(())
    }

    /// Get the sweep rate
    ///
    /// Returns the currently set sweep rate in Hz.
    pub fn sweep_rate(&self) -> f32 {
        unsafe { acc_config_sweep_rate_get(self.inner.as_ptr()) }
    }

    /// Set the number of subsweeps in the radar configuration.
    /// # Arguments
    /// * `num_subsweep` - The number of subsweeps to set
    pub fn set_num_subsweep(&mut self, num_subsweep: u8) -> Result<(), ConfigError> {
        if num_subsweep == 0 {
            return Err(ConfigError::NumSubsweep);
        }
        unsafe { acc_config_num_subsweeps_set(self.inner.as_ptr(), num_subsweep) };
        self.num_subsweep = NonZeroU8::new(num_subsweep);
        Ok(())
    }

    /// Get the number of subsweeps in the radar configuration.
    pub fn num_subsweep(&self) -> u8 {
        unsafe { acc_config_num_subsweeps_get(self.inner.as_ptr()) }
    }

    /// Get a subsweep from the radar configuration.
    /// # Arguments
    /// * `index` - The index of the subsweep to get
    /// # Returns
    /// * `Some(Subsweep)` - The subsweep at the given index
    /// * `None` - If the index is out of bounds
    pub fn get_subsweep(&self, index: u8) -> Option<Subsweep> {
        if index >= self.num_subsweep() {
            return None;
        }
        Some(Subsweep::new(index))
    }

    /// Get the buffer size needed for the current configuration
    /// # Returns
    /// * `Ok(u32)` - The buffer size needed for the current configuration
    /// * `Err(ConfigError::BufferSize)` - If the buffer size could not be determined
    pub fn config_buffer_size(&self) -> Result<u32, ConfigError> {
        use core::mem::MaybeUninit;

        let mut buffer_size = MaybeUninit::<u32>::uninit();

        let result =
            unsafe { acc_rss_get_buffer_size(self.inner.as_ptr(), buffer_size.as_mut_ptr()) };

        if result {
            Ok(unsafe { buffer_size.assume_init() })
        } else {
            Err(ConfigError::BufferSize)
        }
    }
}
