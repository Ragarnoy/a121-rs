use core::ffi::c_void;

use metadata::ProcessingMetaData;

use crate::config::RadarConfig;
use crate::num::AccComplex;
use crate::rss_bindings::{
    acc_processing_create, acc_processing_destroy, acc_processing_execute, acc_processing_result_t,
    acc_processing_t,
};
use crate::sensor::data::RadarData;

mod metadata;

pub struct ProcessingResult {
    inner: acc_processing_result_t,
    pub frame: AccComplex,
}

impl ProcessingResult {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a mutable reference to the frame data
    /// # Safety
    /// This function is unsafe because it returns a mutable reference to the frame data, which is a raw pointer
    pub unsafe fn mut_ptr(&mut self) -> *mut acc_processing_result_t {
        &mut self.inner
    }

    pub fn ptr(&self) -> *const acc_processing_result_t {
        &self.inner
    }
}

impl Default for ProcessingResult {
    fn default() -> Self {
        let mut frame: AccComplex = AccComplex::new();
        let inner = acc_processing_result_t {
            data_saturated: false,
            frame_delayed: false,
            calibration_needed: false,
            temperature: 0,
            frame: unsafe { frame.mut_ptr() },
        };
        Self { inner, frame }
    }
}

pub struct Processing {
    inner: *mut acc_processing_t,
    metadata: ProcessingMetaData,
}

impl Processing {
    pub fn new(config: &RadarConfig) -> Self {
        let mut metadata = ProcessingMetaData::new();
        let inner = unsafe { acc_processing_create(config.ptr(), metadata.mut_ptr()) };
        Self { inner, metadata }
    }

    pub fn metadata(&self) -> &ProcessingMetaData {
        &self.metadata
    }

    pub fn execute(&mut self, buffer: &mut RadarData) -> ProcessingResult {
        let mut result = ProcessingResult::new();
        unsafe {
            acc_processing_execute(
                self.inner,
                buffer.mut_ptr() as *mut c_void,
                result.mut_ptr(),
            );
        }
        result
    }
}

impl Drop for Processing {
    fn drop(&mut self) {
        unsafe {
            acc_processing_destroy(self.inner);
        }
    }
}
