use crate::rss_bindings::{acc_processing_metadata_t, ACC_MAX_NUM_SUBSWEEPS};

pub struct ProcessingMetaData {
    inner: acc_processing_metadata_t,
}

impl Default for ProcessingMetaData {
    fn default() -> Self {
        Self {
            inner: acc_processing_metadata_t {
                frame_data_length: 0,
                sweep_data_length: 0,
                subsweep_data_offset: [0; ACC_MAX_NUM_SUBSWEEPS as usize],
                subsweep_data_length: [0; ACC_MAX_NUM_SUBSWEEPS as usize],
                max_sweep_rate: 0.0,
                high_speed_mode: false,
            },
        }
    }
}

impl ProcessingMetaData {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a mutable reference to the metadata
    /// # Safety
    /// This function is unsafe because it returns a mutable reference to the metadata, which is a raw pointer
    pub unsafe fn mut_ptr(&mut self) -> *mut acc_processing_metadata_t {
        &mut self.inner
    }

    pub fn ptr(&self) -> *const acc_processing_metadata_t {
        &self.inner
    }

    pub fn frame_data_length(&self) -> usize {
        self.inner.frame_data_length as usize
    }

    pub fn sweep_data_length(&self) -> usize {
        self.inner.sweep_data_length as usize
    }

    pub fn subsweep_data_offset(&self, index: usize) -> usize {
        self.inner.subsweep_data_offset[index] as usize
    }

    pub fn subsweep_data_length(&self, index: usize) -> usize {
        self.inner.subsweep_data_length[index] as usize
    }

    pub fn max_sweep_rate(&self) -> f32 {
        self.inner.max_sweep_rate
    }
}
