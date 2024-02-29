use crate::config::RadarConfig;
use crate::detector::distance::InnerRadarDistanceDetector;
use crate::processing::metadata::ProcessingMetaData;
use crate::processing::ProcessingResult;
use crate::rss_bindings::{
    acc_detector_cal_result_dynamic_t, acc_detector_distance_get_sizes,
    acc_detector_distance_result_t, ACC_DETECTOR_CAL_RESULT_DYNAMIC_DATA_SIZE,
    ACC_DETECTOR_DISTANCE_RESULT_MAX_NUM_DISTANCES,
};

#[derive(Debug, Copy, Clone, defmt::Format)]
pub enum ProcessDataError {
    CalibrationNeeded,
    ProcessingFailed,
    Unavailable,
}

#[derive(Debug, Default, Copy, Clone, defmt::Format)]
pub struct Distance {
    pub distance: f32,
    pub strength: f32,
}

pub struct DistanceResult<'a> {
    result: ProcessingResult,
    metadata: ProcessingMetaData,
    radar_config: &'a RadarConfig,
    distances: [Distance; ACC_DETECTOR_DISTANCE_RESULT_MAX_NUM_DISTANCES as usize],
    num_distances: u8,
    near_start_edge_status: bool,
    calibration_needed: bool,
    temperature: i16,
}

impl<'a> DistanceResult<'a> {
    pub fn new(config: &'a RadarConfig) -> Self {
        let proc_result = ProcessingResult::new();
        let proc_metadata = ProcessingMetaData::new();
        Self {
            result: proc_result,
            metadata: proc_metadata,
            radar_config: config,
            distances: [Distance::default();
                ACC_DETECTOR_DISTANCE_RESULT_MAX_NUM_DISTANCES as usize],
            num_distances: 0,
            near_start_edge_status: false,
            calibration_needed: false,
            temperature: 0,
        }
    }

    pub(super) fn inner(&mut self) -> acc_detector_distance_result_t {
        acc_detector_distance_result_t {
            distances: [0.0; ACC_DETECTOR_DISTANCE_RESULT_MAX_NUM_DISTANCES as usize],
            strengths: [0.0; ACC_DETECTOR_DISTANCE_RESULT_MAX_NUM_DISTANCES as usize],
            num_distances: 0,
            near_start_edge_status: false,
            calibration_needed: false,
            temperature: 0,
            processing_result: unsafe { self.result.mut_ptr() },
            processing_metadata: unsafe { self.metadata.mut_ptr() },
            sensor_config: self.radar_config.ptr(),
        }
    }

    pub(super) fn update_from_detector_result(&mut self, inner: acc_detector_distance_result_t) {
        self.num_distances = inner.num_distances;
        for i in 0..inner.num_distances as usize {
            self.distances[i].distance = inner.distances[i];
            self.distances[i].strength = inner.strengths[i];
        }
        self.near_start_edge_status = inner.near_start_edge_status;
        self.calibration_needed = inner.calibration_needed;
        self.temperature = inner.temperature;
    }

    pub fn distances(&self) -> &[Distance] {
        &self.distances[0..self.num_distances as usize]
    }

    pub fn near_start_edge_status(&self) -> bool {
        self.near_start_edge_status
    }

    pub fn calibration_needed(&self) -> bool {
        self.calibration_needed
    }

    pub fn temperature(&self) -> i16 {
        self.temperature
    }

    pub fn num_distances(&self) -> u8 {
        self.num_distances
    }

    pub fn processing_result(&self) -> &ProcessingResult {
        &self.result
    }

    pub fn processing_metadata(&self) -> &ProcessingMetaData {
        &self.metadata
    }
}

pub struct DynamicResult {
    pub(super) inner: acc_detector_cal_result_dynamic_t,
}

impl Default for DynamicResult {
    fn default() -> Self {
        Self {
            inner: acc_detector_cal_result_dynamic_t {
                data: [0; ACC_DETECTOR_CAL_RESULT_DYNAMIC_DATA_SIZE as usize],
            },
        }
    }
}

#[derive(Debug, defmt::Format)]
pub(super) struct DistanceSizes {
    pub buffer_size: usize,
    pub detector_cal_result_static_size: usize,
}

impl DistanceSizes {
    pub fn new(handle: &InnerRadarDistanceDetector) -> Self {
        let mut buffer_size: u32 = 0;
        let mut detector_cal_result_static_size: u32 = 0;

        unsafe {
            acc_detector_distance_get_sizes(
                handle.inner(),
                &mut buffer_size as *mut u32,
                &mut detector_cal_result_static_size as *mut u32,
            );
        }
        Self {
            buffer_size: buffer_size as usize,
            detector_cal_result_static_size: detector_cal_result_static_size as usize,
        }
    }
}
