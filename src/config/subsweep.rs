use crate::config::hwaas::Hwaas;
use crate::config::prf::PulseRepetitionFrequency;
use crate::config::profile::RadarProfile;
use crate::config::RadarConfig;
use crate::rss_bindings::*;

/// Subsweep configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Subsweep {
    index: u8,
}

impl Subsweep {
    pub(super) fn new(index: u8) -> Self {
        Self { index }
    }

    /// Sets start point for subsweep
    /// # Arguments
    /// * `config` - A reference to a `Config` instance.
    /// * `start_point` - The start point for the subsweep.
    pub fn set_start_point(&self, config: &mut RadarConfig, start_point: i32) {
        unsafe { acc_config_subsweep_start_point_set(config.inner, start_point, self.index) };
    }

    /// Gets start point for subsweep
    pub fn start_point(&self, config: &RadarConfig) -> i32 {
        unsafe { acc_config_subsweep_start_point_get(config.inner, self.index) }
    }

    /// Sets number of points for subsweep
    /// # Arguments
    /// * `config` - A reference to a `Config` instance.
    /// * `num_points` - The number of points for the subsweep.
    pub fn set_num_points(&self, config: &mut RadarConfig, num_points: u16) {
        unsafe { acc_config_subsweep_num_points_set(config.inner, num_points, self.index) };
    }

    /// Gets number of points for subsweep
    pub fn num_points(&self, config: &RadarConfig) -> u16 {
        unsafe { acc_config_subsweep_num_points_get(config.inner, self.index) }
    }

    /// Sets step length for subsweep
    /// # Arguments
    /// * `config` - A reference to a `Config` instance.
    /// * `step_length` - The step length for the subsweep.
    pub fn set_step_length(&self, config: &mut RadarConfig, step_length: u16) {
        unsafe { acc_config_subsweep_step_length_set(config.inner, step_length, self.index) };
    }

    /// Gets step length for subsweep
    pub fn step_length(&self, config: &RadarConfig) -> u16 {
        unsafe { acc_config_subsweep_step_length_get(config.inner, self.index) }
    }

    /// Sets profile for subsweep
    /// # Arguments
    /// * `config` - A reference to a `Config` instance.
    /// * `profile` - The profile for the subsweep.
    pub fn set_profile(&self, config: &mut RadarConfig, profile: RadarProfile) {
        unsafe { acc_config_subsweep_profile_set(config.inner, profile as u32, self.index) };
    }

    /// Gets profile for subsweep
    pub fn profile(&self, config: &RadarConfig) -> RadarProfile {
        unsafe { RadarProfile::from(acc_config_subsweep_profile_get(config.inner, self.index)) }
    }

    /// Sets Hardware accelerated average samples for subsweep
    /// # Arguments
    /// * `config` - A reference to a `Config` instance.
    /// * `hwaas` - The Hardware accelerated average samples for the subsweep.
    pub fn set_hwaas(&self, config: &mut RadarConfig, hwaas: Hwaas) {
        unsafe { acc_config_subsweep_hwaas_set(config.inner, hwaas.into(), self.index) };
    }

    /// Gets Hardware accelerated average samples for subsweep
    pub fn hwaas(&self, config: &RadarConfig) -> Hwaas {
        unsafe { Hwaas::try_from(acc_config_subsweep_hwaas_get(config.inner, self.index)).unwrap() }
    }

    /// Sets receiver gain for subsweep
    /// # Arguments
    /// * `config` - A reference to a `Config` instance.
    /// * `gain` - The receiver gain for the subsweep.
    pub fn set_receiver_gain(&self, config: &mut RadarConfig, gain: u8) {
        unsafe { acc_config_subsweep_receiver_gain_set(config.inner, gain, self.index) };
    }

    /// Gets receiver gain for subsweep
    pub fn receiver_gain(&self, config: &RadarConfig) -> u8 {
        unsafe { acc_config_subsweep_receiver_gain_get(config.inner, self.index) }
    }

    /// Sets transmitter enabled for subsweep
    /// # Arguments
    /// * `config` - A reference to a `Config` instance.
    /// * `enable` - The transmitter enabled for the subsweep.
    pub fn set_transmitter_enabled(&self, config: &mut RadarConfig, enable: bool) {
        unsafe { acc_config_subsweep_enable_tx_set(config.inner, enable, self.index) };
    }

    /// Gets transmitter enabled for subsweep
    pub fn is_transmitter_enabled(&self, config: &RadarConfig) -> bool {
        unsafe { acc_config_subsweep_enable_tx_get(config.inner, self.index) }
    }

    /// Sets transmitter power for subsweep
    /// # Arguments
    /// * `config` - A reference to a `Config` instance.
    /// µ 'prf' - Pulse Repetition Frequency for the subsweep.
    pub fn set_prf(&self, config: &mut RadarConfig, prf: PulseRepetitionFrequency) {
        unsafe { acc_config_subsweep_prf_set(config.inner, prf.into(), self.index) };
    }

    /// Gets Pulse Repetition Frequency for subsweep
    pub fn prf(&self, config: &RadarConfig) -> PulseRepetitionFrequency {
        let prf_val = unsafe { acc_config_subsweep_prf_get(config.inner, self.index) };
        PulseRepetitionFrequency::try_from(prf_val).unwrap()
    }

    /// Set the phase enhancement enabled configuration
    pub fn set_phase_enhancement_enabled(&self, config: &mut RadarConfig, enable: bool) {
        unsafe { acc_config_subsweep_phase_enhancement_set(config.inner, enable, self.index) };
    }

    /// Get the phase enhancement enabled configuration
    pub fn is_phase_enhancement_enabled(&self, config: &RadarConfig) -> bool {
        unsafe { acc_config_subsweep_phase_enhancement_get(config.inner, self.index) }
    }

    /// Set the loopback enabled configuration
    pub fn set_loopback_enabled(&self, config: &mut RadarConfig, enable: bool) {
        unsafe { acc_config_subsweep_enable_loopback_set(config.inner, enable, self.index) };
    }

    /// Get the loopback enabled configuration
    pub fn is_loopback_enabled(&self, config: &RadarConfig) -> bool {
        unsafe { acc_config_subsweep_enable_loopback_get(config.inner, self.index) }
    }
}
