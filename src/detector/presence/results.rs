use core::marker::PhantomData;

use a121_sys::{
    acc_config_profile_t_ACC_CONFIG_PROFILE_5, acc_detector_presence_metadata_t,
    acc_detector_presence_result_t,
};

use crate::config::profile::RadarProfile;
use crate::config::RadarConfig;
use crate::processing::metadata::ProcessingMetaData;
use crate::processing::ProcessingResult;

/// Represents the results from a presence detection operation.
///
/// The lifetime `'detector` ties this result to the detector that produced it,
/// ensuring the depthwise score pointers remain valid.
pub struct PresenceResult<'detector> {
    /// Whether presence was detected
    pub presence_detected: bool,
    /// Intra-frame presence score (fast movements)
    pub intra_presence_score: f32,
    /// Inter-frame presence score (slow movements)
    pub inter_presence_score: f32,
    /// Estimated distance to detected presence in meters
    pub presence_distance: f32,
    /// Processing result from the radar
    pub processing_result: ProcessingResult,
    // Internal: raw pointers to depthwise scores (owned by detector)
    depthwise_intra_ptr: *const f32,
    depthwise_inter_ptr: *const f32,
    depthwise_len: usize,
    // Ties lifetime to detector
    _marker: PhantomData<&'detector ()>,
}

impl<'detector> PresenceResult<'detector> {
    /// Creates a new empty PresenceResult
    pub fn new() -> Self {
        Self {
            presence_detected: false,
            intra_presence_score: 0.0,
            inter_presence_score: 0.0,
            presence_distance: 0.0,
            processing_result: ProcessingResult::new(),
            depthwise_intra_ptr: core::ptr::null(),
            depthwise_inter_ptr: core::ptr::null(),
            depthwise_len: 0,
            _marker: PhantomData,
        }
    }

    /// Returns the depthwise intra-frame presence scores.
    ///
    /// These scores indicate fast movement detection at each depth point.
    pub fn depthwise_intra_presence_scores(&self) -> &[f32] {
        if self.depthwise_intra_ptr.is_null() || self.depthwise_len == 0 {
            &[]
        } else {
            // SAFETY: Pointer validity is tied to detector lifetime via PhantomData
            unsafe { core::slice::from_raw_parts(self.depthwise_intra_ptr, self.depthwise_len) }
        }
    }

    /// Returns the depthwise inter-frame presence scores.
    ///
    /// These scores indicate slow movement detection at each depth point.
    pub fn depthwise_inter_presence_scores(&self) -> &[f32] {
        if self.depthwise_inter_ptr.is_null() || self.depthwise_len == 0 {
            &[]
        } else {
            // SAFETY: Pointer validity is tied to detector lifetime via PhantomData
            unsafe { core::slice::from_raw_parts(self.depthwise_inter_ptr, self.depthwise_len) }
        }
    }

    /// Returns the number of depthwise score points
    pub fn depthwise_scores_length(&self) -> usize {
        self.depthwise_len
    }

    /// Updates the presence result with data from the detector.
    pub(super) fn update_from_detector_result(&mut self, result: &acc_detector_presence_result_t) {
        self.presence_detected = result.presence_detected;
        self.intra_presence_score = result.intra_presence_score;
        self.inter_presence_score = result.inter_presence_score;
        self.presence_distance = result.presence_distance;
        self.depthwise_len = result.depthwise_presence_scores_length as usize;
        self.depthwise_intra_ptr = result.depthwise_intra_presence_scores;
        self.depthwise_inter_ptr = result.depthwise_inter_presence_scores;
        self.processing_result = ProcessingResult::from(result.processing_result);
    }
}

impl Default for PresenceResult<'_> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct PresenceMetadata {
    inner: acc_detector_presence_metadata_t,
}

impl PresenceMetadata {
    pub fn start_m(&self) -> f32 {
        self.inner.start_m
    }

    pub fn step_length_m(&self) -> f32 {
        self.inner.step_length_m
    }

    pub fn num_points(&self) -> u16 {
        self.inner.num_points
    }

    pub fn profile(&self) -> RadarProfile {
        self.inner.profile.into()
    }

    pub(super) fn mut_ptr(&mut self) -> *mut acc_detector_presence_metadata_t {
        &mut self.inner
    }
}

impl PresenceMetadata {
    /// Create a new PresenceMetadata with proper radar config and processing metadata
    pub fn new(radar_config: &RadarConfig, processing_metadata: &mut ProcessingMetaData) -> Self {
        Self {
            inner: acc_detector_presence_metadata_t {
                start_m: 0.0,
                end_m: 6.0,
                step_length_m: 0.0,
                num_points: 0,
                profile: acc_config_profile_t_ACC_CONFIG_PROFILE_5,
                sensor_config: radar_config.ptr(),
                processing_metadata: unsafe { processing_metadata.mut_ptr() },
            },
        }
    }
}

impl Default for PresenceMetadata {
    fn default() -> Self {
        Self {
            inner: acc_detector_presence_metadata_t {
                start_m: 0.0,
                end_m: 6.0,
                step_length_m: 0.0,
                num_points: 0,
                profile: acc_config_profile_t_ACC_CONFIG_PROFILE_5,
                sensor_config: core::ptr::null(),
                processing_metadata: core::ptr::null_mut(),
            },
        }
    }
}

impl From<acc_detector_presence_metadata_t> for PresenceMetadata {
    fn from(metadata: acc_detector_presence_metadata_t) -> Self {
        Self { inner: metadata }
    }
}
