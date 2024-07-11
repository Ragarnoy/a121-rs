use crate::config::profile::RadarProfile;
use crate::processing::ProcessingResult;
use a121_sys::{
    acc_config_profile_t_ACC_CONFIG_PROFILE_5, acc_detector_presence_metadata_t,
    acc_detector_presence_result_t,
};

/// Represents the results from a presence detection operation.
pub struct PresenceResult<'r> {
    pub presence_detected: bool,
    pub intra_presence_score: f32,
    pub inter_presence_score: f32,
    pub presence_distance: f32,
    pub depthwise_intra_presence_scores: &'r [f32],
    pub depthwise_inter_presence_scores: &'r [f32],
    pub depthwise_presence_scores_length: u32,
    pub processing_result: ProcessingResult,
}

impl PresenceResult<'_> {
    /// Updates the presence result with data from the detector.
    /// This function should be called after `acc_detector_presence_process`.
    pub fn update_from_detector_result(&mut self, result: &acc_detector_presence_result_t) {
        self.presence_detected = result.presence_detected;
        self.intra_presence_score = result.intra_presence_score;
        self.inter_presence_score = result.inter_presence_score;
        self.presence_distance = result.presence_distance;

        // Assuming buffer contains depthwise scores after calling acc_detector_presence_process
        // and the lengths are correctly populated.
        self.depthwise_presence_scores_length = result.depthwise_presence_scores_length;
        self.depthwise_intra_presence_scores = unsafe {
            core::slice::from_raw_parts(
                result.depthwise_intra_presence_scores as *const f32,
                result.depthwise_presence_scores_length as usize,
            )
        };
        self.depthwise_inter_presence_scores = unsafe {
            core::slice::from_raw_parts(
                result.depthwise_inter_presence_scores as *const f32,
                result.depthwise_presence_scores_length as usize,
            )
        };

        // Processing result is directly assigned for simplicity, might require processing based on use-case
        self.processing_result = ProcessingResult::from(result.processing_result);
    }

    pub(super) fn inner(&mut self) -> acc_detector_presence_result_t {
        let processing_result = self.processing_result.clone();
        acc_detector_presence_result_t {
            presence_detected: self.presence_detected,
            intra_presence_score: self.intra_presence_score,
            inter_presence_score: self.inter_presence_score,
            presence_distance: self.presence_distance,
            depthwise_intra_presence_scores: self.depthwise_intra_presence_scores.as_ptr()
                as *mut _,
            depthwise_inter_presence_scores: self.depthwise_inter_presence_scores.as_ptr()
                as *mut _,
            depthwise_presence_scores_length: self.depthwise_presence_scores_length,
            processing_result: processing_result.into(),
        }
    }
}

impl Default for PresenceResult<'_> {
    fn default() -> Self {
        Self {
            presence_detected: false,
            intra_presence_score: 0.0,
            inter_presence_score: 0.0,
            presence_distance: 0.0,
            depthwise_intra_presence_scores: &[],
            depthwise_inter_presence_scores: &[],
            depthwise_presence_scores_length: 0,
            processing_result: ProcessingResult::default(),
        }
    }
}

/// Enumerates possible errors that can occur during the processing of radar data for presence detection.
#[derive(Debug, Copy, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ProcessDataError {
    CalibrationNeeded,
    ProcessingFailed,
    Unavailable,
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

impl Default for PresenceMetadata {
    fn default() -> Self {
        Self {
            inner: acc_detector_presence_metadata_t {
                start_m: 0.0,
                end_m: 6.0,
                step_length_m: 0.0,
                num_points: 0,
                profile: acc_config_profile_t_ACC_CONFIG_PROFILE_5,
            },
        }
    }
}

impl From<acc_detector_presence_metadata_t> for PresenceMetadata {
    fn from(metadata: acc_detector_presence_metadata_t) -> Self {
        Self { inner: metadata }
    }
}
