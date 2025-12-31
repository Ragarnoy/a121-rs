use crate::config::RadarConfig;
use crate::detector::distance::InnerRadarDistanceDetector;
use crate::processing::ProcessingResult;
use a121_sys::{
    acc_detector_cal_result_dynamic_t, acc_detector_distance_get_sizes,
    acc_detector_distance_result_t, ACC_DETECTOR_DISTANCE_RESULT_MAX_NUM_DISTANCES,
};

/// Represents a single detected distance and its strength.
#[derive(Debug, Default, Copy, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Distance {
    pub distance: f32,
    pub strength: f32,
}

/// Encapsulates the results of a distance detection operation.
///
/// This struct contains the distances detected by the radar, along with metadata
/// such as the temperature during the detection and whether calibration is needed.
pub struct DistanceResult<'a> {
    radar_config: &'a RadarConfig,
    distances: [Distance; ACC_DETECTOR_DISTANCE_RESULT_MAX_NUM_DISTANCES as usize],
    num_distances: u8,
    near_start_edge_status: bool,
    /// Processing result with status flags and temperature
    processing_result: ProcessingResult,
}

impl<'a> DistanceResult<'a> {
    /// Creates a new instance of `DistanceResult`.
    pub fn new(config: &'a RadarConfig) -> Self {
        Self {
            radar_config: config,
            distances: [Distance::default();
                ACC_DETECTOR_DISTANCE_RESULT_MAX_NUM_DISTANCES as usize],
            num_distances: 0,
            near_start_edge_status: false,
            processing_result: ProcessingResult::new(),
        }
    }

    /// Creates the FFI struct for passing to the SDK.
    ///
    /// Note: The `processing_result`, `processing_metadata`, and `sensor_config`
    /// pointers in the returned struct are set to null. The SDK will write
    /// pointers into the buffer for these fields, but we extract only the
    /// value fields in `update_from_detector_result`.
    pub(super) fn inner(&self) -> acc_detector_distance_result_t {
        acc_detector_distance_result_t {
            distances: [0.0; ACC_DETECTOR_DISTANCE_RESULT_MAX_NUM_DISTANCES as usize],
            strengths: [0.0; ACC_DETECTOR_DISTANCE_RESULT_MAX_NUM_DISTANCES as usize],
            num_distances: 0,
            near_start_edge_status: false,
            calibration_needed: false,
            temperature: 0,
            // These pointers will be written by the SDK (pointing into buffer)
            // We don't need to provide storage - just pass the config pointer
            processing_result: core::ptr::null_mut(),
            processing_metadata: core::ptr::null_mut(),
            sensor_config: self.radar_config.ptr(),
        }
    }

    /// Updates the result from the FFI struct returned by the SDK.
    pub(super) fn update_from_detector_result(&mut self, inner: acc_detector_distance_result_t) {
        self.num_distances = inner.num_distances;
        for i in 0..inner.num_distances as usize {
            self.distances[i].distance = inner.distances[i];
            self.distances[i].strength = inner.strengths[i];
        }
        self.near_start_edge_status = inner.near_start_edge_status;

        // Extract only value fields from processing_result (the frame pointer is dangling)
        if !inner.processing_result.is_null() {
            self.processing_result = ProcessingResult::from(unsafe { *inner.processing_result });
        } else {
            // If no processing result, use values from the outer struct
            self.processing_result = ProcessingResult {
                data_saturated: false,
                frame_delayed: false,
                calibration_needed: inner.calibration_needed,
                temperature: inner.temperature,
            };
        }
    }

    /// Returns the detected distances.
    pub fn distances(&self) -> &[Distance] {
        &self.distances[0..self.num_distances as usize]
    }

    /// Returns the near start edge status.
    pub fn near_start_edge_status(&self) -> bool {
        self.near_start_edge_status
    }

    /// Returns whether calibration is needed.
    pub fn calibration_needed(&self) -> bool {
        self.processing_result.calibration_needed
    }

    /// Returns the temperature during the detection.
    pub fn temperature(&self) -> i16 {
        self.processing_result.temperature
    }

    /// Returns the number of detected distances.
    pub fn num_distances(&self) -> u8 {
        self.num_distances
    }

    /// Returns the processing result containing status flags and temperature.
    pub fn processing_result(&self) -> &ProcessingResult {
        &self.processing_result
    }
}

/// Represents the dynamic part of the detector calibration result.
///
/// This struct encapsulates the dynamic calibration data that may need to be updated
/// based on temperature changes or other factors.
pub struct DynamicResult {
    pub(super) inner: acc_detector_cal_result_dynamic_t,
}

impl Default for DynamicResult {
    fn default() -> Self {
        Self {
            inner: acc_detector_cal_result_dynamic_t { data: [0; 2] },
        }
    }
}

/// Stores sizes related to distance detector operations.
///
/// This struct holds information about the required buffer sizes for distance detection
/// operations, including the static part of the detector calibration result.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub(super) struct DistanceSizes {
    pub buffer_size: usize,
    pub detector_cal_result_static_size: usize,
}

impl DistanceSizes {
    pub(super) fn new(handle: &InnerRadarDistanceDetector) -> Self {
        use core::mem::MaybeUninit;

        let mut buffer_size = MaybeUninit::<u32>::uninit();
        let mut detector_cal_result_static_size = MaybeUninit::<u32>::uninit();

        unsafe {
            acc_detector_distance_get_sizes(
                handle.inner(),
                buffer_size.as_mut_ptr(),
                detector_cal_result_static_size.as_mut_ptr(),
            );
            Self {
                buffer_size: buffer_size.assume_init() as usize,
                detector_cal_result_static_size: detector_cal_result_static_size.assume_init()
                    as usize,
            }
        }
    }
}
