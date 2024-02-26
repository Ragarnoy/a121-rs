const RADAR_DATA_SIZE: usize = 1024;

/// Radar data from the sensor.
#[derive(Debug, defmt::Format)]
pub struct RadarData {
    pub data: [u8; RADAR_DATA_SIZE],
}

impl RadarData {
    /// Creates a new radar data instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a mutable pointer to the radar data.
    /// # Safety
    /// The caller must ensure that the radar data is not modified while the pointer is in use.
    pub unsafe fn mut_ptr(&mut self) -> *mut u8 {
        self.data.as_mut_ptr()
    }

    /// Returns an immutable pointer to the radar data.
    pub fn ptr(&self) -> *const u8 {
        self.data.as_ptr()
    }
}

impl Default for RadarData {
    fn default() -> Self {
        Self {
            data: [0; RADAR_DATA_SIZE],
        }
    }
}
