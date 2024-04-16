pub mod config;
pub mod results;

use crate::detector::presence::config::PresenceConfig;
use crate::detector::presence::results::{PresenceMetadata, PresenceResult, ProcessDataError};
use crate::radar::{Radar, Ready};
use crate::sensor::calibration::CalibrationResult;
use crate::sensor::error::SensorError;
use a121_sys::*;
use core::ffi::c_void;
use embedded_hal::digital::OutputPin;
use embedded_hal_async::delay::DelayNs;
use embedded_hal_async::digital::Wait;

struct InnerPresenceDetector {
    presence_metadata: PresenceMetadata,
    inner: *mut acc_detector_presence_handle,
}

impl InnerPresenceDetector {
    fn new(config: &PresenceConfig) -> Self {
        let mut presence_metadata = PresenceMetadata::default();
        Self {
            inner: unsafe {
                acc_detector_presence_create(config.inner, presence_metadata.mut_ptr())
            },
            presence_metadata,
        }
    }

    fn inner(&self) -> *const acc_detector_presence_handle {
        self.inner
    }

    fn inner_mut(&mut self) -> *mut acc_detector_presence_handle {
        self.inner
    }
}

impl Drop for InnerPresenceDetector {
    fn drop(&mut self) {
        unsafe { acc_detector_presence_destroy(self.inner) }
    }
}

pub struct PresenceDetector<'radar, SINT, ENABLE, DLY>
where
    SINT: Wait,
    ENABLE: OutputPin,
    DLY: DelayNs,
{
    pub radar: &'radar mut Radar<Ready, SINT, ENABLE, DLY>,
    inner: InnerPresenceDetector,
    pub config: PresenceConfig,
}

impl<'radar, SINT, ENABLE, DLY> PresenceDetector<'radar, SINT, ENABLE, DLY>
where
    SINT: Wait,
    ENABLE: OutputPin,
    DLY: DelayNs,
{
    pub fn new(radar: &'radar mut Radar<Ready, SINT, ENABLE, DLY>) -> Self {
        let config = PresenceConfig::default();
        let inner = InnerPresenceDetector::new(&config);
        Self {
            radar,
            inner,
            config,
        }
    }

    pub fn with_config(
        radar: &'radar mut Radar<Ready, SINT, ENABLE, DLY>,
        config: PresenceConfig,
    ) -> Self {
        let inner = InnerPresenceDetector::new(&config);
        Self {
            radar,
            inner,
            config,
        }
    }

    pub async fn prepare_detector(
        &mut self,
        sensor_cal_result: &CalibrationResult,
        buffer: &mut [u8],
    ) -> Result<(), SensorError> {
        let buffer_size = self.get_buffer_size();

        if buffer.len() < buffer_size {
            return Err(SensorError::BufferTooSmall);
        }

        let prepare_success = unsafe {
            acc_detector_presence_prepare(
                self.inner.inner_mut(),
                self.config.inner,
                self.radar.inner_sensor(),
                sensor_cal_result.ptr(),
                buffer.as_mut_ptr() as *mut c_void,
                buffer.len() as u32,
            )
        };

        if prepare_success {
            Ok(())
        } else {
            Err(SensorError::PrepareFailed)
        }
    }

    pub fn get_buffer_size(&self) -> usize {
        let mut buffer_size: u32 = 0;
        unsafe {
            acc_detector_presence_get_buffer_size(self.inner.inner(), &mut buffer_size as *mut u32);
        }
        buffer_size as usize
    }

    pub async fn detect_presence(
        &mut self,
        buffer: &mut [u8],
    ) -> Result<PresenceResult, ProcessDataError> {
        let mut result = PresenceResult::default();
        let detection_success = unsafe {
            acc_detector_presence_process(
                self.inner.inner_mut(),
                buffer.as_mut_ptr() as *mut c_void,
                &mut result.inner() as *mut acc_detector_presence_result_t,
            )
        };

        if detection_success {
            Ok(result)
        } else {
            Err(ProcessDataError::ProcessingFailed)
        }
    }
}
