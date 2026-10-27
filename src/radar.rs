pub mod version;

use a121_sys::{acc_sensor_connected, acc_sensor_id_t, acc_sensor_t};
use embedded_hal::digital::OutputPin;
use embedded_hal::spi::{ErrorKind as SpiErrorKind, SpiDevice};
use embedded_hal_async::delay::DelayNs;
use embedded_hal_async::digital::Wait;

use alloc::vec;
use alloc::vec::Vec;
use core::ops::{Deref, DerefMut};

use crate::config::RadarConfig;
use crate::hal::{AccHalImpl, SDK_ALIGN};
use crate::processing::{Processing, ProcessingResult};
use crate::sensor::calibration::CalibrationResult;
use crate::sensor::error::SensorError;
use crate::sensor::Sensor;

/// Default scratch buffer size for radar operations (calibration and sensor preparation).
///
/// The SDK needs at least [`RadarConfig::config_buffer_size`] bytes, which depends on the
/// configuration. [`Radar::calibrate`] and [`Radar::prepare_sensor`] check this at runtime and
/// return [`SensorError::BufferTooSmall`] when the scratch buffer is too small; use a larger
/// `SCRATCH_SIZE` in that case.
pub const DEFAULT_SCRATCH_BUFFER_SIZE: usize = 5_560;

/// Byte buffer aligned for any built-in type, as the SDK requires for the buffers it is given.
///
/// Use it for the buffers passed to [`Radar::measure`] and to the detectors.
#[derive(Clone)]
#[repr(C, align(16))]
pub struct RadarBuffer<const N: usize>([u8; N]);

impl<const N: usize> RadarBuffer<N> {
    /// Creates a zeroed buffer.
    pub const fn new() -> Self {
        Self([0; N])
    }
}

impl<const N: usize> Default for RadarBuffer<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> Deref for RadarBuffer<N> {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.0
    }
}

impl<const N: usize> DerefMut for RadarBuffer<N> {
    fn deref_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

/// Heap-allocated byte buffer aligned like [`RadarBuffer`], for sizes known only at runtime such as
/// the detectors' buffer sizes.
#[derive(Clone)]
pub struct AlignedBuffer {
    chunks: Vec<RadarBuffer<16>>,
    len: usize,
}

impl AlignedBuffer {
    /// Allocates a zeroed buffer of `len` bytes.
    pub fn new(len: usize) -> Self {
        Self {
            chunks: vec![RadarBuffer::new(); len.div_ceil(16)],
            len,
        }
    }
}

impl Deref for AlignedBuffer {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        // SAFETY: the chunks are contiguous plain bytes, at least `len` of them
        unsafe { core::slice::from_raw_parts(self.chunks.as_ptr().cast::<u8>(), self.len) }
    }
}

impl DerefMut for AlignedBuffer {
    fn deref_mut(&mut self) -> &mut [u8] {
        // SAFETY: as in `deref`, and `&mut self` gives exclusive access
        unsafe { core::slice::from_raw_parts_mut(self.chunks.as_mut_ptr().cast::<u8>(), self.len) }
    }
}

// RadarBuffer must satisfy the alignment the SDK asks for on every target
const _: () = assert!(core::mem::align_of::<RadarBuffer<0>>() >= SDK_ALIGN);

/// Checks that a buffer handed to the SDK is long enough and suitably aligned.
fn check_buffer(len: usize, addr: usize, min_len: usize) -> Result<(), SensorError> {
    if len < min_len {
        Err(SensorError::BufferTooSmall)
    } else if !addr.is_multiple_of(SDK_ALIGN) {
        Err(SensorError::BufferMisaligned)
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum RadarState {
    /// Powered on, but not prepared for the current configuration.
    Enabled,
    /// Prepared and ready to measure.
    Ready,
    /// Hibernating with the enable pin low.
    Hibernating,
}

/// Radar sensor controller with configurable scratch buffer size.
///
/// The `SCRATCH_SIZE` const generic sets the size of the internal scratch buffer used for
/// calibration and preparation. [`DEFAULT_SCRATCH_BUFFER_SIZE`] fits most configurations.
///
/// # Example
///
/// ```ignore
/// // Use default buffer size
/// let radar: Radar<_, _, _> = Radar::new(id, spi, interrupt, enable, delay).await?;
///
/// // Use custom buffer size for larger configurations
/// let radar: Radar<_, _, _, 8192> = Radar::new(id, spi, interrupt, enable, delay).await?;
/// ```
pub struct Radar<SINT, ENABLE, DLY, const SCRATCH_SIZE: usize = DEFAULT_SCRATCH_BUFFER_SIZE>
where
    SINT: Wait,
    ENABLE: OutputPin,
    DLY: DelayNs,
{
    id: u32,
    pub(crate) config: RadarConfig,
    sensor: Sensor<ENABLE, DLY>,
    processing: Option<Processing>,
    pub(crate) interrupt: SINT,
    _hal: AccHalImpl,
    state: RadarState,
    /// Set when the configuration changed while hibernating, so waking up needs a new prepare
    needs_prepare: bool,
    /// Buffer size of the configuration prepared by [`Radar::prepare_sensor`]
    buffer_size: Option<usize>,
    scratch: RadarBuffer<SCRATCH_SIZE>,
}

impl<SINT, ENABLE, DLY, const SCRATCH_SIZE: usize> Radar<SINT, ENABLE, DLY, SCRATCH_SIZE>
where
    SINT: Wait,
    ENABLE: OutputPin,
    DLY: DelayNs,
{
    pub async fn new<SPI>(
        id: u32,
        spi: &'static mut SPI,
        interrupt: SINT,
        mut enable_pin: ENABLE,
        mut delay: DLY,
    ) -> Result<Radar<SINT, ENABLE, DLY, SCRATCH_SIZE>, SensorError>
    where
        SPI: SpiDevice<u8, Error = SpiErrorKind> + Send + 'static,
    {
        // Extended power cycle: ensure sensor is off first for longer
        enable_pin
            .set_low()
            .map_err(|_| SensorError::EnablePinFailed)?;
        delay.delay_ms(50).await; // Longer off time
        enable_pin
            .set_high()
            .map_err(|_| SensorError::EnablePinFailed)?;
        delay.delay_ms(50).await; // Longer startup time

        // Create and register HAL before creating sensor
        let hal = AccHalImpl::new(spi);
        hal.register()?;

        // Additional delay after HAL registration for sensor to stabilize
        delay.delay_ms(10).await;

        // Create configuration first
        let config = RadarConfig::new()?;

        // Create sensor after HAL is registered and sensor is stable
        let sensor = Sensor::new(id, enable_pin, delay).ok_or(SensorError::InitFailed)?;

        Ok(Radar {
            id,
            config,
            interrupt,
            sensor,
            processing: None,
            _hal: hal,
            state: RadarState::Enabled,
            needs_prepare: false,
            buffer_size: None,
            scratch: RadarBuffer::new(),
        })
    }

    /// Returns the scratch buffer size for this radar instance.
    pub const fn scratch_buffer_size(&self) -> usize {
        SCRATCH_SIZE
    }

    /// The radar configuration.
    pub fn config(&self) -> &RadarConfig {
        &self.config
    }

    /// The radar configuration, for changing it.
    ///
    /// The sensor keeps measuring with the configuration it was prepared with, so the radar
    /// leaves the ready state and [`Radar::prepare_sensor`] must be called again.
    pub fn config_mut(&mut self) -> &mut RadarConfig {
        match self.state {
            RadarState::Ready => self.state = RadarState::Enabled,
            RadarState::Hibernating => self.needs_prepare = true,
            RadarState::Enabled => {}
        }
        self.processing = None;
        self.buffer_size = None;
        &mut self.config
    }

    /// The processing for the prepared configuration, once [`Radar::prepare_sensor`] succeeded.
    pub fn processing(&self) -> Option<&Processing> {
        self.processing.as_ref()
    }

    /// Processes a frame read by [`Radar::measure`].
    ///
    /// # Errors
    ///
    /// Returns [`SensorError::NotReady`] when the sensor was not prepared with
    /// [`Radar::prepare_sensor`], and [`SensorError::BufferTooSmall`] when `data` is shorter than
    /// the configuration's buffer size.
    pub fn process(&mut self, data: &mut [u8]) -> Result<ProcessingResult, SensorError> {
        self.processing
            .as_mut()
            .ok_or(SensorError::NotReady)?
            .execute(data)
    }

    /// Prepares the sensor for the current configuration and creates the matching processing.
    ///
    /// # Errors
    ///
    /// Returns [`SensorError::BufferTooSmall`] when `SCRATCH_SIZE` is smaller than the
    /// configuration's buffer size.
    pub fn prepare_sensor(
        &mut self,
        calibration_result: &mut CalibrationResult,
    ) -> Result<(), SensorError> {
        if self.state != RadarState::Enabled {
            return Err(SensorError::NotReady);
        }

        let buffer_size = self.required_buffer_size(SensorError::PrepareFailed)?;
        self.processing = None;
        self.buffer_size = None;
        self.sensor
            .prepare(&self.config, calibration_result, &mut self.scratch)?;
        self.processing = Some(Processing::new(&self.config)?);
        self.buffer_size = Some(buffer_size);
        self.state = RadarState::Ready;
        Ok(())
    }

    /// Buffer size the SDK needs for the current configuration, checked against the scratch size.
    fn required_buffer_size(&self, error: SensorError) -> Result<usize, SensorError> {
        let size = self.config.config_buffer_size().map_err(|_| error)? as usize;
        if size > SCRATCH_SIZE {
            return Err(SensorError::BufferTooSmall);
        }
        Ok(size)
    }

    /// Wakes the sensor from hibernation: enables it, then restores its state.
    ///
    /// The radar is ready again, unless the configuration was changed while hibernating, in
    /// which case it must be prepared again.
    pub async fn hibernate_off(&mut self) -> Result<(), SensorError> {
        if self.state != RadarState::Hibernating {
            return Err(SensorError::NotReady);
        }

        self.sensor.enable_sensor().await?;
        self.sensor.hibernate_off()?;
        self.state = if self.needs_prepare {
            RadarState::Enabled
        } else {
            RadarState::Ready
        };
        self.needs_prepare = false;
        Ok(())
    }

    /// Measures and reads a frame into `data`.
    ///
    /// # Errors
    ///
    /// `data` must be aligned for any built-in type (see [`RadarBuffer`]), or this returns
    /// [`SensorError::BufferMisaligned`]. After [`Radar::prepare_sensor`], it must also hold the
    /// configuration's buffer size, or this returns [`SensorError::BufferTooSmall`].
    pub async fn measure(&mut self, data: &mut [u8]) -> Result<(), SensorError> {
        if self.state != RadarState::Ready {
            return Err(SensorError::NotReady);
        }
        check_buffer(
            data.len(),
            data.as_ptr() as usize,
            self.buffer_size.unwrap_or(0),
        )?;

        self.sensor.measure(&mut self.interrupt).await?;
        self.sensor.read(data)?;
        Ok(())
    }

    /// Puts the sensor in hibernation: saves its state, then disables it with the enable pin.
    pub async fn hibernate_on(&mut self) -> Result<(), SensorError> {
        if self.state != RadarState::Ready {
            return Err(SensorError::NotReady);
        }

        self.sensor.hibernate_on()?;
        self.sensor.disable_sensor().await?;
        self.state = RadarState::Hibernating;
        Ok(())
    }

    /// Get the current state of the radar
    pub fn state(&self) -> RadarState {
        self.state
    }

    /// Marks the sensor as prepared after a detector prepared it through the SDK
    ///
    /// Detectors size and process their own buffers, so the radar's processing is dropped.
    #[cfg(any(feature = "distance", feature = "presence"))]
    pub(crate) fn mark_prepared(&mut self) {
        self.processing = None;
        self.buffer_size = None;
        self.needs_prepare = false;
        self.state = RadarState::Ready;
    }

    /// Get the radar sensor ID
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Calibrate the sensor - available in any state
    ///
    /// Calibration power-cycles the sensor, so the radar is back in the enabled state
    /// afterwards and must be prepared again before measuring.
    ///
    /// # Errors
    ///
    /// Returns [`SensorError::BufferTooSmall`] when `SCRATCH_SIZE` is smaller than the buffer
    /// size of the radar configuration. Detectors use their own configuration, which this check
    /// does not cover.
    pub async fn calibrate(&mut self) -> Result<CalibrationResult, SensorError> {
        self.required_buffer_size(SensorError::CalibrationFailed)?;
        self.processing = None;
        self.buffer_size = None;
        self.needs_prepare = false;
        self.state = RadarState::Enabled;
        self.sensor
            .calibrate(&mut self.interrupt, &mut self.scratch)
            .await
    }

    /// Reset the sensor - available in any state
    pub async fn reset_sensor(&mut self) -> Result<(), SensorError> {
        self.processing = None;
        self.buffer_size = None;
        self.needs_prepare = false;
        // The sensor is unprepared after a power cycle, even if re-enabling fails
        self.state = RadarState::Enabled;
        self.sensor.reset_sensor().await
    }

    /// Check if sensor is connected - available in any state
    pub fn is_connected(&self) -> bool {
        unsafe { acc_sensor_connected(self.id as acc_sensor_id_t) }
    }

    /// Check sensor status - available in any state
    pub fn check_status(&self) {
        self.sensor.check_status();
    }

    /// Get raw sensor pointer - available in any state
    /// # Safety
    /// This function is unsafe because it returns a raw pointer to the sensor.
    pub unsafe fn inner_sensor(&self) -> *mut acc_sensor_t {
        debug_assert!(!self.sensor.inner().is_null(), "Sensor pointer is null");
        self.sensor.inner()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn radar_buffer_is_aligned_for_the_sdk() {
        assert_eq!(core::mem::align_of::<RadarBuffer<5560>>(), 16);
        let buffer = RadarBuffer::<5560>::new();
        assert_eq!(buffer.len(), 5560);
        assert!((buffer.as_ptr() as usize).is_multiple_of(SDK_ALIGN));
    }

    #[test]
    fn aligned_buffer_covers_its_length() {
        for len in [0, 1, 16, 17, 5560] {
            let mut buffer = AlignedBuffer::new(len);
            assert_eq!(buffer.len(), len);
            assert!((buffer.as_ptr() as usize).is_multiple_of(SDK_ALIGN));
            buffer.fill(0xAB);
            assert!(buffer.iter().all(|&b| b == 0xAB));
        }
    }

    #[test]
    fn buffer_checks() {
        assert_eq!(check_buffer(10, 0, 11), Err(SensorError::BufferTooSmall));
        assert_eq!(check_buffer(10, 4, 10), Err(SensorError::BufferMisaligned));
        assert_eq!(check_buffer(10, SDK_ALIGN * 3, 10), Ok(()));
        assert_eq!(check_buffer(10, SDK_ALIGN, 0), Ok(()));
    }
}
