use core::ffi::c_void;
use core::ptr::NonNull;

use metadata::ProcessingMetaData;

use crate::config::RadarConfig;
use crate::num::AccComplex;
use crate::radar::check_buffer;
use crate::sensor::error::SensorError;
use a121_sys::{
    acc_int16_complex_t, acc_processing_create, acc_processing_destroy, acc_processing_execute,
    acc_processing_result_t, acc_processing_t,
};

pub mod metadata;

/// Result from radar signal processing.
///
/// Contains status flags and sensor temperature. The frame itself lives in the processed
/// buffer: [`Radar::process_frame`](crate::radar::Radar::process_frame) returns it with this
/// result.
#[derive(Debug, Clone, Copy, Default)]
pub struct ProcessingResult {
    /// Indication of sensor data being saturated, can cause data corruption.
    /// Lower the receiver gain if this indication is set.
    pub data_saturated: bool,
    /// Indication of a delayed frame.
    /// The frame rate might need to be lowered if this indication is set.
    pub frame_delayed: bool,
    /// Indication of calibration needed.
    /// The sensor calibration needs to be redone if this indication is set.
    pub calibration_needed: bool,
    /// Temperature in sensor during measurement (in degree Celsius).
    /// Note that it has poor absolute accuracy and should only be used
    /// for relative temperature measurements.
    pub temperature: i16,
}

impl ProcessingResult {
    /// Creates a new ProcessingResult with default values.
    pub fn new() -> Self {
        Self::default()
    }
}

pub struct Processing {
    inner: NonNull<acc_processing_t>,
    metadata: ProcessingMetaData,
    /// Size of the buffer the SDK reads a frame from
    buffer_size: usize,
}

impl Processing {
    /// Creates the processing for `config`.
    ///
    /// # Errors
    ///
    /// Returns [`SensorError::ProcessingFailed`] when the SDK cannot create it.
    pub(crate) fn new(config: &RadarConfig) -> Result<Self, SensorError> {
        let buffer_size = config
            .config_buffer_size()
            .map_err(|_| SensorError::ProcessingFailed)? as usize;
        let mut metadata = ProcessingMetaData::new();
        let ptr = unsafe { acc_processing_create(config.ptr(), metadata.mut_ptr()) };
        let inner = NonNull::new(ptr).ok_or(SensorError::ProcessingFailed)?;
        Ok(Self {
            inner,
            metadata,
            buffer_size,
        })
    }

    /// Size of the buffer a frame is read into and processed from.
    pub fn buffer_size(&self) -> usize {
        self.buffer_size
    }

    pub fn metadata(&self) -> &ProcessingMetaData {
        &self.metadata
    }

    /// Processes the frame read into `buffer`, and returns the result with the frame, which the
    /// SDK leaves in `buffer`. The frame is only checked by [`frame`](Self::frame).
    ///
    /// # Errors
    ///
    /// Returns [`SensorError::BufferTooSmall`] when `buffer` is shorter than the configuration's
    /// buffer size, and [`SensorError::BufferMisaligned`] when it is not aligned for any built-in
    /// type.
    pub(crate) fn execute(
        &mut self,
        buffer: &mut [u8],
    ) -> Result<(ProcessingResult, *const acc_int16_complex_t), SensorError> {
        use core::mem::MaybeUninit;

        check_buffer(buffer, self.buffer_size)?;
        // Zeroed, so the frame is NULL if the SDK does not set it
        let mut ffi_result = MaybeUninit::<acc_processing_result_t>::zeroed();
        let ffi_result = unsafe {
            acc_processing_execute(
                self.inner.as_ptr(),
                buffer.as_mut_ptr() as *mut c_void,
                ffi_result.as_mut_ptr(),
            );
            ffi_result.assume_init()
        };
        Ok((ProcessingResult::from(ffi_result), ffi_result.frame))
    }

    /// Returns the frame at `frame`, as [`execute`](Self::execute) returned it for `buffer`.
    ///
    /// # Errors
    ///
    /// Returns [`SensorError::ProcessingFailed`] if the frame does not lie in `buffer`.
    pub(crate) fn frame<'buf>(
        &self,
        buffer: &'buf [u8],
        frame: *const acc_int16_complex_t,
    ) -> Result<&'buf [AccComplex], SensorError> {
        frame_in(buffer, frame, self.metadata.frame_data_length())
    }
}

/// Returns the `len` samples at `frame`, after checking that they lie in `buffer`.
fn frame_in(
    buffer: &[u8],
    frame: *const acc_int16_complex_t,
    len: usize,
) -> Result<&[AccComplex], SensorError> {
    if len == 0 {
        return Ok(&[]);
    }
    let size = len
        .checked_mul(core::mem::size_of::<AccComplex>())
        .ok_or(SensorError::ProcessingFailed)?;
    let offset = (frame as usize)
        .checked_sub(buffer.as_ptr() as usize)
        .ok_or(SensorError::ProcessingFailed)?;
    let in_buffer = offset
        .checked_add(size)
        .is_some_and(|end| end <= buffer.len());
    if frame.is_null() || !in_buffer || offset % core::mem::align_of::<AccComplex>() != 0 {
        return Err(SensorError::ProcessingFailed);
    }
    // SAFETY: the samples lie in `buffer` and are aligned, and `AccComplex` is a transparent
    // wrapper of the SDK's sample type, for which any bytes are valid
    Ok(unsafe { core::slice::from_raw_parts(buffer.as_ptr().add(offset).cast(), len) })
}

impl Drop for Processing {
    fn drop(&mut self) {
        // NonNull guarantees non-null pointer
        unsafe { acc_processing_destroy(self.inner.as_ptr()) }
    }
}

impl From<acc_processing_result_t> for ProcessingResult {
    fn from(result: acc_processing_result_t) -> Self {
        Self {
            data_saturated: result.data_saturated,
            frame_delayed: result.frame_delayed,
            calibration_needed: result.calibration_needed,
            temperature: result.temperature,
            // Note: result.frame pointer is intentionally not stored
            // as it points into the buffer and becomes invalid
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::radar::RadarBuffer;

    #[test]
    fn frame_must_lie_in_the_buffer() {
        let buffer = RadarBuffer::<64>::new();
        let base = buffer.as_ptr() as *const acc_int16_complex_t;
        let at = |offset: usize| base.wrapping_byte_add(offset);

        let frame = frame_in(&buffer, at(8), 4).unwrap();
        assert_eq!(frame.len(), 4);
        assert_eq!(frame.as_ptr() as usize, buffer.as_ptr() as usize + 8);
        // The last samples fit exactly
        assert_eq!(frame_in(&buffer, at(48), 4).unwrap().len(), 4);
        assert_eq!(frame_in(&buffer, core::ptr::null(), 0).unwrap().len(), 0);

        for (frame, len) in [
            (at(52), 4),
            (core::ptr::null(), 1),
            (base.wrapping_byte_sub(4), 1),
            (at(1), 1),
            (at(0), usize::MAX),
        ] {
            assert_eq!(
                frame_in(&buffer, frame, len).err(),
                Some(SensorError::ProcessingFailed)
            );
        }
    }
}
